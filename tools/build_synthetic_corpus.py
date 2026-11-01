#!/usr/bin/env python3
"""Create or verify a bounded, deterministic synthetic search corpus."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
from search_benchmark_gate import no_duplicate_keys, strict_equal

ROOT=Path(__file__).resolve().parents[1]
RESERVE=6*1024**3
MAX_FILE=1024**2
EXTENSIONS=('c','py','js')


def content(project, number, size):
    ext=EXTENSIONS[number % len(EXTENSIONS)]
    marker='#' if ext=='py' else '//'
    header=f'{marker} generated synthetic project {project:02} file {number:03}\n'.encode()
    line=f'{marker} public synthetic benchmark content token repeated safely\n'.encode()
    body_size=size-len(header)
    return header + line*(body_size//len(line)) + b' '*(body_size%len(line))


def specification(projects, files_per_project, bytes_per_file):
    if not all(type(n) is int for n in (projects,files_per_project,bytes_per_file)) or not (1 <= projects <= 10 and 1 <= files_per_project <= 50 and 128 <= bytes_per_file <= MAX_FILE):
        raise ValueError('bounds: 1..10 projects, 1..50 files per project, 128..1048576 bytes per file')
    files=[]
    for project in range(projects):
        for number in range(files_per_project):
            extension=EXTENSIONS[number % len(EXTENSIONS)]
            path=f'project{project:02}/synthetic{number:03}.{extension}'
            data=content(project,number,bytes_per_file)
            files.append({'path':path,'bytes':len(data),'sha256':hashlib.sha256(data).hexdigest()})
    return {'schema_version':1,'kind':'independently generated synthetic comments; no real source or SCM history',
            'projects':projects,'files_per_project':files_per_project,'bytes_per_file':bytes_per_file,
            'source_bytes':projects*files_per_project*bytes_per_file,'files':files}


def check_space(path, reserve, upcoming):
    if reserve < RESERVE: raise ValueError('minimum free space cannot be less than 6 GiB')
    free=shutil.disk_usage(path).free
    if free-upcoming < reserve:
        raise ValueError(f'insufficient disk: free={free} upcoming={upcoming} reserve={reserve}')


def create(args):
    output=args.output.absolute()
    if output.exists() or output.is_symlink(): raise ValueError('output must not exist')
    parent=output.parent.resolve(strict=True)
    out=parent/output.name
    if out==ROOT or ROOT in out.parents: raise ValueError('output must be outside the repository')
    spec=specification(args.projects,args.files_per_project,args.bytes_per_file)
    check_space(parent,args.min_free_bytes,spec['source_bytes'])
    out.mkdir()
    for item in spec['files']:
        project=int(item['path'][7:9]); number=int(Path(item['path']).stem[9:])
        dest=out/'source'/item['path']; dest.parent.mkdir(parents=True,exist_ok=True)
        check_space(parent,args.min_free_bytes,item['bytes'])
        with dest.open('xb') as stream: stream.write(content(project,number,args.bytes_per_file))
    (out/'manifest.json').write_text(json.dumps(spec,indent=2)+'\n')
    print(json.dumps({'status':'CREATED','corpus':str(out),'source_bytes':spec['source_bytes'],'files':len(spec['files'])}))


def verify(args):
    root=args.corpus.resolve(strict=True)
    if args.corpus.is_symlink() or (root/'source').is_symlink() or (root/'manifest.json').is_symlink():
        raise ValueError('symlink not allowed')
    manifest=json.loads((root/'manifest.json').read_text(),object_pairs_hook=no_duplicate_keys)
    expected=specification(manifest['projects'],manifest['files_per_project'],manifest['bytes_per_file'])
    if not strict_equal(manifest,expected): raise ValueError('manifest differs from deterministic specification')
    found=set()
    for directory,dirs,files in os.walk(root/'source',followlinks=False):
        parent=Path(directory)
        if any((parent/n).is_symlink() for n in dirs+files): raise ValueError('symlink not allowed')
        found.update(str((parent/n).relative_to(root/'source')) for n in files)
    if found!={x['path'] for x in expected['files']}: raise ValueError('source file set differs')
    for item in expected['files']:
        data=(root/'source'/item['path']).read_bytes()
        if len(data)!=item['bytes'] or hashlib.sha256(data).hexdigest()!=item['sha256']:
            raise ValueError('source hash or length differs: '+item['path'])
    print(json.dumps({'status':'VERIFIED','source_bytes':expected['source_bytes'],'files':len(found),'projects':expected['projects']}))


def main():
    parser=argparse.ArgumentParser(description=__doc__); sub=parser.add_subparsers(dest='command',required=True)
    build=sub.add_parser('create');build.add_argument('--output',type=Path,required=True)
    build.add_argument('--projects',type=int,required=True);build.add_argument('--files-per-project',type=int,required=True)
    build.add_argument('--bytes-per-file',type=int,required=True);build.add_argument('--min-free-bytes',type=int,default=RESERVE)
    check=sub.add_parser('verify');check.add_argument('--corpus',type=Path,required=True)
    args=parser.parse_args()
    try: create(args) if args.command=='create' else verify(args)
    except (OSError,ValueError,KeyError,TypeError) as error:
        print(str(error),file=sys.stderr);return 1
    return 0

if __name__=='__main__': sys.exit(main())
