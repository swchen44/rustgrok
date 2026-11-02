#!/usr/bin/env python3
"""Time one loopback endpoint only after a saved, strict two-sided preflight."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import ipaddress
import json
import math
from pathlib import Path
import shutil
import sys
import urllib.parse

from build_synthetic_corpus import RESERVE
from search_benchmark_gate import comparison, load_response, strict_equal
from sequential_preflight import equivalent, request


def load_eligibility(directory,case):
    item=json.loads((directory/'sequential-preflight.json').read_text())[case]
    if item.get('status')!='MATCHED_PREFLIGHT_NOT_TIMED':raise ValueError('preflight was not matched')
    for role in ('oracle','candidate'):
        for part in ('first','followup'):
            raw,body=load_response(directory/f'preflight-{case}-{role}-{part}.body')
            if hashlib.sha256(raw).hexdigest()!=item[role+'_body_sha256'][part]:raise ValueError('preflight body hash mismatch')
            recorded=item[role][part]['body']
            if not strict_equal(body,recorded) or list(body['results'])!=list(recorded['results']):raise ValueError('preflight body projection mismatch')
    if not equivalent(item['oracle'],item['candidate']):raise ValueError('preflight status/body/headers/session mismatch')
    return item


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--endpoint',required=True);parser.add_argument('--proof',type=Path,required=True);parser.add_argument('--case',choices=('single-hit','no-hit'),required=True)
    parser.add_argument('--role',choices=('oracle','candidate'),required=True);parser.add_argument('--output',type=Path,required=True)
    parser.add_argument('--warmup',type=int,default=20);parser.add_argument('--samples',type=int,default=50);parser.add_argument('--min-free-bytes',type=int,default=RESERVE)
    args=parser.parse_args();u=urllib.parse.urlsplit(args.endpoint)
    try:valid=u.scheme=='http' and ipaddress.ip_address(u.hostname).is_loopback and u.port and not u.username and not u.fragment and u.path in ('','/') and not u.query
    except (ValueError,TypeError):valid=False
    if not valid:parser.error('endpoint must be an HTTP loopback origin with explicit port')
    if args.warmup<1 or args.samples<5 or args.samples%5:parser.error('warmup must be positive; samples must be a multiple of five')
    if args.min_free_bytes<RESERVE:parser.error('minimum free space cannot be below 4.5 GiB')
    if args.output.exists() or args.output.is_symlink():parser.error('output must not exist')
    proof=args.proof.resolve(strict=True);out=args.output.parent.resolve(strict=True)/args.output.name
    if out==proof or proof in out.parents:parser.error('output must be outside preflight proof')
    try:item=load_eligibility(proof,args.case)
    except (OSError,ValueError,KeyError,TypeError) as exc:parser.error(str(exc))
    if shutil.disk_usage(out.parent).free<args.min_free_bytes:parser.error('disk reserve breached before output')
    out.mkdir();url=args.endpoint.rstrip('/')+item['query'];reference=item['oracle'];records=[]
    report={'status':'STARTED','case':args.case,'role':args.role,'query':item['query'],'proof_sha256':hashlib.sha256((proof/'sequential-preflight.json').read_bytes()).hexdigest(),
            'warmup_per_load':args.warmup,'samples_per_load':args.samples,'timed_samples':0,'samples':records}
    def save(): (out/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    def free():return shutil.disk_usage(out).free>=args.min_free_bytes
    def same(obs,expected,first):
        return (obs['status']==200 and obs['error'] is None and obs['cookie_supported'] and obs['cookie_present']==first
                and obs['headers']==expected['headers'] and comparison(obs['body'],expected['body']))
    def fetch(cookie,label):
        if not free():return {'status':None,'headers':[],'cookie_present':False,'cookie_supported':False,'body':None,'error':'DISK_RESERVE','elapsed_ns':0},b'',None
        return request(url,cookie)
    save()
    for concurrency in (1,5):
        cookies=[]
        for client in range(concurrency):
            obs,raw,cookie=fetch(None,'setup');(out/f'setup-c{concurrency}-{client}.body').write_bytes(raw)
            if not same(obs,reference['first'],True) or not cookie or cookie in cookies:
                report.update(status='BLOCKED_SESSION_SETUP',failure={k:v for k,v in obs.items() if k!='body'});save();return 1
            cookies.append(cookie)
        for i in range(args.warmup):
            obs,raw,_=fetch(cookies[i%concurrency],'warmup');(out/f'warmup-c{concurrency}-{i:03}.body').write_bytes(raw)
            if not same(obs,reference['followup'],False):report.update(status='BLOCKED_WARMUP',failure={k:v for k,v in obs.items() if k!='body'});save();return 1
        for offset in range(0,args.samples,concurrency):
            with ThreadPoolExecutor(max_workers=concurrency) as pool:
                futures=[pool.submit(fetch,cookies[i%concurrency],'sample') for i in range(offset,offset+concurrency)]
                group=[future.result() for future in futures]
            for n,(obs,raw,_) in enumerate(group):
                index=offset+n;name=f'sample-c{concurrency}-{index:03}.body';(out/name).write_bytes(raw)
                records.append({'concurrency':concurrency,'client':index%concurrency,'body':name,**{k:v for k,v in obs.items() if k!='body'}})
                report['timed_samples']=len(records)
                if not same(obs,reference['followup'],False):report.update(status='BLOCKED_SAMPLE',failure=records[-1]);save();return 1
    report['summary']={}
    for concurrency in (1,5):
        values=sorted(x['elapsed_ns'] for x in records if x['concurrency']==concurrency)
        report['summary'][str(concurrency)]={'count':len(values),'errors':sum(x['error'] is not None for x in records if x['concurrency']==concurrency),
            **{f'p{p}_ns':values[math.ceil(p*len(values)/100)-1] for p in (50,95,99)}}
    report['status']='COMPLETED_RESTRICTED_ENDPOINT';report['percentile_method']='nearest rank';report['unique_session_cookies_per_load']=True
    save();print(json.dumps({'status':report['status'],'role':args.role,'case':args.case,'timed_samples':len(records)}));return 0

if __name__=='__main__':sys.exit(main())
