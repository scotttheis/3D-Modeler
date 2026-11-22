import numpy as np, sys, os, zipfile
out=sys.argv[1]; rng=np.random.default_rng(7)
def save(name,v,t):
    v=np.asarray(v,np.float64); t=np.asarray(t,np.uint32)
    v.tofile(f'{out}/{name}.v'); t.tofile(f'{out}/{name}.t')
    verts=''.join(f'<vertex x="{a:.6f}" y="{b:.6f}" z="{c:.6f}"/>' for a,b,c in v)
    tris=''.join(f'<triangle v1="{a}" v2="{b}" v3="{c}"/>' for a,b,c in t)
    model=('<?xml version="1.0" encoding="UTF-8"?><model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">'
      f'<resources><object id="1" type="model"><mesh><vertices>{verts}</vertices><triangles>{tris}</triangles></mesh></object></resources>'
      '<build><item objectid="1"/></build></model>')
    with zipfile.ZipFile(f'{out}/{name}.3mf','w',zipfile.ZIP_DEFLATED) as z:
        z.writestr('[Content_Types].xml','<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>')
        z.writestr('_rels/.rels','<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>')
        z.writestr('3D/3dmodel.model',model)
def cylinder(n=64,r=10,h=20,top=True,bottom=True,off=(0,0,0)):
    a=np.linspace(0,2*np.pi,n,endpoint=False)
    v=[(r*np.cos(x),r*np.sin(x),0) for x in a]+[(r*np.cos(x),r*np.sin(x),h) for x in a]+[(0,0,0),(0,0,h)]
    t=[]
    for i in range(n):
        j=(i+1)%n; t+= [(i,j,n+j),(i,n+j,n+i)]
        if bottom: t.append((2*n,j,i))
        if top: t.append((2*n+1,n+i,n+j))
    return np.array(v)+off,np.array(t)
def sphere(nu=64,nv=32,r=15,off=(0,0,0)):
    v=[(0,0,r)]
    for i in range(1,nv):
        th=np.pi*i/nv
        for j in range(nu):
            ph=2*np.pi*j/nu; v.append((r*np.sin(th)*np.cos(ph),r*np.sin(th)*np.sin(ph),r*np.cos(th)))
    v.append((0,0,-r)); S=len(v)-1; t=[]
    idx=lambda i,j:1+(i-1)*nu+(j%nu)
    for j in range(nu): t.append((0,idx(1,j),idx(1,j+1)))
    for i in range(1,nv-1):
        for j in range(nu):
            a,b,c,d=idx(i,j),idx(i,j+1),idx(i+1,j),idx(i+1,j+1); t+=[(a,c,d),(a,d,b)]
    for j in range(nu): t.append((S,idx(nv-1,j+1),idx(nv-1,j)))
    return np.array(v)+off,np.array(t)
def box(lo,hi,base=0):
    x0,y0,z0=lo;x1,y1,z1=hi
    v=[(x0,y0,z0),(x1,y0,z0),(x1,y1,z0),(x0,y1,z0),(x0,y0,z1),(x1,y0,z1),(x1,y1,z1),(x0,y1,z1)]
    t=[(0,2,1),(0,3,2),(4,5,6),(4,6,7),(0,1,5),(0,5,4),(1,2,6),(1,6,5),(2,3,7),(2,7,6),(3,0,4),(3,4,7)]
    return np.array(v,float),np.array(t)+base
# F2 open cylinder (64-edge hole)
v,t=cylinder(top=False); save('f2_open_cylinder',v,t)
# F3 sphere with random holes
v,t=sphere(); keep=rng.random(len(t))>0.03; save('f3_holey_sphere',v,t[keep])
# F4 sphere with 20% flipped faces
v,t=sphere(); t=t.copy(); fl=rng.random(len(t))<0.2; t[fl]=t[fl][:,[0,2,1]]; save('f4_flipped_sphere',v,t)
# F5 STL soup cylinder: unwelded, jitter 0.0003, plus one seam column shifted 0.01mm
v,t=cylinder(); sv=v[t].reshape(-1,3).copy(); sv+=rng.uniform(-3e-4,3e-4,sv.shape)
seam=np.isclose(np.arctan2(sv[:,1],sv[:,0]),0,atol=1e-3)&(np.abs(sv[:,2]-0)>-1)
st=np.arange(len(sv)).reshape(-1,3)
# shift seam vertices only in faces on one side (x>0,y>0 neighbours) to make a 0.01 crack
side=(sv[st].mean(1)[:,1]>0); m=np.zeros(len(sv),bool); m[st[side].ravel()]=True
sv[m&seam,1]+=0.01
save('f5_soup_cracked_cylinder',sv,st)
# F6 two overlapping boxes as one mesh
a,ta=box((0,0,0),(20,20,20)); b,tb=box((10,10,10),(30,30,30),8); save('f6_overlapping_boxes',np.vstack([a,b]),np.vstack([ta,tb]))
# F7 two cubes sharing one edge (non-manifold edge) + a dangling fin face
a,ta=box((0,0,0),(10,10,10)); b,tb=box((10,10,0),(20,20,10),8)
v=np.vstack([a,b]); t=np.vstack([ta,tb])
# weld shared edge: b vertex (10,10,0)=8, (10,10,10)=12 -> a vertices 2 and 6
t[t==8]=2; t[t==12]=6
v=np.vstack([v,[(5,5,15)]]); t=np.vstack([t,[(4,5,16)]])
save('f7_nonmanifold',v,t)
# F8 cube with duplicate faces + needle triangles via T-junction on an edge
v,t=box((0,0,0),(10,10,10)); t=np.vstack([t,t[:2]]); save('f8_duplicates',v,t)
# F9 inside-out sphere
v,t=sphere(); save('f9_inside_out',v,t[:,[0,2,1]])
# F10 sealed cube with sliver triangles (like the book): cube top split with a near-collinear vertex
v,t=box((0,0,0),(10,10,10)); v=np.vstack([v,[(5,0.00005,10)]]); # point almost on edge 4-5
t=[list(x) for x in t]; t.remove([4,5,6]); t+= [[4,8,6],[8,5,6]]
t+= [[4,5,8]]  # sliver closing the gap along edge 4-5 (height 0.00005 mm)
save('f10_sealed_slivers',v,np.array(t))
print(sorted(os.listdir(out)))
