import subprocess,json
from pathlib import Path
root=Path('evidence/rebuild/rb-04-scene-and-precise-transforms'); results=[]
for name,cmd in [('fmt',['cargo','fmt','--all','--','--check']),('tests',['cargo','test','--workspace','--locked']),('clippy',['cargo','clippy','-p','modeler3d-core','-p','modeler3d-app','--all-targets','--locked','--','-D','warnings']),('deny',['cargo','deny','check'])]:
 with (root/'logs'/f'settle-{name}.txt').open('w',encoding='utf-8') as log:r=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT)
 results.append(dict(name=name,exit_code=r.returncode));(root/'settle-gate-results.json').write_text(json.dumps(results,indent=2));print(name,r.returncode,flush=True)
