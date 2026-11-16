from pathlib import Path
import tomllib,hashlib,json,re
root=Path('evidence/rebuild/rb-04-scene-and-precise-transforms'); reference=Path('L:/Builder3D')
assets=tomllib.loads(Path('evidence/rebuild/rb-01-visual-foundations/asset-manifest.toml').read_text())['asset']
for a in assets:
 assert hashlib.sha256(Path(a['source_path']).read_bytes()).hexdigest()==a['source_sha256'],a['source_path']
 assert hashlib.sha256(Path(a['rust_destination']).read_bytes()).hexdigest()==a['packaged_sha256'],a['rust_destination']
parity=tomllib.loads(Path('evidence/rebuild/parity-matrix.toml').read_text()); missing=[]
for row in parity['feature']:
 if row.get('next_slice')!='RB-04':continue
 for field,base in [('rust_paths',Path('.')),('python_paths',reference)]:
  for path in row.get(field,[]):
   if not (base/path).exists():missing.append((row['id'],field,path))
 for entry in row.get('evidence',[]):
  file,_,symbol=entry.partition('::'); choices=[Path(file),reference/file]; p=next((p for p in choices if p.is_file()),None)
  if p is None:missing.append((row['id'],'evidence',entry))
  elif symbol and not all(part in p.read_text(encoding='utf-8-sig') for part in symbol.split('::')):missing.append((row['id'],'symbol',entry))
assert not missing,missing
catalog=tomllib.loads(Path('docs/component-catalog.toml').read_text())['component'];assert len({c['id'] for c in catalog})==len(catalog)
for c in catalog:
 for p in c['rust_paths']:assert Path(p).exists(),p
assert 'dirty_marker_tracks_saved_checkpoint_through_undo_redo_and_branches' in Path('crates/modeler3d-app/src/commands.rs').read_text()
p=root/'logs/settle-tests.txt';s=p.read_text(encoding='utf-16' if p.read_bytes().startswith(b'\xff\xfe') else 'utf-8'); results=re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored',s)
meta=json.loads((root/'capture-metadata.json').read_text())
for c in meta['captures']:assert hashlib.sha256((root/c['path']).read_bytes()).hexdigest()==c['sha256']
text=f'Assets: {len(assets)} reference and packaged SHA-256 pairs unchanged\nComponent catalog: {len(catalog)} unique IDs and all Rust paths resolve\nRB-04 Python/Rust/evidence paths and file::symbol references resolve\nRB-03 historical validator false positive resolved: dirty-marker file::symbol exists\nFinal post-native tests pass/fail/ignored: {[sum(int(r[i]) for r in results) for i in range(3)]}\nCapture SHA-256 matches: {len(meta["captures"])}\n'
(root/'logs/final-evidence-validation.txt').write_text(text);print(text)
