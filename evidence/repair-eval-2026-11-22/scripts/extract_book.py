import zipfile,re,sys,numpy as np
z=zipfile.ZipFile(sys.argv[1]); out=sys.argv[2]
names=[n for n in z.namelist() if n.endswith('.model')]
for n in names:
  x=z.read(n).decode('utf8')
  for m in re.finditer(r'<object[^>]*\bid="(\d+)"[^>]*>(.*?)</object>',x,re.S):
    oid,body=m.group(1),m.group(2)
    v=re.findall(r'<vertex x="([^"]+)" y="([^"]+)" z="([^"]+)"',body)
    t=re.findall(r'<triangle v1="(\d+)" v2="(\d+)" v3="(\d+)"',body)
    print(n,oid,len(v),len(t))
    if oid=='2':
      np.array(v,np.float64).tofile(f'{out}/f1_book.v'); np.array(t,np.uint32).tofile(f'{out}/f1_book.t')
