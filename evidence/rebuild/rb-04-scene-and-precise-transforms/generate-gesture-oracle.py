"""Read-only Python reference: real GizmoManager, no GUI/settings writes."""
import sys, json, math
from pathlib import Path
sys.dont_write_bytecode = True
sys.path.insert(0, 'L:/Builder3D')
import numpy as np
from src.scene import Scene, SceneObject
from src.mesh_ops import make_box
from src.gizmo import GizmoManager

records = []
def setup(kind):
    scene = Scene()
    for i, (size, position) in enumerate([([20,30,40],[7,11,13]),([10,20,30],[42,21,13])][:2 if kind != 'single' else 1]):
        obj = SceneObject(make_box(size)); obj.translate(*position)
        if kind == 'single':
            obj.rotate(27,[0,0,1]); obj.scale(1.2,0.8,1.1)
        obj.visible = i == 0
        scene.objects.append(obj)
    scene.set_selection(set(range(len(scene.objects))))
    if kind == 'group': scene.group_selected()
    g = GizmoManager(); g.on_selection_changed(scene); g.set_object_snap(False)
    return scene,g

def ray(point,axis):
    side = np.cross(axis,[0,0,1] if abs(axis[2]) < .9 else [0,1,0]); side /= np.linalg.norm(side)
    return [np.asarray(point+side*100,dtype=np.float32),np.asarray(-side,dtype=np.float32)]
def plain(pair): return [a.tolist() for a in pair]
def add_case(kind, mode, handle, basis=False):
    scene,g=setup(kind); g.set_mode(mode); g.set_resize_alignment(basis)
    before=[o.transform.tolist() for o in scene.objects]
    center=g._target_center(); descriptor={}
    if mode=='resize':
        geo=g._resize_geometry(); center=geo['center']; axes=geo['axis_basis']; extent=geo['extents']
        if handle.startswith('face_'):
            point,ax,side,axis=geo['faces'][int(handle.split('_')[1])]; index='xyz'.index(ax)
            descriptor=dict(type='face',axis=index,side=side)
            end=center+axis*side*extent[index]*.8
        elif handle.startswith('edge_'):
            point,ax,axis=geo['edges'][int(handle.split('_')[1])]; index='xyz'.index(ax)
            sides=np.rint(2*((point-center)@axes)/extent).tolist()
            descriptor=dict(type='edge',axis=index,sides=sides); end=center+axis*extent[index]*.7
        else:
            point=geo['corners'][int(handle.split('_')[1])]; axis=point-center; axis/=np.linalg.norm(axis)
            sides=np.rint(2*((point-center)@axes)/extent).tolist()
            descriptor=dict(type='corner',sides=sides); end=center+(point-center)*1.25
        start=ray(point,axis); updates=[ray(end,axis)]
    elif mode=='move':
        axis=np.eye(3)['xyz'.index(handle)]; start=ray(center+axis*10,axis); updates=[ray(center+axis*17,axis)]
    else:
        axis=np.eye(3)['xyz'.index(handle)]; u=np.eye(3)[('xyz'.index(handle)+1)%3]; v=np.cross(axis,u)
        def angular(deg):
            angle=math.radians(deg); return [np.asarray(center+10*(u*math.cos(angle)+v*math.sin(angle))+axis*100,dtype=np.float32),np.asarray(-axis,dtype=np.float32)]
        start=angular(170); updates=[angular(a) for a in [-170,-90,0,90]]
    g.start_drag(handle,*start,None)
    for update in updates: g.drag_update(*update)
    records.append(dict(kind=kind,mode=mode,handle=handle,basis=basis,descriptor=descriptor,before=before,start=plain(start),updates=[plain(r) for r in updates],after=[o.transform.tolist() for o in scene.objects]))

for kind in ['single','mixed-hidden','group']:
    for mode in ['move','rotate']:
        for handle in 'xyz': add_case(kind,mode,handle)
    for basis in ([False,True] if kind=='single' else [False]):
        for prefix,count in [('face',6),('edge',12),('corner',8)]:
            for i in range(count): add_case(kind,'resize',f'{prefix}_{i}',basis)
Path(__file__).with_name('gesture-oracle.json').write_text(json.dumps(records,indent=2),encoding='utf-8')
print(f'{len(records)} real Python gizmo cases; no GUI or reference writes')
