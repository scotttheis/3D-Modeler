import json,subprocess,os,time
from pathlib import Path
root=Path('evidence/rebuild/rb-04-scene-and-precise-transforms'); results=[]
commands=[('fmt',['cargo','fmt','--all','--','--check']),('check',['cargo','check','--workspace','--locked']),('tests',['cargo','test','--workspace','--locked']),('clippy',['cargo','clippy','--workspace','--all-targets','--all-features','--locked','--','-D','warnings']),('deny',['cargo','deny','check']),('metadata',['cargo','metadata','--locked','--format-version','1','--no-deps']),('smoke',['cargo','run','-p','builder3d-app','--bin','builder3d-app','--','--smoke']),('release',['cargo','build','--release','--locked','-p','builder3d-app','--bin','builder3d-app']),('bloat-workspace',['cargo','bloat','--release','--workspace']),('bloat-app',['cargo','bloat','--release','-p','builder3d-app','--bin','builder3d-app']),('audit',['cargo','audit','--locked'])]
env=os.environ.copy(); env['LOCALAPPDATA']=str(Path('target/rb04-boundary-profile').resolve())
for name,cmd in commands:
 start=time.time()
 with (root/'logs'/f'boundary-{name}.txt').open('w',encoding='utf-8') as log:
  log.write('COMMAND: '+subprocess.list2cmdline(cmd)+'\n'); log.flush()
  result=subprocess.run(cmd,stdout=log,stderr=subprocess.STDOUT,env=env)
 results.append(dict(name=name,command=cmd,exit_code=result.returncode,seconds=round(time.time()-start,3)))
 (root/'boundary-gate-results.json').write_text(json.dumps(results,indent=2))
 print(name,result.returncode,flush=True)

