# Task card R6: native engine C (Wrap: merge overlapping parts; self-intersection check)

Read first: `docs/tasks/repair-plan.md` (phase R6), `docs/tasks/repair-spec.md`
sections 6, 7 and "Owner decision: hollow models", and
`docs/tasks/repair-r5-card.md` ("Fixtures"). Where this card is more exact
than the spec, the card wins.

Edit only:

- `crates/modeler3d-core/src/mesh_ops.rs` (`inspect_mesh` only),
- one new private module `crates/modeler3d-core/src/self_intersect.rs`
  (declared in `lib.rs` as a private `mod`, items `pub(crate)`),
- `crates/modeler3d-core/src/repair.rs`, `crates/modeler3d-core/src/repair_native.rs`,
- `crates/modeler3d-core/tests/repair_native.rs`,
  `crates/modeler3d-core/tests/mesh_ops_repair.rs`,
  `crates/modeler3d-core/tests/repair_windows.rs`,
- `crates/modeler3d-mcp/src/r4/server.rs` (only the `score` closure in
  `repair_report`: add `"self_intersections"`).

Do not change public signatures of `repair_mesh*` or `inspect_mesh`, or any
`MeshHealthReport` field. The only public type change allowed is one new field
`self_intersections: usize` on `RepairScore`. No new dependencies (Manifold is
already used through `manifold_from_mesh` / `mesh_from_manifold` in
`mesh_ops.rs`; reuse them, they may become `pub(crate)` if not already). No
`unwrap`/`expect`/panics in library code. Do not commit. Do not run the Full
gate. Before cargo, run `scripts/Initialize-BuildClang.ps1` in the same
PowerShell session, then set `$ErrorActionPreference = 'Continue'`.

Finish with `cargo fmt`,
`cargo clippy -p modeler3d-core -p modeler3d-app -p modeler3d-mcp --all-targets -- -D warnings`,
`cargo test -p modeler3d-core -p modeler3d-app -p modeler3d-mcp`.

## Part A: self-intersection check (`self_intersect.rs`, used by `inspect_mesh`)

1. **What counts.** A *crossing pair* is two faces that pass through each
   other. Touching is not crossing. In `f64` from the `f32` points, with
   `eps = 1e-6` mm:
   - skip pairs that share two or three vertex indices (edge neighbours,
     duplicates);
   - let `d1[i]` = signed distance of face 1's corners to face 2's plane, and
     `d2[j]` the reverse (unit normals; skip a face whose normal has length
     below 1e-12, it is degenerate and counted elsewhere). Values with
     `|d| <= eps` are zero;
   - a crossing needs face 1 to have corners strictly on **both** sides of
     plane 2 (some `> eps`, some `< -eps`) **and** face 2 strictly on both
     sides of plane 1. So coplanar faces, faces that only touch at a point,
     along an edge, or with a vertex lying on the other face never count;
   - then project both faces' intersection intervals onto the line of the two
     planes; they cross only if the intervals overlap by more than `eps`.
   A pair sharing one vertex index uses the same test (the shared corner is
   at distance 0 on both sides).
2. **Search.** A bounding-volume hierarchy (or sorted-grid) over face boxes
   (boxes grown by `eps`), deterministic, no recursion deeper than the tree
   (an explicit stack is fine), each unordered pair tested once. The count
   must not depend on face order.
3. **API (crate-private).**
   - `crossing_pairs(mesh) -> Vec<(u32, u32)>` (sorted, each pair
     `(low, high)`), and a counting entry used by health that does not keep
     the list.
4. **Health.** `inspect_mesh` sets `self_intersections: Some(n)` and
   `self_intersection_status` to `"checked"`. When `n > 0`, push the issue
   `"{n} self-intersections"` right after the tiny-shells issue (so
   `valid` becomes false and the Items "needs repair" marker shows). Above
   2,000,000 faces skip the check: `None` and status
   `"not checked: model has more than 2,000,000 faces"`.
5. **Speed (Release, measure and report).** On the book (`f1_book`,
   334,180 faces) the check must add **under 0.4 s** to `inspect_mesh`.
   Report `inspect_mesh` time on the book before and after, and on a
   1,000,000-face sphere (`make_sphere` with enough subdivisions, or a
   generated grid sphere) after.
6. **Score.** `RepairScore.self_intersections` = `health.self_intersections`
   (0 when `None`), added to `fields()` as the 9th field, so the gate treats
   it like every other defect (it may not go up; going down is an
   improvement). Keep the existing shells exception exactly as it is (index
   7). Add the field to the MCP `score` JSON.

## Part B: Wrap (stage `"Wrap"` in `repair.rs`, native Enhanced/Extended)

Replace `wrap_mesh` (today: compact + `fix_winding`) with:

1. **Components** by shared edges (`face_components`). A component is
   *closed* if each of its edges has exactly two faces, both within it, with
   opposite directions. Its signed volume is computed in `f64`.
2. **Correct cavities (owner answer 6).** A closed component `C` with
   negative volume is a *correct cavity* if a closed component `P` with
   positive volume exists such that `C`'s bounding box is inside `P`'s, one
   vertex of `C` has generalized winding number `|w| > 0.5` against `P`
   (solid-angle sum, f64), and no face of `C` is in a crossing pair with
   any face of another component. Correct cavities are left exactly as they
   are (same faces, same winding).
3. **Overlap groups.** For the other closed components (positive volume
   after `fix_winding`): join two in a group (union-find) if a face of one
   crosses a face of the other (Part A pairs), or if one's bounding box is
   inside the other's and a vertex of it has `|w| > 0.5` against the other
   (nested part). Two parts that only touch (shared face, edge or point, or
   a small gap) are **not** joined; they stay separate parts (R4 decision).
4. **Union.** For each group with two or more members, build one Manifold per
   member (`manifold_from_mesh`) and union them (`Manifold` batch union or a
   fold of `union`), then `mesh_from_manifold`. A group whose Manifold step
   fails is left unchanged and the stage records nothing else (do not fail
   the whole stage for one group).
5. **Untouched parts stay bit-identical.** If no group has two or more
   members, return the input mesh unchanged (no compaction, no reorder).
   Otherwise the output is: the faces of every untouched component (open
   parts, single-member groups, correct cavities) in their original order,
   followed by each union result in order of the group's lowest face index;
   then compact vertices. Deterministic.
6. **Self-crossing single part** (one closed component crossing itself) is
   **out of scope**: leave it unchanged (the fix plan allows this). Report how
   many fixtures/tests hit it, if any.

## Part C: keep cavities through `fix_winding`

Today `fix_winding` turns every component with negative volume outward,
which turns a correct cavity into a nested solid. Change it so a closed
component that is a correct cavity by the test in Part B.2 (evaluated after
making its winding consistent) keeps facing inward. Everything else is
unchanged. Share the cavity test between Part B and Part C (one function).

## Tests (generated in code, no binary files)

In `tests/repair_native.rs` (native `repair_mesh`), each result passes
`Mesh::validate` and has 0 open, 0 non-manifold, 0 flipped edges:

1. **f6**: two 20 x 20 x 20 boxes overlapping by 5 mm along x (volume
   16,000 before): `Improved`, one shell, volume `15,000 +- 0.5`,
   `self_intersections == Some(0)` after, `Some(n > 0)` before.
2. Two overlapping spheres (`make_sphere`), one inside-out: one shell,
   volume within 1 % of the analytic union.
3. Nested outward spheres (r 15 and r 5, both outward): result is only the
   outer sphere (one shell, its volume).
4. **Hollow ball, correct cavity**: outer sphere outward, inner sphere
   inward, plus one face removed from the outer: `Improved`, `cavity_shells
   == 1`, volume = outer - inner within 1 %.
5. Hollow ball with the inner sphere **wrong way** (outward): merged, one
   shell, outer volume.
6. Two cubes sharing a face, an edge, and a point, and 0.05 mm apart: each
   stays two parts, `self_intersections == Some(0)` before and after, output
   identical to input when nothing else is wrong.
7. Determinism: f6 twice gives identical output.

In `tests/mesh_ops_repair.rs`, health:

8. A closed sphere, a box, the book-like cases already there: `Some(0)`,
   status `"checked"`, `valid` unchanged.
9. f6: `Some(n)` with `n > 0`, issue text present, `valid == false`.
10. A box with one corner pushed through its bottom face (single part
    crossing itself): `Some(n > 0)`.
11. Coplanar overlap (two boxes sharing part of a face, not overlapping in
    volume): `Some(0)`.

In `tests/repair_windows.rs` (skips itself when the Windows engine is not
available, like the others):

12. **Both engines pass the same fixtures**: build in code the f2-f10 shapes
    (port them from `evidence/repair-eval-2026-11-22/scripts/genfx.py`) and
    run each through the Windows route and through native. For every one:
    both results have 0 open, 0 non-manifold, 0 flipped edges, 0
    self-intersections, the same number of shells, and volumes within 1 % of
    each other (f10, already valid, may be untouched by both). Print one line
    per fixture.
13. `hollow_model_uses_native_and_keeps_its_cavity` must now take the
    `can_apply` branch (assert it).

Existing tests must still pass without being weakened. If an expected number
changes because of the new score field or issue line, say which test and why.

## Fixtures (acceptance numbers)

Regenerate the fixtures outside `E:` as in the R5 card ("Fixtures") and run
`repair_fixture_dir` with `MODELER3D_REPAIR_ENGINE=native` in Release. Also
run it once without that variable (Windows route).

Required after R6, native:

- `f6_overlapping_boxes`: `Improved`, 1 shell, volume 15,000 (+-0.5).
- every other fixture: same verdict as `evidence/repair-eval-2026-11-22/r5/fixtures-native.txt`
  (f10 stays `NoBetter`, it is already valid) and the **same hash**: f1 book
  `98420fce6fcd05a3`, f2 `3bb9b798fbce3444`, f3 `0e450dd9800f5c3d`, f4/f9
  `bb1b3c5432d69fb1`, f5 `dab855099f4c8c4b`, f7 `afcb5e73c69d0965`, f8
  `d1b3896f6fbc3476`, f10 `f34f043160d769d7`, f11 `67a54f9c9962e5c6`. A
  changed hash is a bug unless that fixture had a crossing pair before Wrap;
  report it either way.
- book native repair time under **3 s** (was 2.30 s).
- Windows route: still fixes all of them, none worse.

## Report at the end

Files changed, the fixture lines (native and Windows route), `inspect_mesh`
timings (book before/after, 1M faces), test counts, anything you could not
do, and every departure from this card with the reason.
