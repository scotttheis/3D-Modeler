# Task card R4: native engine A (snap, weld, T-junctions, thin triangles, non-manifold)

Read first: `docs/tasks/repair-plan.md` (phase R4), `docs/tasks/repair-spec.md`
sections 1-3 and 7 (only the fin / non-manifold sentences),
`docs/tasks/repair-r2-card.md` (what `RepairScore`, `gate`, refusals are).
The spec's "We implement" lines are the rules. Where this card is more exact,
the card wins.

Edit only `crates/modeler3d-core/src/repair.rs` (you may move the new code into
a new private module `crates/modeler3d-core/src/repair_native.rs`, declared in
`lib.rs` as a private `mod`), `crates/modeler3d-core/tests/mesh_ops_repair.rs`,
`crates/modeler3d-core/tests/repair_windows.rs`, and a new test file
`crates/modeler3d-core/tests/repair_native.rs`. Do not change the public
signatures of `repair_mesh*`, `RepairReport` fields, `inspect_mesh`, or
anything in the app or MCP crates. Do not commit. Do not run the Full gate.
Before cargo, run `scripts/Initialize-BuildClang.ps1` in the same PowerShell
session. No `unwrap`/`expect`/panics in library code. No new dependencies.
Finish with `cargo fmt`,
`cargo clippy -p modeler3d-core -p modeler3d-app -p modeler3d-mcp --all-targets -- -D warnings`,
`cargo test -p modeler3d-core -p modeler3d-app -p modeler3d-mcp`.

Scope: the native pipeline only (`repair_mesh_with_options`, profiles Enhanced
and Extended). The Windows route, the Simple profile, `close_small_holes`
(3-4 edge holes only; bigger holes are R5), `regularize_planar_regions` (R5
removes it), Wrap and tiny shells (R6) stay as they are.

## 1. The grid (spec section 1)

`struct RepairGrid { origin: [f64; 3], step: f64 }`, computed **once** from the
input mesh of `repair_mesh_with_options` and passed to every stage below (the
grid must not move when later stages change the bounding box).

- `origin` = bounding-box minimum of the input (f64), `L` = largest box side,
  `step = min(0.002, L / 2500)`. If `L` is 0 or not finite, the lattice stages
  do nothing.
- Lattice coordinates: `k = round((x - origin) / step)` as `i64`, per axis.
  Round to **nearest** (not floor). Output position = `(origin + k * step) as f32`.
- Every lattice stage converts the current mesh to lattice coordinates first
  (points already on the lattice map to themselves; off-lattice points such as
  hole-fill centres snap), does all geometry tests in **exact integer
  arithmetic** (`i64` differences, `i128` cross/dot products), and converts back
  at the end. No float epsilon anywhere in these stages.

## 2. Stages

### a. Snap (merge same cell)

All points (open or closed parts) that land on the same lattice point become
one vertex (lowest original index wins). Then run **face clean-up**:

- drop faces with a repeated index (this is the edge collapse);
- faces with the same three indices: same winding, keep one; opposite winding,
  pairs cancel (remove both; a pocket collapsed to zero volume). With n of one
  winding and m of the other, keep `|n - m|` copies (at most one) of the
  majority winding.

### b. Collapse thin triangles (spec section 3)

A face whose three distinct lattice points are collinear (exact `i128` cross
product = 0): let `m` be the corner lying strictly between the other two
(`a`, `b`). Find the faces using the long edge `a-b`.

- Exactly one other face uses `a-b` (it contains the directed edge opposite to
  the degenerate face's, i.e. a manifold neighbour `(.., a, b, ..)` or
  `(.., b, a, ..)` with third corner `x`): replace that neighbour by the two
  triangles obtained by inserting `m` into its `a-b` edge, keeping its winding
  (for neighbour `(a, b, x)`: `(a, m, x)` and `(m, b, x)`). Remove the
  degenerate face.
- Otherwise (open long edge, or a non-manifold one): remove the degenerate
  face only (it has zero area; it cannot open a hole that has area).

Repeat until no collinear face is left (a split can create a new one when `x`
is also on the line), with a hard cap of `2 * face_count` iterations; then run
face clean-up. Never remove a face with non-zero lattice area in this stage.
No angle, edge-length or area threshold.

### c. Weld open-edge points (spec section 2, rule 1)

Open edge = undirected edge used by exactly one face. Open-edge points are the
endpoints of open edges. Weld radius `R = 5 * step`, compared exactly as
squared lattice distance `<= 25`.

- Spatial hash on lattice coordinates with cell size `5` (check the 27
  neighbouring cells). Visit open-edge points in index order; each point maps
  to the **nearest earlier representative** (a point that was not itself mapped)
  within `R`, ties to the lower index; otherwise it becomes a representative.
  No chaining: a point never moves more than `R`.
- Points of closed parts (no open edge) are never touched.
- Then face clean-up, then stage b once more (welding can make collinear faces).

### d. T-junctions (spec section 2, rule 2)

For each open edge `a-b` and each open-edge point `p` that is not `a` or `b`,
whose squared distance to the segment is `<= 25` lattice units and whose
projection parameter is strictly inside `(0, 1)` with `p` farther than `R` from
both `a` and `b`: split the face owning `a-b` at `p` (same rule as stage b,
keeping winding). Use the spatial hash to find candidates; process edges in a
deterministic order; repeat until nothing splits (cap `2 * face_count`). Then
weld (stage c) again and face clean-up.

Never zip loops together across a gap wider than `R` (spec rule 3).

### e. Non-manifold edges and fins (fix plan Step 3 item 4; spec section 7)

1. **Pair the faces at each edge used by more than two faces.** Let the edge be
   `a-b`, `e = b - a`. For each face on it take its third corner `p` and the
   angle `theta` of `p - a` around the axis `e` (any fixed reference direction
   perpendicular to `e`; f64 is fine here). Sort by `theta`. A face that
   traverses `b -> a` has its solid side toward increasing `theta`; a face that
   traverses `a -> b` toward decreasing `theta`. Pair each `b -> a` face with
   the **first unpaired `a -> b` face after it** in increasing `theta`
   (cyclic). Paired faces stay joined along the edge; every other face on the
   edge is cut loose there.
2. **Split vertices.** For each vertex that is an endpoint of such an edge,
   group its faces into fans joined across its edges (ordinary two-face edges,
   plus the pairs chosen in step 1). The first fan keeps the vertex; every
   other fan gets a new copy of it (same position).
3. **Remove fins.** If at least one component is closed (no open edges), remove
   every open component whose points all lie in one plane (exact lattice
   test: every point coplanar with the first non-collinear triple; a
   component with no such triple is a line and is also removed). Curved open
   sheets stay (R5/R6 deal with them). If no component is closed, remove
   nothing.

### f. Pipeline order

Replace the Enhanced/Extended stage list with (stage names are reported to the
user and MCP; keep these exact strings):

1. `"StitchGaps (insert vertices)"`: a snap, b collapse, c weld, d T-junctions.
2. `"CloseTrivialHoles"`
3. `"RemoveDoubleTriangles"`: face clean-up (with the cancel rule above).
4. `"SplitNonManifoldEdges"` (new): stage e.
5. `"FixFlippedTriangles"`
6. `"CloseTrivialHoles (second pass)"`
7. `"StitchGaps (reuse vertices)"`: a snap (moves any hole-fill centre onto
   the lattice) and c weld, then face clean-up.
8. `"FixFlippedTriangles (second pass)"`
9. `"CloseAllHoles"`
10. `"Wrap"`
11. `"RemoveDegeneratedFaces"`: stage b (collapse, never delete with a hole),
    then the existing repeated-index drop.
12. `"RemoveTinyShells"`
13. `"PrepareForExport"`

Update `extended_repair_closes_a_trivial_open_box_and_reports_exact_stage_names`
and the stage-count asserts for the new list; nothing else in that file may be
weakened.

## 3. Gate change (`gate` in `repair.rs`)

Splitting a non-manifold edge turns one connected piece into the parts that
only touched along it, so `shells` rises. Allow exactly that: `shells` may go
up **only when** `non_manifold_edges` went strictly down; every other field
keeps the "never worse" rule. Unit test both directions (shells up with
non-manifold down = Improved; shells up with non-manifold equal = Worse).

## 4. Tests (generated in code, no binary files), in `tests/repair_native.rs`

Use `repair_mesh` (native). Helpers for volume, open edges and "max distance
from each output point to the nearest input point" (grid hash). For each, also
assert the result passes `Mesh::validate`.

1. f10: 10 mm box whose top face is split by a point `(5, 0.00005, 10)` with a
   sealing sliver `[4,5,8]` (as `genfx.py`): result 0 open edges, 0
   non-manifold, every face has non-zero area and no face height below
   `0.5 * step`, volume within 0.5 % of 1000.
2. Same with the point at height `0.01`: the sliver is kept (face count of the
   top stays, min face height within one step of 0.01), 0 open edges.
3. Needle: a closed box where one top corner fan has an extra point `0.001`
   from the corner: merged, sealed, 0 open edges.
4. Book-like: closed box with one face whose three distinct corners are exactly
   collinear (zero area, sealed by its neighbours): `degenerate_faces` 0 after,
   0 open edges, `verdict == Improved`, `can_apply`.
5. Crack weld: sphere (`make_sphere`) cut open along one ring with the ring's
   copy moved `0.008 mm`: welded, 0 open edges, one shell. Moved `0.03 mm`:
   not welded by stage c (it may still be closed later only as 3-4 edge holes;
   assert no point moved more than `5 * step + step * sqrt(3)/2`).
6. Two closed cubes `0.005 mm` apart: still two shells, no point moved more
   than `step * sqrt(3)/2` (closed parts never weld).
7. T-junction: open seam where one side has an extra point `0.004 mm` off the
   edge: absorbed, 0 open edges.
8. f7 exactly as `genfx.py` (two cubes sharing edge 2-6 plus fin `[4,5,16]`):
   0 non-manifold, 0 open edges, 2 shells, total volume 2000 +/- 1, the fin
   point `(5,5,15)` gone, `verdict == Improved`.
9. f8 (box + two duplicated faces): fixed as before.
10. f5 exactly as `genfx.py` (unwelded cylinder, jitter `3e-4` with
    `rng`-like deterministic jitter of your own, seam shifted `0.01`): 0 open
    edges, 1 shell, `Improved`.
11. Bound: on tests 1, 4, 5, 8 and 10 every output point is within
    `5 * step + step * sqrt(3)/2` of an input point; on sealed inputs (1-4, 6)
    within `step * sqrt(3)/2` (plus f32 rounding, `1e-5`).
12. Determinism: repairing the same mesh twice gives identical output.

In `tests/repair_windows.rs`, extend the ignored `repair_fixture_dir` line with
`open_edges`, `non_manifold_edges`, `shells` after repair (already in the
score) and `max_move_mm` (largest distance from an output point to the nearest
input point, grid hash). Keep it `#[ignore]`.

Existing tests (`mesh_ops_repair.rs`, `repair_windows.rs`, app and MCP tests)
must still pass without being weakened. If one fails because Repair now does
better (for example a stage changed a count), report it and explain; do not
loosen an assert to make it pass.

## 5. Report at the end

Files changed, test counts, the fixture numbers if you ran them, anything you
could not do, and any place you departed from this card and why.

## Overseer notes (2026-11-23)

Two card errors found in review and fixed by the overseer: stage 7 must not
merge by cell (it rejoined the f7 cubes the non-manifold split had
separated; it now only moves points onto the grid, `align`), and
`regularize_planar_regions` added centre points and faces (broke the
distance bound on f5 and the 0.01 mm sliver test), so it was removed from
Repair here instead of in R5; its test now checks flat patches are left
alone. Results are in `repair-plan.md` under R4.
