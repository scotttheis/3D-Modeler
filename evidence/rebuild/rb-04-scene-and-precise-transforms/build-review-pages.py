"""Small agent-review pages; original client-sized captures remain unchanged."""
from pathlib import Path
from PIL import Image,ImageDraw
import json,sys
root=Path(__file__).resolve().parent
out=root/'review';out.mkdir(exist_ok=True)
theme=sys.argv[1] if len(sys.argv)>1 else 'light'
specs=json.loads((root/'native-scene-case-specs.json').read_text())
actions=['scene-'+s['name']+'-after' for s in specs]
actions += [p.stem.removeprefix(f'python-{theme}-') for p in sorted((root/'captures').glob(f'python-{theme}-scene-*-dialog-*.png'))]
actions += [p.stem.removeprefix(f'python-{theme}-') for p in sorted((root/'captures').glob(f'python-{theme}-viewport-*-after.png'))]
actions += ['settle-1-after','settle-2-after','pick-after','pick-clear']
actions=[a for a in actions if all((root/'captures'/f'{app}-{theme}-{a}.png').is_file() for app in ['python','rust'])]
for start in range(0,len(actions),4):
 chunk=actions[start:start+4];page=Image.new('RGB',(1280,460*len(chunk)),'white');draw=ImageDraw.Draw(page)
 for j,action in enumerate(chunk):
  draw.text((5,j*460+2),f'{theme}: {action}   Python LEFT / Rust RIGHT',fill='black')
  for i,app in enumerate(['python','rust']):
   with Image.open(root/'captures'/f'{app}-{theme}-{action}.png') as im:
    im.thumbnail((640,420));page.paste(im,(i*640,j*460+24))
 page.save(out/f'{theme}-{start//4+1:02}.png')
print('REVIEW_PAGES',theme,(len(actions)+3)//4,'ACTION_PAIRS',len(actions))
