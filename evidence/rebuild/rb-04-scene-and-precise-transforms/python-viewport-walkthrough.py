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

import numpy as np
from src.project_io import save_project,load_project
from PyQt6.QtCore import QPoint
from PyQt6.QtTest import QTest
records=json.loads((root/'python-viewport-results.json').read_text(encoding='utf-8')) if any(arg.startswith('--resume') for arg in sys.argv) else []
operations=[]
fixtures=root/'native-viewport-fixtures';fixtures.mkdir(exist_ok=True)
def objects():
 return [dict(id=o.object_id,name=o.name,visible=o.visible,transform=o.transform.tolist(),children=[dict(id=c['object_id'],name=c['name'],visible=c.get('visible',True)) for c in getattr(o,'_children',[])]) for o in w.scene.objects]
def capture(name,theme,extra=None):
 activate();require_default_desktop()
 if user.GetForegroundWindow()!=int(w.winId()):raise RuntimeError('Reference lost foreground; no capture sent')
 pix=app.primaryScreen().grabWindow(0,w.mapToGlobal(w.rect().topLeft()).x(),w.mapToGlobal(w.rect().topLeft()).y(),w.width(),w.height())
 path=root/'captures'/f'python-{theme}-{name}.png';assert pix.save(str(path))
 record=dict(action=name,theme=theme,width=w.width(),height=w.height(),dpi=96,image=path.relative_to(root).as_posix(),route='Visible Qt/OpenGL window; OS mouse/key events for drag; real widget activation for mode/fixture setup',selection=sorted(w.scene.selected_indices),objects=objects(),undo=len(w.scene.undo_stack),camera=dict(yaw=float(w.viewport.camera.yaw),pitch=float(w.viewport.camera.pitch),dist=float(w.viewport.camera.dist),center=w.viewport.camera.center.tolist()))
 if extra:record.update(extra)
 records[:]=[r for r in records if (r['action'],r['theme'])!=(name,theme)]
 records.append(record);(root/'python-viewport-results.json').write_text(json.dumps(records,indent=2),encoding='utf-8')

cases=[
 ('viewport-move-single','move','x','single',False,'commit'),
 ('viewport-move-hidden','move','x','hidden',False,'commit'),
 ('viewport-move-group','move','x','group',False,'commit'),
 ('viewport-rotate-single','rotate','z','single',False,'commit'),
 ('viewport-rotate-group','rotate','z','group',False,'commit'),
 ('viewport-resize-grid-face','resize','face_0','rotated',False,'commit'),
 ('viewport-resize-object-face','resize','face_0','rotated',True,'commit'),
 ('viewport-resize-corner','resize','corner_6','single',False,'commit'),
 ('viewport-move-noop','move','x','single',False,'noop'),
 ('viewport-move-cancel','move','x','single',False,'escape'),
 ('viewport-move-focus','move','x','single',False,'focus'),
]
current={}
def setup(theme,case):
 name,mode,handle,kind,basis,end=case
 w._set_theme_mode(theme);w.scene.clear()
 w.scene.add_object(make_box((20,30,40)),'Box A');w.scene.objects[0].translate(0,15,0)
 w.scene.add_object(make_box((10,20,30)),'Box B');w.scene.objects[1].translate(40,10,0)
 if kind=='rotated':w.scene.objects[0].rotate(25,[0,0,1])
 if kind=='hidden':w.scene.objects[1].visible=False
 if kind=='group':w.scene.set_selection({0,1});w.scene.group_selected()
 w.scene.set_selection(set(range(len(w.scene.objects))) if kind=='hidden' else {0})
 fixture=fixtures/f'{name}.3dp'
 if fixture.exists():
  loaded=load_project(str(fixture));w.scene.objects=loaded.objects;w.scene.objects_changed.emit()
  w.scene.set_selection(set(range(len(w.scene.objects))) if kind=='hidden' else {0})
 else:save_project(w.scene,str(fixture))
 w._show_workspace();w.set_object_snapping(False);w.transform_bar._set_display_unit('mm')
 getattr(w.transform_bar,mode+'_btn').click();w.transform_bar.resize_alignment_btn.setChecked(basis)
 w.viewport.camera.reset_view();w.viewport.frame_scene();w.viewport.update();app.processEvents()
 current.update(theme=theme,case=case,before=objects(),history=len(w.scene.undo_stack),fixture=fixture.relative_to(root).as_posix())

def find_handle(desired):
 vp=w.viewport
 view=vp.camera.get_view_matrix();focal=1/np.tan(np.radians(45)/2);width,height=vp.viewport_size
 points=[]
 for primitive in vp.gizmo.build_render_data(vp.camera,vp.viewport_size):
  vertices=np.asarray(primitive['vertices']);points.extend(vertices)
  step=3 if primitive['kind']=='triangles' else 2
  points.extend(vertices[i:i+step].mean(axis=0) for i in range(0,len(vertices),step))
 seen=set();fallback=None;started=time.monotonic()
 for point in points:
  eye=view@np.append(point,1.)
  if eye[2]>=0:continue
  sx=(1+(eye[0]*focal/(width/height))/-eye[2])*width/2;sy=(1-eye[1]*focal/-eye[2])*height/2
  for dx,dy in [(0,0),(-2,0),(2,0),(0,-2),(0,2)]:
   x,y=int(round(sx+dx)),int(round(sy+dy))
   if (x,y) in seen or not(0<=x<width and 0<=y<height):continue
   if app.widgetAt(vp.mapToGlobal(QPoint(x,y))) is not vp:continue
   seen.add((x,y));origin,direction=vp.screen_ray(x,y);hit=vp.gizmo.hit_test(origin,direction,vp.camera,vp.viewport_size)
   if hit==desired:return x,y
   if desired.startswith('edge_') and hit and hit.startswith('edge_') and int(hit.split('_')[1])//4==int(desired.split('_')[1])//4:fallback=(x,y,hit)
   if len(seen)%30==0:app.processEvents()
   if time.monotonic()-started>8:break
  if time.monotonic()-started>8:break
 if fallback:
  current['actual_reference_handle']=fallback[2];return fallback[:2]
 for y in range(0,vp.height(),10):
  for x in range(0,vp.width(),10):
   if app.widgetAt(vp.mapToGlobal(QPoint(x,y))) is not vp:continue
   origin,direction=vp.screen_ray(x,y);hit=vp.gizmo.hit_test(origin,direction,vp.camera,vp.viewport_size)
   if hit==desired:return x,y
   if time.monotonic()-started>10:raise RuntimeError('Reference visible-handle search timed out: '+desired)
  app.processEvents()
 raise RuntimeError('No visible reference handle '+desired)
def pointer(x,y):
 require_default_desktop()
 if user.GetForegroundWindow()!=int(w.winId()):raise RuntimeError('Reference not foreground; no pointer event sent')
 pos=w.viewport.mapToGlobal(QPoint(int(x),int(y)));user.SetCursorPos(pos.x(),pos.y());app.processEvents();QTest.qWait(60)
def key(vk):
 require_default_desktop()
 if user.GetForegroundWindow()!=int(w.winId()):raise RuntimeError('Reference not foreground; no key event sent')
 user.keybd_event(vk,0,0,0);user.keybd_event(vk,0,2,0);app.processEvents();QTest.qWait(80)
def perform():
 theme=current['theme'];name,mode,handle,kind,basis,end=current['case'];activate()
 x,y=find_handle(handle);pointer(x,y);user.mouse_event(2,0,0,0,0);app.processEvents();QTest.qWait(100)
 current['os_pressed']=True;current['start']=[x,y];current['handle_active']=bool(w.viewport.gizmo.drag_active)
 if not current['handle_active']:raise RuntimeError('OS press did not start reference gizmo '+handle)
 if end!='noop':
  for dx,dy in [(8,3),(16,7),(24,12),(32,18),(40,24)]:pointer(x+dx,y+dy)
 if end=='escape':key(0x1B)
 if end=='focus':
  require_default_desktop();user.ShowWindowAsync(int(w.winId()),6);app.processEvents();QTest.qWait(250)
  user.ShowWindowAsync(int(w.winId()),9);QTest.qWait(200);activate()
  pointer(x+40,y+24)
 user.mouse_event(4,0,0,0,0);current['os_pressed']=False;app.processEvents();QTest.qWait(250)
 changed=objects()!=current['before'];delta=len(w.scene.undo_stack)-current['history']
 if end=='commit':
  assert changed,(name,'no model change');assert delta==1,(name,'history',delta)
 if end=='noop':assert not changed and delta==0,(name,changed,delta)
 current['after']=objects();current['history_delta']=delta;current['changed']=changed
 capture(name+'-after',theme,dict(fixture=current['fixture'],handle=handle,local_pointer_start=[x,y],local_pointer_end=[x+40,y+24] if end!='noop' else [x,y],history_delta=delta,changed=changed,termination=end))
 save_project(w.scene,str(root/'native-scene-states'/f'python-{theme}-{name}-after.3dp'))
def undo():w._on_undo();w.viewport.update()
def redo():w._on_redo();w.viewport.update()
for theme in ['light','dark']:
 for case in cases:
  if '--resume-dark' in sys.argv and (theme!='dark' or cases.index(case)<1):continue
  if '--resume' in sys.argv and theme=='light' and cases.index(case)<8:continue
  name=case[0]
  operations.append((name+'-before',lambda t=theme,c=case:setup(t,c),theme,True))
  operations.append((name+'-after',perform,theme,False))
  if case[-1]=='commit':
   operations.append((name+'-undo',undo,theme,True));operations.append((name+'-redo',redo,theme,True))
index=0
def tick():
 global index
 try:
  if index>=len(operations):os._exit(0)
  activate();name,fn,theme,take=operations[index];fn();w.viewport.update();app.processEvents()
  if take:QTimer.singleShot(400,lambda:finish(name,theme))
  else:index+=1;QTimer.singleShot(100,tick)
 except Exception:
  if current.get('os_pressed'):
   try:activate();user.mouse_event(4,0,0,0,0)
   except Exception:pass
  (root/'logs/python-viewport-error.txt').write_text(traceback.format_exc(),encoding='utf-8');(root/'logs/python-viewport-error-context.json').write_text(json.dumps(current,default=lambda v:v.item() if hasattr(v,'item') else str(v),indent=2),encoding='utf-8');os._exit(1)
def finish(name,theme):
 global index
 try:capture(name,theme,dict(fixture=current['fixture']));index+=1;QTimer.singleShot(100,tick)
 except Exception:
  (root/'logs/python-viewport-error.txt').write_text(traceback.format_exc(),encoding='utf-8');os._exit(1)
QTimer.singleShot(1800,tick)
sys.exit(app.exec())
