import sys,os,shutil,pathlib,xml.etree.ElementTree as ET,subprocess,socket,time,urllib.request,urllib.error,urllib.parse,json,hashlib,argparse,tempfile
from prepare_oracle_copy import tree_hash
import capture_opengrok_baseline as baseline
parser=argparse.ArgumentParser(description='Capture selected-project ordering from one isolated original index')
parser.add_argument('--capture',required=True,type=pathlib.Path);parser.add_argument('--output',required=True,type=pathlib.Path);args=parser.parse_args()
base=args.capture.resolve(); cache=pathlib.Path.home()/'Library/Caches/rustgrok/oracle';tomcat=cache/'toolchains/apache-tomcat-10.1.60';jdk=cache/'toolchains/jdk-21.0.13+9/Contents/Home'; war=cache/'opengrok-1.14.20/release/opengrok-1.14.20/lib/source.war';out=args.output.resolve(); root=pathlib.Path(__file__).resolve().parents[1]
if out==base or base in out.parents or out==root or root in out.parents: parser.error('output must be outside capture and repository')
receipt=json.loads((base/'result.json').read_text());expected=receipt['configuration_sha256']
if hashlib.sha256((base/'configuration.xml').read_bytes()).hexdigest()!=expected: parser.error('capture config differs from receipt')
baseline.verified_tomcat_runtime(cache/'toolchains/apache-tomcat-10.1.60.tar.gz',tomcat)
baseline.verified(war,receipt['war_sha256'])
baseline.verified_archive_member(cache/'toolchains/OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.13_9.tar.gz','jdk-21.0.13+9/Contents/Home/bin/java',jdk/'bin/java')
out.mkdir();work=pathlib.Path(tempfile.mkdtemp(prefix='rustgrok-project-order-'));before=tree_hash(base)
shutil.copytree(base/'index',work/'index');config=ET.parse(base/'configuration.xml');config.find('.//void[@property="dataRoot"]/string').text=str(work/'index');config.write(work/'configuration.xml',encoding='utf-8',xml_declaration=True)
shutil.copytree(tomcat/'conf',work/'tomcat/conf')
for name in ('logs','temp','work','webapps','conf/Catalina/localhost'):(work/'tomcat'/name).mkdir(parents=True,exist_ok=True)
with socket.socket() as sock:sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
server=ET.parse(work/'tomcat/conf/server.xml');server.getroot().set('port','-1');connector=server.getroot().find('./Service/Connector');connector.set('address','127.0.0.1');connector.set('port',str(port));server.write(work/'tomcat/conf/server.xml',encoding='utf-8',xml_declaration=True)
context=ET.Element('Context',{'docBase':str(war)});ET.SubElement(context,'Parameter',{'name':'CONFIGURATION','value':str(work/'configuration.xml'),'override':'false'});ET.ElementTree(context).write(work/'tomcat/conf/Catalina/localhost/source.xml',encoding='utf-8',xml_declaration=True)
env={k:v for k,v in os.environ.items() if not k.startswith('GIT_') and k not in {'JAVA_TOOL_OPTIONS','JDK_JAVA_OPTIONS','_JAVA_OPTIONS','JAVA_OPTS','CLASSPATH','JAVA_HOME','JRE_HOME','CATALINA_OPTS','CATALINA_HOME','CATALINA_BASE','CATALINA_TMPDIR'}};env.update(JAVA_HOME=str(jdk),CATALINA_HOME=str(tomcat),CATALINA_BASE=str(work/'tomcat'),CATALINA_TMPDIR=str(work/'tomcat/temp'),CATALINA_OPTS='-Xms128m -Xmx512m --add-exports=java.base/jdk.internal.ref=ALL-UNNAMED --add-exports=java.base/sun.nio.ch=ALL-UNNAMED --add-exports=jdk.unsupported/sun.misc=ALL-UNNAMED --add-exports=jdk.compiler/com.sun.tools.javac.file=ALL-UNNAMED --add-opens=jdk.compiler/com.sun.tools.javac=ALL-UNNAMED --add-opens=java.base/java.lang=ALL-UNNAMED --add-opens=java.base/java.lang.reflect=ALL-UNNAMED --add-opens=java.base/java.io=ALL-UNNAMED --add-opens=java.base/java.util=ALL-UNNAMED')
log=(out/'tomcat.log').open('wb');p=subprocess.Popen([str(tomcat/'bin/catalina.sh'),'run'],env=env,stdout=log,stderr=subprocess.STDOUT);opener=urllib.request.build_opener(urllib.request.ProxyHandler({}));cases=[]
try:
 for _ in range(90):
  try:
   with opener.open(f'http://127.0.0.1:{port}/source/',timeout=2):break
  except (OSError,urllib.error.URLError):time.sleep(1)
 else:raise RuntimeError('oracle startup timeout')
 for field in ('path','full'):
  for projects in (('alpha','beta'),('beta','alpha')):
   for start,maxresults in ((0,10),(0,1),(1,1),(2,1),(0,0),(100,1)):
    for repeat in range(2):
     args=[(field,'main')]+[('projects',x) for x in projects]+[('start',start),('maxresults',maxresults)];path='/source/api/v1/search?'+urllib.parse.urlencode(args)
     with opener.open(f'http://127.0.0.1:{port}'+path,timeout=20) as resp:raw=resp.read();status=resp.status
     name=f'{field}-{projects[0]}-start{start}-max{maxresults}-r{repeat}.json';(out/name).write_bytes(raw);value=json.loads(raw);cases.append({'field':field,'projects':projects,'start':start,'maxresults':maxresults,'repeat':repeat,'path':path,'status':status,'body':name,'sha256':hashlib.sha256(raw).hexdigest(),'resultCount':value['resultCount'],'ordered_paths':list(value['results'])})
finally:
 p.terminate()
 try:p.wait(timeout=10)
 except subprocess.TimeoutExpired:p.kill();p.wait()
 log.close()
after=tree_hash(base);shutil.rmtree(work);(out/'result.json').write_text(json.dumps({'scope':'original only, default popularity/config unchanged except dataRoot copied to isolated index and loopback port; same index and process for all requests','original_capture_tree_before':before,'original_capture_tree_after':after,'unchanged':before==after,'cases':cases},indent=2)+'\n');print('unchanged',before==after,'cases',len(cases))
if before != after: raise RuntimeError('original capture changed')
for field in ('path','full'):
 for start,maxresults in ((0,10),(0,1),(1,1),(2,1),(0,0),(100,1)):
  for first in ('alpha','beta'):
   c=next(x for x in cases if x['field']==field and x['projects'][0]==first and x['start']==start and x['maxresults']==maxresults and x['repeat']==0)
   print(field,start,maxresults,first,c['ordered_paths'])
