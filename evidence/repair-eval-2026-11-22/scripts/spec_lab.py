# R1 lab helpers (repair-spec.md). Writes a fixture as 3MF, runs the Windows
# engine through windows_repair_probe (built outside E:), loads the result.
# Paths: set R1_WORK (scratch dir with fx/ and out/) and R1_PROBE (probe exe).
import numpy as np, zipfile, subprocess, os, json, re
import tempfile
W=os.environ.get('R1_WORK',os.path.join(tempfile.gettempdir(),'r1'))
os.makedirs(os.path.join(W,'fx'),exist_ok=True); os.makedirs(os.path.join(W,'out'),exist_ok=True)
EXE=os.environ.get('R1_PROBE',os.path.join(tempfile.gettempdir(),'rrt','release','rwr.exe'))
def save3mf(path,v,t):
    v=np.asarray(v,np.float64); t=np.asarray(t,np.int64)
    verts=''.join(f'<vertex x="{a:.9f}" y="{b:.9f}" z="{c:.9f}"/>' for a,b,c in v)
    tris=''.join(f'<triangle v1="{a}" v2="{b}" v3="{c}"/>' for a,b,c in t)
    model=('<?xml version="1.0" encoding="UTF-8"?><model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">'
      f'<resources><object id="1" type="model"><mesh><vertices>{verts}</vertices><triangles>{tris}</triangles></mesh></object></resources>'
      '<build><item objectid="1"/></build></model>')
    with zipfile.ZipFile(path,'w',zipfile.ZIP_DEFLATED) as z:
        z.writestr('[Content_Types].xml','<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>')
        z.writestr('_rels/.rels','<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>')
        z.writestr('3D/3dmodel.model',model)
def run(name,v,t,repair=True):
    fx=os.path.join(W,'fx',name+'.3mf'); out=os.path.join(W,'out',name)
    save3mf(fx,v,t)
    for f in os.listdir(os.path.join(W,'out')):
        if f.startswith(name+'.'): os.remove(os.path.join(W,'out',f))
    r=subprocess.run([EXE,fx,out]+([] if repair else ['norepair']),capture_output=True,text=True)
    if r.returncode: raise RuntimeError(r.stderr+r.stdout)
    ms=int(re.search(r'repair ms (\d+)',r.stdout).group(1))
    meshes=[]
    i=0
    while os.path.exists(f'{out}.{i}.v'):
        vv=np.fromfile(f'{out}.{i}.v','<f8').reshape(-1,3); tt=np.fromfile(f'{out}.{i}.t','<u4').reshape(-1,3).astype(np.int64)
        meshes.append((vv,tt)); i+=1
    return meshes,ms,r.stdout
def edges(t):
    e=np.concatenate([t[:,[0,1]],t[:,[1,2]],t[:,[2,0]]]); k=np.sort(e,1)
    u,c=np.unique(k,axis=0,return_counts=True); return u,c
def box(lo,hi):
    x0,y0,z0=lo;x1,y1,z1=hi
    v=[(x0,y0,z0),(x1,y0,z0),(x1,y1,z0),(x0,y1,z0),(x0,y0,z1),(x1,y0,z1),(x1,y1,z1),(x0,y1,z1)]
    t=[(0,2,1),(0,3,2),(4,5,6),(4,6,7),(0,1,5),(0,5,4),(1,2,6),(1,6,5),(2,3,7),(2,7,6),(3,0,4),(3,4,7)]
    return np.array(v,float),np.array(t)
def vol(v,t):
    a,b,c=v[t[:,0]],v[t[:,1]],v[t[:,2]]; return float((a*np.cross(b,c)).sum()/6)
def summary(v,t):
    u,c=edges(t)
    return dict(V=len(v),F=len(t),open=int((c==1).sum()),nonman=int((c>2).sum()),vol=round(vol(v,t),4),
                lo=np.round(v.min(0),6).tolist(),hi=np.round(v.max(0),6).tolist())
def shells(t,nv):
    p=np.arange(nv)
    def f(x):
        while p[x]!=x: p[x]=p[p[x]]; x=p[x]
        return x
    for a,b,c in t:
        ra,rb,rc=f(a),f(b),f(c); p[rb]=ra; p[f(c)]=f(a)
    roots={}
    for i,tri in enumerate(t): roots.setdefault(f(tri[0]),[]).append(i)
    return list(roots.values())
def shell_report(v,t):
    out=[]
    for fs in shells(t,len(v)):
        tt=t[fs]; u,c=edges(tt); out.append(dict(F=len(fs),vol=round(vol(v,tt),3),open=int((c==1).sum()),zmin=round(float(v[np.unique(tt)][:,2].min()),3),zmax=round(float(v[np.unique(tt)][:,2].max()),3)))
    return sorted(out,key=lambda d:-d['F'])
def cylinder(n=64,r=10,h=20,top=True,bottom=True):
    a=np.linspace(0,2*np.pi,n,endpoint=False)
    v=[(r*np.cos(x),r*np.sin(x),0) for x in a]+[(r*np.cos(x),r*np.sin(x),h) for x in a]+[(0,0,0),(0,0,h)]
    t=[]
    for i in range(n):
        j=(i+1)%n; t+= [(i,j,n+j),(i,n+j,n+i)]
        if bottom: t.append((2*n,j,i))
        if top: t.append((2*n+1,n+i,n+j))
    return np.array(v,float),np.array(t)
def patch_stats(v_in,t_in,vv,tt,region):
    # faces of output whose centroid is in region(mask fn) = patch
    c=vv[tt].mean(1); m=region(c); pt=tt[m]; pv=np.unique(pt)
    a,b,cc=vv[pt[:,0]],vv[pt[:,1]],vv[pt[:,2]]
    L=np.concatenate([np.linalg.norm(b-a,axis=1),np.linalg.norm(cc-b,axis=1),np.linalg.norm(a-cc,axis=1)])
    return pt,pv,L

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
