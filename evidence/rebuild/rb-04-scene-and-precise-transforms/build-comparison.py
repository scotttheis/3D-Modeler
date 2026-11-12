from pathlib import Path
from PIL import Image, ImageDraw
import hashlib, html, json
root=Path(__file__).resolve().parent
captures=root/'captures'
common="Python uses tall actions, large wireframe tiles, compact icon controls, strong selection outlines and flatter shading. Rust uses wrapped actions, shaded rows, a side inspector and larger text-mode numeric controls, with different grid/lighting. Visual and owner acceptance remain pending."
notes={'items': 'Python starts with Box A selected; Rust project load starts with no selection.', 'move': 'Python handler updates object 0 after ungroup with no current selection; Rust selects Box A and edits the bottom numeric fields.', 'rotate': 'Numeric rotation route in these older captures. Current code adds all-axis grouped viewport ring drags and passes 122 independent gesture cases; native pointer captures are recorded under viewport actions.', 'resize': 'Numeric resizing in these older captures. Current code matches the reference 14 visible face/corner handles; internal edge math is tested separately. Native evidence is recorded under viewport actions.', 'settle': 'These images predate the QHull/composition repair. Exact repaired-code single/group matrices pass unchanged in focused tests. The matched native before/after/undo/redo replay is recorded under settle-1/2 actions.', 'mirror-y': 'Python capture is the result of its semantic handler. Rust chose Y in its placement dialog. Dark Rust uses the intermediate window presentation; Light uses the final modal presentation.', 'measure': 'Python shows a native Dimensions message box. Final Rust captures in both themes show an in-app modal with per-object Width/Depth/Height in mm; the status strip stays compact. The Measure pair uses the original two-object fixture in both apps.', 'align-z-max': 'Both runs request Z/max. Python uses the shared handler; Rust chooses the controls in the live dialog.', 'arrange-z-gap5': 'Both runs request Z with 5 mm gap. Rust Light additionally captures rejected negative input.', 'hidden-first': 'Python has no row visibility button; the harness changed its visibility flag. Rust used its existing Eye control to satisfy the requested visibility scope. The visible difference remains for review; owner acceptance is pending.', 'delete': 'Python and Rust have different accumulated selections in this capture. This is route evidence, not an exact scene-state comparison.', 'delete-undo': 'Both show restoration after undo; exact durable identity is asserted in focused tests rather than inferred from screenshots.', 'group': 'Grouped object appears in Items. Child identity/metadata is covered by core tests.', 'ungroup': 'Both clear selection after restoring children, following the reference.', 'sticky': 'Both expose Sticky Selection. This image proves reachability, not the entire empty-click/modifier matrix.'}

pairs=[]
output=['<!doctype html><meta charset="utf-8"><title>RB-04 comparison</title><style>body{font:16px Segoe UI;margin:24px}section{border:1px solid #aaa;padding:14px;margin:18px 0}.images{display:flex;gap:12px}figure{width:50%;margin:0}img{width:100%}p{max-width:1200px}</style><h1>RB-04 comparison: verified native action results</h1><p>Python is left; Rust is right. 1280 x 840 client pixels, 96 DPI. Sequential target windows. Native scene, viewport and Settle results are classified separately below with saved state verification and genuine per-frame build identities. Other pairs are historical intermediate captures and may contain accumulated-state differences explicitly labelled below. Agent inspection does not constitute visual review or owner acceptance; those statuses remain pending.</p>']
for theme in ['light','dark']:
 for py in sorted(captures.glob(f'python-{theme}-*.png')):
  action=py.stem.removeprefix(f'python-{theme}-'); rs=captures/f'rust-{theme}-{action}.png'
  if not rs.exists():continue
  current_settle=bool(__import__('re').fullmatch(r'settle-[12]-(before|after|undo|redo)',action))
  note=notes.get(action,'Rust native pointer/keyboard route and Python visible Qt widget/semantic route were recorded. Current history/identity contracts are independently tested; these frames do not prove final-code parity.')
  if current_settle:note='Current-code native Settle replay: matched one/two-box input poses, before/after/one undo/one redo. Saved Rust matrices match Python within 9.32e-9 per entry; IDs are preserved and one history action is verified. Python uses a real visible Qt/OpenGL window and semantic handlers; Rust uses guarded physical pointer/keyboard. The different camera framing, grid, soft versus flat shading, selection outlines, Items rows/tiles and numeric strips remain visible.'
  current_viewport=action.startswith('viewport-')
  current_scene=action.startswith('scene-')
  current_pick=action.startswith('pick-')
  if current_viewport:
   note='Guarded physical pointer gesture in both applications using the shared input fixture. Each committed Rust drag preserves IDs, changes every selected target, and saves exact undo/redo matrices with one history transition per action. Python and Rust camera/gizmo placement produce different world deltas for the same screen travel; compare each route against its pointer geometry rather than claiming identical drag endpoints.'
   if 'cancel' in action: note+=' Python Escape leaves its moved pose without a history entry and clears selection. Rust discards preview and preserves the input matrices/history.'
   if 'focus' in action: note+=' Python focus loss completes the drag; Rust safely discards its preview. No shared result is claimed for this observed variance.'
   if 'hidden' in action: note+=' Both retain the hidden selected item in Items and transform it with the visible selected item.'
   if 'resize' in action: note+=' Reference and Rust expose 6 face and 8 corner handles. Corners resize uniformly; face resizing fixes the opposite face. Grid/Object basis is identified by the action name.'
   note+=' Rust currently displays inches in these native viewport frames; Python displays millimetres. Stored model values remain millimetres.'
  if current_pick:
   note='Physical viewport hit and Ctrl/ordinary empty-space clicks in both visible applications. Rust updates the selected row/count while its previous status message can remain; Python names the picked object in the status bar. Selection goes empty -> object0 -> object0 -> empty without changing geometry, IDs or history. Source chooses a point by exact front-face ray search; Rust input is checked through highlighted Items rows and saved unchanged scenes. Camera framing, selected outlines and centered versus near-face gizmo placement differ visibly.'
  if current_scene:
   note='Shared scene fixture; Python real visible widgets/handlers and Rust guarded physical controls. Saved output, ID, mesh and history evidence accompanies the visible result. Visual/owner acceptance remains pending.'
   if any(key in action for key in ['row-toggle','ctrl-toggle','shift-range','select-all','deselect-all','invert','sticky','empty']):note+=' Python highlights large wireframe tiles under tall actions; Rust highlights shaded rows under wrapped actions. The compared highlighted indices/counts and empty-space result are checked against the source capture records. Selection creates no geometry history.'
   if 'shift-range' in action:note+=' Observed Python Shift-click selects endpoints [0,2]; Rust implements the handoff contiguous anchor range [0,1,2]. This difference is explicit and owner/visual review remains pending.'
   if any(key in action for key in ['duplicate','copy','paste','cut','delete']):note+=' The visible object count, names and selected targets are compared. Python uses ribbon/shortcut routes; Rust also exposes inline Items actions. Copy preserves the scene; paste survives a selection change and creates independent IDs; model edits undo/redo once.'
   if 'group' in action:note+=' Python shows a combined wireframe Group tile; Rust shows a shaded Group row and child count. Hidden children remain retained for ungrouping; the group renders its combined root mesh. Existing child IDs and metadata are verified.'
   if any(key in action for key in ['move-mm','move-inch','rotate-degree','resize-inch','locked','multi-move','multi-rotate']):note+=' Python uses compact two-decimal spin boxes and immediate axis edits; Rust uses larger three-decimal fields with explicit suffixes and Apply/Enter. Untouched exact axes and millimetre storage are verified. The Python multi-selection proportional-lock widget is hidden; the harness sets its supported state explicitly, while Rust exposes the checkbox.'
   if any(key in action for key in ['move-mm','multi-resize-locked']):note+=' Rust retains its inch display profile while the reference case displays mm: 12 mm appears as 0.472 in for Move, and 60 mm as 2.362 in for the locked multi-resize Y field. Explicit input suffixes and saved millimetres are verified.'
   if 'numeric-invalid' in action:note+=' Python spin-box validation rejects nonnumeric typing. Rust retains raw invalid text and refuses Apply without changing scene/history; its validation detail is available in the transform inspector.'
   if 'text-focus' in action:note+=' Delete edits the focused numeric text without removing an object or adding model history in either application.'
   if 'units-only' in action:note+=' Python uses the bottom unit-button menu; Rust uses Settings. Display changes leave exact millimetre model values/history unchanged.'
   if 'rename-reference' in action:note+=' Python has no visible rename control: its reference name was prepared through object data. Rust uses its requested Name/Rename controls and adds one undoable command; this is an explicit extension, not observed source rename interaction.'
   if 'visibility-reference' in action:note+=' Python has no visible Eye control: the reference visibility attribute was prepared directly. Rust uses its existing Eye control and one undoable command. Both retain the hidden item row/selection.'
   if any(key in action for key in ['mirror','align','arrange']):note+=' Python uses native axis/target/gap dialogs (two steps for Align/Arrange); Rust combines the choices in one in-app modal. Axis, landmark, ordering, gap, cancellation and saved results are recorded separately.'
   if 'invalid-cancel' in action:note+=' Python clamps its gap spin box at zero and the dialog is canceled; Rust rejects a typed negative gap, then cancels unchanged.'
   if 'measure' in action:note+=' The Light Rust replay is nearly front-on while the reference remains angled; exact measured world values are independently checked. Python uses a native Dimensions message box; Rust uses an in-app modal. Both list each selected object with Width X / Depth Z / Height Y in mm, including hidden selected objects.'
   if 'health' in action:note+=' Python shows a warning glyph/tooltip on the large tile; Rust shows a readable ! Repair control on its shaded row. Python gives a large native Repair Report with remaining issues, geometry counts and named stages; Rust retains its compact Repair form with a stage-count message. Both still flag the flat zero-volume fixture. Repair reachability is verified; detailed report/mesh-operation parity remains RB-06.'

   if 'health-repair' in action:note+=' The repaired open triangle has four faces, equal bounds and surface area, and exact native mesh undo/redo. Python fan point [5,5,0] differs from Rust [6.6666665,6.6666665,0]; detailed tessellation parity remains RB-06.'

  if theme=='dark' and not current_settle and not current_viewport and not current_scene and not current_pick and action not in ['measure','items','select-all','invert','sticky','selected','copy','paste','duplicate','cut','undo','redo','select-all-group','group','ungroup']:note+=' Dark Rust resumed the two-object fixture; Python retained its third clipboard-created object.'
  pair=dict(theme=theme,action=action,build_phase='native-pick' if current_pick else 'native-viewport' if current_viewport else 'native-scene' if current_scene else 'current-native-settle' if current_settle else 'historical',python=py.relative_to(root).as_posix(),rust=rs.relative_to(root).as_posix(),differences=note)
  pairs.append(pair)
  output.append(f'<section><h2>{theme.title()} - {html.escape(action)}</h2><div class="images">'+''.join(f'<figure><figcaption>{label}</figcaption><a href="{path}"><img src="{path}"></a></figure>' for label,path in [('Python',pair['python']),('Rust',pair['rust'])])+f'</div><p>{html.escape(note)}</p><p>{common}</p></section>')
(root/'visual-comparison.html').write_text('\n'.join(output),encoding='utf-8')
(root/'comparison-pairs.json').write_text(json.dumps(pairs,indent=2),encoding='utf-8')
old=json.loads((root/'capture-metadata.json').read_text(encoding='utf-8')) if (root/'capture-metadata.json').exists() else {}
for old_key,new_key in [('final_native_build_sha256','historical_measure_health_build_sha256'),('final_native_capture_paths','historical_measure_health_capture_paths'),('source_hashes','historical_capture_source_hashes')]:
 if old_key in old:old[new_key]=old.pop(old_key)
meta=[]
current_build='3f84eb8c288f6db21107763ad289c4b9874bed5312f339ad679a18bd0df705cf'
for path in sorted(captures.glob('*.png')):
 with Image.open(path) as im:meta.append(dict(path=path.relative_to(root).as_posix(),width=im.width,height=im.height,sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
old['historical_native_settle_build_sha256']=current_build
old['current_native_settle_capture_paths']=[p.relative_to(root).as_posix() for p in sorted(captures.glob('rust-*-settle-[12]-*.png'))]
journal={}
for line in (root/'rust-current-native-captures.ndjson').read_text(encoding='utf-8-sig').splitlines():
 if line.strip():
  record=json.loads(line);journal[Path(record['Path']).name]=record
old['native_rust_capture_journal_latest']=list(journal.values())
settle_builds=sorted({record['BuildSha256'] for name,record in journal.items() if __import__('re').fullmatch(r'rust-(light|dark)-settle-[12]-(before|after|undo|redo)\.png',name)})
old['current_native_settle_build_sha256']=settle_builds[0] if len(settle_builds)==1 else settle_builds
old['native_reference_records']={name:json.loads((root/name).read_text(encoding='utf-8-sig')) for name in ['python-settle-results.json','python-viewport-results.json','python-scene-results.json','python-pick-results.json'] if (root/name).exists()}

reference=Path('L:/Builder3D')
import subprocess,sys
old['reference_build_identity']=dict(python_executable=sys.executable,python_version=sys.version,python_executable_sha256=hashlib.sha256(Path(sys.executable).read_bytes()).hexdigest(),git_head=subprocess.run(['git','--no-optional-locks','-c','safe.directory=L:/Builder3D','-C',str(reference),'rev-parse','HEAD'],capture_output=True,check=True,text=True).stdout.strip(),source_sha256={p.relative_to(reference).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((reference/'src').rglob('*.py'))},scope='Read-only reference identity recorded at completed native boundary; pre-existing dirty Git status remains unchanged. Source widgets/handlers versus physical input are labelled per action.')
old.update(client_logical=[1280,840],dpi=96,scale=1,limitations=['Original scene captures are historical; current native Settle/viewport/scene evidence is classified per action and journal build identity','Python scene widget/handler routes are labelled; viewport gestures use guarded OS pointer/key events','Historical Dark accumulated scene differs from Python after ungroup; matched native-scene cases use shared fixtures','Visual and owner review pending'],captures=meta)
(root/'capture-metadata.json').write_text(json.dumps(old,indent=2),encoding='utf-8')
for app in ['python','rust']:
 for theme in ['light','dark']:
  paths=sorted(captures.glob(f'{app}-{theme}-*.png'));cellw,cellh=320,230
  sheet=Image.new('RGB',(cellw*4,cellh*((len(paths)+3)//4)),'white');draw=ImageDraw.Draw(sheet)
  for i,path in enumerate(paths):
   with Image.open(path) as im:im.thumbnail((320,210));sheet.paste(im,((i%4)*cellw,(i//4)*cellh))
   draw.text(((i%4)*cellw+3,(i//4)*cellh+211),path.stem,fill='black')
  sheet.save(root/f'{app}-{theme}-contact.png')
print('PAIRS',len(pairs),'CAPTURES',len(meta),'historical build metadata preserved')
