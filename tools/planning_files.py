"""Select explicitly indexed repository files for a planning package."""
import os
from pathlib import Path
import subprocess


EXCLUDED={'.git','dist','target','node_modules','__pycache__'}


def git(root,*args):
    env={key:value for key,value in os.environ.items() if not key.startswith('GIT_')}
    return subprocess.run(['git','-C',str(root),*args],env=env,capture_output=True)


def require_repository_root(root):
    root=Path(root).resolve(strict=True)
    result=git(root,'rev-parse','--show-toplevel')
    if result.returncode or Path(os.fsdecode(result.stdout.strip())).resolve()!=root:
        raise ValueError('planning package requires the root of a Git worktree')
    return root


def require_staged_worktree(root):
    root=require_repository_root(root)
    result=git(root,'diff','--quiet','--no-ext-diff','--')
    if result.returncode==1:raise ValueError('stage all tracked changes before building the planning package')
    if result.returncode:raise ValueError('could not inspect worktree changes')


def package_files(root):
    root=require_repository_root(root)
    result=git(root,'ls-files','--cached','-z')
    if result.returncode:raise ValueError('could not enumerate staged repository paths')
    paths=[]
    for raw in result.stdout.split(b'\0'):
        if not raw:continue
        rel=Path(os.fsdecode(raw))
        if EXCLUDED.intersection(rel.parts):continue
        path=root/rel
        if rel.is_absolute() or '..' in rel.parts or any((root/Path(*rel.parts[:i])).is_symlink() for i in range(1,len(rel.parts)+1)):
            raise ValueError('symlinks or unsafe repository paths cannot enter a planning package')
        if not path.is_file() or root not in path.resolve(strict=True).parents:
            raise ValueError('staged package path missing or outside repository: '+str(rel))
        paths.append(path)
    return sorted(set(paths))
