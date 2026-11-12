from pathlib import Path
import json,hashlib,os
text=Path('target/rb04-completed-snapshot-receipt.txt').read_text(encoding='utf-16')
receipt=json.loads(text[text.index('{'):]); snapshot=Path(receipt['SnapshotPath'])
entry='\n### RB-04 completed implementation/native evidence: verified additive snapshot\n\n'+f'- Immutable snapshot: `{snapshot}`. Verified {receipt["VerifiedAt"]}; Robocopy exit {receipt["RobocopyExitCode"]}.\n- Source and snapshot: {receipt["SourceFileCount"]:,} files / {receipt["SourceBytes"]:,} bytes each; relative-path differences 0; size/SHA-256 differences 0; reparse entries 0 in each tree. Root target/, .cache/ and reparse entries excluded. Source, Git metadata, tests, documentation and evidence included.\n- RB-04 implementation, automated gates and sequential native Light/Dark evidence are complete:261 workspace tests,74 scene cases,22 viewport cases,4 Settle cases,8 paired physical-pick states and modal controls verified. Visual/owner acceptance remains pending. Python is unchanged; all32 RB-03 statuses are preserved. Both owned target apps are closed. STOP before RB-05; no staging or commit. Next slice is full viewport gestures/presets, gizmo style/placement, snapping, floor/grid, lighting/colors, wireframe/X-Ray and remaining resize interaction. See report.md for visible differences and Repair tessellation variance retained for RB-06.\n- Pre-receipt recovery-critical SHA-256 values:\n'+''.join(f'  - `{k}`: `{v}`\n' for k,v in receipt['RecoveryCriticalSha256'].items())+'\nThe receipt postdates the immutable snapshot. Only WORK_LOG.md and the RB-04 checkpoint are expected to differ afterward.\n'
for p in [Path('WORK_LOG.md'),Path('evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md')]:
 with p.open('a',encoding='utf-8') as f:f.write(entry)
def manifest(root):
 out={}; reparses=0
 for here,dirs,files in os.walk(root,followlinks=False):
  if Path(here)==root:dirs[:]=[d for d in dirs if d not in ['target','.cache']]
  keep=[]
  for d in dirs:
   if (Path(here)/d).lstat().st_file_attributes & 0x400:reparses+=1
   else:keep.append(d)
  dirs[:]=keep
  for name in files:
   p=Path(here)/name
   if p.lstat().st_file_attributes & 0x400:reparses+=1;continue
   out[p.relative_to(root).as_posix()]=(p.stat().st_size,hashlib.sha256(p.read_bytes()).hexdigest())
 return out,reparses
source=Path.cwd(); a,ar=manifest(source); b,br=manifest(snapshot);assert a.keys()==b.keys();delta=[k for k in a if a[k]!=b[k]]
expected=['WORK_LOG.md','evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md'];assert sorted(delta)==sorted(expected),delta;assert ar==br==0
line=f'\nPost-receipt comparison: {len(a)} included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-04 checkpoint differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.\n'
for p in [source/'WORK_LOG.md',source/expected[1]]:
 with p.open('a',encoding='utf-8') as f:f.write(line)
print('VERIFIED_SNAPSHOT',snapshot,'POST_RECEIPT_DELTAS',delta)

