"""Invoke real reference TransformBar multi-selection semantics without a window."""
import sys,json
from pathlib import Path
from types import SimpleNamespace
sys.dont_write_bytecode=True
sys.path.insert(0,'L:/Builder3D')
import numpy as np
from src.scene import Scene, SceneObject, get_world_aabb
from src.mesh_ops import make_box
from src.application_services import TransformService
from src.toolbar import TransformBar

records=[]
for mode in ['move','rotate','resize']:
 for unit in ['mm','in']:
  for axis in [None,0,1,2]:
   for locked in ([False,True] if mode=='resize' else [False]):
    scene=Scene()
    for i,(size,position) in enumerate([([20,30,40],[7,11,13]),([10,20,30],[42,21,13])]):
     obj=SceneObject(make_box(size));obj.translate(*position);obj.rotate(27,[0,0,1]);obj.scale(1.2,.8,1.1);obj.visible=i==0;scene.objects.append(obj)
    scene.set_selection({1});scene.add_to_selection(0) # numeric uses scene order, not click anchor order
    service=TransformService(scene);scale=25.4 if unit=='in' else 1.
    def bounds():
     boxes=[get_world_aabb(o) for o in scene.objects];return np.min([b[0] for b in boxes],axis=0),np.max([b[1] for b in boxes],axis=0)
    low,high=bounds();current=service.rotation_degrees(scene.objects[0].transform) if mode=='rotate' else ((low+high)/2 if mode=='move' else high-low)/scale
    values=current.copy();values[axis if axis is not None else 0] += 5 if mode!='resize' else current[axis if axis is not None else 0]*.25
    before=[o.transform.tolist() for o in scene.objects]
    adapter=SimpleNamespace(_selected_indices=lambda:[0,1],_selection_world_bounds=bounds,_mode=mode,_obj=scene.objects[0],transforms=service,_display_scale=lambda:scale,_to_model_units=lambda v:v*scale,lock_btn=SimpleNamespace(isChecked=lambda:locked))
    TransformBar._apply_multi_field_change(adapter,values,axis)
    target=values if mode=='rotate' else values*scale
    if mode=='resize' and locked:
     actual=high-low;lockaxis=0 if axis is None else axis;target=actual*(target[lockaxis]/actual[lockaxis])
    records.append(dict(mode=mode,unit=unit,axis=axis,locked=locked,before=before,target=target.tolist(),after=[o.transform.tolist() for o in scene.objects]))
Path(__file__).with_name('numeric-selection-oracle.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
print(len(records),'real TransformBar multi-selection cases; no GUI/settings writes')
