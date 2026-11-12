"""Current source gates; preserve ordinary release before bloat's symbol build."""
import datetime, hashlib, json, os, shutil, subprocess, time
from pathlib import Path
workspace=Path.cwd();root=workspace/'evidence/rebuild/rb-04-scene-and-precise-transforms'
def read(path):
 data=path.read_bytes();return data.decode('utf-16' if data.startswith(b'\xff\xfe') else 'utf-8-sig')
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
sources=sorted(list((workspace/'crates').rglob('*.rs'))+[workspace/'Cargo.toml',workspace/'Cargo.lock',workspace/'rust-toolchain.toml',workspace/'deny.toml'])
source_hashes={p.relative_to(workspace).as_posix():sha(p) for p in sources}
axis=json.loads(read(root/'modal-complete-gate-results.json'))
assert axis['source'].lower()==sha(workspace/'crates/builder3d-app/src/shell.rs')
assert axis['debug'].lower()==sha(workspace/'target/debug/builder3d-app.exe')
results=[]
for name in ['fmt','tests','clippy']:
 assert axis[name]==0
 results.append(dict(name=name,exit_code=0,log=f'logs/modal-complete-{name}.txt',adopted_current_source=True))
env=os.environ.copy();env['LOCALAPPDATA']=str(workspace/'target/rb04-final-native-gate-profile')
store=workspace/'target'/('rb04-final-native-build-'+datetime.datetime.now().strftime('%Y%m%d-%H%M%S'));store.mkdir()
artifacts={}
def save_artifact(name,path):
 target=store/(name+'-builder3d-app.exe');shutil.copyfile(path,target)
 artifacts[name]=dict(path=target.relative_to(workspace).as_posix(),bytes=target.stat().st_size,sha256=sha(target))
def run(name,command):
 start=time.monotonic();log=root/'logs'/f'final-native-{name}.txt'
 with log.open('w',encoding='utf-8') as output:
  output.write('COMMAND: '+subprocess.list2cmdline(command)+'\n');output.flush()
  result=subprocess.run(command,stdout=output,stderr=subprocess.STDOUT,env=env)
 results.append(dict(name=name,command=command,exit_code=result.returncode,seconds=round(time.monotonic()-start,3),log=log.relative_to(root).as_posix()))
 (root/'final-native-gate-results.json').write_text(json.dumps(results,indent=2))
 print(name,result.returncode,flush=True)
 assert result.returncode==0,(name,result.returncode)
save_artifact('debug',workspace/'target/debug/builder3d-app.exe')
for name,cmd in [('check',['cargo','check','--workspace','--locked']),('deny',['cargo','deny','check']),('metadata',['cargo','metadata','--locked','--format-version','1','--no-deps']),('smoke',[str(workspace/'target/debug/builder3d-app.exe'),'--smoke']),('release',['cargo','build','--release','--locked','-p','builder3d-app','--bin','builder3d-app'])]:run(name,cmd)
save_artifact('release',workspace/'target/release/builder3d-app.exe')
run('release-smoke',[str(workspace/artifacts['release']['path']),'--smoke'])
run('bloat-workspace',['cargo','bloat','--release','--workspace'])
run('bloat-app',['cargo','bloat','--release','-p','builder3d-app','--bin','builder3d-app'])
save_artifact('bloat_app',workspace/'target/release/builder3d-app.exe')
assert {p.relative_to(workspace).as_posix():sha(p) for p in sources}==source_hashes,'Source changed during gates'
(root/'final-native-build-artifacts.json').write_text(json.dumps(dict(recorded_at=datetime.datetime.now().astimezone().isoformat(),artifacts=artifacts,source_sha256=source_hashes,cargo_audit='Unavailable: cargo audit is not installed; earlier recorded exit101, not a pass'),indent=2))
print('FINAL_CURRENT_SOURCE_GATES_COMPLETE',flush=True)
