#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# Copyright (c) 2026 swchen44
"""Capture pinned reader/FULL/query/postings correctness serially; argument is a new output directory."""
import sys,hashlib,json,subprocess,tempfile,os
from pathlib import Path
root=Path(__file__).resolve().parents[1];sys.path.insert(0,str(root/'tools'))
import capture_opengrok_baseline as base
cache=Path.home()/'Library/Caches/rustgrok/oracle';lib=cache/'opengrok-1.14.20/release/opengrok-1.14.20/lib';jdk=cache/'toolchains/jdk-21.0.13+9/Contents/Home';out=Path(sys.argv[1]).resolve();out.mkdir(exist_ok=False)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
base.verified(cache/'opengrok-1.14.20/opengrok-1.14.20.tar.gz',base.RELEASE_SHA)
archive=cache/'toolchains/OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.13_9.tar.gz';base.verified(archive,base.JDK_SHA)
for name in ('java','javac'):base.verified_archive_member(archive,f'jdk-21.0.13+9/Contents/Home/bin/{name}',jdk/'bin'/name)
prev=json.loads((root/'tests/evidence/full-analysis-2026-11-04/receipt.json').read_text());jars=prev['classpath_jars']
for name,digest in jars.items():base.verified(lib/name,digest)
source=cache/'opengrok-1.14.20/source';inventory=json.loads((root/'upstream/opengrok-source-tree.json').read_text());sources={}
for suffix in ('analysis/TextAnalyzer.java','analysis/ZeroReader.java','analysis/plain/PlainAnalyzer.java','analysis/ExpandTabsReader.java','util/IOUtils.java','search/CustomQueryParser.java'):
 name='opengrok-indexer/src/main/java/org/opengrok/indexer/'+suffix;item=next(x for x in inventory['files'] if x['path']==name);sources[name]=base.verified(source/name,item['sha256'])
env={k:v for k,v in os.environ.items() if k not in {'JAVA_TOOL_OPTIONS','JDK_JAVA_OPTIONS','_JAVA_OPTIONS','JAVA_OPTS','CLASSPATH','JAVA_HOME','JRE_HOME'}}
cp=os.pathsep.join(str(lib/n) for n in jars);helper=root/'tests/evidence/full-postings-2026-11-05/FullPostingsOracle.java';classes=Path(tempfile.mkdtemp(prefix='rustgrok-reader-oracle-'))
compile_cmd=[str(jdk/'bin/javac'),'-cp',cp,'-d',str(classes),str(helper)];r=subprocess.run(compile_cmd,capture_output=True,env=env,timeout=60)
(out/'javac.stdout').write_bytes(r.stdout);(out/'javac.stderr').write_bytes(r.stderr)
if r.returncode:raise RuntimeError(r.stderr.decode())
records=[]
cases=[('corpus-c','c',root/'tests/fixtures/public-source/c/main.c',['main','int','alpha']),('corpus-python','python',root/'tests/fixtures/public-source/python/main.py.fixture',['main','def','value']),('corpus-js','javascript',root/'tests/fixtures/public-source/javascript/function.js',['function','return','value'])]
for fixture in sorted((root/'tests/fixtures/full-reader-2026-11-05').glob('*.hex')):
 path=classes/fixture.stem;path.write_bytes(bytes.fromhex(fixture.read_text()))
 for lang in ('c','python','javascript'):cases.append((fixture.name.split('.')[0]+'-'+lang,lang,path,['Alpha','Beta','alpha_1','end','İ','Σ','AΣ','中']))
for name,lang,path,terms in cases:
 cmd=[str(jdk/'bin/java'),'-cp',str(classes)+os.pathsep+cp,'FullPostingsOracle',lang,str(path),*terms]
 r=subprocess.run(cmd,capture_output=True,env=env,timeout=60)
 stdout=out/(name+'.stdout');stderr=out/(name+'.stderr');stdout.write_bytes(r.stdout);stderr.write_bytes(r.stderr)
 records.append({'name':name,'language':lang,'input':str(path),'input_hex_fixture':('tests/fixtures/full-reader-2026-11-05/'+name.rsplit('-',1)[0]+'.bin.hex') if path.parent==classes else None,'input_sha256':sha(path),'terms':terms,'exit':r.returncode,'command':cmd,'stdout':stdout.name,'stdout_sha256':sha(stdout),'stderr':stderr.name,'stderr_sha256':sha(stderr)})
 if r.returncode:raise RuntimeError(name+': '+r.stderr.decode())
binaries=Path.home()/'Library/Caches/rustgrok/evidence-binaries';binaries.mkdir(exist_ok=True)
cl=classes/'FullPostingsOracle.class';dest=binaries/sha(cl);dest.write_bytes(cl.read_bytes())
receipt={'scope':'serial independent reader, FULL stream, original CustomQueryParser single-term Lucene hits/postings; no timing/ranking/full indexer acceptance','upstream_commit':base.COMMIT,'release_archive_sha256':base.RELEASE_SHA,'jdk_archive_sha256':base.JDK_SHA,'java_sha256':sha(jdk/'bin/java'),'javac_sha256':sha(jdk/'bin/javac'),'classpath_jars':jars,'upstream_source_sha256':sources,'helper_sha256':sha(helper),'class_sha256':sha(cl),'retained_class':str(dest),'javac_command':compile_cmd,'capture_script_sha256':sha(Path(__file__)),'cases':records}
(out/'oracle-receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
print('captured',len(records),'serial original reader/postings cases; Java exited before Rust work')
