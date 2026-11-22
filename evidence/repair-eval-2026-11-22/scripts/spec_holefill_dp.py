import numpy as np
from functools import lru_cache
def tri_area(P,i,j,k): return 0.5*np.linalg.norm(np.cross(P[j]-P[i],P[k]-P[i]))
def nrm(P,i,j,k):
    n=np.cross(P[j]-P[i],P[k]-P[i]); l=np.linalg.norm(n); return n/l if l>0 else n
def ang(n1,n2): return np.degrees(np.arccos(np.clip(np.dot(n1,n2),-1,1)))
def solve(P, mode, outer_normals=None):
    n=len(P); INF=(1e18,1e18)
    W={}; O={}
    for i in range(n-1): W[(i,i+1)]=(0.0,0.0)
    for i in range(n-2): 
        W[(i,i+2)]=None
    def w(i,j):
        return W[(i,j)]
    for L in range(2,n):
        for i in range(0,n-L):
            j=i+L; best=None; bm=None
            for m in range(i+1,j):
                a=W[(i,m)]; b=W[(m,j)]
                A=tri_area(P,i,m,j)
                if mode=='area': cand=(0.0,a[1]+b[1]+A)
                else:
                    nn=nrm(P,i,m,j); d=0.0
                    for (x,y) in ((i,m),(m,j)):
                        if y-x>1: d=max(d,ang(nn,nrm(P,x,O[(x,y)],y)))
                    if (i,j)==(0,n-1): pass
                    cand=(max(a[0],b[0],d),a[1]+b[1]+A)
                if best is None or cand<best: best=cand; bm=m
            W[(i,j)]=best; O[(i,j)]=bm
    tris=[]
    def rec(i,j):
        if j-i<2: return
        m=O[(i,j)]; tris.append(tuple(sorted((i,m,j)))); rec(i,m); rec(m,j)
    rec(0,n-1); return set(tris)
