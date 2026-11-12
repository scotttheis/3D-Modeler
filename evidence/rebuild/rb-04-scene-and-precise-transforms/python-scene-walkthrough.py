"""Read-only reference: live widgets/handlers, shared fixtures, exact scene records."""
from pathlib import Path
prefix=(Path(__file__).with_name('python-viewport-walkthrough.py')).read_text()
exec(compile(prefix[:prefix.index('records=')],str(__file__)+'-bootstrap','exec'))
import numpy as np
from PyQt6.QtCore import QPoint, Qt
from PyQt6.QtTest import QTest
from PyQt6.QtWidgets import QComboBox, QDoubleSpinBox
from PyQt6.QtCore import QEventLoop
from src.project_io import load_project, save_project
records=[]; operations=[]; current={}; specs=[]
fixtures=root/'native-scene-fixtures';fixtures.mkdir(exist_ok=True)
def objects():
 return [dict(id=o.object_id,name=o.name,visible=o.visible,transform=o.transform.tolist(),children=[dict(id=c['object_id'],name=c['name']) for c in getattr(o,'_children',[])]) for o in w.scene.objects]
def capture(name,theme):
 require_default_desktop()
 if app.activeModalWidget() is None:activate()
 foreground=user.GetForegroundWindow();owner=ctypes.c_uint();user.GetWindowThreadProcessId(foreground,ctypes.byref(owner));assert owner.value==os.getpid()
 app.processEvents();QTest.qWait(160)
 pix=app.primaryScreen().grabWindow(0,w.mapToGlobal(w.rect().topLeft()).x(),w.mapToGlobal(w.rect().topLeft()).y(),w.width(),w.height())
 path=root/'captures'/f'python-{theme}-scene-{name}.png';assert pix.save(str(path))
 records.append(dict(action='scene-'+name,theme=theme,width=w.width(),height=w.height(),dpi=int(user.GetDpiForWindow(int(w.winId()))),device_pixel_ratio=w.devicePixelRatioF(),image=path.relative_to(root).as_posix(),route='Visible Qt/OpenGL; real widget actions and application handlers. Qt mouse/key event delivery for selection/focus; prepared attributes for absent naming/visibility controls; multi-selection proportional lock is set through its hidden widget.',selection=sorted(w.scene.selected_indices),objects=objects(),undo=len(w.scene.undo_stack),fixture=current['fixture'],history_delta=len(w.scene.undo_stack)-current['history'],display_unit=w.transform_bar._display_unit,transform_mode=w.transform_bar._mode))
 (root/'python-scene-results.json').write_text(json.dumps(records,indent=2))
def setup(theme,name,selection=(0,),kind='base'):
 w._set_theme_mode(theme);w.scene.clear();w.object_panel.sticky_sel.setChecked(False)
 for i,(size,pos) in enumerate([((20,30,40),(0,15,0)),((10,20,30),(40,10,0)),((12,12,12),(-30,6,0))]):
  w.scene.add_object(make_box(size),'Box '+chr(65+i));w.scene.objects[i].translate(*pos)
 if kind=='group':w.scene.set_selection({0,1});w.scene.group_selected()
 if kind=='hidden':w.scene.objects[1].visible=False
 if kind=='named-hidden':w.scene.objects[0].name='Precision part';w.scene.objects[1].visible=False
 if kind=='unhealthy':
  import trimesh
  w.scene.clear();w.scene.add_object(trimesh.load_mesh(str(root/'open-triangle.stl')),'Open triangle')
 fixture=fixtures/(name+'.3dp')
 if not fixture.exists():save_project(w.scene,str(fixture))
 w.scene.replace_contents(load_project(str(fixture)))
 w.scene.set_selection(set())
 for selected_index in selection:w.scene.add_to_selection(selected_index)
 w._show_workspace();w.set_object_snapping(False)
 w.ribbon.set_active_tab('Insert')
 w.transform_bar.move_btn.click();w.transform_bar._set_display_unit('mm');w.transform_bar.lock_btn.setChecked(False)
 w.viewport.camera.reset_view();w.viewport.frame_scene();w.viewport.update();app.processEvents()
 if name=='shift-range':row(0);row(0)
 current.update(fixture=fixture.relative_to(root).as_posix(),history=len(w.scene.undo_stack),before=objects(),name=name)
 if theme=='light':specs.append(dict(name=name,selection=list(selection),kind=kind,fixture=current['fixture']))
 (root/'native-scene-case-specs.json').write_text(json.dumps(specs,indent=2))
def row(index,modifier=Qt.KeyboardModifier.NoModifier):
 lw=w.object_panel.list_widget;QTest.mouseClick(lw.viewport(),Qt.MouseButton.LeftButton,modifier,lw.visualItemRect(lw.item(index)).center())
def empty(modifier=Qt.KeyboardModifier.NoModifier):
 QTest.mouseClick(w.viewport,Qt.MouseButton.LeftButton,modifier,QPoint(12,35))
def number(mode,axis,value,unit='mm',lock=False):
 getattr(w.transform_bar,mode+'_btn').click();w.transform_bar._set_display_unit(unit);w.transform_bar.lock_btn.setChecked(lock)
 getattr(w.transform_bar,axis+'_field').setValue(value)
def focus_delete():
 w.transform_bar.x_field.setFocus();app.processEvents();QTest.keyClick(w.transform_bar.x_field,Qt.Key.Key_Delete)
 assert objects()==current['before'];assert len(w.scene.undo_stack)==current['history']
 w.viewport.setFocus()
def paste_other_selection():
 w._on_copy();w.scene.set_selection({2});w._on_paste()
def invalid_numeric():
 field=w.transform_bar.x_field;field.setFocus();field.selectAll();QTest.keyClicks(field,'NaN');QTest.keyClick(field,Qt.Key.Key_Return)
 assert objects()==current['before'];assert len(w.scene.undo_stack)==current['history'];w.viewport.setFocus()
def modal_action(name,handler,steps):
 def respond(index):
  try:
   require_default_desktop();dialog=app.activeModalWidget();assert dialog is not None
   kind,value=steps[index]
   if kind=='combo':dialog.findChild(QComboBox).setCurrentText(value)
   if kind=='gap':dialog.findChild(QDoubleSpinBox).setValue(value)
   capture(name+'-dialog-'+str(index),current['theme'])
   if index+1<len(steps):QTimer.singleShot(400,lambda:respond(index+1))
   if kind in ['cancel','invalid-gap']:
    if kind=='invalid-gap':
     field=dialog.findChild(QDoubleSpinBox);field.setValue(-1);assert field.value()==0
     capture(name+'-clamped-zero',current['theme'])
    dialog.reject()
   else:dialog.accept()
  except Exception:
   (root/'logs/python-scene-error.txt').write_text(traceback.format_exc());os._exit(1)
 QTimer.singleShot(500,lambda:respond(0));handler()
def health_repair():
 w.ribbon.set_active_tab('Object');loop=QEventLoop();deadline=time.monotonic()+20
 def poll():
  try:
   require_default_desktop();dialog=app.activeModalWidget()
   if dialog is not None:
    capture('health-repair-report',current['theme']);dialog.accept();loop.quit();return
   if time.monotonic()>deadline:raise RuntimeError('Reference repair report timed out')
   QTimer.singleShot(150,poll)
  except Exception:
   (root/'logs/python-scene-error.txt').write_text(traceback.format_exc());os._exit(1)
 w._on_repair();QTimer.singleShot(200,poll);loop.exec()
cases=[
 ('row-toggle',lambda:row(1),(0,),'base'),
 ('ctrl-toggle',lambda:row(0,Qt.KeyboardModifier.ControlModifier),(0,),'base'),
 ('shift-range',lambda:row(2,Qt.KeyboardModifier.ShiftModifier),(0,),'base'),
 ('select-all',w.object_panel._on_select_all,(0,),'base'),
 ('deselect-all',w.object_panel._on_deselect_all,(0,1),'base'),
 ('invert',w.object_panel._on_invert_selection,(0,),'base'),
 ('sticky-empty',lambda:(w.object_panel.sticky_sel.setChecked(True),empty()),(0,),'base'),
 ('empty-clear',empty,(0,),'base'),
 ('ctrl-empty',lambda:empty(Qt.KeyboardModifier.ControlModifier),(0,),'base'),
 ('duplicate',w._on_duplicate,(0,),'base'),
 ('copy',w._on_copy,(0,),'base'),
 ('paste-other-selection',paste_other_selection,(0,),'base'),
 ('cut',w._on_cut,(0,1),'base'),
 ('delete',w._on_delete,(0,1),'base'),
 ('group',w._on_group,(0,1),'hidden'),
 ('ungroup',w._on_ungroup,(1,),'group'),
 ('move-mm',lambda:number('move','x',12),(0,),'base'),
 ('move-inch',lambda:number('move','x',1,'in'),(0,),'base'),
 ('rotate-degree',lambda:number('rotate','z',30),(0,),'base'),
 ('resize-inch',lambda:number('resize','x',1,'in'),(0,),'base'),
 ('resize-locked-inch',lambda:number('resize','y',2,'in',True),(0,),'base'),
 ('multi-move-inch',lambda:number('move','y',2,'in'),(0,1),'hidden'),
 ('multi-rotate',lambda:number('rotate','z',30),(0,1),'base'),
 ('multi-resize-locked',lambda:number('resize','y',60,'mm',True),(0,1),'base'),
 ('delete-text-focus',focus_delete,(0,),'base'),
 ('numeric-invalid',invalid_numeric,(0,),'base'),
 ('units-only',lambda:w.transform_bar._set_display_unit('in'),(0,),'base'),
 ('rename-reference',lambda:(setattr(w.scene.objects[0],'name','Precision part'),w.scene.objects_changed.emit()),(0,),'base'),
 ('visibility-reference',lambda:(setattr(w.scene.objects[1],'visible',False),w.viewport.update()),(0,1),'base'),
 ('mirror-y',lambda:modal_action('mirror-y',w._on_mirror,[('combo','y')]),(0,1),'hidden'),
 ('align-z-max',lambda:modal_action('align-z-max',w._on_align,[('combo','z'),('combo','max')]),(1,0),'base'),
 ('arrange-z-gap5',lambda:modal_action('arrange-z-gap5',w._on_arrange,[('combo','z'),('gap',5)]),(1,0),'base'),
 ('mirror-cancel',lambda:modal_action('mirror-cancel',w._on_mirror,[('cancel',None)]),(0,1),'base'),
 ('arrange-invalid-cancel',lambda:modal_action('arrange-invalid-cancel',w._on_arrange,[('combo','z'),('invalid-gap',None)]),(0,1),'base'),
 ('measure',lambda:modal_action('measure',w._on_measure,[('measure',None)]),(0,1),'hidden'),
 ('health-warning',lambda:w.ribbon.set_active_tab('Object'),(0,),'unhealthy'),
 ('health-repair',health_repair,(0,),'unhealthy'),
]
for theme in ['light','dark']:
 for name,fn,selection,kind in cases:
  operations.append((name+'-before',lambda t=theme,n=name,s=selection,k=kind:setup(t,n,s,k),theme))
  operations.append((name+'-after',fn,theme))
  if name in ['duplicate','paste-other-selection','cut','delete','group','ungroup','move-mm','move-inch','rotate-degree','resize-inch','resize-locked-inch','multi-move-inch','multi-rotate','multi-resize-locked','mirror-y','align-z-max','arrange-z-gap5']:
   operations.append((name+'-undo',w._on_undo,theme))
   operations.append((name+'-redo',w._on_redo,theme))
index=0
def tick():
 global index
 try:
  if index>=len(operations):os._exit(0)
  activate();name,fn,theme=operations[index];current['theme']=theme;fn();w.viewport.update();app.processEvents();QTimer.singleShot(350,lambda:finish(name,theme))
 except Exception:
  (root/'logs/python-scene-error.txt').write_text(traceback.format_exc());os._exit(1)
def finish(name,theme):
 global index
 try:
  capture(name,theme)
  if not name.endswith('-before'):save_project(w.scene,str(root/'native-scene-states'/f'python-{theme}-scene-{name}.3dp'))
  index+=1;QTimer.singleShot(80,tick)
 except Exception:
  (root/'logs/python-scene-error.txt').write_text(traceback.format_exc());os._exit(1)
QTimer.singleShot(1800,tick)
sys.exit(app.exec())
