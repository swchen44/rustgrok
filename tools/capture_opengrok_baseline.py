#!/usr/bin/env python3
"""Capture a limited fixed OpenGrok search baseline over loopback on macOS."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import socket
import subprocess
import tarfile
import time
import urllib.error
import urllib.parse
import urllib.request
import xml.etree.ElementTree as ET

from build_public_corpus import FILES, SOURCE_HASHES, source_path, git as fixture_git

ROOT = Path(__file__).resolve().parents[1]
COMMIT = '3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3'
JDK_SHA = 'bf036b35b071bfb2f54f2c0af8ee8d88afa95ee6eb3dce02c0f94cfc2524d04a'
TOMCAT_SHA512 = 'aa06508300ca137a023b74b8600f2c1b3248412eb85d4fc5e2f337c6c4d3776f4491e272f79856ac541cfab0fc35537111ae4f3cfcd0bbe702c0a3610a61bd04'
RELEASE_SHA = 'f33d78f950ba7ed974caa9153120e4f26f9c7f4d63c1f48b7a0350e12d8e346d'
CASES = [
    ('full_alpha', [('full', 'main'), ('projects', 'alpha')]),
    ('path_alpha', [('path', 'main.c'), ('projects', 'alpha')]),
    ('definition_alpha', [('def', 'main'), ('projects', 'alpha')]),
    ('reference_beta', [('symbol', 'main'), ('projects', 'beta')]),
    ('full_both', [('full', 'main'), ('projects', 'alpha'), ('projects', 'beta')]),
]


def digest(path, algorithm='sha256'):
    h = hashlib.new(algorithm)
    with path.open('rb') as f:
        for block in iter(lambda: f.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def verified(path, expected, algorithm='sha256'):
    actual = digest(path, algorithm)
    if actual != expected:
        raise ValueError(f'hash mismatch for {path}: {actual}')
    return actual


def verified_archive_member(archive, member, path):
    with tarfile.open(archive) as tar:
        extracted = tar.extractfile(member)
        if extracted is None:
            raise ValueError(f'missing archive member: {member}')
        actual = hashlib.sha256(extracted.read()).hexdigest()
    return verified(path, actual)


def verified_tomcat_runtime(archive, home):
    """Match every actual bin/conf/lib file to the already-verified archive."""
    sections = {'bin', 'conf', 'lib'}
    expected = {}
    with tarfile.open(archive) as tar:
        for member in tar:
            parts = Path(member.name).parts
            if len(parts) < 2 or parts[0] != 'apache-tomcat-10.1.60' or parts[1] not in sections:
                continue
            if member.isdir():
                continue
            if not member.isfile() or any(part in ('.', '..') for part in parts):
                raise ValueError('Tomcat runtime mismatch: unsupported archive member')
            rel = Path(*parts[1:])
            if str(rel) in expected:
                raise ValueError('Tomcat runtime mismatch: duplicate archive member')
            data = tar.extractfile(member).read()
            target = home / rel
            if not target.is_file() or target.is_symlink():
                raise ValueError(f'Tomcat runtime mismatch: missing or symlink {rel}')
            expected_sha = hashlib.sha256(data).hexdigest()
            if digest(target) != expected_sha or bool(target.stat().st_mode & 0o100) != bool(member.mode & 0o100):
                raise ValueError(f'Tomcat runtime mismatch: bytes or executable mode {rel}')
            expected[str(rel)] = {'path': str(rel), 'bytes': len(data), 'sha256': expected_sha,
                                  'owner_executable': bool(member.mode & 0o100)}
    found = set()
    for section in sections:
        root = home / section
        if root.is_symlink() or not root.is_dir():
            raise ValueError(f'Tomcat runtime mismatch: missing or symlink directory {section}')
        for directory, dirs, files in os.walk(root, followlinks=False):
            parent = Path(directory)
            if any((parent / name).is_symlink() for name in dirs + files):
                raise ValueError('Tomcat runtime mismatch: unexpected symlink')
            found.update(str((parent / name).relative_to(home)) for name in files)
    if found != set(expected):
        raise ValueError('Tomcat runtime mismatch: missing or extra files')
    return [expected[key] for key in sorted(expected)]


def check_corpus(corpus):
    manifest = corpus / 'manifest.json'
    if not manifest.is_file():
        raise ValueError('corpus manifest missing')
    data = json.loads(manifest.read_text())
    if data.get('upstream_commit') != COMMIT or data.get('private_data') is not False:
        raise ValueError('corpus provenance is not the fixed public fixture')
    expected = {(project + '/' + component, Path(rel).name): SOURCE_HASHES[rel]
                for project, component, _language, rel in FILES}
    actual = {(item['path'], item['file']): item['source_sha256'] for item in data['repositories']}
    if actual != expected or len(data['repositories']) != len(expected):
        raise ValueError('corpus does not match the fixed public fixtures')
    for rel, sha in SOURCE_HASHES.items():
        if digest(source_path(rel)) != sha:
            raise ValueError('fixed source fixture hash mismatch')
    root = corpus / 'source'
    expected_files = {str(Path(repo) / filename) for repo, filename in expected}
    discovered = set()
    for directory, dirs, files in os.walk(root, followlinks=False):
        parent = Path(directory)
        for name in list(dirs):
            candidate = parent / name
            if candidate.is_symlink():
                raise ValueError('unexpected source files or symlinks')
            if name == '.git':
                if str(parent.relative_to(root)) not in {repo for repo, _ in expected}:
                    raise ValueError('unexpected source files or repositories')
                dirs.remove(name)
        for name in files:
            candidate = parent / name
            if candidate.is_symlink():
                raise ValueError('unexpected source files or symlinks')
            discovered.add(str(candidate.relative_to(root)))
    if discovered != expected_files:
        raise ValueError('unexpected source files in corpus')
    for item in data['repositories']:
        repo = (corpus / 'source' / item['path']).resolve()
        source = (corpus / 'source').resolve()
        if source not in repo.parents:
            raise ValueError('repository path outside corpus')
        if digest(repo / item['file']) != item['source_sha256']:
            raise ValueError('corpus source hash mismatch')
        head = fixture_git(repo, 'rev-parse', 'HEAD')
        dirty = fixture_git(repo, 'status', '--porcelain')
        if head != item['revision'] or dirty:
            raise ValueError('corpus Git revision changed or worktree dirty')
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--corpus', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--cache', type=Path, default=Path.home() / 'Library/Caches/rustgrok/oracle')
    parser.add_argument('--ctags', type=Path, default=Path('/opt/homebrew/bin/ctags'))
    args = parser.parse_args()
    corpus, output, cache = args.corpus.resolve(), args.output.resolve(), args.cache.resolve()
    data = check_corpus(corpus)
    if output == corpus or corpus in output.parents:
        parser.error('output must not be inside corpus')
    if output.exists() or output == ROOT or ROOT in output.parents:
        parser.error('output must be a new directory outside the repository')
    protected = (cache / 'toolchains', cache / 'opengrok-1.14.20')
    if any(p == output or p in output.parents for p in protected):
        parser.error('output must not be inside a toolchain or upstream source cache')
    tools = cache / 'toolchains'
    jdk_archive = tools / 'OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.13_9.tar.gz'
    tomcat_archive = tools / 'apache-tomcat-10.1.60.tar.gz'
    release_archive = cache / 'opengrok-1.14.20/opengrok-1.14.20.tar.gz'
    verified(jdk_archive, JDK_SHA)
    verified(tomcat_archive, TOMCAT_SHA512, 'sha512')
    verified(release_archive, RELEASE_SHA)
    java_home = tools / 'jdk-21.0.13+9/Contents/Home'
    tomcat = tools / 'apache-tomcat-10.1.60'
    release = cache / 'opengrok-1.14.20/release/opengrok-1.14.20'
    tomcat_runtime = verified_tomcat_runtime(tomcat_archive, tomcat)
    inventory = json.loads((ROOT / 'upstream/opengrok-release-inventory.json').read_text())
    jar, war = release / 'lib/opengrok.jar', release / 'lib/source.war'
    verified(jar, inventory['indexer_jar']['sha256'])
    verified(war, inventory['web_war']['sha256'])
    output.mkdir(parents=True)
    (output / 'tomcat-runtime-manifest.json').write_text(json.dumps(
        {'scope': ['bin', 'conf', 'lib'], 'files': tomcat_runtime}, indent=2) + '\n')
    index, config = output / 'index', output / 'configuration.xml'
    index.mkdir()
    java = java_home / 'bin/java'
    javac = java_home / 'bin/javac'
    verified_archive_member(jdk_archive, 'jdk-21.0.13+9/Contents/Home/bin/java', java)
    verified_archive_member(jdk_archive, 'jdk-21.0.13+9/Contents/Home/bin/javac', javac)
    index_cmd = [str(java), '-Xms128m', '-Xmx512m', '-jar', str(jar), '-c', str(args.ctags),
                 '-s', str(corpus / 'source'), '-d', str(index), '-W', str(config), '-P', '-S', '-H', '-T', '2']
    ignored_java_env = {'JAVA_TOOL_OPTIONS', 'JDK_JAVA_OPTIONS', '_JAVA_OPTIONS', 'JAVA_OPTS',
                        'CLASSPATH', 'JAVA_HOME', 'JRE_HOME', 'CATALINA_OPTS', 'CATALINA_HOME',
                        'CATALINA_BASE', 'CATALINA_TMPDIR'}
    oracle_env = {key: value for key, value in os.environ.items()
                  if not key.startswith('GIT_') and key not in ignored_java_env}
    oracle_env.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull)
    p = subprocess.run(index_cmd, capture_output=True, timeout=180, env=oracle_env)
    (output / 'index.stdout.txt').write_bytes(p.stdout)
    (output / 'index.stderr.txt').write_bytes(p.stderr)
    if p.returncode != 0:
        raise RuntimeError(f'OpenGrok indexer failed: exit {p.returncode}; see {output}')
    base = output / 'tomcat'
    shutil.copytree(tomcat / 'conf', base / 'conf')
    for name in ('logs', 'temp', 'work', 'webapps', 'conf/Catalina/localhost'):
        (base / name).mkdir(parents=True, exist_ok=True)
    server = ET.parse(base / 'conf/server.xml')
    server.getroot().set('port', '-1')
    connectors = server.getroot().findall('./Service/Connector')
    if len(connectors) != 1:
        raise ValueError('expected exactly one Tomcat connector for loopback-only oracle')
    connector = connectors[0]
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    connector.set('address', '127.0.0.1')
    connector.set('port', str(port))
    server.write(base / 'conf/server.xml', encoding='utf-8', xml_declaration=True)
    context = ET.Element('Context', {'docBase': str(war)})
    ET.SubElement(context, 'Parameter', {'name': 'CONFIGURATION', 'value': str(config), 'override': 'false'})
    ET.ElementTree(context).write(base / 'conf/Catalina/localhost/source.xml', encoding='utf-8', xml_declaration=True)
    environment = oracle_env.copy()
    environment.pop('JRE_HOME', None)
    environment.pop('JAVA_TOOL_OPTIONS', None)
    environment.pop('JDK_JAVA_OPTIONS', None)
    environment.update(JAVA_HOME=str(java_home), CATALINA_HOME=str(tomcat), CATALINA_BASE=str(base),
                       CATALINA_TMPDIR=str(base / 'temp'), CATALINA_OPTS=(
        '-Xms128m -Xmx512m --add-exports=java.base/jdk.internal.ref=ALL-UNNAMED '
        '--add-exports=java.base/sun.nio.ch=ALL-UNNAMED --add-exports=jdk.unsupported/sun.misc=ALL-UNNAMED '
        '--add-exports=jdk.compiler/com.sun.tools.javac.file=ALL-UNNAMED '
        '--add-opens=jdk.compiler/com.sun.tools.javac=ALL-UNNAMED --add-opens=java.base/java.lang=ALL-UNNAMED '
        '--add-opens=java.base/java.lang.reflect=ALL-UNNAMED --add-opens=java.base/java.io=ALL-UNNAMED '
        '--add-opens=java.base/java.util=ALL-UNNAMED'))
    responses = []
    with (output / 'tomcat.stdout.txt').open('wb') as log:
        proc = subprocess.Popen([str(tomcat / 'bin/catalina.sh'), 'run'], stdout=log,
                                stderr=subprocess.STDOUT, env=environment, start_new_session=True)
        try:
            base_url = f'http://127.0.0.1:{port}/source/'
            opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
            for _ in range(45):
                if proc.poll() is not None:
                    raise RuntimeError(f'Tomcat exited before readiness: {proc.returncode}')
                try:
                    with opener.open(base_url, timeout=2) as r:
                        if r.status == 200:
                            break
                except (OSError, urllib.error.HTTPError):
                    time.sleep(1)
            else:
                raise RuntimeError('Tomcat readiness timeout')
            for name, params in CASES:
                path = '/source/api/v1/search?' + urllib.parse.urlencode(params)
                try:
                    with opener.open(f'http://127.0.0.1:{port}' + path, timeout=20) as response:
                        body, status = response.read(), response.status
                except urllib.error.HTTPError as err:
                    body, status = err.read(), err.code
                filename = 'response-' + name + '.json'
                (output / filename).write_bytes(body)
                responses.append({'case': name, 'path': path, 'status': status, 'file': filename,
                                  'bytes': len(body), 'sha256': hashlib.sha256(body).hexdigest()})
                if status != 200:
                    raise RuntimeError(f'{name} returned HTTP {status}')
        finally:
            proc.terminate()
            try:
                proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                proc.kill(); proc.wait()
    if check_corpus(corpus) != data:
        raise RuntimeError('corpus changed during oracle capture')
    version = subprocess.check_output([str(java), '--version'], text=True, env=oracle_env)
    ctags_version = subprocess.check_output([str(args.ctags), '--version'], text=True, env=oracle_env)
    git_path = Path(shutil.which('git')).resolve()
    git_version = subprocess.check_output([str(git_path), '--version'], text=True, env=oracle_env)
    javac_version = subprocess.check_output([str(javac), '--version'], text=True, env=oracle_env)
    javac_format = subprocess.check_output(['file', '-b', str(javac)], text=True).strip()
    macos_version = subprocess.check_output(['sw_vers', '-productVersion'], text=True).strip()
    macos_build = subprocess.check_output(['sw_vers', '-buildVersion'], text=True).strip()
    result = {'schema_version': 1, 'role': 'limited_original_oracle_capture_not_full_acceptance',
              'captured_at': datetime.datetime.now().astimezone().isoformat(timespec='seconds'),
              'upstream_commit': COMMIT, 'archive_sha256': RELEASE_SHA,
              'jdk_archive_sha256': JDK_SHA, 'jdk_archive_bytes': jdk_archive.stat().st_size,
              'tomcat_archive_sha512': TOMCAT_SHA512,
              'jar_sha256': digest(jar), 'war_sha256': digest(war), 'java_sha256': digest(java),
              'ctags_sha256': digest(args.ctags), 'java_version': version, 'ctags_version': ctags_version,
              'javac_sha256': digest(javac), 'javac_version': javac_version, 'javac_format': javac_format,
              'git_path': str(git_path), 'git_sha256': digest(git_path), 'git_version': git_version,
              'tomcat_version': '10.1.60', 'tomcat_runtime_scope': ['bin', 'conf', 'lib'],
              'tomcat_runtime_file_count': len(tomcat_runtime),
              'tomcat_runtime_manifest_sha256': digest(output / 'tomcat-runtime-manifest.json'),
              'platform': {'system': platform.system(), 'release': platform.release(), 'machine': platform.machine(),
                           'macos_version': macos_version, 'macos_build': macos_build},
              'corpus_manifest_sha256': digest(corpus / 'manifest.json'), 'repositories': data['repositories'],
              'corpus_verified_unchanged_after_run': True,
              'configuration_sha256': digest(config), 'index_command': index_cmd,
              'index_exit_code': p.returncode, 'loopback': '127.0.0.1', 'port': port,
              'tomcat_exit_code': proc.returncode, 'responses': responses,
              'index_stdout_sha256': digest(output / 'index.stdout.txt'),
              'index_stderr_sha256': digest(output / 'index.stderr.txt'),
              'tomcat_log_sha256': digest(output / 'tomcat.stdout.txt'),
              'limitations': ['small public corpus, not representative workload',
                              'only five query cases; not full query/SCM/UI compatibility',
                              'unmodified raw responses include nondeterministic time field',
                              'no product runtime, performance or full test-case PASS asserted',
                              'JDK archive and java/javac binaries verified; extracted JDK library tree not fully rehashed']}
    (output / 'result.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps({'output': str(output), 'cases': len(responses), 'statuses': [r['status'] for r in responses]}))


if __name__ == '__main__':
    main()
