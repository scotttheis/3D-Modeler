"""Replay only placement dialogs, waiting for actual repaint before capture."""
from pathlib import Path
p=Path(__file__).with_name('python-scene-walkthrough.py')
code=p.read_text();code=code[:code.index("for theme in ['light','dark']:")]
code=code.replace('records=[]; operations=[]; current={}; specs=[]',"records=json.loads((root/'python-scene-results.json').read_text()); operations=[]; current={}; specs=json.loads((root/'native-scene-case-specs.json').read_text())")
# The shared capture function now waits for real Qt repaint.
code=code.replace(" records.append(dict(action='scene-'+name", " records[:]=[r for r in records if (r['action'],r['theme'])!=('scene-'+name,theme)]\n records.append(dict(action='scene-'+name")
code=code.replace(" if theme=='light':specs.append", " if theme=='light' and not any(s['name']==name for s in specs):specs.append")
exec(compile(code,str(p)+'-modal-repaint','exec'))
selected={'mirror-y','align-z-max','arrange-z-gap5','mirror-cancel','arrange-invalid-cancel','measure'}
for theme in ['light','dark']:
 for name,fn,selection,kind in cases:
  if name not in selected:continue
  operations.append((name+'-before',lambda t=theme,n=name,s=selection,k=kind:setup(t,n,s,k),theme))
  operations.append((name+'-after',fn,theme))
  if name in ['mirror-y','align-z-max','arrange-z-gap5']:
   operations.append((name+'-undo',w._on_undo,theme));operations.append((name+'-redo',w._on_redo,theme))
index=0
def tick():
 global index
 try:
  if index>=len(operations):os._exit(0)
  activate();name,fn,theme=operations[index];current['theme']=theme;fn();w.viewport.update();app.processEvents();QTimer.singleShot(350,lambda:finish(name,theme))
 except Exception:
  (root/'logs/python-modal-repaint-error.txt').write_text(traceback.format_exc());os._exit(1)
def finish(name,theme):
 global index
 try:
  capture(name,theme)
  if not name.endswith('-before'):save_project(w.scene,str(root/'native-scene-states'/f'python-{theme}-scene-{name}.3dp'))
  index+=1;QTimer.singleShot(80,tick)
 except Exception:
  (root/'logs/python-modal-repaint-error.txt').write_text(traceback.format_exc());os._exit(1)
QTimer.singleShot(1500,tick)
sys.exit(app.exec())
