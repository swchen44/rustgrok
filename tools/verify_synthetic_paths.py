#!/usr/bin/env python3
"""Observe each synthetic source path through the isolated original HTTP API."""
import argparse
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
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET

import build_synthetic_corpus as synthetic
import capture_opengrok_baseline as baseline
from indexer_evidence import record_indexer_logs, verify_candidate_coverage, verify_path_coverage
from oracle_http import configure_server, opener
from prepare_oracle_copy import tree_hash
from search_benchmark_gate import finite_float, no_duplicate_keys

ROOT=Path(__file__).resolve().parents[1]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus',type=Path,required=True);parser.add_argument('--probe',type=Path,required=True)
    parser.add_argument('--output',type=Path,required=True);parser.add_argument('--min-free-bytes',type=int,default=synthetic.RESERVE)
    parser.add_argument('--benchmark',action='store_true');parser.add_argument('--index-proof',type=Path)
    parser.add_argument('--warmup',type=int,default=20);parser.add_argument('--samples',type=int,default=50)
    args=parser.parse_args()
    if args.output.exists() or args.output.is_symlink():parser.error('output must not exist')
    if args.min_free_bytes < synthetic.RESERVE:parser.error('minimum free space cannot be below 6 GiB')
    if args.warmup < 1 or args.samples < 5 or args.samples % 5:parser.error('warmup must be positive and samples must be a multiple of five, at least five')
    if args.benchmark and args.index_proof is None:parser.error('benchmark requires a prior index-proof coverage receipt')
    corpus=args.corpus.resolve(strict=True);probe=args.probe.resolve(strict=True);out=args.output.parent.resolve(strict=True)/args.output.name
    if out==ROOT or ROOT in out.parents or out==corpus or corpus in out.parents or out==probe or probe in out.parents:parser.error('output must be outside repo, corpus and input probe')
    manifest=json.loads((corpus/'manifest.json').read_text())
    if type(manifest.get('source_bytes')) is not int or manifest['source_bytes']>16*1024**2:parser.error('pilot capped at 16 MiB')
    synthetic.verify(argparse.Namespace(corpus=corpus))
    old_report=json.loads((probe/'report.json').read_text())
    if old_report['status']!='INDEX_COST_OBSERVED_PATH_COVERAGE_NOT_YET_VERIFIED':parser.error('probe did not complete indexing')
    if (probe/'synthetic-manifest.json').read_bytes()!=(corpus/'manifest.json').read_bytes():parser.error('probe corpus manifest differs')
    if old_report['original_process']['status']!='COMPLETED' or old_report['candidate_process']['status']!='COMPLETED':parser.error('indexing processes did not complete')
    candidate=ROOT/'experiments/search-spike/target/release/rustgrok-search-spike'
    candidate_index=probe/'candidate-index.json'
    if args.benchmark:
        if baseline.digest(candidate)!=old_report['candidate_binary_sha256'] or baseline.digest(candidate_index)!=old_report['candidate_index_sha256']:parser.error('candidate index or binary hash mismatch')
        expected={'/'+x['path']:(corpus/'source'/x['path']).read_text() for x in manifest['files']}
        if not verify_candidate_coverage(expected,json.loads(candidate_index.read_bytes(),object_pairs_hook=no_duplicate_keys))['complete']:parser.error('candidate index source coverage mismatch')
    config=ET.parse(probe/'configuration.xml')
    for prop,expected in (('sourceRoot',corpus/'source'),('dataRoot',probe/'oracle-index')):
        node=config.find(f'.//void[@property="{prop}"]/string')
        if node is None or Path(node.text).resolve()!=expected.resolve():parser.error('original configuration root mismatch: '+prop)
    synthetic.check_space(out.parent,args.min_free_bytes,0)
    cache=Path.home()/'Library/Caches/rustgrok/oracle';tools=cache/'toolchains'
    jdk_archive=tools/'OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.13_9.tar.gz';tomcat_archive=tools/'apache-tomcat-10.1.60.tar.gz'
    release_archive=cache/'opengrok-1.14.20/opengrok-1.14.20.tar.gz'
    baseline.verified(jdk_archive,baseline.JDK_SHA);baseline.verified(tomcat_archive,baseline.TOMCAT_SHA512,'sha512');baseline.verified(release_archive,baseline.RELEASE_SHA)
    tomcat=tools/'apache-tomcat-10.1.60';jdk=tools/'jdk-21.0.13+9/Contents/Home'
    baseline.verified_archive_member(jdk_archive,'jdk-21.0.13+9/Contents/Home/bin/java',jdk/'bin/java');baseline.verified_tomcat_runtime(tomcat_archive,tomcat)
    war=cache/'opengrok-1.14.20/release/opengrok-1.14.20/lib/source.war';inventory=json.loads((ROOT/'upstream/opengrok-release-inventory.json').read_text());baseline.verified(war,inventory['web_war']['sha256'])
    original_index=probe/'oracle-index';before=tree_hash(original_index)
    if args.benchmark:
        proof=json.loads(args.index_proof.read_text())
        if proof['status']!='MATCHED_ALL_SYNTHETIC_PATHS' or proof['original_index_tree_before']!=before or proof['original_index_tree_after']!=before:parser.error('original index differs from prior coverage proof')
    index_bytes=sum(p.stat().st_size for p in original_index.rglob('*') if p.is_file())
    synthetic.check_space(out.parent,args.min_free_bytes,index_bytes+(candidate_index.stat().st_size if args.benchmark else 0))
    out.mkdir();work=out/'oracle-work';work.mkdir();shutil.copytree(original_index,work/'index')
    logs=record_indexer_logs(probe,out)
    if logs['status']!='NO_KNOWN_FILE_FAILURE_INDICATORS' or logs['streams']!=old_report['original_log_check']['streams']:raise ValueError('original index log failure or receipt mismatch')
    if args.benchmark:shutil.copyfile(candidate_index,out/'candidate-index.json')
    config.find('.//void[@property="dataRoot"]/string').text=str(work/'index')
    sugg=config.find('.//void[@property="suggesterConfig"]')
    if sugg is None:raise ValueError('missing suggester configuration')
    for node in sugg.findall('./void[@property="allowMostPopular"]'):sugg.remove(node)
    ET.SubElement(ET.SubElement(sugg,'void',{'property':'allowMostPopular'}),'boolean').text='false'
    config.write(work/'configuration.xml',encoding='utf-8',xml_declaration=True)
    shutil.copytree(tomcat/'conf',out/'tomcat/conf')
    for name in ('logs','temp','work','webapps','conf/Catalina/localhost'):(out/'tomcat'/name).mkdir(parents=True,exist_ok=True)
    with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
    configure_server(out/'tomcat/conf/server.xml',port)
    context=ET.Element('Context',{'docBase':str(war)});ET.SubElement(context,'Parameter',{'name':'CONFIGURATION','value':str(work/'configuration.xml'),'override':'false'})
    ET.ElementTree(context).write(out/'tomcat/conf/Catalina/localhost/source.xml',encoding='utf-8',xml_declaration=True)
    ignored={'JAVA_TOOL_OPTIONS','JDK_JAVA_OPTIONS','_JAVA_OPTIONS','JAVA_OPTS','CLASSPATH','JAVA_HOME','JRE_HOME','CATALINA_OPTS','CATALINA_HOME','CATALINA_BASE','CATALINA_TMPDIR'}
    env={k:v for k,v in os.environ.items() if not k.startswith('GIT_') and k not in ignored}
    env.update(JAVA_HOME=str(jdk),CATALINA_HOME=str(tomcat),CATALINA_BASE=str(out/'tomcat'),CATALINA_TMPDIR=str(out/'tomcat/temp'),CATALINA_OPTS='-Xms128m -Xmx512m --add-exports=java.base/jdk.internal.ref=ALL-UNNAMED --add-exports=java.base/sun.nio.ch=ALL-UNNAMED --add-exports=jdk.unsupported/sun.misc=ALL-UNNAMED --add-exports=jdk.compiler/com.sun.tools.javac.file=ALL-UNNAMED --add-opens=jdk.compiler/com.sun.tools.javac=ALL-UNNAMED --add-opens=java.base/java.lang=ALL-UNNAMED --add-opens=java.base/java.lang.reflect=ALL-UNNAMED --add-opens=java.base/java.io=ALL-UNNAMED --add-opens=java.base/java.util=ALL-UNNAMED')
    samples=[];resources=[];processes=[];stop=threading.Event();breached=threading.Event();start=time.monotonic();cases=[]
    with (out/'tomcat.stdout-stderr.txt').open('wb') as log:
        proc=subprocess.Popen([str(tomcat/'bin/catalina.sh'),'run'],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True);processes.append(proc)
        candidate_log=None
        if args.benchmark:
            with socket.socket() as sock:sock.bind(('127.0.0.1',0));candidate_port=sock.getsockname()[1]
            candidate_log=(out/'candidate.stdout-stderr.txt').open('wb')
            try:candidate_proc=subprocess.Popen([str(candidate),'serve','--index',str(out/'candidate-index.json'),'--listen',f'127.0.0.1:{candidate_port}'],stdout=candidate_log,stderr=subprocess.STDOUT,start_new_session=True);processes.append(candidate_proc)
            except Exception:
                try:os.killpg(proc.pid,signal.SIGTERM)
                except ProcessLookupError:pass
                proc.wait(timeout=10);candidate_log.close();raise
        def watch():
            while not stop.is_set():
                free=shutil.disk_usage(out).free;samples.append({'elapsed_seconds':round(time.monotonic()-start,3),'free_bytes':free})
                if free<args.min_free_bytes:
                    breached.set()
                    for process in list(processes):
                        if process.poll() is None:
                            try:os.killpg(process.pid,signal.SIGTERM)
                            except ProcessLookupError:pass
                    return
                stop.wait(.25)
        thread=threading.Thread(target=watch,daemon=True);thread.start();client=opener();bodies={}
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
            for _ in range(90):
                if proc.poll() is not None or breached.is_set():raise RuntimeError('Tomcat exited or disk reserve breached')
                try:
                    with client.open(f'http://127.0.0.1:{port}/source/',timeout=2) as response:
                        if response.status==200:break
                except (OSError,urllib.error.URLError):time.sleep(1)
            else:raise RuntimeError('Tomcat readiness timeout')
            projects=sorted({x['path'].split('/')[0] for x in manifest['files']})
            for name in sorted({Path(x['path']).name for x in manifest['files']}):
                if breached.is_set():raise RuntimeError('disk reserve breached')
                params=[('path',name)]+[('projects',project) for project in projects]+[('start','0'),('maxresults','100')]
                with client.open(f'http://127.0.0.1:{port}/source/api/v1/search?'+urllib.parse.urlencode(params),timeout=20) as response:
                    raw=response.read()
                    if response.status!=200:raise RuntimeError('oracle HTTP status mismatch')
                (out/('response-'+name+'.json')).write_bytes(raw)
                bodies[name]=json.loads(raw,object_pairs_hook=no_duplicate_keys,parse_float=finite_float,parse_constant=lambda _: (_ for _ in ()).throw(ValueError('non-finite')))
            converted={'repositories':[{'path':str(Path(x['path']).parent),'file':Path(x['path']).name} for x in manifest['files']]}
            coverage=verify_path_coverage(converted,bodies)
            if not coverage['complete']:raise RuntimeError('original path coverage incomplete; benchmark not started')
            if args.benchmark:
                if candidate_proc.poll() is not None:raise RuntimeError('candidate server exited')
                (out/'iostat-before.txt').write_text(subprocess.check_output(['/usr/sbin/iostat','-Id','-c','1'],text=True))
                choices=[('single-hit','synthetic000.c',[projects[0]]),('no-hit','absent.c',[projects[0]]),('multiple-hit','synthetic000.c',projects)]
                for name,term,selected in choices:
                    if breached.is_set():raise RuntimeError('disk reserve breached')
                    params=[('path',term)]+[('projects',project) for project in selected]+[('start','0'),('maxresults','100')]
                    query='/source/api/v1/search?'+urllib.parse.urlencode(params)
                    command=[sys.executable,str(ROOT/'tools/search_live_benchmark.py'),'--oracle',f'http://127.0.0.1:{port}'+query,'--candidate',f'http://127.0.0.1:{candidate_port}'+query,'--output',str(out/('benchmark-'+name)),'--warmup',str(args.warmup),'--samples',str(args.samples)]
                    with (out/('benchmark-'+name+'.stdout.txt')).open('wb') as sout,(out/('benchmark-'+name+'.stderr.txt')).open('wb') as serr:
                        task=subprocess.Popen(command,stdout=sout,stderr=serr,start_new_session=True);processes.append(task)
                        try:exit_code=task.wait(timeout=180)
                        except subprocess.TimeoutExpired:
                            os.killpg(task.pid,signal.SIGTERM);task.wait(timeout=10);raise RuntimeError('benchmark timeout')
                    if breached.is_set():raise RuntimeError('disk reserve breached')
                    result=json.loads((out/('benchmark-'+name)/'report.json').read_text())
                    cases.append({'case':name,'exit_code':exit_code,'status':result['status'],'timed_samples':result['timed_samples'],'query':query})
                    if exit_code and not result['status'].startswith('BLOCKED_'):raise RuntimeError('unexpected benchmark failure')
                (out/'iostat-after.txt').write_text(subprocess.check_output(['/usr/sbin/iostat','-Id','-c','1'],text=True))
        finally:
            stop.set();thread.join();monitor_thread.join()
            for process in processes:
                if process.poll() is None:
                    try:os.killpg(process.pid,signal.SIGTERM)
                    except ProcessLookupError:pass
                    try:process.wait(timeout=10)
                    except subprocess.TimeoutExpired:os.killpg(process.pid,signal.SIGKILL);process.wait()
            if candidate_log:candidate_log.close()
            (out/'disk-watch.json').write_text(json.dumps({'reserve_bytes':args.min_free_bytes,'poll_seconds':.25,'breached':breached.is_set(),'samples':samples},indent=2)+'\n')
            (out/'resources.json').write_text(json.dumps({'poll_seconds':.25,'samples':resources,'note':'process-group ps snapshots; not peak or complete process accounting; iostat is device-wide'},indent=2)+'\n')
    after=tree_hash(probe/'oracle-index');synthetic.verify(argparse.Namespace(corpus=corpus))
    report={'status':'MATCHED_ALL_SYNTHETIC_PATHS' if coverage['complete'] and before==after else 'BLOCKED',
            'coverage':coverage,'original_index_tree_before':before,'original_index_tree_after':after,
            'benchmark_cases':cases,'candidate_binary_sha256':old_report['candidate_binary_sha256'] if args.benchmark else None,
            'sampled_disk_reserve_bytes':args.min_free_bytes,'disk_poll_seconds':.25,'disk_samples':samples,
            'configuration_change':'isolated copied dataRoot; allowMostPopular=false (upstream default true, user deferred suggestions)',
            'limitations':['path presence does not prove all source tokens indexed','disk samples do not bound writes between polls','synthetic data has no SCM history and is not representative']}
    (out/'coverage.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps({'status':report['status'],'observed_paths':len(coverage['observed_paths'])},indent=2))
    return 0 if report['status']!='BLOCKED' else 1

if __name__=='__main__':sys.exit(main())
