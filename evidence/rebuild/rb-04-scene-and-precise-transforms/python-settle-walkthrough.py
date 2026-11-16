import os,sys,json,ctypes,time,traceback
from native_desktop_guard import require_default_desktop
from pathlib import Path
sys.dont_write_bytecode=True
sys.path.insert(0,'L:/Builder3D')
from src.main import configure_opengl_defaults
configure_opengl_defaults()
from src.qt_compat import QApplication,QSettings
from PyQt6.QtCore import QTimer
import src.desktop_adapters as adapters
root=Path('E:/3D-Modeler/evidence/rebuild/rb-04-scene-and-precise-transforms')
(root/'captures').mkdir(exist_ok=True)
settings=QSettings('E:/3D-Modeler/target/rb04-python.ini',QSettings.Format.IniFormat)
adapters.QSettings=lambda *args:settings
from src.main_window import MainWindow, ObjectPanel
from src.style import apply_style
from src.mesh_ops import make_box
app=QApplication([]); apply_style(app,'light'); w=MainWindow(); w.resize(1280,840); w.show(); w.raise_(); w.activateWindow()
w.object_panel=w.findChild(ObjectPanel)
user=ctypes.windll.user32; user.GetForegroundWindow.restype=ctypes.c_void_p
user.SetForegroundWindow.argtypes=[ctypes.c_void_p]
user.BringWindowToTop.argtypes=[ctypes.c_void_p]
user.GetWindowThreadProcessId.argtypes=[ctypes.c_void_p,ctypes.c_void_p]
user.ShowWindowAsync.argtypes=[ctypes.c_void_p,ctypes.c_int]
def activate():
 require_default_desktop()
 front=user.GetForegroundWindow()
 if not front: raise RuntimeError('Desktop unavailable; no input sent')
 target=int(w.winId()); ft=user.GetWindowThreadProcessId(front,None); ct=ctypes.windll.kernel32.GetCurrentThreadId()
 user.AttachThreadInput(ft,ct,True)
 user.ShowWindowAsync(target,9); user.BringWindowToTop(target); user.SetForegroundWindow(target)
 user.AttachThreadInput(ft,ct,False)
 app.processEvents()
 if user.GetForegroundWindow()!=target: raise RuntimeError('Reference did not become foreground; no input or capture sent')
records=[]; operations=[]
def capture(name,theme):
 activate()
 require_default_desktop()
 if user.GetForegroundWindow()!=int(w.winId()): raise RuntimeError('Target is not foreground; no input or capture sent')
 pix=app.primaryScreen().grabWindow(0,w.mapToGlobal(w.rect().topLeft()).x(),w.mapToGlobal(w.rect().topLeft()).y(),w.width(),w.height())
 path=root/'captures'/f'python-{theme}-{name}.png'; pix.save(str(path))
 records.append(dict(action=name,theme=theme,width=w.width(),height=w.height(),device_pixel_ratio=w.devicePixelRatioF(),image=str(path.relative_to(root)),route='visible Qt widget / semantic handler; no pointer evidence claimed',selection=sorted(w.scene.selected_indices),objects=[dict(id=o.object_id,name=o.name,visible=o.visible,transform=o.transform.tolist()) for o in w.scene.objects],undo=len(w.scene.undo_stack)))
def setup(theme):
 w._set_theme_mode(theme); w.scene.clear(); w.scene.add_object(make_box((20,30,40)),'Box A'); w.scene.add_object(make_box((10,20,30)),'Box B'); w.scene.objects[1].translate(35,10,0); w.scene.set_selection({0}); w._show_workspace(); w.viewport.update()
def add(name,fn,theme): operations.append((name,fn,theme))

import json
cases=json.loads((root/'settle-oracle.json').read_text())
def setup_settle(theme,count):
 w._set_theme_mode(theme); w.scene.clear()
 for i,rows in enumerate(cases[count-1]['before']):
  w.scene.add_object(make_box((20,30,40)),f'box {i}')
  w.scene.objects[i].transform=__import__('numpy').array(rows,dtype='float32')
 w.scene.set_selection(set(range(count))); w._show_workspace(); w.viewport.frame_scene(); w.viewport.update()
for theme in ['light','dark']:
 for count in [1,2]:
  add(f'settle-{count}-before',lambda t=theme,c=count:setup_settle(t,c),theme)
  add(f'settle-{count}-after',w._on_settle,theme)
  add(f'settle-{count}-undo',w._on_undo,theme)
  add(f'settle-{count}-redo',w._on_redo,theme)
index=0
def tick():
 global index
 try:
  if index>=len(operations):
   (root/'python-settle-results.json').write_text(json.dumps(records,indent=2)); os._exit(0)
  activate(); name,fn,theme=operations[index]; fn(); w.viewport.update(); app.processEvents(); QTimer.singleShot(700,lambda:capture_then_next(name,theme))
 except Exception:
  (root/'logs'/'python-settle-error.txt').write_text(traceback.format_exc()); os._exit(1)
def capture_then_next(name,theme):
 global index
 try: capture(name,theme); index+=1; QTimer.singleShot(150,tick)
 except Exception:
  (root/'logs'/'python-settle-error.txt').write_text(traceback.format_exc()); os._exit(1)
QTimer.singleShot(2000,tick)
sys.exit(app.exec())



