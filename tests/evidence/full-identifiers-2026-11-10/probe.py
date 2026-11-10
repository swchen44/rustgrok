#!/usr/bin/env python3
"""Capture a finite original OpenGrok path-query correctness oracle, without timing."""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import xml.etree.ElementTree as ET

import capture_opengrok_baseline as baseline
from oracle_http import configure_server, opener

ROOT = Path('/Users/swchen.tw/git/rustgrok-integration')
BASE_OUT = ROOT / 'tests/evidence/path-query-2026-11-03'
OUT = BASE_OUT
CACHE = Path.home() / 'Library/Caches/rustgrok/oracle'
TOOLS = CACHE / 'toolchains'
JDK = TOOLS / 'jdk-21.0.13+9/Contents/Home'
TOMCAT = TOOLS / 'apache-tomcat-10.1.60'
LIB = CACHE / 'opengrok-1.14.20/release/opengrok-1.14.20/lib'
CASES = ['alpha1', 'ALPHA1', 'foo_bar', 'foo42', '_hidden', 'alpha1 AND foo_bar', 'alpha1 OR foo_bar', 'foo_bar NOT alpha1', 'foo_bar AND (other2 OR alpha1)', '1alpha', '123']
FILES = ['alpha/beta/one.c', 'alpha/gamma/two.c', 'beta/gamma/three.c', 'alpha/alone/four.c', 'other/delta/five.c']

def sha(p): return hashlib.sha256(p.read_bytes()).hexdigest()

def main():
    global OUT, CASES
    parser = argparse.ArgumentParser()
    parser.add_argument('--differential', action='store_true')
    parser.add_argument('--output', required=True, type=Path, help='new output directory outside repository')
    args = parser.parse_args()
    OUT = args.output.parent.resolve(strict=True) / args.output.name
    if OUT == ROOT or ROOT in OUT.parents or OUT.exists() or OUT.is_symlink():
        raise ValueError('output must be a new directory outside the repository')
    OUT.mkdir()
    if args.differential:
        CASES = ['alpha beta gamma', 'alpha AND beta AND gamma', 'alpha OR beta OR gamma',
                 'alpha NOT beta gamma', 'alpha beta NOT gamma', 'alpha NOT beta NOT gamma',
                 '(alpha beta) OR gamma', 'alpha OR (beta gamma)', '(alpha NOT beta) OR gamma',
                 'alpha AND (beta NOT gamma)', 'NOT (alpha OR beta)', '(NOT alpha) OR beta',
                 'alpha OR (NOT beta)', '+alpha beta', '+alpha NOT beta', 'alpha(beta OR gamma)',
                 '(alpha OR beta)gamma', 'alpha-beta', 'alpha+beta', 'alpha_foo', 'one1',
                 'ALPHA OR BETA', '(alpha)', '((alpha))', 'alpha OR (beta OR gamma)',
                 '(alpha OR beta) OR gamma', '(alpha beta) gamma', 'alpha (beta gamma)',
                 'alpha AND alpha', 'alpha OR alpha', 'NOT NOT alpha']
    archive = CACHE / 'opengrok-1.14.20/opengrok-1.14.20.tar.gz'
    jdk_archive = TOOLS / 'OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.13_9.tar.gz'
    tomcat_archive = TOOLS / 'apache-tomcat-10.1.60.tar.gz'
    baseline.verified(archive, baseline.RELEASE_SHA)
    baseline.verified(jdk_archive, baseline.JDK_SHA)
    baseline.verified(tomcat_archive, baseline.TOMCAT_SHA512, 'sha512')
    baseline.verified_tomcat_runtime(tomcat_archive, TOMCAT)
    for name in ('java', 'javac'):
        baseline.verified_archive_member(jdk_archive, f'jdk-21.0.13+9/Contents/Home/bin/{name}', JDK / 'bin' / name)
    inventory = json.loads((ROOT / 'upstream/opengrok-release-inventory.json').read_text())
    baseline.verified(LIB / 'opengrok.jar', inventory['indexer_jar']['sha256'])
    baseline.verified(LIB / 'source.war', inventory['web_war']['sha256'])
    libs = {}
    for jar in sorted(LIB.glob('*.jar')):
        if jar.name == 'opengrok.jar': continue
        item = next((x for x in inventory['web_war']['web_inf_lib'] if Path(x['path']).name == jar.name), None)
        if item is not None:
            libs[jar.name] = baseline.verified(jar, item['sha256'])
    work = Path(tempfile.mkdtemp(prefix='rustgrok-path-query-oracle-'))
    source = work / 'source'; index = work / 'index'; index.mkdir()
    for name in FILES:
        file = source / 'demo' / name; file.parent.mkdir(parents=True, exist_ok=True)
        file.write_text({'alpha/beta/one.c':'/* alpha1 foo_bar one */\n', 'alpha/gamma/two.c':'/* alpha1 foo_baz two */\n', 'beta/gamma/three.c':'/* foo_bar other2 three */\n', 'alpha/alone/four.c':'/* _hidden foo42 four */\n', 'other/delta/five.c':'/* a1b _hidden five */\n'}[name])
    classes = work / 'classes'; classes.mkdir()
    env = {k:v for k,v in os.environ.items() if not k.startswith('GIT_') and k not in {'JAVA_TOOL_OPTIONS','JDK_JAVA_OPTIONS','_JAVA_OPTIONS','JAVA_OPTS','CLASSPATH','JAVA_HOME','JRE_HOME','CATALINA_OPTS','CATALINA_HOME','CATALINA_BASE','CATALINA_TMPDIR'}}
    env.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull)
    cp = ':'.join([str(LIB/'opengrok.jar')] + [str(LIB/name) for name in libs])
    build = [str(JDK/'bin/javac'), '-cp', cp, '-d', str(classes), str(Path('/tmp/rustgrok-full-identifiers-probe/PathQueryOracle.java'))]
    p = subprocess.run(build, capture_output=True, env=env, timeout=120)
    (OUT/'javac.stdout.txt').write_bytes(p.stdout); (OUT/'javac.stderr.txt').write_bytes(p.stderr)
    if p.returncode: raise RuntimeError('javac failed')
    encoded = [base64.b64encode(q.encode()).decode() for q in CASES]
    parse = [str(JDK/'bin/java'), '-cp', str(classes)+':'+cp, 'PathQueryOracle', *encoded]
    p = subprocess.run(parse, capture_output=True, env=env, timeout=120)
    (OUT/'parse.stdout.txt').write_bytes(p.stdout); (OUT/'parse.stderr.txt').write_bytes(p.stderr)
    if p.returncode: raise RuntimeError('parser oracle failed')
    config = work/'configuration.xml'; ctags = Path('/opt/homebrew/bin/ctags')
    index_cmd = [str(JDK/'bin/java'), '-Xms128m','-Xmx512m','-jar',str(LIB/'opengrok.jar'),'-c',str(ctags),'-s',str(source),'-d',str(index),'-W',str(config),'-P','-S','-H','-T','2']
    p = subprocess.run(index_cmd, capture_output=True, env=env, timeout=120)
    (OUT/'index.stdout.txt').write_bytes(p.stdout); (OUT/'index.stderr.txt').write_bytes(p.stderr)
    if p.returncode: raise RuntimeError('indexer failed')
    (OUT/'configuration.xml').write_bytes(config.read_bytes())
    base=work/'tomcat'; shutil.copytree(TOMCAT/'conf',base/'conf')
    for name in ('logs','temp','work','webapps','conf/Catalina/localhost'): (base/name).mkdir(parents=True,exist_ok=True)
    with socket.socket() as sock: sock.bind(('127.0.0.1',0)); port=sock.getsockname()[1]
    configure_server(base/'conf/server.xml',port)
    context=ET.Element('Context',{'docBase':str(LIB/'source.war')})
    ET.SubElement(context,'Parameter',{'name':'CONFIGURATION','value':str(config),'override':'false'})
    ET.ElementTree(context).write(base/'conf/Catalina/localhost/source.xml',encoding='utf-8',xml_declaration=True)
    env.update(JAVA_HOME=str(JDK), CATALINA_HOME=str(TOMCAT), CATALINA_BASE=str(base), CATALINA_TMPDIR=str(base/'temp'), CATALINA_OPTS='-Xms128m -Xmx512m --add-exports=java.base/jdk.internal.ref=ALL-UNNAMED --add-exports=java.base/sun.nio.ch=ALL-UNNAMED --add-exports=jdk.unsupported/sun.misc=ALL-UNNAMED --add-exports=jdk.compiler/com.sun.tools.javac.file=ALL-UNNAMED --add-opens=jdk.compiler/com.sun.tools.javac=ALL-UNNAMED --add-opens=java.base/java.lang=ALL-UNNAMED --add-opens=java.base/java.lang.reflect=ALL-UNNAMED --add-opens=java.base/java.io=ALL-UNNAMED --add-opens=java.base/java.util=ALL-UNNAMED')
    cases=[]
    with (OUT/'tomcat.stdout.txt').open('wb') as log:
        server=subprocess.Popen([str(TOMCAT/'bin/catalina.sh'),'run'],stdout=log,stderr=subprocess.STDOUT,env=env,start_new_session=True)
        try:
            client=opener(); origin=f'http://127.0.0.1:{port}'
            for _ in range(45):
                if server.poll() is not None: raise RuntimeError('Tomcat exited')
                try:
                    with client.open(origin+'/source/',timeout=2) as response:
                        if response.status == 200: break
                except (OSError,urllib.error.HTTPError): time.sleep(1)
            else: raise RuntimeError('Tomcat readiness timed out')
            for i,query in enumerate(CASES + ['one','two','three','four','five']):
                path='/source/api/v1/search?'+urllib.parse.urlencode([('full',query),('projects','demo'),('start','0'),('maxresults','100')])
                try:
                    with client.open(origin+path,timeout=20) as response: body=response.read(); status=response.status
                except urllib.error.HTTPError as error: body=error.read(); status=error.code
                name=f'http-{i:02}.body'; (OUT/name).write_bytes(body)
                cases.append({'query':query,'path':path,'status':status,'body':name,'sha256':hashlib.sha256(body).hexdigest()})
        finally:
            server.terminate()
            try: server.wait(timeout=10)
            except subprocess.TimeoutExpired: server.kill();server.wait()
    receipt={'scope':'finite original parser and HTTP correctness; no benchmark or full compatibility','upstream_commit':baseline.COMMIT,'synthetic_files':FILES,'synthetic_file_contents':'per-file identifier fixtures defined in executed probe','cases':cases,'query_builder_source_sha256':sha(CACHE/'opengrok-1.14.20/source/opengrok-indexer/src/main/java/org/opengrok/indexer/search/QueryBuilder.java'),'java_sha256':sha(JDK/'bin/java'),'javac_sha256':sha(JDK/'bin/javac'),'jar_sha256':sha(LIB/'opengrok.jar'),'war_sha256':sha(LIB/'source.war'),'classpath_jar_sha256':libs,'ctags_sha256':sha(ctags),'javac_command':build,'parser_command':parse,'index_command':index_cmd,'work_directory':str(work),'config_sha256':sha(config),'parser_stdout_sha256':sha(OUT/'parse.stdout.txt'),'parser_stderr_sha256':sha(OUT/'parse.stderr.txt'),'index_stdout_sha256':sha(OUT/'index.stdout.txt'),'index_stderr_sha256':sha(OUT/'index.stderr.txt')}
    (OUT/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    for case, expected in zip(cases[-5:], FILES):
        body = json.loads((OUT/case['body']).read_bytes())
        if case['status'] != 200 or body['resultCount'] != 1 or set(body['results']) != {'/demo/'+expected}:
            raise RuntimeError('original source coverage not established')
    print('captured',len(cases),'HTTP bodies; work:',work)

if __name__=='__main__': main()
