#!/usr/bin/env python3
"""Run a narrow loopback HTTP experiment only after full response preflight."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import ipaddress
import json
import math
import re
from pathlib import Path
import time
import urllib.error
import urllib.parse
import urllib.request
from search_benchmark_gate import comparison, no_duplicate_keys


def main():
    p = argparse.ArgumentParser(description=__doc__)
    for name in ('oracle', 'candidate'): p.add_argument('--'+name, required=True)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--warmup', type=int, default=20)
    p.add_argument('--samples', type=int, default=50)
    a = p.parse_args()
    urls = [urllib.parse.urlsplit(x) for x in (a.oracle, a.candidate)]
    for u in urls:
        try: valid = u.scheme == 'http' and ipaddress.ip_address(u.hostname).is_loopback and u.port and not u.username and not u.fragment
        except (ValueError, TypeError): valid = False
        if not valid: p.error('URLs must use a loopback IP and explicit port')
    if (urls[0].path, urls[0].query) != (urls[1].path, urls[1].query): p.error('request paths and query strings must match')
    if a.warmup < 1 or a.samples < 5 or a.samples % 5: p.error('warmup must be positive; samples must be a positive multiple of 5')
    a.output.mkdir(parents=True, exist_ok=False)
    report = {'status':'STARTED', 'timed_samples':0, 'warmup_per_endpoint_per_load':a.warmup, 'samples_per_endpoint_per_load':a.samples, 'query':urls[0].path+'?'+urls[0].query, 'body_normalization':['top-level time value'], 'header_comparison_ignores':['date','content-length','connection'], 'full_product_parity':'NOT_ASSERTED'}
    def save(): (a.output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    def fetch(url, label, cookie=None):
        start=time.perf_counter_ns(); body=b''; status=None; headers={}; error=None
        try:
            opener=urllib.request.build_opener(urllib.request.ProxyHandler({}))
            request=urllib.request.Request(url,headers={'Cookie':cookie} if cookie else {})
            with opener.open(request,timeout=20) as r: body=r.read(); status=r.status; headers=dict(r.headers.items())
        except urllib.error.HTTPError as e: status=e.code; body=e.read(); headers=dict(e.headers.items()); error=str(e)
        except (OSError, urllib.error.URLError) as e: error=str(e)
        elapsed=time.perf_counter_ns()-start
        (a.output/(label+'.body')).write_bytes(body)
        raw_cookie=next((v.split(';',1)[0] for k,v in headers.items() if k.lower() == 'set-cookie'),None)
        has_cookie=raw_cookie is not None
        cookie_supported=not has_cookie or bool(re.fullmatch(r'JSESSIONID=[0-9A-F]{32}',raw_cookie))
        def redact_cookie(value):
            first, *attributes = value.split(';')
            name = first.split('=', 1)[0].strip()
            return name + '=<redacted ephemeral session value>' + ''.join(';'+part for part in attributes)
        headers={k: (redact_cookie(v) if k.lower() == 'set-cookie' else v) for k,v in headers.items()}
        item={'body':label+'.body','status':status,'headers':headers,'set_cookie_present':has_cookie,'cookie_supported':cookie_supported,'error':error,'elapsed_ns':elapsed,'_cookie':raw_cookie}
        try: item['_parsed']=json.loads(body,object_pairs_hook=no_duplicate_keys,parse_constant=lambda x: (_ for _ in ()).throw(ValueError('non-finite')))
        except (ValueError,UnicodeDecodeError): item['_parsed']=None
        return item
    def equal(x,y):
        def media(v): return v.get('headers',{}).get('Content-Type','').split(';')[0].strip().lower()
        return x['status']==y['status']==200 and media(x)==media(y)=='application/json' and comparison(x['_parsed'],y['_parsed'])
    def headers_equal(x,y):
        ignored={'date','content-length','connection'}
        def selected(item): return {k.lower():v for k,v in item['headers'].items() if k.lower() not in ignored}
        return selected(x)==selected(y)
    def strip(item): return {k:v for k,v in item.items() if not k.startswith('_')}
    base=[fetch(a.oracle,'preflight-oracle'),fetch(a.candidate,'preflight-candidate')]
    report['preflight']=[strip(x) for x in base]
    report['preflight_body_and_content_type_match']=equal(*base)
    if not all(item['cookie_supported'] for item in base) or base[0]['set_cookie_present'] != base[1]['set_cookie_present']:
        report['status']='BLOCKED_SESSION_BEHAVIOR';save();return 1
    if not headers_equal(*base): report['status']='BLOCKED_HEADER_DIFFERENCE';save();return 1
    if not equal(*base): report['status']='BLOCKED_OUTPUT_DIFFERENCE';save();return 1
    if base[0]['set_cookie_present']:
        follow=[fetch(url,f'followup-{n}',base[n]['_cookie']) for n,url in enumerate((a.oracle,a.candidate))]
        report['cookie_followup']=[strip(item) for item in follow]
        if any(item['set_cookie_present'] for item in follow) or not headers_equal(*follow) or not equal(*follow) or not equal(base[0],follow[0]):
            report['status']='BLOCKED_SESSION_BEHAVIOR';save();return 1
    records=[]
    for concurrency in (1,5):
        for n,url in enumerate((a.oracle,a.candidate)):
            cookies=[]
            for client in range(concurrency):
                initial=fetch(url,f'setup-c{concurrency}-endpoint{n}-client{client}')
                if not initial['cookie_supported'] or initial['set_cookie_present'] != base[0]['set_cookie_present'] or not headers_equal(base[0],initial) or not equal(base[0],initial):
                    report.update(status='BLOCKED_SESSION_BEHAVIOR',failure=strip(initial),timed_samples=len(records),samples=records);save();return 1
                cookies.append(initial['_cookie'])
            expected=follow[0] if base[0]['set_cookie_present'] else base[0]
            for i in range(a.warmup):
                client=i % concurrency
                item=fetch(url,f'warmup-c{concurrency}-endpoint{n}-{i:03}',cookies[client])
                if item['set_cookie_present'] or not headers_equal(expected,item) or not equal(expected,item):
                    report.update(status='BLOCKED_WARMUP_DIFFERENCE',failure=strip(item),timed_samples=len(records),samples=records);save();return 1
            # Each batch starts together; client IDs retain their own in-memory cookies.
            for offset in range(0,a.samples,concurrency):
                with ThreadPoolExecutor(max_workers=concurrency) as pool:
                    futures=[pool.submit(fetch,url,f'sample-c{concurrency}-endpoint{n}-{i:03}',cookies[i % concurrency]) for i in range(offset,offset+concurrency)]
                    group=[f.result() for f in futures]
                for client,item in enumerate(group):
                    records.append({'concurrency':concurrency,'endpoint':n,'client':client,**strip(item)})
                    if item['set_cookie_present'] or not headers_equal(expected,item) or not equal(expected,item):
                        report.update(status='BLOCKED_SAMPLE_DIFFERENCE',timed_samples=len(records),failure=strip(item),samples=records);save();return 1
    report['samples']=records;report['timed_samples']=len(records);report['summary']={}
    for concurrency in (1,5):
        for n,name in enumerate(('oracle','candidate')):
            values=sorted(r['elapsed_ns'] for r in records if r['concurrency']==concurrency and r['endpoint']==n)
            report['summary'][f'{name}_c{concurrency}']={'count':len(values),'errors':sum(r['error'] is not None for r in records if r['concurrency']==concurrency and r['endpoint']==n),'p50_ns':values[math.ceil(.50*len(values))-1],'p95_ns':values[math.ceil(.95*len(values))-1],'p99_ns':values[math.ceil(.99*len(values))-1]}
    report['percentile_method']='nearest rank';report['status']='COMPLETED_RESTRICTED_CASE';save();return 0
if __name__ == '__main__': raise SystemExit(main())
