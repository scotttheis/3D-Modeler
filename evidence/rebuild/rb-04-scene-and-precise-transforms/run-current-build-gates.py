"""Finish current-source build evidence after the final identity correction.

Reuse checks for unchanged manifests and the newer full workspace tests/Clippy.
Retain both the symbol-enabled size artifact and the ordinary release artifact.
"""
from pathlib import Path
import datetime,hashlib,json,os,shutil,subprocess,time

root=Path(__file__).resolve().parent
workspace=root.parents[2]
os.chdir(workspace)
deadline=time.monotonic()+1800
while True:
    previous=json.loads((root/'boundary-gate-results.json').read_text(encoding='utf-8'))
    if {'bloat-app','audit'} <= {entry['name'] for entry in previous}:break
    if time.monotonic()>deadline:raise RuntimeError('Original size runner has not finished; no final build recorded')
    time.sleep(1)
assert all(entry['exit_code']==0 for entry in previous if entry['name']!='audit')
assert all(entry['exit_code']==0 for entry in json.loads((root/'identity-final-gates.json').read_text(encoding='utf-8')))

def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def source_hashes():
    paths=sorted([p for p in (workspace/'crates').rglob('*.rs') if 'target' not in p.parts]+[workspace/'Cargo.toml',workspace/'Cargo.lock',workspace/'rust-toolchain.toml',workspace/'deny.toml'])
    return {p.relative_to(workspace).as_posix():sha(p) for p in paths}

initial_sources=source_hashes()
bloat_log=root/'logs/boundary-bloat-app.txt'
# This explicit app-size command starts after the last Rust correction. Cargo
# resolves its ordinary source fingerprints itself; reject ambiguous chronology.
log_created=getattr(bloat_log.stat(),'st_birthtime',bloat_log.stat().st_ctime)
assert all((workspace/name).stat().st_mtime<=log_created for name in initial_sources),'App bloat predates current source'
stamp=datetime.datetime.now().strftime('%Y%m%d-%H%M%S')
artifact_dir=workspace/'target'/f'rb04-verified-build-{stamp}'
artifact_dir.mkdir(exist_ok=False)
measured=artifact_dir/'bloat-modeler3d-app.exe'
shutil.copyfile(workspace/'target/release/modeler3d-app.exe',measured)
artifacts={'bloat_app':dict(path=str(measured.relative_to(workspace)),sha256=sha(measured),bytes=measured.stat().st_size,scope='Symbol-enabled explicit app bloat artifact, not an installed package')}
results=[]
for entry in previous:
    if entry['name'] in {'deny','metadata','bloat-workspace','bloat-app','audit'}:
        record=entry.copy();record['log_path']=f'logs/boundary-{entry["name"]}.txt'
        record['reuse_reason']='Dependency manifests unchanged; size commands completed after the app-only correction (explicit app chronology verified).'
        results.append(record)
for name in ['tests','clippy']:
    results.append(dict(name=name,exit_code=0,log_path=f'logs/identity-final-{name}.txt',scope='Current-source full workspace; Clippy all targets/all features'))
env=os.environ.copy();env['LOCALAPPDATA']=str(workspace/'target/rb04-current-build-profile')
def record_results():
    (root/'current-boundary-gate-results.json').write_text(json.dumps(results,indent=2),encoding='utf-8')
def run(name,command):
    started=time.monotonic();log_path=f'logs/current-{name}.txt'
    with (root/log_path).open('w',encoding='utf-8') as log:
        log.write('COMMAND: '+subprocess.list2cmdline(command)+'\n');log.flush()
        result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,env=env)
    results.append(dict(name=name,command=command,exit_code=result.returncode,seconds=round(time.monotonic()-started,3),log_path=log_path))
    record_results();print(name,result.returncode,flush=True)
    if result.returncode:raise RuntimeError(f'{name} failed; see {log_path}')
record_results()
run('fmt',['cargo','fmt','--all','--','--check'])
run('check',['cargo','check','--workspace','--locked'])
run('smoke',['cargo','run','-p','modeler3d-app','--bin','modeler3d-app','--','--smoke'])
run('release',['cargo','build','--release','--locked','-p','modeler3d-app','--bin','modeler3d-app'])
canonical=artifact_dir/'modeler3d-app.exe'
shutil.copyfile(workspace/'target/release/modeler3d-app.exe',canonical)
artifacts['release']=dict(path=str(canonical.relative_to(workspace)),sha256=sha(canonical),bytes=canonical.stat().st_size,scope='Ordinary locked release profile, preserved separately from bloat artifact')
run('release-smoke',[str(canonical),'--smoke'])
assert initial_sources==source_hashes(),'Source changed during current-build checks'
(root/'current-build-artifacts.json').write_text(json.dumps(dict(recorded_at=datetime.datetime.now().astimezone().isoformat(),artifacts=artifacts,source_sha256=initial_sources,native_replay='pending while Windows is locked'),indent=2),encoding='utf-8')
print('Current source/build identities verified; native replay remains pending',flush=True)
