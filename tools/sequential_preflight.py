"""Capture and compare bounded HTTP observations without persisting session values."""
import json
import re
import time
import urllib.error
import urllib.parse
import urllib.request

from oracle_http import opener
from search_benchmark_gate import comparison, finite_float, no_duplicate_keys

IGNORED={'date','content-length','connection'}


def case_queries(project):
    return {name:'/source/api/v1/search?'+urllib.parse.urlencode([('path',term),('projects',project),('start','0'),('maxresults','100')])
            for name,term in (('single-hit','synthetic000.c'),('no-hit','absent.c'))}


def request(url, cookie=None):
    raw=b'';status=None;header_items=[];error=None;started=time.perf_counter_ns()
    try:
        req=urllib.request.Request(url,headers={'Cookie':cookie} if cookie else {})
        with opener().open(req,timeout=20) as response:
            raw=response.read();status=response.status;header_items=list(response.headers.raw_items())
    except urllib.error.HTTPError as response:
        raw=response.read();status=response.code;header_items=list(response.headers.raw_items());error=str(response)
    except (OSError,urllib.error.URLError) as exc:error=str(exc)
    elapsed_ns=time.perf_counter_ns()-started
    cookies=[v for k,v in header_items if k.lower()=='set-cookie']
    raw_cookie=cookies[0].split(';',1)[0] if len(cookies)==1 else None
    supported=not cookies or (len(cookies)==1 and bool(re.fullmatch(r'JSESSIONID=[0-9A-F]{32}',raw_cookie or '')))
    headers=[]
    for key,value in header_items:
        key=key.lower()
        if key in IGNORED:continue
        if key=='set-cookie':
            first,*attrs=value.split(';');value=first.split('=',1)[0]+'=<redacted>'+''.join(';'+part for part in attrs)
        headers.append([key,value])
    headers.sort()
    try:body=json.loads(raw,object_pairs_hook=no_duplicate_keys,parse_float=finite_float,parse_constant=lambda _:(_ for _ in ()).throw(ValueError('non-finite')))
    except (ValueError,UnicodeDecodeError):body=None
    return {'status':status,'headers':headers,'cookie_present':bool(cookies),'cookie_supported':supported,'body':body,'error':error,'elapsed_ns':elapsed_ns},raw,raw_cookie


def observe(url, directory, prefix):
    first,raw,cookie=request(url);(directory/(prefix+'-first.body')).write_bytes(raw)
    follow,raw,_=request(url,cookie);(directory/(prefix+'-followup.body')).write_bytes(raw)
    return {'first':first,'followup':follow}


def equivalent(original,candidate):
    try:
        for observation in (original,candidate):
            first,follow=observation['first'],observation['followup']
            if any(x['status']!=200 or not x['cookie_supported'] or x.get('error') for x in (first,follow)):return False
            if not first['cookie_present'] or follow['cookie_present'] or not comparison(first['body'],follow['body']):return False
            if sum(1 for key,value in first['headers'] if key=='content-type' and value.split(';')[0].strip().lower()=='application/json')!=1:return False
            if sum(1 for key,value in follow['headers'] if key=='content-type' and value.split(';')[0].strip().lower()=='application/json')!=1:return False
        return (original['first']['cookie_present']==candidate['first']['cookie_present']
                and original['first']['headers']==candidate['first']['headers']
                and original['followup']['headers']==candidate['followup']['headers']
                and comparison(original['first']['body'],candidate['first']['body'])
                and comparison(original['followup']['body'],candidate['followup']['body']))
    except (KeyError,TypeError,ValueError):return False
