"""Compare shared-fixture native scene outputs and history, retaining UI differences."""
import argparse,hashlib,json,struct,zipfile
from pathlib import Path
import numpy as np
from PIL import Image
root=Path(__file__).resolve().parent
parser=argparse.ArgumentParser();parser.add_argument('--partial',action='store_true');parser.add_argument('--step',nargs=2,metavar=('THEME','CASE'));parser.add_argument('--prefix');args=parser.parse_args()
def load(path):return json.loads(zipfile.ZipFile(path).read('scene.json'))
def ids(objects):return [i for o in objects for i in [o['object_id']]+ids(o.get('children',[]))]
def mesh_summary(archive,obj):
 data=archive.read(obj['mesh_file']);count=struct.unpack_from('<I',data,80)[0]
 triangles=np.array([struct.unpack_from('<9f',data,84+50*j+12) for j in range(count)],dtype=np.float64).reshape(-1,3,3)
 points=triangles.reshape(-1,3)
 area=float(np.linalg.norm(np.cross(triangles[:,1]-triangles[:,0],triangles[:,2]-triangles[:,0]),axis=1).sum()*.5)
 return dict(faces=count,minimum=points.min(axis=0).tolist(),maximum=points.max(axis=0).tolist(),surface_area=area,vertices=np.unique(np.round(points,4),axis=0).tolist())
def compare_meshes(left_archive,right_archive,left,right,context):
 for a,b in zip(left,right):
  clouds=[]
  for archive,obj in [(left_archive,a),(right_archive,b)]:
   data=archive.read(obj['mesh_file']);count=struct.unpack_from('<I',data,80)[0]
   vertices=np.array([struct.unpack_from('<9f',data,84+50*j+12) for j in range(count)],dtype=np.float64).reshape(-1,3)
   vertices=np.round(vertices,4);vertices=vertices[np.lexsort(vertices.T[::-1])];clouds.append(vertices)
  assert clouds[0].shape==clouds[1].shape and np.array_equal(*clouds),(context,'triangle vertex multiset')
  compare_meshes(left_archive,right_archive,a.get('children',[]),b.get('children',[]),context+' child')
def compare(left,right,original_ids,context):
 assert len(left)==len(right),(context,'object count')
 error=0.
 for a,b in zip(left,right):
  for key in ['name','visible','metadata']:assert a.get(key)==b.get(key),(context,key,a.get(key),b.get(key))
  if a['object_id'] in original_ids:assert a['object_id']==b['object_id'],(context,'durable ID')
  else:assert b['object_id'] not in original_ids,(context,'new independent ID')
  delta=float(np.max(np.abs(np.array(a['transform'])-np.array(b['transform']))));error=max(error,delta)
  assert delta<0.0001,(context,'transform',delta)
  assert np.allclose(a['color'],b['color'],atol=1e-6),(context,'color')
  error=max(error,compare(a.get('children',[]),b.get('children',[]),original_ids,context+' child'))
 return error
mutations={'duplicate','paste-other-selection','cut','delete','group','ungroup','move-mm','move-inch','rotate-degree','resize-inch','resize-locked-inch','multi-move-inch','multi-rotate','multi-resize-locked','rename-reference','visibility-reference','mirror-y','align-z-max','arrange-z-gap5','health-repair'}
records=[];missing=[]
specs=json.loads((root/'native-scene-case-specs.json').read_text())
reference={(r['theme'],r['action']):r for r in json.loads((root/'python-scene-results.json').read_text())}
routes={}
if (root/'rust-scene-output-routes.ndjson').exists():
 for line in (root/'rust-scene-output-routes.ndjson').read_text(encoding='utf-8-sig').splitlines():
  r=json.loads(line);routes[(r['theme'],r['case'])]=r
def selected_rows(image,count):
 with Image.open(image) as frame:
  selected=[]
  for i in range(count):
   if any(frame.getpixel((x,453+35*i))[:3] in [(225,230,255),(26,75,130)] for x in range(1120,1250)):
    selected.append(i)
  return selected
for theme in ['light','dark']:
 for spec in specs:
  name=spec['name'];prefix=args.prefix if args.step else routes.get((theme,name),{}).get('prefix',f'rust-{theme}-scene-{name}')
  path=root/'native-scene-states'/f'{prefix}-after.3dp'
  if args.step and args.step!=[theme,name]:continue
  if not path.exists():missing.append([theme,name]);continue
  before=load(root/spec['fixture']);after=load(path)
  assert before['project_id']==after['project_id'],(theme,name,'project identity')
  original_ids=set(ids(before['objects']));actual_ids=ids(after['objects']);assert len(actual_ids)==len(set(actual_ids)),(theme,name,'unique tree IDs')
  expected_delta=int(name in mutations)
  assert after['scene_revision']==before['scene_revision']+expected_delta,(theme,name,'one/no history')
  source_path=root/'native-scene-states'/f'python-{theme}-scene-{name}-after.3dp'
  source=load(source_path)
  error=compare(source['objects'],after['objects'],original_ids,f'{theme} {name}')
  repair_variance=None
  with zipfile.ZipFile(source_path) as source_archive,zipfile.ZipFile(path) as rust_archive:
   if name=='health-repair':
    source_mesh=mesh_summary(source_archive,source['objects'][0]);rust_mesh=mesh_summary(rust_archive,after['objects'][0])
    assert source_mesh['faces']==rust_mesh['faces']==4
    assert source_mesh['minimum']==rust_mesh['minimum'] and source_mesh['maximum']==rust_mesh['maximum']
    assert abs(source_mesh['surface_area']-rust_mesh['surface_area'])<0.001
    repair_variance=dict(source=source_mesh,rust=rust_mesh,exact_tessellation_match=source_mesh['vertices']==rust_mesh['vertices'],scope='RB-04 verifies warning/Repair reachability, identity, history and surface bounds/area. Fan point differs: Python5,5; Rust6.6667,6.6667. Detailed mesh-operation parity remains RB-06; no acceptance is inferred.')
   else:compare_meshes(source_archive,rust_archive,source['objects'],after['objects'],f'{theme} {name}')
  for state,scene in [('before',before),('after',after)]:
   expected=reference[(theme,f'scene-{name}-{state}')]['selection']
   if name=='shift-range' and state=='after':
    assert expected==[0,2],(theme,'source Shift observation changed')
    expected=[0,1,2] # Explicit handoff requires a contiguous anchor range.
   actual=selected_rows(root/'captures'/f'rust-{theme}-scene-{name}-{state}.png',len(scene['objects']))
   assert actual==expected,(theme,name,state,'visible selection',actual,expected)
  if expected_delta and not args.step:
   undo=load(root/'native-scene-states'/f'{prefix}-undo.3dp');redo=load(root/'native-scene-states'/f'{prefix}-redo.3dp')
   assert undo['objects']==before['objects'],(theme,name,'exact undo')
   assert redo['objects']==after['objects'],(theme,name,'exact redo')
   with zipfile.ZipFile(root/spec['fixture']) as original_archive,zipfile.ZipFile(root/'native-scene-states'/f'{prefix}-undo.3dp') as undo_archive:
    compare_meshes(original_archive,undo_archive,before['objects'],undo['objects'],f'{theme} {name} exact mesh undo')
   with zipfile.ZipFile(path) as changed_archive,zipfile.ZipFile(root/'native-scene-states'/f'{prefix}-redo.3dp') as redo_archive:
    compare_meshes(changed_archive,redo_archive,after['objects'],redo['objects'],f'{theme} {name} exact mesh redo')
   assert [undo['scene_revision'],redo['scene_revision']]==[before['scene_revision']+2,before['scene_revision']+3],(theme,name,'undo/redo history')
   for state,scene in [('undo',undo),('redo',redo)]:
    key=(theme,f'scene-{name}-{state}')
    if key in reference:
     actual=selected_rows(root/'captures'/f'rust-{theme}-scene-{name}-{state}.png',len(scene['objects']))
     assert actual==reference[key]['selection'],(theme,name,state,'visible undo/redo selection',actual,reference[key]['selection'])
  elif not expected_delta:assert after['objects']==before['objects'],(theme,name,'unchanged geometry/metadata')
  records.append(dict(theme=theme,case=name,history_delta=expected_delta,max_matrix_error=error,stable_identity=True,exact_undo_redo=bool(expected_delta),repair_tessellation_variance=repair_variance,after_sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
if not args.partial and not args.step:assert not missing,missing
report=dict(verified=records,missing=missing,complete=not missing,naming_visibility='Source prepared attributes have no interactive controls/history; Rust explicit requested controls each create one undoable command',source_selection_records='python-scene-results.json; compare visible selected rows/counts against paired Rust captures',shift_variance='Observed Python MultiSelection Shift-click toggles endpoint: [0,2]. Rust implements the explicit handoff contiguous anchor range [0,1,2]. Owner/visual acceptance remains pending.')
if not args.step:(root/'native-scene-verification.json').write_text(json.dumps(report,indent=2))
else:assert len(records)==1,'Step output missing'
print('VERIFIED_SCENE_CASES',len(records),'MISSING',len(missing))
