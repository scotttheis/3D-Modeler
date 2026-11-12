"""Bounded physical viewport selection replay; source files stay read-only."""
from pathlib import Path
bootstrap=Path(__file__).with_name('python-viewport-walkthrough.py').read_text()
exec(compile(bootstrap[:bootstrap.index('records=')],str(__file__)+'-bootstrap','exec'))
records=[]
fixture=root/'native-scene-fixtures/row-toggle.3dp'
def objects():
 return [dict(id=o.object_id,name=o.name,visible=o.visible,transform=o.transform.tolist()) for o in w.scene.objects]
def capture(theme,state):
 activate();app.processEvents();QTest.qWait(300)
 top=w.mapToGlobal(w.rect().topLeft());path=root/'captures'/f'python-{theme}-pick-{state}.png'
 assert app.primaryScreen().grabWindow(0,top.x(),top.y(),w.width(),w.height()).save(str(path))
 records.append(dict(theme=theme,action='pick-'+state,selection=sorted(w.scene.selected_indices),objects=objects(),undo=len(w.scene.undo_stack),width=w.width(),height=w.height(),dpi=int(user.GetDpiForWindow(int(w.winId()))),fixture=fixture.relative_to(root).as_posix(),route='Guarded OS mouse/key events in visible Qt/OpenGL viewport; ray search chooses an interior face point, no direct selection mutation after setup'))
 (root/'python-pick-results.json').write_text(json.dumps(records,indent=2))
def click(x,y,ctrl=False):
 activate();require_default_desktop()
 p=w.viewport.mapToGlobal(QPoint(x,y));user.SetCursorPos(p.x(),p.y());QTest.qWait(60)
 if ctrl:user.keybd_event(17,0,0,0)
 try:
  user.mouse_event(2,0,0,0,0);QTest.qWait(80);user.mouse_event(4,0,0,0,0);QTest.qWait(400)
 finally:
  if ctrl:user.keybd_event(17,0,2,0)
def run():
 try:
  for theme in ['light','dark']:
   require_default_desktop();w._set_theme_mode(theme)
   w.scene.replace_contents(load_project(str(fixture)));w.scene.set_selection(set())
   w.object_panel.sticky_sel.setChecked(False);w._show_workspace();w.ribbon.set_active_tab('Insert')
   w.transform_bar.move_btn.click();w.viewport.camera.reset_view();w.viewport.frame_scene();app.processEvents();QTest.qWait(600)
   before=objects();history=len(w.scene.undo_stack);capture(theme,'before')
   candidates=[]
   for y in range(80,w.viewport.height()-60,15):
    for x in range(40,w.viewport.width()-40,15):
     hit,face,distance=w.scene.hit_test_face(*w.viewport.screen_ray(x,y))
     if hit==0 and app.widgetAt(w.viewport.mapToGlobal(QPoint(x,y))) is w.viewport:candidates.append((x,y))
   assert candidates,'No visible front face of object0'
   x,y=candidates[len(candidates)//2];click(x,y);assert w.scene.selected_indices=={0};capture(theme,'after')
   click(12,35,True);assert w.scene.selected_indices=={0};capture(theme,'ctrl-empty')
   click(12,35);assert not w.scene.selected_indices;capture(theme,'clear')
   assert objects()==before and len(w.scene.undo_stack)==history
  (root/'python-pick-verification.json').write_text(json.dumps(dict(complete=True,themes=2,states=len(records),physical_input=True,geometry_history_unchanged=True),indent=2))
  os._exit(0) # Close only this owned fixture process; avoid a dirty-exit prompt.
 except Exception:
  (root/'logs/python-pick-error.txt').write_text(traceback.format_exc());os._exit(1)
QTimer.singleShot(1200,run)
sys.exit(app.exec())
