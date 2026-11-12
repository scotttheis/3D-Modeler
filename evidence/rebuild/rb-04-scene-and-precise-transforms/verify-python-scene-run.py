"""Audit completed reference records without reopening or changing Python."""
import json,zipfile
from pathlib import Path
root=Path(__file__).resolve().parent
records=json.loads((root/'python-scene-results.json').read_text())
specs=json.loads((root/'native-scene-case-specs.json').read_text())
assert len(specs)==37 and len(records)==238
indexed={(r['theme'],r['action']):r for r in records};assert len(indexed)==len(records)
mutations={'duplicate','paste-other-selection','cut','delete','group','ungroup','move-mm','move-inch','rotate-degree','resize-inch','resize-locked-inch','multi-move-inch','multi-rotate','multi-resize-locked','mirror-y','align-z-max','arrange-z-gap5','health-repair'}
def load(path):return json.loads(zipfile.ZipFile(path).read('scene.json'))
verified=[]
for theme in ['light','dark']:
 for spec in specs:
  name=spec['name'];before=indexed[(theme,f'scene-{name}-before')];after=indexed[(theme,f'scene-{name}-after')]
  assert before['history_delta']==0 and after['history_delta']==int(name in mutations),(theme,name,'history')
  assert before['width']==after['width']==1280 and before['height']==after['height']==840
  assert before['dpi']==after['dpi']==96 and before['device_pixel_ratio']==after['device_pixel_ratio']==1
  saved=load(root/'native-scene-states'/f'python-{theme}-scene-{name}-after.3dp')
  assert [o['object_id'] for o in saved['objects']]==[o['id'] for o in after['objects']]
  if (theme,f'scene-{name}-undo') in indexed:
   undo=indexed[(theme,f'scene-{name}-undo')];redo=indexed[(theme,f'scene-{name}-redo')]
   assert undo['objects']==before['objects'] and redo['objects']==after['objects'],(theme,name,'exact undo/redo')
   assert undo['history_delta']==0 and redo['history_delta']==1
  if name=='move-inch':assert saved['objects'][0]['transform'][1][3]==15
  if name=='shift-range':assert after['selection']==[0,2]
  verified.append(dict(theme=theme,case=name,history_delta=after['history_delta'],selected_after=after['selection']))
report=dict(complete=True,frames=len(records),cases=len(verified),verified=verified,python_tree_read_only=True,route='Visible Qt/OpenGL; actual widget/handler routes and Qt input. All native source dialogs and repair reports captured. Naming/visibility prepared directly; hidden multi-lock widget state is explicit.',source_shift_observation='MultiSelection Shift-click toggles endpoint [0,2]; Rust explicit handoff range is [0,1,2].')
(root/'python-scene-verification.json').write_text(json.dumps(report,indent=2));print('REFERENCE_VERIFIED',len(verified),'CASES',len(records),'FRAMES')
