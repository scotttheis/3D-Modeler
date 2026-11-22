# blender -b --factory-startup --python compare.py -- <fxdir> <outdir> name1 name2 ...
import sys, os, numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from mathutils.bvhtree import BVHTree
from meshstat import load, stats
a=sys.argv[sys.argv.index('--')+1:]; fx,od,names=a[0],a[1],a[2:]
def bvh(v,t): return BVHTree.FromPolygons([tuple(x) for x in v],[tuple(int(i) for i in f) for f in t])
def dev(src_v, tree):
    d=[tree.find_nearest(tuple(p))[3] for p in src_v[:: max(1,len(src_v)//20000)]]
    d=[x for x in d if x is not None]; return (max(d) if d else -1)
rows=[]
for n in names:
    ov,ot=load(f'{fx}/{n}'); otree=bvh(ov,ot)
    for tool in ['orig','ours','win','blender']:
        p=f'{fx}/{n}' if tool=='orig' else f'{od}/{n}.{tool}'
        if tool=='win': p=p+'.0'
        if not os.path.exists(p+'.v'): print(f'{n:26} {tool:8} (missing)'); continue
        v,t=load(p)
        if len(t)==0: print(f'{n:26} {tool:8} EMPTY'); continue
        s=stats(v,t)
        far = dev(v,otree) if tool!='orig' else 0.0
        back= dev(ov,bvh(v,t)) if tool!='orig' else 0.0
        print(f"{n:26} {tool:8} faces={s['faces']:>7} holes_edges={s['boundary_edges']:>4} nonman={s['nonmanifold_edges']:>3} "
              f"flips={s['flipped_edge_pairs']:>4} shells={s['shells']:>3} vol={s['volume_mm3']:>10} zero={s['zero_area']} "
              f"dup={s['dup_faces']} out->orig={far:.4f} orig->out={back:.4f}")
    print()
