#!/usr/bin/env python3
"""Bounded index-cost probe for an independently generated synthetic corpus.

This measures indexing feasibility, not comparable search latency or product parity.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time

import build_synthetic_corpus as synthetic
import capture_opengrok_baseline as baseline
from indexer_evidence import record_indexer_logs, verify_candidate_coverage

ROOT=Path(__file__).resolve().parents[1]


def run_guarded(command, output, label, env, reserve, free_space=None, interval=.25, timeout=180):
    free_space=free_space or (lambda:shutil.disk_usage(output).free)
    samples=[]; start=time.monotonic(); status='COMPLETED'
    with (output/(label+'.stdout.txt')).open('wb') as stdout, (output/(label+'.stderr.txt')).open('wb') as stderr:
        if free_space() < reserve:
            return {'status':'STOPPED_DISK_RESERVE','exit_code':None,'samples':samples}
        proc=subprocess.Popen(['/usr/bin/time','-l','-o',str(output/(label+'.time.txt')),*command],
                              stdout=stdout,stderr=stderr,env=env,start_new_session=True)
        try:
            while proc.poll() is None:
                free=free_space(); elapsed=time.monotonic()-start
                if free < reserve or elapsed > timeout:
                    status='STOPPED_DISK_RESERVE' if free < reserve else 'STOPPED_TIMEOUT'
                    samples.append({'elapsed_seconds':round(elapsed,3),'free_bytes':free,'process_group_ps':[]})
                    try:os.killpg(proc.pid,signal.SIGTERM)
                    except ProcessLookupError:pass
                    try: proc.wait(timeout=5)
                    except subprocess.TimeoutExpired: os.killpg(proc.pid,signal.SIGKILL);proc.wait()
                    break
                try:
                    ps=subprocess.run(['ps','-axo','pgid=,pid=,ppid=,rss=,%cpu=,time='],capture_output=True,text=True,timeout=.25)
                    rows=[line for line in ps.stdout.splitlines() if line.split() and line.split()[0]==str(proc.pid)]
                except subprocess.TimeoutExpired: rows=[]
                samples.append({'elapsed_seconds':round(elapsed,3),'free_bytes':free,'process_group_ps':rows})
                time.sleep(interval)
        finally:
            if proc.poll() is None:
                try:os.killpg(proc.pid,signal.SIGTERM)
                except ProcessLookupError:pass
                try:proc.wait(timeout=5)
                except subprocess.TimeoutExpired:os.killpg(proc.pid,signal.SIGKILL);proc.wait()
    if status=='COMPLETED' and proc.returncode!=0: status='PROCESS_FAILED'
    return {'status':status,'exit_code':proc.returncode,'elapsed_seconds':round(time.monotonic()-start,3),'samples':samples,
            'limitations':'sampled process group only; time -l IO reports block operation counts, not bytes'}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus',type=Path,required=True);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--min-free-bytes',type=int,default=synthetic.RESERVE)
    args=parser.parse_args();corpus=args.corpus.resolve(strict=True);output=args.output.absolute()
    if args.min_free_bytes < synthetic.RESERVE: parser.error('minimum free space cannot be below 6 GiB')
    if output.exists() or output.is_symlink(): parser.error('output must not exist')
    output=output.parent.resolve(strict=True)/output.name
    if output==ROOT or ROOT in output.parents or output==corpus or corpus in output.parents: parser.error('output must be external to repo and corpus')
    manifest=json.loads((corpus/'manifest.json').read_text())
    if type(manifest.get('source_bytes')) is not int or manifest['source_bytes'] > 16*1024**2: parser.error('pilot index probe capped at 16 MiB until resource scaling is assessed')
    synthetic.verify(argparse.Namespace(corpus=corpus))
    synthetic.check_space(output.parent,args.min_free_bytes,0)
    cache=Path.home()/'Library/Caches/rustgrok/oracle';tools=cache/'toolchains';release=cache/'opengrok-1.14.20/release/opengrok-1.14.20'
    jdk_archive=tools/'OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.13_9.tar.gz'
    release_archive=cache/'opengrok-1.14.20/opengrok-1.14.20.tar.gz'
    baseline.verified(jdk_archive,baseline.JDK_SHA);baseline.verified(release_archive,baseline.RELEASE_SHA)
    java=tools/'jdk-21.0.13+9/Contents/Home/bin/java';jar=release/'lib/opengrok.jar'
    baseline.verified_archive_member(jdk_archive,'jdk-21.0.13+9/Contents/Home/bin/java',java)
    inventory=json.loads((ROOT/'upstream/opengrok-release-inventory.json').read_text())
    baseline.verified(jar,inventory['indexer_jar']['sha256'])
    candidate=ROOT/'experiments/search-spike/target/release/rustgrok-search-spike'
    if not candidate.is_file(): parser.error('build the existing release probe before running this experiment')
    output.mkdir();(output/'oracle-index').mkdir()
    (output/'synthetic-manifest.json').write_bytes((corpus/'manifest.json').read_bytes())
    ignored={'JAVA_TOOL_OPTIONS','JDK_JAVA_OPTIONS','_JAVA_OPTIONS','JAVA_OPTS','CLASSPATH','JAVA_HOME','JRE_HOME','CATALINA_OPTS','CATALINA_HOME','CATALINA_BASE','CATALINA_TMPDIR'}
    env={k:v for k,v in os.environ.items() if not k.startswith('GIT_') and k not in ignored}
    env.update(GIT_CONFIG_NOSYSTEM='1',GIT_CONFIG_GLOBAL=os.devnull)
    ctags='/opt/homebrew/bin/ctags'; index=output/'oracle-index';config=output/'configuration.xml'
    original=[str(java),'-Xms128m','-Xmx512m','-jar',str(jar),'-c',ctags,'-s',str(corpus/'source'),'-d',str(index),'-W',str(config),'-P','-S','-H','-T','2']
    report={'status':'STARTED','scope':'synthetic indexing feasibility only, no HTTP/search timing','source_bytes':manifest['source_bytes'],
            'source_files':len(manifest['files']),'projects':manifest['projects'],'reserve_bytes':args.min_free_bytes,
            'candidate_binary_sha256':baseline.digest(candidate),'original_command':original}
    def save(): (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    save();result=run_guarded(original,output,'index',env,args.min_free_bytes);report['original_process']=result
    if result['status']!='COMPLETED':report['status']='BLOCKED_ORIGINAL_INDEX';save();return 1
    log=record_indexer_logs(output,output);report['original_log_check']=log
    if log['status']!='NO_KNOWN_FILE_FAILURE_INDICATORS':report['status']='BLOCKED_ORIGINAL_LOGS';save();return 1
    candidate_index=output/'candidate-index.json'
    command=[str(candidate),'index','--source',str(corpus/'source'),'--index',str(candidate_index),'--ctags',ctags]
    report['candidate_command']=command;result=run_guarded(command,output,'candidate',env,args.min_free_bytes);report['candidate_process']=result
    if result['status']!='COMPLETED':report['status']='BLOCKED_CANDIDATE_INDEX';save();return 1
    expected={'/'+r['path']:(corpus/'source'/r['path']).read_text() for r in manifest['files']}
    report['candidate_coverage']=verify_candidate_coverage(expected,json.loads(candidate_index.read_bytes()))
    if not report['candidate_coverage']['complete']:report['status']='BLOCKED_CANDIDATE_COVERAGE';save();return 1
    report['original_index_bytes']=sum(p.stat().st_size for p in index.rglob('*') if p.is_file())
    report['candidate_index_bytes']=candidate_index.stat().st_size
    report['candidate_index_sha256']=baseline.digest(candidate_index)
    report['original_path_coverage']='NOT_MEASURED';report['status']='INDEX_COST_OBSERVED_PATH_COVERAGE_NOT_YET_VERIFIED'
    report['limitations']=['synthetic repeated comments, no SCM history, not representative of real repositories',
                           'no original file path inventory yet; no search comparison',
                           'RSS/CPU/IO are sampled or process-level, not a product resource acceptance',
                           'source, indexes and temporary growth may exceed this pilot at larger sizes']
    save();print(json.dumps({'status':report['status'],'report':str(output/'report.json')},indent=2));return 0

if __name__=='__main__': sys.exit(main())
