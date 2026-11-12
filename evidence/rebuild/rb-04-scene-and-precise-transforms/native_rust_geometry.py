"""Compute native test pointer coordinates from the existing Rust camera contract.

This helper only reads .3dp fixtures. Actual model edits use guarded OS input.
"""
import argparse,json,math,struct,zipfile
import numpy as np
def stl_vertices(data):
 count=struct.unpack_from('<I',data,80)[0]
 return np.array([struct.unpack_from('<3f',data,84+i*50+12+j*12) for i in range(count) for j in range(3)],dtype=np.float32)
def normalize(v):return v/np.linalg.norm(v)
def project(matrix,point,width,height,top):
 clip=matrix@np.append(point,np.float32(1)).astype(np.float32)
 if clip[3]<=0:return None
 return np.array([(clip[0]/clip[3]+1)*width/2,top+(1-clip[1]/clip[3])*height/2])
def ray(matrix,point,width,height,top):
 inv=np.linalg.inv(matrix).astype(np.float32);x=point[0]/width*2-1;y=1-(point[1]-top)/height*2
 a=inv@np.array([x,y,0,1],dtype=np.float32);b=inv@np.array([x,y,1,1],dtype=np.float32)
 a=a[:3]/a[3];b=b[:3]/b[3];return a,normalize(b-a)
def ring_hit(matrix,screen,center,radius,tolerance,width,height,top):
 origin,direction=ray(matrix,screen,width,height,top);hits=[]
 for i,axis in enumerate(np.eye(3,dtype=np.float32)):
  denom=direction@axis
  if abs(denom)<1e-6:continue
  t=((center-origin)@axis)/denom
  if t<0:continue
  if abs(np.linalg.norm(origin+direction*t-center)-radius)<=tolerance:hits.append((t,i))
 return min(hits)[1] if hits else None
def coordinates(path,mode,handle,selected,basis=False,top=168,width=1044,height=672):
 with zipfile.ZipFile(path) as archive:
  scene=json.loads(archive.read('scene.json'));items=[]
  for obj in scene['objects']:
   mesh=stl_vertices(archive.read(obj['mesh_file']));transform=np.array(obj['transform'],dtype=np.float32)
   world=mesh@transform[:3,:3].T+transform[:3,3]
   low,high=mesh.min(axis=0),mesh.max(axis=0)
   corners=np.array([[x,y,z] for x in [low[0],high[0]] for y in [low[1],high[1]] for z in [low[2],high[2]]],dtype=np.float32)
   bounds=corners@transform[:3,:3].T+transform[:3,3]
   items.append(dict(obj=obj,world=world,transform=transform,minimum=bounds.min(axis=0),maximum=bounds.max(axis=0)))
 visible=[i for i in items if i['obj']['visible']]
 minimum=np.min([i['minimum'] for i in visible],axis=0);maximum=np.max([i['maximum'] for i in visible],axis=0)
 camera_center=(minimum+maximum)*np.float32(.5);radius=np.linalg.norm(maximum-minimum)*np.float32(.5)
 fov=np.float32(math.radians(45));horizontal=2*math.atan(math.tan(fov/2)*width/height)
 distance=np.float32(np.clip(max(radius/math.tan(min(fov,horizontal)/2)*1.15,radius*2),.5,5000))
 yaw,pitch=np.float32(math.radians(45)),np.float32(math.radians(35))
 position=camera_center+distance*np.array([np.cos(pitch)*np.sin(yaw),np.sin(pitch),np.cos(pitch)*np.cos(yaw)],dtype=np.float32)
 forward=normalize(camera_center-position);right=normalize(np.cross(forward,[0,1,0]).astype(np.float32));up=normalize(np.cross(right,forward))
 view=np.eye(4,dtype=np.float32);view[0,:3]=right;view[1,:3]=up;view[2,:3]=-forward;view[:3,3]=[-right@position,-up@position,forward@position]
 focal=np.float32(1/math.tan(fov/2));near=max(float(distance-radius*4),.01);far=max(float(distance+radius*4),near+1)
 projection=np.array([[focal/(width/height),0,0,0],[0,focal,0,0],[0,0,far/(near-far),near*far/(near-far)],[0,0,-1,0]],dtype=np.float32);matrix=projection@view
 targets=[items[i] for i in selected];points=np.vstack([i['world'] for i in targets]);low,high=points.min(axis=0),points.max(axis=0);center=(low+high)*np.float32(.5);extents=high-low;axes=np.eye(3,dtype=np.float32)
 if basis and len(targets)==1:
  transform=targets[0]['transform'];left,singular,right_svd=np.linalg.svd(transform[:3,:3]);rotation=left@right_svd
  if np.linalg.det(rotation)<0:left[:,-1]*=-1;rotation=left@right_svd
  axes=rotation.T;origin=transform[:3,3];local=(points-origin)@rotation;low,high=local.min(axis=0),local.max(axis=0);extents=high-low;center=origin+rotation@((low+high)*.5)
 length=float(distance*2*math.tan(fov/2)*96/height)
 if mode=='move':
  axis='xyz'.index(handle);point=center+np.eye(3,dtype=np.float32)[axis]*length*.85;screen=project(matrix,point,width,height,top)
 elif mode=='rotate':
  axis='xyz'.index(handle);normal=np.eye(3,dtype=np.float32)[axis];u=normalize(np.cross(normal,[1,0,0] if abs(normal[0])<.9 else [0,1,0]));v=normalize(np.cross(normal,u));screen=None
  for angle in np.linspace(.22,2*math.pi-.22,40):
   point=center+length*.82*(u*math.cos(angle)+v*math.sin(angle));candidate=project(matrix,point,width,height,top)
   if candidate is not None and ring_hit(matrix,np.rint(candidate),center,length*.82,length/8,width,height,top)==axis:
    screen=candidate;break
  if screen is None:raise RuntimeError('No unambiguous projected rotation ring point')
 else:
  if handle.startswith('face_'):
   index=int(handle.split('_')[1]);axis=index//2;side=1 if index%2==0 else -1;point=center+axes[axis]*extents[axis]*side*.5
  else:point=center+sum((axes[i]*extents[i]*.5 for i in range(3)),start=np.zeros(3,dtype=np.float32))
  screen=project(matrix,point,width,height,top)
 if screen is None:raise RuntimeError('Handle is behind the camera')
 start=np.rint(screen).astype(int).tolist();end=[start[0]+40,start[1]+24]
 return dict(start=start,end=end,viewport=[0,top,width,height],camera=dict(center=camera_center.tolist(),distance=float(distance),yaw=45,pitch=35),gizmo_center=center.tolist(),extents=extents.tolist(),axis_basis=axes.tolist(),mode=mode,handle=handle,selection=selected,basis='object' if basis else 'grid',fixture=str(path))
if __name__=='__main__':
 parser=argparse.ArgumentParser();parser.add_argument('fixture');parser.add_argument('--mode',default='move');parser.add_argument('--handle',default='x');parser.add_argument('--selection',default='0');parser.add_argument('--basis',action='store_true');args=parser.parse_args()
 print(json.dumps(coordinates(args.fixture,args.mode,args.handle,[int(i) for i in args.selection.split(',')],args.basis),indent=2))
