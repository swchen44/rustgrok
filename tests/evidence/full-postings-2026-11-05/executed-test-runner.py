import subprocess,sys,os,time,json,hashlib,shutil,signal
from pathlib import Path
root=Path('/Users/swchen.tw/git/rustgrok-full-postings');out=root/'tests/evidence/full-postings-2026-11-05';name=sys.argv[1];cmd=sys.argv[2:];reserve=2684354560;free=shutil.disk_usage(root).free
if free<reserve:raise SystemExit(f'BLOCKED_RESERVE before {name}: {free}')
env=os.environ.copy();env['CARGO_TARGET_DIR']='/Users/swchen.tw/git/rustgrok-durable-path/crates/path-index/target';env['CARGO_INCREMENTAL']='0'
start=time.time();minimum=free
with (out/(name+'.stdout')).open('wb') as so,(out/(name+'.stderr')).open('wb') as se:
 p=subprocess.Popen(cmd,cwd=root,stdout=so,stderr=se,env=env,start_new_session=True)
 blocked=False
 while p.poll() is None:
  free=shutil.disk_usage(root).free;minimum=min(minimum,free)
  if free<reserve:blocked=True;os.killpg(p.pid,signal.SIGTERM);break
  time.sleep(.2)
 code=p.wait()
receipt={'command':cmd,'exit':code,'status':'BLOCKED_RESERVE' if blocked else 'passed' if code==0 else 'failed','free_before':minimum if False else shutil.disk_usage(root).free,'minimum_sampled_free_bytes':minimum,'reserve_bytes':reserve,'cargo_incremental':'0','target_dir':env['CARGO_TARGET_DIR']}
(out/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n');print(name,receipt['status'],code,'minimum free',minimum)
print((out/(name+'.stdout')).read_text(errors='replace')[-1500:]);print((out/(name+'.stderr')).read_text(errors='replace')[-1500:]);sys.exit(code or (1 if blocked else 0))
