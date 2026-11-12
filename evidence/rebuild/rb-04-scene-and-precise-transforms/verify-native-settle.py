from pathlib import Path
import json,zipfile,hashlib
root=Path(__file__).resolve().parent
reference=json.loads((root/'python-settle-results.json').read_text(encoding='utf-8'))
assert len(reference)==16
records=[]
routes={}
if (root/'rust-settle-output-routes.ndjson').exists():
 for line in (root/'rust-settle-output-routes.ndjson').read_text(encoding='utf-8-sig').splitlines():
  r=json.loads(line);routes[(r['theme'],r['count'])]=r
def load(path):
 with zipfile.ZipFile(path) as archive:return json.loads(archive.read('scene.json'))
for theme in ['light','dark']:
 for count in [1,2]:
  fixture=root/f'settle-{count}-before.3dp';source=load(fixture)
  route=routes.get((theme,count))
  saved=Path(route['saved']) if route else root/'native-scene-states'/f'rust-{theme}-settle-{count}-redo.3dp';actual=load(saved)
  before=next(r for r in reference if r['theme']==theme and r['action']==f'settle-{count}-before')
  after=next(r for r in reference if r['theme']==theme and r['action']==f'settle-{count}-after')
  undo=next(r for r in reference if r['theme']==theme and r['action']==f'settle-{count}-undo')
  redo=next(r for r in reference if r['theme']==theme and r['action']==f'settle-{count}-redo')
  assert actual['scene_revision']==source['scene_revision']+3,(theme,count,'one Settle/Undo/Redo transition each')
  assert [o['object_id'] for o in actual['objects']]==[o['object_id'] for o in source['objects']]
  assert [o['id'] for o in before['objects']]==[o['id'] for o in after['objects']]==[o['id'] for o in undo['objects']]==[o['id'] for o in redo['objects']]
  max_error=0.
  for rust,python,initial,py_before,py_undo,py_redo in zip(actual['objects'],after['objects'],source['objects'],before['objects'],undo['objects'],redo['objects']):
   assert rust['name']==python['name']==initial['name']
   for a,b in zip(sum(rust['transform'],[]),sum(python['transform'],[])):
    max_error=max(max_error,abs(a-b));assert abs(a-b)<.002,(theme,count,a,b)
   assert py_before['transform']==py_undo['transform'];assert python['transform']==py_redo['transform']
  for stage in ['before','after','undo','redo']:
   for app in ['python','rust']:assert (root/'captures'/f'{app}-{theme}-settle-{count}-{stage}.png').is_file()
  records.append(dict(theme=theme,count=count,reference_route='Real visible Qt/OpenGL window and semantic Settle/Undo/Redo handlers',rust_route='Guarded physical native pointer and keyboard',client=[1280,840],dpi=96,fixture=str(fixture.relative_to(root)),saved=str(saved.relative_to(root)),saved_sha256=hashlib.sha256(saved.read_bytes()).hexdigest(),max_matrix_error=max_error,rust_revision_before=source['scene_revision'],rust_revision_after=actual['scene_revision'],rust_ids_preserved=True,python_ids_preserved=True,reference_history_delta=after['undo']-before['undo'],status='pass'))
(root/'native-settle-verification.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
print(json.dumps(records,indent=2))
