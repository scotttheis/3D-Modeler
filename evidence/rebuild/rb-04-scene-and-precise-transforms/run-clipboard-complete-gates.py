"""Capture native compiler progress as data, with actual process exit codes."""
import hashlib,json,os,subprocess,time
from pathlib import Path
root=Path('evidence/rebuild/rb-04-scene-and-precise-transforms');results={}
env=os.environ.copy();env['LOCALAPPDATA']=str(Path('target/rb04-boundary-profile').resolve())
commands=[('fmt',['cargo','fmt','--all','--','--check']),('check',['cargo','check','--workspace','--locked']),('tests',['cargo','test','--workspace','--locked']),('clippy',['cargo','clippy','--workspace','--all-targets','--all-features','--locked','--','-D','warnings']),('deny',['cargo','deny','check']),('metadata',['cargo','metadata','--locked','--format-version','1','--no-deps']),('build',['cargo','build','--locked','-p','builder3d-app'])]
for name,command in commands:
 start=time.monotonic()
 with (root/'logs'/f'clipboard-complete-{name}.txt').open('w',encoding='utf-8') as log:
  log.write('COMMAND: '+subprocess.list2cmdline(command)+'\n');log.flush()
  result=subprocess.run(command,stdout=log,stderr=subprocess.STDOUT,env=env)
 results[name]=result.returncode
 (root/'clipboard-complete-gate-results.json').write_text(json.dumps(results,indent=2))
 print(name,result.returncode,round(time.monotonic()-start,2),flush=True)
 assert result.returncode==0,(name,result.returncode)
results.update(source=hashlib.sha256(Path('crates/builder3d-app/src/shell.rs').read_bytes()).hexdigest(),debug=hashlib.sha256(Path('target/debug/builder3d-app.exe').read_bytes()).hexdigest())
(root/'clipboard-complete-gate-results.json').write_text(json.dumps(results,indent=2));print('CURRENT_CLIPBOARD_GATES_COMPLETE',flush=True)
