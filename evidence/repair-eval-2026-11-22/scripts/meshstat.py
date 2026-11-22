import sys, numpy as np
def load(prefix):
    v=np.fromfile(prefix+'.v',dtype='<f8').reshape(-1,3); t=np.fromfile(prefix+'.t',dtype='<u4').reshape(-1,3).astype(np.int64); return v,t
def stats(v,t):
    a,b,c=v[t[:,0]],v[t[:,1]],v[t[:,2]]
    cr=np.cross(b-a,c-a); m2=(cr*cr).sum(1)
    e=np.concatenate([t[:,[0,1]],t[:,[1,2]],t[:,[2,0]]])
    fwd=e[:,0]<e[:,1]; key=np.sort(e,1); k=key[:,0]*(len(v)+1)+key[:,1]
    order=np.argsort(k,kind='stable'); ks=k[order]
    uniq,idx,cnt=np.unique(ks,return_index=True,return_counts=True)
    boundary=(cnt==1).sum(); nonman=(cnt>2).sum()
    two=idx[cnt==2]; f=fwd[order]
    flipped=(f[two]==f[two+1]).sum()
    # shells via union find on faces
    fi=np.concatenate([np.arange(len(t))]*3)[order]
    parent=np.arange(len(t))
    def find(x):
        r=x
        while parent[r]!=r: r=parent[r]
        while parent[x]!=r: parent[x],x=r,parent[x]
        return r
    starts=idx; 
    for s,cc in zip(idx,cnt):
        if cc>1:
            r0=find(fi[s])
            for j in range(1,cc):
                rj=find(fi[s+j])
                if rj!=r0: parent[rj]=r0
    roots=set(find(i) for i in range(len(t)))
    vol=(a*np.cross(b,c)).sum()/6
    # duplicates
    st=np.sort(t,1); _,dc=np.unique(st,axis=0,return_counts=True)
    unused=len(v)-len(np.unique(t))
    lens=np.linalg.norm(np.concatenate([b-a,c-b,a-c]),axis=1)
    return dict(verts=len(v),faces=len(t),boundary_edges=int(boundary),nonmanifold_edges=int(nonman),
        flipped_edge_pairs=int(flipped),shells=len(roots),volume_mm3=round(float(vol),3),
        zero_area=int((m2<=1e-24).sum()),slivers_1e6=int((m2<=1e-6).sum()),dup_faces=int((dc>1).sum()),unused_verts=int(unused),
        bbox=[np.round(v.min(0),3).tolist(),np.round(v.max(0),3).tolist()],min_edge=float(lens.min()))
if __name__=="__main__":
  for p in sys.argv[1:]:
    v,t=load(p); print(p.split('/')[-1], stats(v,t))
