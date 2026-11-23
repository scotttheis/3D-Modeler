# Task card R5: native engine B (fill holes of any size; book under 5 s)

Read first: `docs/tasks/repair-plan.md` (phase R5), `docs/tasks/repair-spec.md`
section 4 (Holes), and `docs/tasks/repair-r4-card.md` sections 1 and 2f (the
grid and the stage list). Where this card is more exact than the spec, the
card wins.

Edit only `crates/modeler3d-core/src/repair.rs`,
`crates/modeler3d-core/src/repair_native.rs` (you may add one new private
module `crates/modeler3d-core/src/repair_fill.rs`, declared in `lib.rs` as a
private `mod`), `crates/modeler3d-core/tests/repair_native.rs`,
`crates/modeler3d-core/tests/mesh_ops_repair.rs` and
`crates/modeler3d-core/tests/repair_windows.rs`. Do not change public
signatures of `repair_mesh*`, `RepairReport` fields, `inspect_mesh`, or
anything in the app or MCP crates. No new dependencies. No
`unwrap`/`expect`/panics in library code. Do not commit. Do not run the Full
gate. Before cargo, run `scripts/Initialize-BuildClang.ps1` in the same
PowerShell session, then set `$ErrorActionPreference = 'Continue'` (the script
sets `Stop`, which aborts on cargo's stderr).

Finish with `cargo fmt`,
`cargo clippy -p modeler3d-core -p modeler3d-app -p modeler3d-mcp --all-targets -- -D warnings`,
`cargo test -p modeler3d-core -p modeler3d-app -p modeler3d-mcp`.

## Part A: hole fill (spec section 4)

Replace `close_small_holes` (it adds a centre point per hole) with a fill
that adds **no new points**: each hole loop of `n` points becomes exactly
`n - 2` triangles using only the loop's points.

1. **Loops.** Keep `boundary_loops` (it already splits loops at a shared
   vertex). A loop follows the open faces' own edge direction, so every fill
   triangle must traverse each loop edge in the **opposite** direction (the
   fill is wound consistently with its neighbours without needing
   `fix_winding`).
2. **Up to 300 edges: minimum total area.** Dynamic programming over the loop
   (`O(n^3)`): `W(i,k) = min over i<m<k of W(i,m) + W(m,k) + area(i,m,k)`.
   Area in `f64` from the mesh points. Ties go to the smallest `m`
   (deterministic).
3. **Above 300 edges: greedy ear clipping, smallest area first.** Ear =
   three consecutive remaining points; cut the valid ear with the smallest
   area (ties to the lowest loop position), with a binary heap and lazy
   invalidation (`O(n log n)`), until 3 points are left.
4. **Invalid triangles** (weight infinite in the DP; never cut as an ear):
   - its three points are exactly collinear or two are the same point, tested
     **on the R4 lattice** (`RepairGrid`, exact `i128` cross product = 0, as
     in stage b); a flat hole with collinear points along a side must still
     fill without a zero-area triangle;
   - it would add an edge between two loop points that is not a loop edge and
     already exists in the mesh (that would make a non-manifold edge), or that
     another fill triangle of the same hole already uses twice.
   If no valid fill exists, leave that hole open (do not fall back to a
   centre point).
5. **Flat holes stay flat** automatically (no new points). No refine, no
   smoothing.
6. **Stages.** `CloseTrivialHoles` and `CloseTrivialHoles (second pass)`: the
   new fill, loops of 3 and 4 points only. `CloseAllHoles`: the new fill, every
   loop. Simple profile uses the same functions. Stage names do not change.
   The fill must be deterministic (same mesh in, same mesh out).
7. A loop longer than `MeshOperationLimits` allow, or overflow, returns the
   typed error the stage already uses; it must not panic.

## Part B: book under 5 s (native, Release)

Today the book (`f1_book`, 334,180 faces) takes **5.18 s** on the native
engine in Release (`repair_fixture_dir`, see "Fixtures" below). Target:
**under 5 s, aim for under 3 s**, measured the same way.

1. First measure: add temporary per-stage timing (eprintln or a local
   `Instant`), run the book, and write the per-stage numbers in your report.
   Remove the timing code before you finish.
2. Then speed up what the numbers show. Known suspects: edge maps are
   `BTreeMap<(u32,u32), Vec<..>>` rebuilt in almost every stage
   (`edge_map`, `edges`, `boundary_loops`, `face_components`,
   `fix_winding`, `has_closed_cavity`, `split_non_manifold`); a `Vec` per
   edge allocates. Prefer a sorted `Vec` of `(edge, face, dir)` or a
   `HashMap` with an explicit sort where iteration order reaches the output.
   Skip a stage's rebuild when the mesh has no open / non-manifold edges and
   the stage would do nothing.
3. **Output must not change from speed work.** The fixture line now prints
   `faces=` and `hash=` (FNV over vertex bits and indices). Hashes before R5
   (native, Release):

   | fixture | faces | hash |
   |---|---|---|
   | f1_book | 334166 | 98420fce6fcd05a3 |
   | f4_flipped_sphere | 3968 | bb1b3c5432d69fb1 |
   | f6_overlapping_boxes | 24 | 5129314692b22665 |
   | f7_nonmanifold | 24 | afcb5e73c69d0965 |
   | f8_duplicates | 12 | d1b3896f6fbc3476 |
   | f9_inside_out | 3968 | bb1b3c5432d69fb1 |
   | f10_sealed_slivers | 14 | f34f043160d769d7 |

   These have no holes, so after R5 their hash must be **identical**. Any
   difference is a bug in Part B; fix it, do not explain it away. (f2, f3,
   f5, f11 change because of Part A.)

## Tests (generated in code, no binary files) in `tests/repair_native.rs`

Use `repair_mesh` (native). For each, assert the result passes
`Mesh::validate`, has 0 open and 0 non-manifold edges, 0 degenerate faces in
`inspect_mesh`, `verdict == Improved`, and **no output point that is not an
input point** (exact, after snapping: every output point within
`step * sqrt(3)/2 + 1e-5` of an input point).

1. Open-top cylinder, 64 sides (f2): one shell, top filled with 62
   triangles, every top triangle at exactly `z = h` (flat, spread 0), volume
   within 1 % of the closed cylinder.
2. Same with 500 sides: filled (ear-clipping path), top flat, finishes in
   under 1 s in a Release run (do not assert time in Debug).
3. Same with 3, 4 and 10 sides: `n - 2` fill triangles each.
4. Holey sphere like f3 (sphere with about 3 % of faces removed by a fixed
   deterministic pattern of your own): sealed, volume within 1 % of the
   closed sphere, largest move within the bound above (today a centre point
   moves 0.9 mm).
5. Flat hole with collinear points: a 10 mm box missing its top face, where
   the top boundary has an extra point at the middle of each side (8 loop
   points, 4 collinear triples): sealed, no zero-area triangle.
6. Non-planar hole: a "cup" of your own (curved open surface, 12-point rim at
   varying heights): sealed, the fill equals the minimum-area triangulation
   (check by a brute-force or independent DP in the test for this small n).
7. Hole whose obvious diagonal already exists in the mesh (a fan-like case
   you construct): sealed, 0 non-manifold edges.
8. Winding: after the fill the mesh has 0 flipped edges, also when the input
   hole's faces are wound inward (inside-out shell with a hole).
9. Determinism: the same mesh twice gives identical output.

Existing tests must still pass without being weakened. A test that checked
the old centre-point fill (face/vertex counts) may change its expected
numbers only to the `n - 2`, no-new-point rule; say which in the report.

## Fixtures (acceptance numbers)

Regenerate if missing (outside `E:`):

```powershell
$fx = "$env:TEMP\r5fx"; New-Item -ItemType Directory -Force $fx | Out-Null
python -I evidence\repair-eval-2026-11-22\scripts\genfx.py $fx
python -I evidence\repair-eval-2026-11-22\scripts\genfx_open500.py $fx
python -I evidence\repair-eval-2026-11-22\scripts\extract_book.py "$env:USERPROFILE\Desktop\Secret Book A.3mf" $fx
$env:MODELER3D_REPAIR_ENGINE = 'native'; $env:MODELER3D_REPAIR_FIXTURES = $fx
cargo test --release -p modeler3d-core --test repair_windows repair_fixture_dir -- --ignored --nocapture
```

Required after R5: f2 and f11_open500 `Improved` with 0 open edges; f3
`Improved`, 0 open edges and `max_move_mm` below 0.002; the seven hashes
above unchanged; book under 5 s; every other fixture no worse than before.

## Report at the end

Files changed, per-stage timing before and after, the fixture lines, test
counts, anything you could not do, and every place you departed from this
card and why.
