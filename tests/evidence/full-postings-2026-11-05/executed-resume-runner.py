# SPDX-License-Identifier: MIT
# Copyright (c) 2026 swchen44
import subprocess,sys,os,time,json,hashlib,shutil,signal,re
from pathlib import Path
root=Path('/Users/swchen.tw/git/rustgrok-full-postings');out=root/'tests/evidence/full-postings-2026-11-05';name=sys.argv[1];cmd=sys.argv[2:];reserve=2684354560;before=shutil.disk_usage(root).free
assert not (out/(name+'.json')).exists(), 'new run name required'
if before<reserve:raise SystemExit(f'BLOCKED_RESERVE before {name}: {before}')
env=os.environ.copy();env['CARGO_TARGET_DIR']='/Users/swchen.tw/git/rustgrok-durable-path/crates/path-index/target';env['CARGO_INCREMENTAL']='0';env['CARGO_BUILD_JOBS']='1';env['PYTHONDONTWRITEBYTECODE']='1'
minimum=before
with (out/(name+'.stdout')).open('wb') as so,(out/(name+'.stderr')).open('wb') as se:
 p=subprocess.Popen(cmd,cwd=root,stdout=so,stderr=se,env=env,start_new_session=True)
 blocked=False
 while p.poll() is None:
  free=shutil.disk_usage(root).free;minimum=min(minimum,free)
  if free<reserve:blocked=True;os.killpg(p.pid,signal.SIGTERM);break
  time.sleep(.2)
 code=p.wait()
paths=[]
for line in (out/(name+'.stderr')).read_text(errors='replace').splitlines():
 if 'Running ' in line:
  matches=re.findall(r'\((/[^)]+)\)',line)
  for text in matches:
   path=Path(text)
   if path.is_file() and not path.is_symlink() and path.stat().st_mode&0o111:paths.append(path)
if paths and 'apps/grok-host/Cargo.toml' in cmd:paths.append(Path(env['CARGO_TARGET_DIR'])/'debug/rustgrok')
binaries=[];cache=Path.home()/'Library/Caches/rustgrok/evidence-binaries'
for path in dict.fromkeys(paths):
 data=path.read_bytes();digest=hashlib.sha256(data).hexdigest();dest=cache/digest
 if not dest.exists():
  free=shutil.disk_usage(root).free
  if free-len(data)<reserve:blocked=True;break
  dest.write_bytes(data);dest.chmod(0o700)
 assert dest.read_bytes()==data
 binaries.append({'path':str(path),'sha256':digest,'retained':str(dest),'size':len(data)})
minimum=min(minimum,shutil.disk_usage(root).free)
receipt={'command':cmd,'exit':code,'status':'BLOCKED_RESERVE' if blocked else 'passed' if code==0 else 'failed','free_before_bytes':before,'free_after_bytes':shutil.disk_usage(root).free,'minimum_sampled_free_bytes':minimum,'reserve_bytes':reserve,'cargo_incremental':'0','cargo_build_jobs':'1','config_note':'incremental=0 was also used in original runs; build jobs=1 is newly explicit for resumed verification, producing separately retained binaries','target_dir':env['CARGO_TARGET_DIR'],'base_checkpoint':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'runner_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'executed_binaries':binaries}
(out/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n');print(name,receipt['status'],code,'minimum free',minimum)
print((out/(name+'.stdout')).read_text(errors='replace')[-2200:]);print((out/(name+'.stderr')).read_text(errors='replace')[-1500:]);sys.exit(code or (1 if blocked else 0))
