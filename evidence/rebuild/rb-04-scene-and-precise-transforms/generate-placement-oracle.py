"""Real Python placement commands; source tree remains read-only."""
import sys,json
from pathlib import Path
sys.dont_write_bytecode=True;sys.path.insert(0,'L:/Builder3D')
from src.scene import Scene,SceneObject
from src.mesh_ops import make_box
from src.application_services import TransformService
records=[]
for order in [[0,1,2],[2,0,1]]:
 for axis in 'xyz':
  for operation,argument in [('mirror',None)]+[('align',s) for s in ['min','center','max']]+[('arrange',v) for v in [0,5,10000]]:
   scene=Scene()
   for i,(size,position,angle) in enumerate([([20,30,40],[7,11,13],27),([10,20,30],[42,21,13],45),([25,15,35],[-25,4,7],-13)]):
    obj=SceneObject(make_box(size));obj.translate(*position);obj.rotate(angle,[0,0,1]);obj.visible=i!=1;scene.objects.append(obj)
   scene.set_selection(set(order));service=TransformService(scene);before=[o.transform.tolist() for o in scene.objects]
   if operation=='mirror':service.mirror(order,axis)
   elif operation=='align':service.align(order,axis,argument)
   else:service.arrange(order,axis,argument)
   records.append(dict(operation=operation,axis=axis,argument=argument,order=order,before=before,after=[o.transform.tolist() for o in scene.objects]))
Path(__file__).with_name('placement-oracle.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
print(len(records),'Python placement cases')
