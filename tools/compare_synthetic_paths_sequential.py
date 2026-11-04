#!/usr/bin/env python3
"""Compare original saved path bodies with a Rust server started after original exit.

This is body-only diagnostic evidence, not an HTTP/session parity or timing gate.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import socket
import subprocess
import sys
import threading
import time
import urllib.parse
import urllib.request

import build_synthetic_corpus as synthetic
import capture_opengrok_baseline as baseline
from indexer_evidence import verify_candidate_coverage, verify_path_coverage
from oracle_http import opener
from prepare_oracle_copy import tree_hash
from run_synthetic_index_probe import ensure_group_exit, wait_group_exit
from search_benchmark_gate import comparison, finite_float, load_response, no_duplicate_keys, strict_equal
from sequential_preflight import case_queries, equivalent, observe
from sequential_http_measure import load_eligibility

ROOT=Path(__file__).resolve().parents[1]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ('corpus','probe','original','output'):parser.add_argument('--'+name,type=Path,required=True)
    parser.add_argument('--min-free-bytes',type=int,default=synthetic.RESERVE)
    parser.add_argument('--sequential-preflight',action='store_true')
    parser.add_argument('--measurement-proof',type=Path);parser.add_argument('--oracle-measurement',type=Path)
    parser.add_argument('--warmup',type=int,default=20);parser.add_argument('--samples',type=int,default=50)
    args=parser.parse_args()
    if args.output.exists() or args.output.is_symlink():parser.error('output must not exist')
    if args.min_free_bytes<synthetic.RESERVE:parser.error('minimum free space cannot be below 4.5 GiB')
    if args.warmup<1 or args.samples<5 or args.samples%5:parser.error('warmup must be positive and samples a multiple of five')
    if bool(args.measurement_proof)!=bool(args.oracle_measurement):parser.error('measurement requires preflight proof and stopped oracle measurement')
    prior=args.original.resolve(strict=True);proof=json.loads((prior/'coverage.json').read_text())
    groups=proof.get('process_groups_stopped')
    if proof.get('status')!='MATCHED_ALL_SYNTHETIC_PATHS' or not groups or not all(groups.values()):parser.error('original process groups are not proven stopped')
    if not all(wait_group_exit(int(group),timeout=0) for group in groups):parser.error('original process group remains alive or was reused')
    corpus=args.corpus.resolve(strict=True);probe=args.probe.resolve(strict=True);out=args.output.parent.resolve(strict=True)/args.output.name
    if out==ROOT or ROOT in out.parents or out in (corpus,probe,prior) or any(p in out.parents for p in (corpus,probe,prior)):parser.error('output must be outside all inputs and repository')
    manifest=json.loads((corpus/'manifest.json').read_text())
    if type(manifest.get('source_bytes')) is not int or manifest['source_bytes']>32*1024**2:parser.error('pilot capped at 32 MiB')
    synthetic.verify(argparse.Namespace(corpus=corpus));receipt=json.loads((probe/'report.json').read_text())
    if (probe/'synthetic-manifest.json').read_bytes()!=(corpus/'manifest.json').read_bytes():parser.error('manifest mismatch')
    if not receipt['original_process'].get('group_stopped') or not receipt['candidate_process'].get('group_stopped'):parser.error('index process group not confirmed stopped')
    index=probe/'candidate-index.json';binary=ROOT/'experiments/search-spike/target/release/rustgrok-search-spike'
    if baseline.digest(index)!=receipt['candidate_index_sha256'] or baseline.digest(binary)!=receipt['candidate_binary_sha256']:parser.error('candidate index or binary hash mismatch')
    if tree_hash(probe/'oracle-index')!=proof['original_index_tree_before'] or proof['original_index_tree_before']!=proof['original_index_tree_after']:parser.error('original index tree differs from proof')
    expected={'/'+x['path']:(corpus/'source'/x['path']).read_text() for x in manifest['files']}
    if not verify_candidate_coverage(expected,json.loads(index.read_bytes(),object_pairs_hook=no_duplicate_keys))['complete']:parser.error('candidate index coverage mismatch')
    bodies={};original_raw={}
    for name in sorted({Path(x['path']).name for x in manifest['files']}):
        raw,body=load_response(prior/('response-'+name+'.json'));bodies[name]=body;original_raw[name]=raw
    converted={'repositories':[{'path':str(Path(x['path']).parent),'file':Path(x['path']).name} for x in manifest['files']]}
    if not verify_path_coverage(converted,bodies)['complete']:parser.error('original paths differ from corpus')
    measurement_proof=None
    if args.measurement_proof:
        measurement_proof=args.measurement_proof.resolve(strict=True);original_measurement=args.oracle_measurement.resolve(strict=True)
        measured=json.loads((original_measurement/'coverage.json').read_text());stopped=measured.get('process_groups_stopped',{})
        if not stopped or not all(stopped.values()) or not all(wait_group_exit(int(g),timeout=0) for g in stopped):parser.error('original measurement process groups are not confirmed stopped')
        if measured['original_index_tree_before']!=proof['original_index_tree_before'] or measured['original_index_tree_after']!=proof['original_index_tree_after']:parser.error('original measurement index differs')
        proof_hash=hashlib.sha256((measurement_proof/'sequential-preflight.json').read_bytes()).hexdigest()
        for name in ('single-hit','no-hit'):
            load_eligibility(measurement_proof,name)
            r=json.loads((original_measurement/('measure-oracle-'+name)/'report.json').read_text())
            if r['status']!='COMPLETED_RESTRICTED_ENDPOINT' or r['role']!='oracle' or r['proof_sha256']!=proof_hash or r['timed_samples']!=2*args.samples:parser.error('original measurement receipt differs')
    original_preflight={}
    if args.sequential_preflight:
        original_preflight=json.loads((prior/'original-preflight.json').read_text())
        project=sorted({x['path'].split('/')[0] for x in manifest['files']})[0]
        for name,query in case_queries(project).items():
            item=original_preflight[name]
            if item['query']!=query or not equivalent(item['observation'],item['observation']):parser.error('invalid original preflight receipt')
            for part in ('first','followup'):
                raw,body=load_response(prior/('preflight-'+name+'-oracle-'+part+'.body'))
                observed=item['observation'][part]['body']
                if hashlib.sha256(raw).hexdigest()!=item['body_sha256'][part] or not strict_equal(body,observed) or list(body['results'])!=list(observed['results']):parser.error('original preflight body hash or parsed data mismatch')
    synthetic.check_space(out.parent,args.min_free_bytes,0);out.mkdir()
    with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
    stop=threading.Event();breached=threading.Event();disk_samples=[];resources=[];processes=[];start=time.monotonic();cases=[];preflight={};measurements=[]
    with (out/'candidate.stdout-stderr.txt').open('wb') as log:
        proc=subprocess.Popen([str(binary),'serve','--index',str(index),'--listen',f'127.0.0.1:{port}'],stdout=log,stderr=subprocess.STDOUT,start_new_session=True);processes.append(proc)
        def watch():
            while not stop.is_set():
                free=shutil.disk_usage(out).free;disk_samples.append({'elapsed_seconds':round(time.monotonic()-start,3),'free_bytes':free})
                if free<args.min_free_bytes:
                    breached.set()
                    for process in list(processes):
                        if process.poll() is None:
                            try:os.killpg(process.pid,signal.SIGTERM)
                            except ProcessLookupError:pass
                    return
                stop.wait(.25)
        watcher=threading.Thread(target=watch,daemon=True);watcher.start()
        def monitor():
            while not stop.is_set():
                try:
                    ps=subprocess.run(['ps','-axo','pgid=,pid=,ppid=,rss=,%cpu=,time='],capture_output=True,text=True,timeout=.25)
                    groups={str(p.pid) for p in list(processes)}
                    rows=[line for line in ps.stdout.splitlines() if line.split() and line.split()[0] in groups]
                except subprocess.TimeoutExpired:rows=[]
                resources.append({'elapsed_seconds':round(time.monotonic()-start,3),'process_group_ps':rows})
                stop.wait(.25)
        monitor_thread=threading.Thread(target=monitor,daemon=True);monitor_thread.start()
        try:
            for _ in range(100):
                if proc.poll() is not None or breached.is_set():raise RuntimeError('candidate server exited or disk reserve breached')
                with socket.socket() as sock:
                    if sock.connect_ex(('127.0.0.1',port))==0:break
                time.sleep(.05)
            else:raise RuntimeError('candidate readiness timeout')
            projects=sorted({x['path'].split('/')[0] for x in manifest['files']});client=opener()
            for name in sorted(bodies):
                if breached.is_set():raise RuntimeError('disk reserve breached')
                params=[('path',name)]+[('projects',project) for project in projects]+[('start','0'),('maxresults','100')]
                query='/source/api/v1/search?'+urllib.parse.urlencode(params)
                with client.open(f'http://127.0.0.1:{port}'+query,timeout=20) as response:
                    raw=response.read();status=response.status
                (out/('oracle-'+name+'.json')).write_bytes(original_raw[name]);(out/('candidate-'+name+'.json')).write_bytes(raw)
                value=json.loads(raw,object_pairs_hook=no_duplicate_keys,parse_float=finite_float,parse_constant=lambda _:(_ for _ in ()).throw(ValueError('non-finite')))
                cases.append({'query':query,'oracle_body':'oracle-'+name+'.json','candidate_body':'candidate-'+name+'.json',
                              'candidate_status':status,'oracle_count':bodies[name]['resultCount'],'candidate_count':value['resultCount'],
                              'body_matches_except_time':status==200 and comparison(bodies[name],value),
                              'oracle_order':list(bodies[name]['results']),'candidate_order':list(value['results'])})
            if args.sequential_preflight:
                for name,item in original_preflight.items():
                    if breached.is_set():raise RuntimeError('disk reserve breached')
                    candidate_observation=observe(f'http://127.0.0.1:{port}'+item['query'],out,'preflight-'+name+'-candidate')
                    for part in ('first','followup'):
                        (out/('preflight-'+name+'-oracle-'+part+'.body')).write_bytes((prior/('preflight-'+name+'-oracle-'+part+'.body')).read_bytes())
                    preflight[name]={'query':item['query'],'status':'MATCHED_PREFLIGHT_NOT_TIMED' if equivalent(item['observation'],candidate_observation) else 'BLOCKED_PREFLIGHT',
                                     'oracle':item['observation'],'candidate':candidate_observation,
                                     'candidate_body_sha256':{part:hashlib.sha256((out/('preflight-'+name+'-candidate-'+part+'.body')).read_bytes()).hexdigest() for part in ('first','followup')},
                                     'oracle_body_sha256':item['body_sha256']}
                (out/'sequential-preflight.json').write_text(json.dumps(preflight,indent=2)+'\n')
            if measurement_proof is not None:
                (out/'iostat-before.txt').write_text(subprocess.check_output(['/usr/sbin/iostat','-Id','-c','1'],text=True))
                for name in ('single-hit','no-hit'):
                    if breached.is_set():raise RuntimeError('disk reserve breached')
                    command=[sys.executable,str(ROOT/'tools/sequential_http_measure.py'),'--endpoint',f'http://127.0.0.1:{port}',
                             '--proof',str(measurement_proof),'--case',name,'--role','candidate','--output',str(out/('measure-candidate-'+name)),
                             '--warmup',str(args.warmup),'--samples',str(args.samples),'--min-free-bytes',str(args.min_free_bytes)]
                    with (out/('measure-candidate-'+name+'.stdout.txt')).open('wb') as sout,(out/('measure-candidate-'+name+'.stderr.txt')).open('wb') as serr:
                        task=subprocess.Popen(command,stdout=sout,stderr=serr,start_new_session=True);processes.append(task)
                        try:exit_code=task.wait(timeout=300)
                        except subprocess.TimeoutExpired:
                            os.killpg(task.pid,signal.SIGTERM);task.wait(timeout=10);raise RuntimeError('sequential measurement timeout')
                    if not wait_group_exit(task.pid) or breached.is_set() or exit_code:raise RuntimeError('sequential candidate measurement stopped or failed')
                    data=json.loads((out/('measure-candidate-'+name)/'report.json').read_text())
                    measurements.append({'case':name,'role':'candidate','status':data['status'],'timed_samples':data['timed_samples'],'process_group':task.pid,'group_stopped':True})
                (out/'iostat-after.txt').write_text(subprocess.check_output(['/usr/sbin/iostat','-Id','-c','1'],text=True))
        finally:
            for process in processes:
                if process.poll() is None:
                    try:os.killpg(process.pid,signal.SIGTERM)
                    except ProcessLookupError:pass
                    try:process.wait(timeout=10)
                    except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait()
            cleanup={str(process.pid):ensure_group_exit(process.pid) for process in processes}
            stopped_groups={group:result[0] for group,result in cleanup.items()}
            group_stopped=all(stopped_groups.values()) and not any(x[1] for x in cleanup.values());stop.set();watcher.join();monitor_thread.join()
            (out/'disk-watch.json').write_text(json.dumps({'reserve_bytes':args.min_free_bytes,'poll_seconds':.25,'breached':breached.is_set(),'samples':disk_samples,'candidate_group':proc.pid,'candidate_group_stopped':group_stopped},indent=2)+'\n')
            (out/'resources.json').write_text(json.dumps({'poll_seconds':.25,'samples':resources,'note':'process-group ps snapshots; not a peak or full accounting; iostat is device-wide'},indent=2)+'\n')
    synthetic.verify(argparse.Namespace(corpus=corpus))
    report={'status':'RESTRICTED_ENDPOINT_TIMINGS_RECORDED' if measurements else 'BODY_ONLY_DIAGNOSTIC_NOT_TIMED','cases':cases,'candidate_group_stopped':group_stopped,
            'sequential_preflight_status':{name:item['status'] for name,item in preflight.items()},
            'measurements':measurements,'candidate_server_process_group':proc.pid,'process_groups_stopped':stopped_groups,
            'forced_group_cleanup':{group:result[1] for group,result in cleanup.items()},
            'original_groups_previously_stopped':groups,'original_index_tree_sha256':proof['original_index_tree_before'],
            'candidate_index_sha256':receipt['candidate_index_sha256'],'candidate_binary_sha256':receipt['candidate_binary_sha256'],
            'limitations':['no HTTP header or session comparison','no timing','separate processes and server windows','synthetic data is not representative']}
    (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'status':report['status'],'body_matches':sum(x['body_matches_except_time'] for x in cases),'cases':len(cases),'candidate_group_stopped':group_stopped},indent=2))
    return 0 if group_stopped and not breached.is_set() else 1

if __name__=='__main__':sys.exit(main())
