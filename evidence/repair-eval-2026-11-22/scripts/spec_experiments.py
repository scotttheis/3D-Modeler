# R1 experiments behind docs/tasks/repair-spec.md.
# Black-box only: every number comes from calling the public WinRT API
# Printing3DModel.RepairAsync through windows_repair_probe.rs on models made here.
#
# Setup (outside E:, see repair-plan.md "How the sessions run"):
#   copy windows_repair_probe.Cargo.toml -> <tmp>\r1\rwr\Cargo.toml
#   copy windows_repair_probe.rs        -> <tmp>\r1\rwr\src\main.rs
#   CARGO_TARGET_DIR=<tmp>\rrt cargo build --release   (gives <tmp>\rrt\release\rwr.exe)
# Run with a Python that has numpy (Blender's bundled python works):
#   python spec_experiments.py [section ...]     sections: snap recut weld holes orient tiny time
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from spec_lab import *
from spec_holefill_dp import solve

def cat(*parts):
    V, T, o = [], [], 0
    for v, t in parts:
        V.append(v); T.append(t + o); o += len(v)
    return np.vstack(V), np.vstack(T)

def inv(p):
    v, t = p
    return v, t[:, [0, 2, 1]]

def go(name, v, t):
    m, ms, _ = run(re.sub(r'\W', '_', name)[:20], v, t)
    vv, tt = m[0]; s = summary(vv, tt)
    print(f'{name:52} F {len(t)}->{len(tt)} open={s["open"]} nonman={s["nonman"]} '
          f'vol in {vol(v, t):10.2f} out {s["vol"]:10.2f} ms={ms} shells:',
          [(d['F'], d['vol']) for d in shell_report(vv, tt)], flush=True)
    return vv, tt

S = lambda r, off=(0, 0, 0): sphere(nu=32, nv=16, r=r, off=off)
# A 1 mm cube far away enlarges the bounding-box diagonal so the re-cut (edges over
# 0.303 D) leaves hole patches alone. Keep L under 230 mm so the snap step stays 0.002.
FAR = box((100, 100, 100), (101, 101, 101))
FAR_BIG = box((4000, 4000, 4000), (4001, 4001, 4001))  # timing only (coarser snap there)

def with_far(v, t, far=FAR):
    return np.vstack([v, far[0]]), np.vstack([t, far[1] + len(v)])

def min_spacing(vv, k=0):
    u = np.unique(np.round(vv[:, k], 7)); d = np.diff(u); return d[d > 1e-6].min()

def snap():
    print('== snap: lattice origin and step')
    rng = np.random.default_rng(1)
    for r, off in [(15, (0.00037, 0.00113, -0.00071)), (15, (7.3, 2.1, -4.4)), (40, (0.3, 0.7, 0.1))]:
        v, t = sphere(r=r, off=off); v = v + rng.uniform(-0.01, 0.01, v.shape)
        vv, tt = run('snap', v, t)[0][0]
        d = np.linalg.norm(v[:, None, :] - vv[None, :, :], axis=2); out = vv[d.argmin(1)]
        lo = v.min(0); u = (v - lo) / 0.002; ko = np.round((out - lo) / 0.002)
        lat = np.abs((vv - lo) / 0.002 - np.round((vv - lo) / 0.002)).max()
        k, c = np.unique(ko - np.floor(u), return_counts=True)
        print(f' r={r} off={off}: off-lattice (origin=input bbox min, step 0.002) {lat:.4f} cells;'
              f' out cell - floor cell: {dict(zip(k.tolist(), c.tolist()))};'
              f' max move {np.linalg.norm(out - v, axis=1).max():.4f} mm')
    print('== snap: step vs largest bbox extent L (smallest coordinate spacing in the result)')
    rng = np.random.default_rng(9)
    for dims in [(1, 1, 1), (2, 2, 2), (3, 3, 3), (4, 4, 4), (1, 2, 3), (0.5, 4, 4), (4, 1, 1), (5, 1, 1), (10, 10, 10), (1, 1, 100)]:
        v, t = sphere(nu=128, nv=64, r=1.0); v = v * np.array(dims) / 2 + rng.uniform(-1, 1, v.shape) * np.array(dims) * 1e-4
        vv, tt = run('step', v, t)[0][0]; L = (v.max(0) - v.min(0)).max()
        print(f' dims {dims}: spacing x,y,z {[round(float(min_spacing(vv, k)), 6) for k in range(3)]}; L/2500={L / 2500:.6f}')
    for Lsz in [50, 200, 230, 250, 330, 360, 400, 1000, 2000, 4000, 8000]:
        v, t = sphere(nu=128, nv=64, r=Lsz / 2); v = v + rng.uniform(-1, 1, v.shape) * Lsz * 1e-4
        vv, tt = run('bigstep', v, t)[0][0]
        print(f' sphere L={Lsz}: spacing {min_spacing(vv):.6f}', flush=True)
    print('== snap: single-vertex sweep (tetrahedron apex x)')
    T = np.array([(0, 2, 1), (0, 1, 3), (1, 2, 3), (2, 0, 3)]); prev = None
    for x in np.round(np.arange(4.9940, 5.0110, 0.0001), 6):
        v = np.array([(0, 0, 0), (20, 0, 0), (0, 20, 0), (x, 5.0, 10.0)], float)
        vv, _ = run('tet', v, T)[0][0]; o = vv[np.linalg.norm(vv - v[3], axis=1).argmin()][0]
        if prev is None or abs(o - prev) > 1e-7: print(f'  x {x:.4f} -> {o:.6f}')
        prev = o

def recut():
    print('== recut: small cube beside a fine sphere (threshold vs bbox diagonal D)')
    for R, ees in [(15, np.arange(17, 20.01, 0.25)), (30, np.arange(28, 40, 1.0))]:
        sv, st = sphere(r=R)
        for e in ees:
            bv, bt = box((R + 25, 0, 0), (R + 25 + e, e, e)); v, t = cat((sv, st), (bv, bt))
            vv, tt = run('recut', v, t)[0][0]; D = np.linalg.norm(v.max(0) - v.min(0))
            cube = (vv[tt].mean(1)[:, 0] > R + 24).sum()
            print(f'  R={R} e={e:6.2f} cube faces {cube} longest edge/D {e * np.sqrt(2) / D:.4f}')
    for name, (v, t) in [('box 10', box((0, 0, 0), (10, 10, 10))), ('box 10x1x1', box((0, 0, 0), (10, 1, 1))),
                         ('box 1x10x10', box((0, 0, 0), (1, 10, 10))), ('sphere 8x4', sphere(nu=8, nv=4, r=10)),
                         ('sphere 6x3', sphere(nu=6, nv=3, r=10))]:
        vv, tt = run('recut2', v, t)[0][0]; print(f'  {name}: F {len(t)} -> {len(tt)}')

def weld():
    print('== weld: one vertex duplicated at distance d (half its fan uses the copy)')
    for R in [1.5, 15, 150]:
        v, t = sphere(r=R); k = 500; fan = np.where((t == k).any(1))[0][:3]
        n = v[k] / np.linalg.norm(v[k]); tg = np.cross(n, [0, 0, 1]); tg /= np.linalg.norm(tg); res = []
        for d in [0.001, 0.002, 0.004, 0.006, 0.008, 0.01, 0.012, 0.02, 0.05]:
            v2 = np.vstack([v, [v[k] + d * tg]]); t2 = t.copy()
            for f in fan: t2[f][t2[f] == k] = len(v)
            vv, tt = run('weld', v2, t2)[0][0]; res.append((d, 'merged' if len(vv) == len(v) else 'kept'))
        print(f'  R={R}: {res}')
    print('== weld: unwelded soup sphere r=15, corners jittered +-j')
    rng = np.random.default_rng(5); v, t = sphere(r=15)
    for j in [0.0005, 0.002, 0.004, 0.01, 0.02]:
        sv = v[t].reshape(-1, 3) + rng.uniform(-j, j, (len(t) * 3, 3))
        vv, tt = run('soup', sv, np.arange(len(sv)).reshape(-1, 3))[0][0]
        print(f'  j={j}: {summary(vv, tt)}')
    print('== stitch: sphere cap detached (own vertices) and lifted by g')
    v, t = sphere(r=15); capv = np.arange(0, 1 + 4 * 64); capf = np.all(t <= capv.max(), axis=1)
    for g in [0.005, 0.02, 1.0]:
        v2 = np.vstack([v, v[capv] + [0, 0, g]]); t2 = t.copy(); t2[capf] += len(v)
        vv, tt = run('cap', v2, t2)[0][0]; print(f'  g={g}: shells {[(d["F"], d["vol"], d["zmax"]) for d in shell_report(vv, tt)]}')
    print('== T-junction: midpoint of box edge 4-5 used only by the top face, dropped by dz')
    for dz in [0, 0.005, 0.02, 0.1]:
        v, t = box((0, 0, 0), (10, 10, 10)); v = np.vstack([v, [(5, 0, 10 - dz)]]); t = [list(x) for x in t]
        t.remove([4, 5, 6]); t += [[4, 8, 6], [8, 5, 6]]; vv, tt = run('tj', v, np.array(t))[0][0]
        print(f'  dz={dz}: {summary(vv, tt)}')
    print('== slivers (cap) and needles')
    for h in [0.0001, 0.001, 0.002, 0.01, 0.2]:
        v, t = box((0, 0, 0), (10, 10, 10)); v = np.vstack([v, [(5, h, 10)]]); t = [list(x) for x in t]
        t.remove([4, 5, 6]); t += [[4, 8, 6], [8, 5, 6], [4, 5, 8]]; vv, tt = run('cap', v, np.array(t))[0][0]
        print(f'  cap sliver height {h}: F={len(tt)} V={len(vv)} open={summary(vv, tt)["open"]} nearest out vertex to apex {np.round(vv[np.linalg.norm(vv - v[8], axis=1).argmin()], 4).tolist()}')
    for e in [0.0001, 0.002, 0.004, 0.01, 0.2]:
        v, t = box((0, 0, 0), (10, 10, 10)); q = np.array([10, 10, 10]) - e / np.sqrt(2) * np.array([1, 1, 0])
        v = np.vstack([v, [q]]); t = [list(x) for x in t]; t.remove([4, 5, 6]); t.remove([4, 6, 7])
        t += [[4, 5, 8], [5, 6, 8], [6, 7, 8], [7, 4, 8]]; vv, tt = run('needle', v, np.array(t))[0][0]
        print(f'  needle short edge {e}: F={len(tt)} V={len(vv)} (52 = merged into corner)')

def holes():
    print('== holes: open cylinder, re-cut suppressed')
    for n in [3, 4, 10, 64, 500]:
        v, t = cylinder(n=n, top=False); v = v[:-1]; v2, t2 = with_far(v, t)
        m, ms, _ = run(f'hole{n}', v2, t2); vv, tt = m[0]; tt = tt[vv[tt].mean(1)[:, 0] < 90]
        c = vv[tt].mean(1); pt = tt[c[:, 2] > 19.99]; pv = np.unique(pt)
        interior = (np.abs(np.linalg.norm(vv[pv][:, :2], axis=1) - 10) > 0.05).sum()
        print(f'  n={n}: patch faces {len(pt)} (n-2={n - 2}) interior points {interior} z spread {np.ptp(vv[pv][:, 2]):.4f} ms={ms}')
    print('== holes: random non-planar holes (cup), compare with min-area and Liepa DP')
    rng = np.random.default_rng(11); agree = {'area': 0, 'liepa': 0}
    for trial in range(12):
        n = int(rng.integers(6, 13)); a = np.sort(rng.uniform(0, 2 * np.pi, n)); r = rng.uniform(6, 10, n); z = rng.uniform(8, 12, n)
        P = np.stack([r * np.cos(a), r * np.sin(a), z], 1)
        v = np.vstack([P, [(0, 0, -5)]]); t = np.array([(n, (i + 1) % n, i) for i in range(n)])
        v2, t2 = with_far(v, t); vv, tt = run('cup', v2, t2)[0][0]
        idx = [int(np.linalg.norm(vv - p, axis=1).argmin()) for p in P]; back = {o: i for i, o in enumerate(idx)}
        patch = {tuple(sorted(back[x] for x in f)) for f in tt if all(x in back for x in f)}
        for md in agree: agree[md] += solve(P, md) == patch
    print(f'  identical triangulation out of 12: {agree}')
    print('== holes: curved window cut in a tube wall')
    for w in [4, 16, 32]:
        n, rows = 64, 8; a = np.linspace(0, 2 * np.pi, n, endpoint=False)
        v = [(10 * np.cos(x), 10 * np.sin(x), 20 * k / rows) for k in range(rows + 1) for x in a] + [(0, 0, 0), (0, 0, 20)]
        t = []
        for k in range(rows):
            for i in range(n):
                j = (i + 1) % n; A, B, C, D = k * n + i, k * n + j, (k + 1) * n + j, (k + 1) * n + i; t += [(A, B, C), (A, C, D)]
        for i in range(n):
            j = (i + 1) % n; t += [(len(v) - 2, j, i), (len(v) - 1, rows * n + i, rows * n + j)]
        v, t = np.array(v, float), np.array(t); c = v[t].mean(1); ang = np.arctan2(c[:, 1], c[:, 0])
        win = (np.abs(ang) < w * np.pi / 64) & (c[:, 2] > 5) & (c[:, 2] < 15) & (np.linalg.norm(c[:, :2], axis=1) > 9)
        v2, t2 = with_far(v, t[~win]); vv, tt = run('tube', v2, t2)[0][0]; tt = tt[vv[tt].mean(1)[:, 0] < 90]
        cc = vv[tt].mean(1); a2 = np.arctan2(cc[:, 1], cc[:, 0])
        pm = (np.abs(a2) < w * np.pi / 64) & (cc[:, 2] > 5) & (cc[:, 2] < 15)
        rad = np.linalg.norm(cc[pm][:, :2], axis=1)
        print(f'  window {w} cols: patch faces {pm.sum()}, patch centroid radius {rad.min():.3f}..{rad.max():.3f} (surface 10)')

def orient():
    print('== orientation, nesting, overlaps, fins')
    go('two disjoint spheres, second inside-out', *cat(S(15), inv(S(8, (40, 0, 0)))))
    go('hollow ball: outer out + inner inward (cavity)', *cat(S(15), inv(S(8))))
    go('nested both outward', *cat(S(15), S(8)))
    go('nested both inside-out', *cat(inv(S(15)), inv(S(8))))
    go('ball inside a cavity (15 out, 10 in, 5 out)', *cat(S(15), inv(S(10)), S(5)))
    go('two overlapping spheres', *cat(S(15), S(15, (15, 0, 0))))
    go('overlapping spheres, one inside-out', *cat(S(15), inv(S(15, (15, 0, 0)))))
    a, ta = box((0, 0, 0), (20, 20, 20)); b, tb = box((10, 10, 10), (30, 30, 30))
    go('f6 overlapping boxes', *cat((a, ta), (b, tb)))
    v, t = box((0, 0, 0), (10, 10, 10))
    go('fin triangle on a box edge', np.vstack([v, [(5, -5, 15)]]), np.vstack([t, [(4, 5, 8)]]))
    go('fin pointing inside the box', np.vstack([v, [(5, 5, 5)]]), np.vstack([t, [(4, 5, 8)]]))
    go('box with loose internal sheet', np.vstack([v, [(5, 0, 0), (5, 10, 0), (5, 10, 10), (5, 0, 10)]]), np.vstack([t, [(8, 9, 10), (8, 10, 11)]]))
    go('loose flat sheet beside box', np.vstack([v, [(20, 0, 0), (30, 0, 0), (30, 10, 0), (20, 10, 0)]]), np.vstack([t, [(8, 9, 10), (8, 10, 11)]]))
    hv, ht = sphere(r=5, off=(30, 0, 0)); keep = np.all(hv[ht][:, :, 2] >= -1e-9, axis=1)
    go('open hemisphere beside box', *cat((v, t), (hv, ht[keep])))
    a, ta = box((0, 0, 0), (10, 10, 10)); b, tb = box((10, 10, 0), (20, 20, 10)); vv = np.vstack([a, b]); tt = np.vstack([ta, tb + 8]); tt[tt == 8] = 2; tt[tt == 12] = 6
    go('two cubes sharing one edge (welded)', vv, tt)
    go('two cubes face to face', *cat(box((0, 0, 0), (10, 10, 10)), box((10, 0, 0), (20, 10, 10))))
    go('two cubes face to face, gap 0.005', *cat(box((0, 0, 0), (10, 10, 10)), box((10.005, 0, 0), (20, 10, 10))))
    v, t = box((0, 0, 0), (10, 10, 10)); v[6] = (10, 10, -5)
    go('one shell folded through itself', v, t)

def tiny():
    print('== tiny shells: smallest kept cube beside a big cube of side B (gap B)')
    for Bs in [0.5, 1, 2, 3, 10, 100, 300, 1000, 3000]:
        lo = None
        for s in np.geomspace(0.01, Bs * 0.3, 80):
            v, t = cat(box((0, 0, 0), (Bs, Bs, Bs)), box((2 * Bs, 0, 0), (2 * Bs + s, s, s)))
            vv, tt = run('tiny', v, t)[0][0]
            if len(shell_report(vv, tt)) > 1: break
            lo = s
        D = np.linalg.norm(v.max(0) - v.min(0))
        print(f'  B={Bs}: removed at {lo}, kept at {s:.4g}; scene diagonal {D:.4g}')
    v, t = cat(box((0, 0, 0), (10, 10, 10)), box((20, 0, 0), (20.01, 1, 1)))
    vv, tt = run('plate', v, t)[0][0]; print(f'  1 x 1 x 0.01 plate beside cube 10: shells {len(shell_report(vv, tt))}')

def time_():
    print('== time vs faces')
    rng = np.random.default_rng(2)
    for nu, nv in [(64, 32), (128, 64), (256, 128), (512, 256)]:
        v, t = sphere(nu=nu, nv=nv, r=30); row = []
        for kind in ['clean', 'holes', 'soup']:
            if kind == 'clean': vv, tt = v, t
            elif kind == 'holes': vv, tt = v, t[rng.random(len(t)) > 0.01]
            else:
                vv = v[t].reshape(-1, 3) + rng.uniform(-3e-4, 3e-4, (len(t) * 3, 3)); tt = np.arange(len(vv)).reshape(-1, 3)
            row.append(f'{kind} {run("time", vv, tt)[1]} ms')
        print(f'  F={len(t)}: ' + ', '.join(row), flush=True)
    for n in [500, 1000]:
        v, t = cylinder(n=n, r=50, top=False); v = v[:-1]; v2, t2 = with_far(v, t, FAR_BIG)
        print(f'  one flat hole of {n} edges: {run("hbig", v2, t2)[1]} ms', flush=True)

SECTIONS = dict(snap=snap, recut=recut, weld=weld, holes=holes, orient=orient, tiny=tiny, time=time_)
if __name__ == '__main__':
    for s in (sys.argv[1:] or list(SECTIONS)):
        SECTIONS[s]()
