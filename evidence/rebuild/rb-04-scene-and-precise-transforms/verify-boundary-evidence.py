"""Validate current evidence without treating unavailable native work as passed."""
from pathlib import Path
import ctypes,datetime,hashlib,json,re,subprocess,tomllib,sys
root=Path(__file__).resolve().parent
workspace=root.parents[2]
reference=Path('L:/Builder3D')
def read(path):
 data=path.read_bytes()
 return data.decode('utf-16' if data.startswith(b'\xff\xfe') else 'utf-8-sig')
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
assets=tomllib.loads(read(workspace/'evidence/rebuild/rb-01-visual-foundations/asset-manifest.toml'))['asset']
for item in assets:
 assert sha(Path(item['source_path']))==item['source_sha256'],item['source_path']
 assert sha(workspace/item['rust_destination'])==item['packaged_sha256'],item['rust_destination']
matrix=tomllib.loads(read(workspace/'evidence/rebuild/parity-matrix.toml'))['feature']
assert len({r['id'] for r in matrix})==len(matrix)
rows=[r for r in matrix if r['next_slice']=='RB-04' or r['id'].startswith('builder3d.ui.gizmo.')]
for row in rows:
 for key,base in [('rust_paths',workspace),('python_paths',reference)]:
  for name in row[key]:assert (base/name).is_file(),(row['id'],key,name)
 for entry in row['evidence']:
  name,_,symbol=entry.partition('::')
  path=next((p for p in [workspace/name,reference/name] if p.is_file()),None)
  assert path is not None,(row['id'],entry)
  if symbol:assert all(s in read(path) for s in symbol.split('::')),(row['id'],entry)
catalog=tomllib.loads(read(workspace/'docs/component-catalog.toml'))['component']
assert len({r['id'] for r in catalog})==len(catalog)
for component in catalog:
 for name in component['rust_paths']:assert (workspace/name).exists(),name
catalog_ids={component['id'] for component in catalog}
for row in rows:
 for component in row['rust_component_ids']:assert component in catalog_ids,(row['id'],component)
meta=json.loads(read(root/'capture-metadata.json'))
for capture in meta['captures']:
 assert sha(root/capture['path'])==capture['sha256'],capture['path']
 assert [capture['width'],capture['height']]==[1280,840],capture['path']
for name,digest in meta['reference_build_identity']['source_sha256'].items():
 assert sha(reference/name)==digest,name
pairs=json.loads(read(root/'comparison-pairs.json'))
assert len(pairs)>=50
assert len({(p['theme'],p['action']) for p in pairs})==len(pairs)
assert {c['path'] for c in meta['captures']}=={p.relative_to(root).as_posix() for p in (root/'captures').glob('*.png')}
for pair in pairs:
 assert (root/pair['python']).is_file() and (root/pair['rust']).is_file()
expected_oracles={'gesture':122,'numeric-selection':32,'placement':42,'rotation':9}
for name,count in expected_oracles.items():assert len(json.loads(read(root/f'{name}-oracle.json')))==count
status=subprocess.run(['git','--no-optional-locks','-c','safe.directory=L:/Builder3D','-C',str(reference),'status','--short'],capture_output=True,check=True).stdout.decode('utf-8')
baseline=read(root/'logs/python-status-before.txt')
assert status.splitlines()==baseline.splitlines(),'Python tree status changed'
(root/'logs/python-status-boundary.txt').write_text(status,encoding='utf-8')
old=tomllib.loads(read(Path('G:/Builder3D-Rust_snapshot_20261031-194408_RB-04-baseline/evidence/rebuild/parity-matrix.toml')))['feature']
rb03=[r for r in old if any('rb-03-lifecycle-and-file-completion/' in p for p in r['evidence'])]
assert len(rb03)==32
current={r['id']:r for r in matrix};fields=['implementation','automated','interactive','visual','owner_acceptance']
for row in rb03:assert all(row[k]==current[row['id']][k] for k in fields),row['id']
assert sum(current[r['id']]['interactive']=='partial' for r in rb03)==2
if '--preflight' in sys.argv:
 print(json.dumps(dict(assets_verified=len(assets),unique_feature_ids=len(matrix),catalog_ids=len(catalog),validated_rows=len(rows),historical_pairs=len(pairs),historical_capture_sha256=len(meta['captures']),python_tree_status_unchanged=True,rb03_status_rows_unchanged=32),indent=2))
 sys.exit(0)
tests_path=root/'logs/modal-complete-tests.txt'
counts=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',read(tests_path))
assert counts and not any(int(f) or int(i) for p,f,i in counts)
total=sum(int(p) for p,f,i in counts)
gates=json.loads(read(root/'final-native-gate-results.json'))
required={'fmt','check','tests','clippy','deny','metadata','smoke','release','bloat-workspace','bloat-app'}
assert required <= {g['name'] for g in gates},'Boundary gates still running'
assert all(g['exit_code']==0 for g in gates if g['name'] in required)
source_paths=sorted([p for p in (workspace/'crates').rglob('*.rs') if 'target' not in p.parts]+[workspace/'Cargo.toml',workspace/'Cargo.lock',workspace/'rust-toolchain.toml',workspace/'deny.toml'])
artifacts=json.loads(read(root/'final-native-build-artifacts.json'))
for artifact in artifacts['artifacts'].values():assert sha(workspace/artifact['path'])==artifact['sha256']
sources={p.relative_to(workspace).as_posix():sha(p) for p in source_paths}
assert artifacts['source_sha256']==sources
builds={'debug':sha(workspace/'target/debug/builder3d-app.exe'),'release':artifacts['artifacts']['release']['sha256'],'bloat_app':artifacts['artifacts']['bloat_app']['sha256']}
identity=dict(recorded_at=datetime.datetime.now().astimezone().isoformat(),build_sha256=builds,artifacts=artifacts['artifacts'],source_sha256=sources,native_capture_status='Current native viewport/Settle/scene verification is recorded separately; visual/owner acceptance remains pending')
(root/'boundary-build-identity.json').write_text(json.dumps(identity,indent=2),encoding='utf-8')
viewport=json.loads(read(root/'native-viewport-verification.json'))
scenes=json.loads(read(root/'native-scene-verification.json'))
settle=json.loads(read(root/'native-settle-verification.json'))
pick=json.loads(read(root/'native-pick-verification.json'))
assert pick['complete'] and pick['physical_input'] and pick['paired_capture_states']==8
assert len(pick['verified'])==4 and all(r['geometry_history_unchanged'] for r in pick['verified'])
assert viewport['complete'] and len(viewport['verified'])==22
assert scenes['complete'] and not scenes['missing']
assert len(settle)==4 and all(r['status']=='pass' for r in settle)
from native_desktop_guard import require_default_desktop
require_default_desktop()
summary=dict(assets_verified=len(assets),unique_feature_ids=len(matrix),catalog_ids=len(catalog),validated_requirement_rows=len(rows),paired_actions=len(pairs),capture_sha256_verified=len(meta['captures']),workspace_tests_passed=total,workspace_tests_failed=0,workspace_tests_ignored=0,python_oracle_cases=expected_oracles,python_tree_status_unchanged=True,rb03_status_rows_unchanged=32,native_viewport_cases=22,native_settle_cases=4,native_scene_cases=len(scenes['verified']),native_pick_capture_states=8,visual_owner_acceptance='pending',engineering_build_sha256=builds)
(root/'boundary-evidence-validation.json').write_text(json.dumps(summary,indent=2),encoding='utf-8')
print(json.dumps(summary,indent=2))
