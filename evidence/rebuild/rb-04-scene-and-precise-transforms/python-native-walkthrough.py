import os,sys,json,ctypes,time,traceback
from pathlib import Path
sys.dont_write_bytecode=True
sys.path.insert(0,'L:/Builder3D')
from src.main import configure_opengl_defaults
configure_opengl_defaults()
from src.qt_compat import QApplication,QSettings
from PyQt6.QtCore import QTimer
import src.desktop_adapters as adapters
root=Path('E:/Builder3D-Rust/evidence/rebuild/rb-04-scene-and-precise-transforms')
(root/'captures').mkdir(exist_ok=True)
settings=QSettings('E:/Builder3D-Rust/target/rb04-python.ini',QSettings.Format.IniFormat)
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
 front=user.GetForegroundWindow()
 if not front: raise RuntimeError('Desktop unavailable; no input sent')
 target=int(w.winId()); ft=user.GetWindowThreadProcessId(front,None); ct=ctypes.windll.kernel32.GetCurrentThreadId()
 user.AttachThreadInput(ft,ct,True)
 user.ShowWindowAsync(target,9); user.BringWindowToTop(target); user.SetForegroundWindow(target)
 user.AttachThreadInput(ft,ct,False)
records=[]; operations=[]
def capture(name,theme):
 if user.GetForegroundWindow()!=int(w.winId()): raise RuntimeError('Target is not foreground; no input or capture sent')
 pix=app.primaryScreen().grabWindow(0,w.mapToGlobal(w.rect().topLeft()).x(),w.mapToGlobal(w.rect().topLeft()).y(),w.width(),w.height())
 path=root/'captures'/f'python-{theme}-{name}.png'; pix.save(str(path))
 records.append(dict(action=name,theme=theme,width=w.width(),height=w.height(),device_pixel_ratio=w.devicePixelRatioF(),image=str(path.relative_to(root)),route='visible Qt widget / semantic handler; no pointer evidence claimed',selection=sorted(w.scene.selected_indices),objects=[dict(id=o.object_id,name=o.name,visible=o.visible,transform=o.transform.tolist()) for o in w.scene.objects],undo=len(w.scene.undo_stack)))
def setup(theme):
 w._set_theme_mode(theme); w.scene.clear(); w.scene.add_object(make_box((20,30,40)),'Box A'); w.scene.add_object(make_box((10,20,30)),'Box B'); w.scene.objects[1].translate(35,10,0); w.scene.set_selection({0}); w._show_workspace(); w.viewport.update()
def add(name,fn,theme): operations.append((name,fn,theme))
for theme in ['light','dark']:
 add('items',lambda t=theme:setup(t),theme)
 add('select-all',w.object_panel._on_select_all,theme)
 add('invert',w.object_panel._on_invert_selection,theme)
 add('sticky',lambda:w.object_panel.sticky_sel.setChecked(True),theme)
 add('selected',lambda:w.scene.set_selection({0}),theme)
 add('copy',w._on_copy,theme)
 add('paste',w._on_paste,theme)
 add('duplicate',w._on_duplicate,theme)
 add('cut',w._on_cut,theme)
 add('undo',w._on_undo,theme)
 add('redo',w._on_redo,theme)
 add('select-all-group',w.object_panel._on_select_all,theme)
 add('group',w._on_group,theme)
 add('ungroup',w._on_ungroup,theme)
 add('move',lambda:w.services.transforms.set_numeric(0,position_mm=[12,24,36]),theme)
 add('rotate',lambda:(w.scene.set_selection({0}),w.transform_bar.rotate_btn.click(),w.transform_bar.z_field.setValue(30)),theme)
 add('resize',lambda:(w.transform_bar.resize_btn.click(),w.transform_bar.lock_btn.setChecked(True),w.transform_bar.x_field.setValue(50)),theme)
 add('settle',w._on_settle,theme)
 add('mirror-y',lambda:w.services.transforms.mirror([0],'y'),theme)
 add('align-z-max',lambda:w.services.transforms.align([1,0],'z','max'),theme)
 add('arrange-z-gap5',lambda:w.services.transforms.arrange([1,0],'z',5),theme)
 add('hidden-first',lambda:(setattr(w.scene.objects[0],'visible',False),w.viewport.update()),theme)
 add('delete',w._on_delete,theme)
 add('delete-undo',w._on_undo,theme)
index=0
def tick():
 global index
 try:
  if index>=len(operations):
   (root/'python-native-results.json').write_text(json.dumps(records,indent=2)); os._exit(0)
  activate(); name,fn,theme=operations[index]; fn(); w.viewport.update(); app.processEvents(); QTimer.singleShot(700,lambda:capture_then_next(name,theme))
 except Exception:
  (root/'logs'/'python-native-error.txt').write_text(traceback.format_exc()); os._exit(1)
def capture_then_next(name,theme):
 global index
 try: capture(name,theme); index+=1; QTimer.singleShot(150,tick)
 except Exception:
  (root/'logs'/'python-native-error.txt').write_text(traceback.format_exc()); os._exit(1)
QTimer.singleShot(2000,tick)
sys.exit(app.exec())



