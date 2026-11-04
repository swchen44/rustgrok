#!/usr/bin/env python3
"""Capture fixed OpenGrok FULL stream and term-normalization correctness, not timing."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

import capture_opengrok_baseline as baseline

ROOT = Path(__file__).resolve().parents[1]
CACHE = Path.home() / 'Library/Caches/rustgrok/oracle'
TOOLS = CACHE / 'toolchains'
JDK = TOOLS / 'jdk-21.0.13+9/Contents/Home'
LIB = CACHE / 'opengrok-1.14.20/release/opengrok-1.14.20/lib'
FIX = ROOT / 'tests/fixtures/full-analysis-2026-11-04'
HELPER = ROOT / 'tests/evidence/full-analysis-2026-11-04/FullAnalysisOracle.java'
PUBLIC = {'c':ROOT/'tests/fixtures/public-source/c/main.c', 'python':ROOT/'tests/fixtures/public-source/python/main.py.fixture', 'javascript':ROOT/'tests/fixtures/public-source/javascript/function.js'}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--extra', action='store_true', help='capture finite bounds and mapping cases')
    args = parser.parse_args()
    output = args.output.parent.resolve(strict=True) / args.output.name
    if output.exists() or output.is_symlink() or output == ROOT or ROOT in output.parents:
        parser.error('output must be a new directory outside this repository')
    archive = CACHE/'opengrok-1.14.20/opengrok-1.14.20.tar.gz'
    jdk_archive = TOOLS/'OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.13_9.tar.gz'
    baseline.verified(archive, baseline.RELEASE_SHA)
    baseline.verified(jdk_archive, baseline.JDK_SHA)
    for name in ('java','javac'):
        baseline.verified_archive_member(jdk_archive, f'jdk-21.0.13+9/Contents/Home/bin/{name}', JDK/'bin'/name)
    inventory = json.loads((ROOT/'upstream/opengrok-release-inventory.json').read_text())
    baseline.verified(LIB/'opengrok.jar', inventory['indexer_jar']['sha256'])
    jars = {'opengrok.jar': sha(LIB/'opengrok.jar')}
    for jar in sorted(LIB.glob('*.jar')):
        if jar.name == 'opengrok.jar': continue
        item=next((i for i in inventory['web_war']['web_inf_lib'] if Path(i['path']).name == jar.name),None)
        if item is not None:
            jars[jar.name]=baseline.verified(jar,item['sha256'])
    if not any(name.startswith('lucene-core-9.12.3') for name in jars): raise RuntimeError('fixed Lucene missing')
    upstream=CACHE/'opengrok-1.14.20/source'
    inventory_source=json.loads((ROOT/'upstream/opengrok-source-tree.json').read_text())
    files=['opengrok-indexer/src/main/jflex/analysis/plain/PlainFullTokenizer.lex','opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/FileAnalyzer.java','opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/CompatibleAnalyser.java','opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/JFlexTokenizer.java','opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/JFlexSymbolMatcher.java','opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/plain/PlainAnalyzer.java','opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/plain/AbstractSourceCodeAnalyzer.java','opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/TextAnalyzer.java','opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/ExpandTabsReader.java','opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/c/CAnalyzer.java','opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/python/PythonAnalyzer.java','opengrok-indexer/src/main/java/org/opengrok/indexer/analysis/javascript/JavaScriptAnalyzer.java','opengrok-indexer/src/main/java/org/opengrok/indexer/search/CustomQueryParser.java']
    source={}
    for name in files:
        item=next(i for i in inventory_source['files'] if i['path']==name)
        source[name]=baseline.verified(upstream/name,item['sha256'])
    env={k:v for k,v in os.environ.items() if k not in {'JAVA_TOOL_OPTIONS','JDK_JAVA_OPTIONS','_JAVA_OPTIONS','JAVA_OPTS','CLASSPATH','JAVA_HOME','JRE_HOME'}}
    output.mkdir();classes=Path(tempfile.mkdtemp(prefix='rustgrok-full-oracle-'))
    cp=os.pathsep.join(str(LIB/name) for name in jars)
    compile_cmd=[str(JDK/'bin/javac'),'-cp',cp,'-d',str(classes),str(HELPER)]
    compiled=subprocess.run(compile_cmd,capture_output=True,env=env,timeout=120)
    (output/'javac.stdout').write_bytes(compiled.stdout);(output/'javac.stderr').write_bytes(compiled.stderr)
    if compiled.returncode:raise RuntimeError('javac failed')
    cases=[]
    for lang,path in PUBLIC.items():
        cases += [(f'corpus-{lang}-{kind}','stream',kind,path) for kind in (lang,'query')]
    for case in ('ascii','unicode'):
        cases += [(f'{case}-{kind}','stream',kind,FIX/(case+'.txt')) for kind in ('c','python','javascript','query')]
    cases.append(('long-query','stream','query',FIX/'long.txt'))
    for case in ('norm-ascii','norm-turkish','norm-sigma','norm-chinese'):
        cases += [(f'{case}-{kind}','normalize',kind,FIX/(case+'.txt')) for kind in ('c','python','javascript','query')]
    if args.extra:
        cases = [
            ('ascii-all-query','stream','query',FIX/'ascii-all.txt'),
            ('cjk-limit-query','stream','query',FIX/'cjk-limit.txt'),
            ('surrogate-boundary-query','stream','query',FIX/'surrogate-boundary.txt'),
            ('norm-mapping-query','normalize','query',FIX/'norm-mapping.txt'),
        ]
    observations=[]
    for name,mode,kind,path in cases:
        cmd=[str(JDK/'bin/java'),'-cp',str(classes)+os.pathsep+cp,'FullAnalysisOracle',mode,kind,str(path)]
        result=subprocess.run(cmd,capture_output=True,env=env,timeout=120)
        stdout=output/(name+'.stdout');stderr=output/(name+'.stderr');stdout.write_bytes(result.stdout);stderr.write_bytes(result.stderr)
        observations.append({'name':name,'mode':mode,'analyzer':kind,'input':str(path.relative_to(ROOT)),'input_sha256':sha(path),'stdout':stdout.name,'stdout_sha256':sha(stdout),'stderr':stderr.name,'stderr_sha256':sha(stderr),'exit':result.returncode,'command':cmd})
        if result.returncode:raise RuntimeError('oracle failed: '+name)
    receipt={'scope':'direct FULL analyzer tokenStream/normalize on decoded text; no reader expansion, query parsing, indexing, ranking or performance assertion','upstream_commit':baseline.COMMIT,'java_sha256':sha(JDK/'bin/java'),'javac_sha256':sha(JDK/'bin/javac'),'jdk_archive_sha256':baseline.JDK_SHA,'release_archive_sha256':baseline.RELEASE_SHA,'classpath_jars':jars,'upstream_source_sha256':source,'helper_sha256':sha(HELPER),'helper_class_sha256':sha(classes/'FullAnalysisOracle.class'),'javac_command':compile_cmd,'javac_exit':compiled.returncode,'javac_stdout_sha256':sha(output/'javac.stdout'),'javac_stderr_sha256':sha(output/'javac.stderr'),'cases':observations}
    (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n')
    print('captured',len(observations),'original cases; classes:',classes)

if __name__=='__main__': main()
