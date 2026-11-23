import numpy as np,sys
out=sys.argv[1]; n=500; r=10; h=20
a=np.linspace(0,2*np.pi,n,endpoint=False)
v=[(r*np.cos(x),r*np.sin(x),0) for x in a]+[(r*np.cos(x),r*np.sin(x),h) for x in a]+[(0,0,0)]
t=[]
for i in range(n):
  j=(i+1)%n; t+=[(i,j,n+j),(i,n+j,n+i),(2*n,j,i)]
np.array(v,np.float64).tofile(f'{out}/f11_open500.v'); np.array(t,np.uint32).tofile(f'{out}/f11_open500.t')
