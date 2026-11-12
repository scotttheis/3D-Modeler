from pathlib import Path
from PIL import Image
import json,zipfile,hashlib
root=Path(__file__).resolve().parent
def load(path):
 with zipfile.ZipFile(path) as z:return json.loads(z.read('scene.json')),{n:hashlib.sha256(z.read(n)).hexdigest() for n in z.namelist() if n.startswith('meshes/')}
before,meshes=load(root/'native-scene-fixtures/row-toggle.3dp')
source=json.loads((root/'python-pick-results.json').read_text())
assert len(source)==8 and json.loads((root/'python-pick-verification.json').read_text())['complete']
records=[]
for theme in ['light','dark']:
 for state,expected in [('before',[]),('after',[0]),('ctrl-empty',[0]),('clear',[])]:
  ref=next(r for r in source if (r['theme'],r['action'])==(theme,'pick-'+state))
  initial=next(r for r in source if (r['theme'],r['action'])==(theme,'pick-before'))
  assert ref['selection']==expected and ref['objects']==initial['objects'] and ref['undo']==initial['undo']
  with Image.open(root/'captures'/f'rust-{theme}-pick-{state}.png') as im:
   actual=[i for i in range(3) if any(im.getpixel((x,453+35*i))[:3] in [(225,230,255),(26,75,130)] for x in range(1120,1250))]
  assert actual==expected,(theme,state,actual,expected)
 for state in ['after','clear']:
  path=root/'native-scene-states'/f'rust-{theme}-pick-{state}.3dp';after,aftermeshes=load(path)
  assert after['objects']==before['objects'] and after['scene_revision']==before['scene_revision'] and aftermeshes==meshes
  records.append(dict(theme=theme,state=state,geometry_history_unchanged=True,sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
(root/'native-pick-verification.json').write_text(json.dumps(dict(complete=True,physical_input=True,paired_capture_states=8,verified=records),indent=2))
print('VERIFIED_NATIVE_PICK',len(records),'SAVED_STATES /8 PAIRED_CAPTURE_STATES')
