#!/usr/bin/env python3
"""Build a small reproducible public OpenGrok search corpus and Git histories."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
INPUT = ROOT / 'tests/fixtures/public-source'
UPSTREAM_COMMIT = '3ca9d73ebb0ddb5560fbc7631322d29d9fdc2ec3'
SOURCE_HASHES = {
    'c/main.c': '0afb5b45d1538ee86e23b046100c8e458614ba71e50f16c05bda1a52de0edb17',
    'javascript/function.js': '7e37130f0b5a078fea055a885632cdc00e1989def53e4fafe8b52a75824f8805',
    'python/main.py': 'c30bc720a38c39709e22f6bba1996b58c0a4e2e25d8cc6ea1bff7a348efde3f0',
}
FILES = [
    ('alpha', 'component-c', 'C', 'c/main.c'),
    ('alpha', 'component-js', 'JavaScript', 'javascript/function.js'),
    ('alpha', 'component-c/nested-js', 'JavaScript', 'javascript/function.js'),
    ('beta', 'component-c', 'C', 'c/main.c'),
    ('beta', 'component-py', 'Python', 'python/main.py'),
]


def source_path(rel):
    # Keep the upstream Python 2 input out of Python 3 module discovery.
    return INPUT / (rel + '.fixture' if rel == 'python/main.py' else rel)


def git(repo, *args, date='2020-01-01T00:00:00Z'):
    environment = os.environ.copy()
    for key in ('GIT_DIR', 'GIT_WORK_TREE', 'GIT_INDEX_FILE', 'GIT_OBJECT_DIRECTORY',
                'GIT_ALTERNATE_OBJECT_DIRECTORIES', 'GIT_CONFIG_COUNT', 'GIT_CONFIG_PARAMETERS'):
        environment.pop(key, None)
    environment.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull,
                       GIT_AUTHOR_NAME='Rustgrok Public Fixture', GIT_AUTHOR_EMAIL='fixture@rustgrok.invalid',
                       GIT_COMMITTER_NAME='Rustgrok Public Fixture', GIT_COMMITTER_EMAIL='fixture@rustgrok.invalid',
                       GIT_AUTHOR_DATE=date, GIT_COMMITTER_DATE=date)
    options = ['-c', 'core.hooksPath=' + os.devnull, '-c', 'core.autocrlf=false',
               '-c', 'core.excludesfile=' + os.devnull, '-c', 'commit.gpgsign=false',
               '-c', 'user.name=Rustgrok Public Fixture',
               '-c', 'user.email=fixture@rustgrok.invalid']
    return subprocess.check_output(['git', '-C', str(repo), *options, *args],
                                   env=environment, stderr=subprocess.PIPE, text=True).strip()


def build(output):
    # Never remove or replace existing data. Source roots live inside this new directory only.
    for rel, expected in SOURCE_HASHES.items():
        if hashlib.sha256(source_path(rel).read_bytes()).hexdigest() != expected:
            raise ValueError('fixed upstream fixture hash mismatch: ' + rel)
    output.mkdir(parents=True, exist_ok=False)
    source = output / 'source'
    template = output / 'empty-git-template'
    template.mkdir()
    entries = []
    for project, component, language, rel in FILES:
        repo = source / project / component
        repo.mkdir(parents=True)
        git(repo, 'init', '--quiet', '--object-format=sha1', '--initial-branch=main', '--template=' + str(template))
        if project == 'alpha' and component == 'component-c':
            # Keep the parent repository clean while the separate nested Git repo exists.
            (repo / '.git/info').mkdir(exist_ok=True)
            (repo / '.git/info/exclude').write_text('nested-js/\n')
        original = source_path(rel)
        name = Path(rel).name
        initial_name = 'old_' + name if language == 'C' else name
        path = repo / initial_name
        shutil.copyfile(original, path)
        path.chmod(0o644)
        git(repo, 'add', '--', initial_name)
        git(repo, 'commit', '--quiet', '-m', 'Add public upstream fixture')
        if language == 'C':
            git(repo, 'mv', '--', initial_name, name)
            git(repo, 'commit', '--quiet', '-m', 'Rename C fixture', date='2020-01-02T00:00:00Z')
        data = (repo / name).read_bytes()
        entries.append({'project': project, 'path': project + '/' + component,
                        'file': name, 'language': language,
                        'source_sha256': hashlib.sha256(data).hexdigest(),
                        'source_bytes': len(data), 'revision': git(repo, 'rev-parse', 'HEAD'),
                        'tree': git(repo, 'rev-parse', 'HEAD^{tree}'),
                        'commits': git(repo, 'rev-list', '--reverse', 'HEAD').splitlines(),
                        'origin': 'opengrok-indexer/src/test/resources/sources/' + rel})
    shutil.rmtree(template)
    manifest = {'schema_version': 1, 'role': 'public_small_search_fixture_not_performance_corpus',
                'upstream_repository': 'https://github.com/oracle/opengrok',
                'upstream_commit': UPSTREAM_COMMIT, 'private_data': False,
                'license': 'Retain upstream LICENSE.txt and NOTICE.txt and per-file terms',
                'repositories': entries}
    (output / 'manifest.json').write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + '\n')
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='new directory outside the repository')
    args = parser.parse_args()
    output = args.output.resolve()
    if output == ROOT or ROOT in output.parents:
        parser.error('output must be outside the repository')
    if output.exists():
        parser.error('output already exists; refusing to overwrite it')
    manifest = build(output)
    print(json.dumps({'output': str(output), 'repositories': len(manifest['repositories'])}))


if __name__ == '__main__':
    main()
