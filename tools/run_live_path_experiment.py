import os,sys,subprocess,socket,time,pathlib,xml.etree.ElementTree as ET,urllib.request,urllib.error,urllib.parse,threading,json,hashlib,platform,shutil,argparse
import capture_opengrok_baseline as baseline
import http.cookiejar
from search_benchmark_gate import comparison, finite_float, no_duplicate_keys
from prepare_oracle_copy import prepare, tree_hash
from oracle_http import configure_server, opener as safe_opener
from indexer_evidence import record_indexer_logs, verify_candidate_coverage, verify_path_coverage
root=pathlib.Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser(description='Run a restricted live path-only HTTP preflight with a previously verified isolated oracle capture')
for option in ('oracle-capture','corpus','output'): parser.add_argument('--'+option, type=pathlib.Path, required=True)
parser.add_argument('--case', choices=('single-hit', 'no-hit', 'cross-project'), default='single-hit')
parser.add_argument('--warmup', type=int, default=20)
parser.add_argument('--samples', type=int, default=50)
a=parser.parse_args();base=a.oracle_capture.resolve();out=a.output.resolve();corpus=a.corpus.resolve()
if a.warmup < 1 or a.samples < 5 or a.samples % 5: parser.error('warmup must be positive; samples must be a positive multiple of 5')
cache=pathlib.Path.home()/'Library/Caches/rustgrok/oracle'; tomcat=cache/'toolchains/apache-tomcat-10.1.60'; jdk=cache/'toolchains/jdk-21.0.13+9/Contents/Home'; binary=root/'experiments/search-spike/target/release/rustgrok-search-spike'
if out == base or base in out.parents: parser.error('output must be outside capture')
if out == root or root in out.parents or out == corpus or corpus in out.parents: parser.error('output must be outside repository and corpus')
manifest=baseline.check_corpus(corpus)
capture=json.loads((base/'result.json').read_text())
if hashlib.sha256((corpus/'manifest.json').read_bytes()).hexdigest()!=capture['corpus_manifest_sha256']: parser.error('capture and corpus manifest differ')
baseline.verified_tomcat_runtime(cache/'toolchains/apache-tomcat-10.1.60.tar.gz',tomcat)
baseline.verified_archive_member(cache/'toolchains/OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.13_9.tar.gz','jdk-21.0.13+9/Contents/Home/bin/java',jdk/'bin/java')
source_bytes=sum((corpus/'source'/r['path']/r['file']).stat().st_size for r in manifest['repositories'])
out.mkdir(exist_ok=False)
original_hash=tree_hash(base)
# Rebuild both indexes from the verified public corpus during this run.
fresh=out/'fresh-oracle-capture'
capture_cmd=[sys.executable,str(root/'tools/capture_opengrok_baseline.py'),'--corpus',str(corpus),'--output',str(fresh)]
capture_process=subprocess.run(capture_cmd,capture_output=True,text=True,timeout=240)
(out/'indexer.stdout.txt').write_text(capture_process.stdout)
(out/'indexer.stderr.txt').write_text(capture_process.stderr)
index_log_check=record_indexer_logs(fresh,out) if (fresh/'index.stdout.txt').is_file() and (fresh/'index.stderr.txt').is_file() else None
if capture_process.returncode: raise RuntimeError('fresh original capture/index failed')
if index_log_check is None: raise RuntimeError('fresh original indexer logs are missing')
if index_log_check['status'] != 'NO_KNOWN_FILE_FAILURE_INDICATORS': raise RuntimeError('original indexing logs report failure or lack required completion markers')
fresh_receipt=json.loads((fresh/'result.json').read_text())
source_root=ET.parse(fresh/'configuration.xml').find('.//void[@property="sourceRoot"]/string')
if source_root is None or pathlib.Path(source_root.text).resolve() != (corpus/'source').resolve(): raise RuntimeError('original sourceRoot differs from verified corpus')
subprocess.run(['cargo','build','--locked','--offline','--release','--manifest-path',str(root/'experiments/search-spike/Cargo.toml')],check=True)
index=out/'candidate-index.json'
index_cmd=[str(binary),'index','--source',str(corpus/'source'),'--index',str(index),'--ctags','/opt/homebrew/bin/ctags']
index_process=subprocess.run(index_cmd,capture_output=True,text=True,timeout=180)
(out/'candidate-indexer.stdout.txt').write_text(index_process.stdout)
(out/'candidate-indexer.stderr.txt').write_text(index_process.stderr)
if index_process.returncode: raise RuntimeError('candidate indexing failed')
expected_candidate={'/'+r['path'].strip('/')+'/'+r['file']:(corpus/'source'/r['path']/r['file']).read_text() for r in manifest['repositories']}
candidate_coverage=verify_candidate_coverage(expected_candidate,json.loads(index.read_bytes(),object_pairs_hook=no_duplicate_keys))
(out/'candidate-source-coverage.json').write_text(json.dumps(candidate_coverage,indent=2)+'\n')
if not candidate_coverage['complete']: raise RuntimeError('candidate index does not contain the expected source files and contents')
work=out/'oracle-work'
copy_receipt=prepare(fresh,work)
config=work/'configuration.xml'
(out/'configuration.xml').write_bytes(config.read_bytes())
shutil.copytree(tomcat/'conf',work/'tomcat/conf')
for name in ('logs','temp','work','webapps','conf/Catalina/localhost'):
 (work/'tomcat'/name).mkdir(parents=True,exist_ok=True)
war=cache/'opengrok-1.14.20/release/opengrok-1.14.20/lib/source.war'
context=ET.Element('Context',{'docBase':str(war)})
ET.SubElement(context,'Parameter',{'name':'CONFIGURATION','value':str(config),'override':'false'})
ET.ElementTree(context).write(work/'tomcat/conf/Catalina/localhost/source.xml',encoding='utf-8',xml_declaration=True)
def port():
 with socket.socket() as s: s.bind(('127.0.0.1',0)); return s.getsockname()[1]
op=port(); cp=port(); configure_server(work/'tomcat/conf/server.xml',op)
env={k:v for k,v in os.environ.items() if not k.startswith('GIT_') and k not in {'JAVA_TOOL_OPTIONS','JDK_JAVA_OPTIONS','_JAVA_OPTIONS','JAVA_OPTS','CLASSPATH','JAVA_HOME','JRE_HOME','CATALINA_OPTS','CATALINA_HOME','CATALINA_BASE','CATALINA_TMPDIR'}}
env.update(GIT_CONFIG_NOSYSTEM='1',GIT_CONFIG_GLOBAL=os.devnull,JAVA_HOME=str(jdk),CATALINA_HOME=str(tomcat),CATALINA_BASE=str(work/'tomcat'),CATALINA_TMPDIR=str(work/'tomcat/temp'),CATALINA_OPTS='-Xms128m -Xmx512m --add-exports=java.base/jdk.internal.ref=ALL-UNNAMED --add-exports=java.base/sun.nio.ch=ALL-UNNAMED --add-exports=jdk.unsupported/sun.misc=ALL-UNNAMED --add-exports=jdk.compiler/com.sun.tools.javac.file=ALL-UNNAMED --add-opens=jdk.compiler/com.sun.tools.javac=ALL-UNNAMED --add-opens=java.base/java.lang=ALL-UNNAMED --add-opens=java.base/java.lang.reflect=ALL-UNNAMED --add-opens=java.base/java.io=ALL-UNNAMED --add-opens=java.base/java.util=ALL-UNNAMED')
subprocess.run(['cargo','build','--locked','--offline','--release','--manifest-path',str(root/'experiments/search-spike/Cargo.toml')],check=True)
log=(out/'tomcat.log').open('wb'); rlog=(out/'candidate.log').open('wb'); processes=[]; stop=threading.Event(); snapshots=[]
try:
 oracle=subprocess.Popen([str(tomcat/'bin/catalina.sh'),'run'],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True);processes.append(oracle)
 candidate=subprocess.Popen([str(binary),'serve','--index',str(index),'--listen',f'127.0.0.1:{cp}'],stdout=rlog,stderr=subprocess.STDOUT,start_new_session=True);processes.append(candidate)
 opener=safe_opener()
 for i in range(90):
  if oracle.poll() is not None or candidate.poll() is not None: raise RuntimeError('server exited')
  try:
   with opener.open(f'http://127.0.0.1:{op}/source/',timeout=2) as r:
    if r.status==200: break
  except (OSError,urllib.error.URLError): time.sleep(1)
 else: raise RuntimeError('oracle readiness timeout')
 # Check every expected fixture path by querying the independent original API.
 # Preserve each response and reject a missing or extra document before timing.
 coverage_responses={}
 for basename in sorted({r['file'] for r in manifest['repositories']}):
  params=[('path',basename),('projects','alpha'),('projects','beta'),('start','0'),('maxresults','100')]
  with opener.open(f'http://127.0.0.1:{op}/source/api/v1/search?'+urllib.parse.urlencode(params),timeout=20) as response:
   body=response.read()
   if response.status != 200: raise RuntimeError('original source coverage query was not HTTP 200')
  (out/('coverage-'+basename+'.body')).write_bytes(body)
  coverage_responses[basename]=json.loads(body,object_pairs_hook=no_duplicate_keys,parse_float=finite_float,parse_constant=lambda x: (_ for _ in ()).throw(ValueError('non-finite')))
 coverage=verify_path_coverage(manifest,coverage_responses)
 (out/'original-path-coverage.json').write_text(json.dumps(coverage,indent=2)+'\n')
 if not coverage['complete']: raise RuntimeError('original did not return all expected source paths')
 # Observe original first and cookie-bearing follow-up using an isolated in-memory jar.
 query={'single-hit':'/source/api/v1/search?path=main.c&projects=alpha&start=0&maxresults=10',
        'no-hit':'/source/api/v1/search?path=absentfile.c&projects=alpha&start=0&maxresults=10',
        'cross-project':'/source/api/v1/search?path=main.c&projects=alpha&projects=beta&start=0&maxresults=10'}[a.case]
 jar=http.cookiejar.CookieJar()
 session_opener=safe_opener(urllib.request.HTTPCookieProcessor(jar))
 session_steps=[]; session_values=[]
 for label in ('first','followup'):
  cookie_sent = bool(list(jar))
  with session_opener.open(f'http://127.0.0.1:{op}'+query,timeout=20) as response:
   data=response.read(); raw_cookie=response.headers.get('Set-Cookie'); headers=dict(response.headers.items()); status=response.status
  if raw_cookie:
   first,*attributes=raw_cookie.split(';');name=first.split('=',1)[0];headers['Set-Cookie']=name+'=<redacted ephemeral session value>'+''.join(';'+x for x in attributes)
  (out/('session-'+label+'.body')).write_bytes(data)
  session_values.append(json.loads(data));session_steps.append({'step':label,'cookie_sent':cookie_sent,'cookie_names_in_jar':[c.name for c in jar],'status':status,'headers':headers,'body':'session-'+label+'.body'})
 for label, cookie in (('new-client',None),('invalid-cookie','JSESSIONID=unknown')):
  request=urllib.request.Request(f'http://127.0.0.1:{op}'+query,headers={'Cookie':cookie} if cookie else {})
  with opener.open(request,timeout=20) as response:
   data=response.read(); raw_cookie=response.headers.get('Set-Cookie'); headers=dict(response.headers.items()); status=response.status
  if raw_cookie:
   first,*attributes=raw_cookie.split(';');name=first.split('=',1)[0];headers['Set-Cookie']=name+'=<redacted ephemeral session value>'+''.join(';'+x for x in attributes)
  (out/('session-'+label+'.body')).write_bytes(data)
  session_steps.append({'step':label,'cookie_sent':bool(cookie),'status':status,'headers':headers,'body':'session-'+label+'.body','body_matches_first_except_time':comparison(session_values[0],json.loads(data))})
 (out/'session-followup.json').write_text(json.dumps({'steps':session_steps,'bodies_match_except_top_level_time':comparison(*session_values),'scope':'original only; not equivalent candidate behavior'},indent=2)+'\n')
 # Cross-project, multi-hit pages: preserve the servers' raw order and both bodies.
 paging=[]
 for start in (0,1):
  path='/source/api/v1/search?path=main&projects=alpha&projects=beta&start='+str(start)+'&maxresults=1'
  pair=[]
  for label,portnum in (('oracle',op),('candidate',cp)):
   with opener.open(f'http://127.0.0.1:{portnum}'+path,timeout=20) as response:
    data=response.read(); status=response.status
   filename=f'page-{start}-{label}.body';(out/filename).write_bytes(data)
   pair.append({'endpoint':label,'status':status,'body':filename,'parsed':json.loads(data)})
  paging.append({'path':path,'oracle_status':pair[0]['status'],'candidate_status':pair[1]['status'],'body_matches_except_time':comparison(pair[0]['parsed'],pair[1]['parsed']),'oracle_ordered_paths':list(pair[0]['parsed']['results']),'candidate_ordered_paths':list(pair[1]['parsed']['results']),'oracle_body':pair[0]['body'],'candidate_body':pair[1]['body']})
 (out/'paging-comparison.json').write_text(json.dumps({'cases':paging,'scope':'two pages of one cross-project query; no general parity assertion'},indent=2)+'\n')
 def monitor():
  while not stop.wait(.05):
   p=subprocess.run(['ps','-p',f'{oracle.pid},{candidate.pid}','-o','pid=,ppid=,rss=,%cpu=,time='],capture_output=True,text=True)
   snapshots.append({'wall_ns':time.time_ns(),'raw_ps':p.stdout})
 thread=threading.Thread(target=monitor,daemon=True);thread.start()
 cmd=[sys.executable,str(root/'tools/search_live_benchmark.py'),'--oracle',f'http://127.0.0.1:{op}'+query,'--candidate',f'http://127.0.0.1:{cp}'+query,'--output',str(out/'run'),'--warmup',str(a.warmup),'--samples',str(a.samples)]
 proc=subprocess.run(cmd,cwd=root,capture_output=True,text=True); stop.set();thread.join(); (out/'harness.stdout.txt').write_text(proc.stdout);(out/'harness.stderr.txt').write_text(proc.stderr)
 (out/'resources.json').write_text(json.dumps({'note':'50ms process snapshots, RSS KiB and instantaneous ps CPU percent; RSS includes shared pages and is not measured peak. Short processes and child processes may be missed.','oracle_pid':oracle.pid,'candidate_pid':candidate.pid,'samples':snapshots},indent=2)+'\n')
 def sha(p):return hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()
 def total(p):return sum(f.stat().st_size for f in pathlib.Path(p).rglob('*') if f.is_file())
 def cmdout(*c):return subprocess.check_output(c,text=True).strip()
 meta={'scope':'path-only live HTTP experiment; not full product parity or representative workload','case':a.case,'query':query,'time':time.strftime('%Y-%m-%dT%H:%M:%S%z'),'platform':platform.platform(),'cpu':cmdout('sysctl','-n','machdep.cpu.brand_string'),'memory_bytes':int(cmdout('sysctl','-n','hw.memsize')),'load_average':os.getloadavg(),'cache':f'OS page cache and index caches not flushed; {a.warmup} warmup requests per endpoint per load','source_bytes':source_bytes,'public_corpus_manifest_sha256':sha(corpus/'manifest.json'),'oracle_baseline':fresh_receipt,'candidate_index_command':index_cmd,'candidate_index_exit':index_process.returncode,'corpus_verified_after_run':baseline.check_corpus(corpus)==manifest,'both_indexes_rebuilt_from_verified_corpus':True,'original_source_root_verified':str((corpus/'source').resolve()),'oracle_configuration_sha256':sha(config),'config_change':'allowMostPopular explicitly false; upstream default is true; most-popular is deferred','oracle_index_bytes':total(work/'index'),'oracle_index_tree_sha256':tree_hash(work/'index'),'candidate_index_bytes':index.stat().st_size,'candidate_index_sha256':sha(index),'candidate_binary_sha256':sha(binary),'candidate_lockfile_sha256':sha(root/'experiments/search-spike/Cargo.lock'),'rustc':cmdout('rustc','--version'),'cargo':cmdout('cargo','--version'),'JVM_options':env['CATALINA_OPTS'],'measurement_command':cmd,'benchmark_exit':proc.returncode,'limitations':['5 clients are a synthetic burst, endpoints measured sequentially, no concurrent production users','small source corpus only','only one path query per run','OS caches/background load/power not controlled','resource snapshots are not peak; only root server processes tracked','candidate scan is experimental, not complete OpenGrok REST or product']}
 (out/'metadata.json').write_text(json.dumps(meta,indent=2)+'\n');print('benchmark exit',proc.returncode); print((out/'run/report.json').read_text()[:3500]);sys.exit(proc.returncode)
finally:
 stop.set()
 for p in processes:
  p.terminate()
 for p in processes:
  try:p.wait(timeout=10)
  except subprocess.TimeoutExpired:p.kill();p.wait()
 log.close();rlog.close()
 after=tree_hash(base)
 fresh_after=tree_hash(fresh)
 (out/'capture-integrity.json').write_text(json.dumps({'original_capture_tree_sha256_before':original_hash,'original_capture_tree_sha256_after_run':after,'original_capture_unchanged':original_hash==after,**copy_receipt,'fresh_capture_tree_sha256_after_run':fresh_after,'fresh_capture_unchanged':fresh_after==copy_receipt['capture_tree_sha256_before']},indent=2)+'\n')
 shutil.rmtree(work)
 shutil.rmtree(fresh)
 index.unlink()
 if fresh_after!=copy_receipt['capture_tree_sha256_before']: raise RuntimeError('fresh capture changed during experiment')
 if original_hash!=after: raise RuntimeError('original capture changed during experiment')
