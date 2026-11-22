# Run: blender -b --factory-startup --python blender_repair.py -- in1=out1 in2=out2 ...
import bpy, bmesh, sys, numpy as np, time
args=sys.argv[sys.argv.index('--')+1:]
for pair in args:
    inp,outp=pair.split('=')
    v=np.fromfile(inp+'.v','<f8').reshape(-1,3); t=np.fromfile(inp+'.t','<u4').reshape(-1,3)
    bm=bmesh.new()
    bv=[bm.verts.new(p) for p in v]; bm.verts.ensure_lookup_table()
    skipped=0
    for f in t:
        try: bm.faces.new([bv[i] for i in f])
        except ValueError: skipped+=1   # duplicate / degenerate index faces cannot exist in bmesh
    t0=time.time()
    # Same steps as Blender's 3D-Print Toolbox "Make Manifold"
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=0.001)
    bmesh.ops.dissolve_degenerate(bm, edges=bm.edges, dist=0.001)
    loose=[x for x in bm.verts if not x.link_faces]; bmesh.ops.delete(bm, geom=loose, context='VERTS')
    loose=[e for e in bm.edges if not e.link_faces]; bmesh.ops.delete(bm, geom=loose, context='EDGES')
    nm=[e for e in bm.edges if len(e.link_faces)>2]
    bmesh.ops.holes_fill(bm, edges=[e for e in bm.edges if e.is_boundary], sides=0)
    bmesh.ops.triangulate(bm, faces=bm.faces)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.verts.index_update()
    ov=np.array([x.co[:] for x in bm.verts],np.float64); of=np.array([[x.index for x in f.verts] for f in bm.faces],np.uint32)
    ov.tofile(outp+'.v'); of.tofile(outp+'.t')
    print(f'BLENDER {inp.split("/")[-1]} skipped_on_import={skipped} nonmanifold_edges_left_alone={len(nm)} {int((time.time()-t0)*1000)} ms')
    bm.free()
