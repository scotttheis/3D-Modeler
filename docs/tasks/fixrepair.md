# fixrepair: Repair makes Secret Book A worse (findings only, nothing changed)

Status (2026-11-22): **investigated, not fixed.** No source change was made.
Owner asked for findings first.

## What the owner sees

Select the book (`6`), press Repair. The dialog says "Repair complete: 333,984
face(s); 12 repair stage(s)". The book looks the same and is still marked as
needing repair. It is actually worse than before.

## Evidence

Live Release app (pid 37364, `Secret Book A.3mf`, scene revision 2), read with
the read-only MCP `scene_summary` (no repair was re-run, nothing was edited):

| | file on disk | in the app after Repair |
|---|---|---|
| faces | 334,180 | 333,984 (196 fewer) |
| vertices | 167,084 | 167,076 |
| boundary edges | **0** | **402** (87 loops) |
| watertight / encloses volume | yes | **no / no** |
| winding consistent | yes | yes |

Objects `7` (12 faces) and `8` (139,774 faces) are valid and untouched. The
app log shows two Repair jobs: the first `unchanged` (a valid object), the
second `ok` (the book, committed).

Offline check of the file (Python, read-only, scratchpad): the book has 195
sliver triangles (cross-product squared <= 1e-6) and exactly 1 with zero
area. Dropping just those 195 faces, nothing else, gives **403** boundary
edges, matching the app's 402. Slivers are 0.0002 mm to 94 mm long (median
0.12 mm): they are real sealing triangles, not stray junk.

## Cause (read from the code, not yet proven stage by stage)

1. **Two different "degenerate" thresholds.** The health check
   (`mesh_ops.rs` ~line 318) calls a face degenerate at area measure
   `<= 1e-24`: 1 face on the book, so the book is flagged invalid and Repair
   proceeds (`nothing_to_fix()` is `before.valid`). Repair's
   `RemoveDegeneratedFaces` (`repair.rs` ~line 355) uses `<= 1e-6`: 195 faces.
   The slivers Repair deletes are ones the health check never complained about.
2. **Deleting them tears the sealed surface open.** The stage runs after
   `CloseAllHoles`, so nothing patches the new holes. `CloseTrivialHoles` and
   `CloseAllHoles` both call `close_small_holes`, which only closes loops of 3
   or 4 edges (`repair.rs` ~line 458). "All" does not mean all. `Wrap` is only
   compact + fix-winding, not a real wrap.
3. **No "is the result better?" gate.** `can_apply` is just `candidate_safe`
   (finite numbers, valid indices) (`repair.rs` ~line 225). The desktop job
   commits and says "complete" whatever the after-health is
   (`desktop_jobs.rs` ~line 149, `shell.rs` ~line 2334). Nothing compares
   `report.after` with `report.before`.

Related: the same repair opening 87 holes is the cause already recorded in
`subtract-and-side-bar.md` section 1. That fix only made Subtract skip Repair
for such meshes; Repair itself was not changed.

## Not checked

- Which stage(s) account for the last face and the 8 fewer vertices
  (`StitchGaps` welding at 1 um merges 2 vertices on the file). A stage-by-stage
  run of `repair_mesh` on the loaded file would show it.
- Whether the book had 1 or 2 shells before Repair (the app reports 2 now).
- MCP `repair` uses the same core path, so it should do the same. Not run, to
  avoid a third edit on the open model. Undo in the app restores the original.

## Open question for the owner (decision, not for the code)

What should Repair do for a sealed model whose only flaw is slivers? Options:
leave them alone and say so; remove them only when the surface stays sealed;
or collapse them (merge their short edge) instead of deleting. Also whether a
repair that leaves the model with more holes than before should be refused.
Check the Python build's behaviour before changing anything.

## How to reproduce

1. Open `C:\Users\<owner>\Desktop\Secret Book A.3mf` (objects `6` book,
   `7` box, `8`).
2. Select `6`, press Repair. Dialog says complete; the book now has 87 hole
   loops and is not watertight. Ctrl+Z restores it.
3. Read-only check: MCP `scene_summary` shows `boundary_edges` 0 before and
   402 after.

## Next session

Owner answered the decisions on 2026-11-22. The work now runs from
[`repair-plan.md`](repair-plan.md) (phases R1-R7, one per session).

---

# Deeper evaluation (2026-11-22, second session): why Repair is weak overall

Status: **evaluation only. No product source was changed.** Scripts and raw
numbers are in `evidence/repair-eval-2026-11-22/` (`compare.txt` is the full
table; `scripts/` rebuilds every test model and every measurement).

## In one paragraph

Our Repair copied the **names** of 3D Builder's twelve repair steps (found by
reverse engineering, `L:\Builder3D\docs\reversing_notes\08-repair-trace.md`)
but not what the steps do. 3D Builder's steps come from Netfabb's repair
engine ("NMR"). Ours are small stand-ins: "Close all holes" only closes holes
of 3 or 4 edges, "Wrap" does nothing useful, "Stitch gaps" only merges points
that land in the same 0.001 mm grid box, and "Remove degenerated faces"
deletes thin triangles, which tears sealed models open. On top of that, one
bug switches hole closing off completely for most real models, and nothing
checks whether the result is better before it is committed. The same engine
3D Builder uses **ships inside Windows** and can be called from Rust; I ran it
on every test model below and it fixed all of them.

## How the comparison was done

- 10 broken test models built by `scripts/genfx.py` plus the real book
  (object `6` of `Secret Book A.3mf`, from a copy; the original file and the
  running app were not touched).
- Each model repaired three ways:
  - **ours**: `modeler3d_core::repair_mesh` (the exact desktop path), via a
    throwaway harness built outside the project (`ours_repair_probe.rs`).
  - **win**: Windows' built-in
    `Windows.Graphics.Printing3D.Printing3DModel.RepairAsync` (the 3D Builder /
    Netfabb engine), called from Rust with the `windows` crate we already
    depend on, version 0.62.2 (`windows_repair_probe.rs`).
  - **blender**: Blender 5.2's "3D-Print Toolbox / Make Manifold" steps
    (merge by distance, dissolve degenerate, delete loose, fill holes,
    recalculate normals) (`blender_repair.py`).
- Measured with one script for all three (`meshstat.py`, `compare.py`): open
  (boundary) edges, non-manifold edges, flipped neighbour pairs, shells,
  volume, zero-area faces, duplicates, and the largest distance between the
  result and the original surface in both directions (Blender BVH).

## Results (fixed = closed, one consistent outward shell, nothing left over)

| Test model | What is wrong | ours | Windows (3D Builder engine) | Blender Make Manifold |
|---|---|---|---|---|
| f1 book (real) | 1 zero-area face, 195 thin slivers, otherwise sealed | **worse**: 402 open edges, 2 shells | fixed, sealed | fixed, sealed |
| f10 sealed cube + 1 sliver | same kind of problem as the book, tiny | **worse**: opens a 3-edge hole (health calls it "valid", so the app would skip it) | fixed | fixed |
| f2 cylinder, top missing | one 64-edge hole | not closed | fixed | fixed |
| f3 sphere, 3% faces missing | 87 holes, 71 of them single triangles | **none** closed | fixed | fixed |
| f4 sphere, 20% faces flipped | flipped faces | fixed | fixed | fixed |
| f5 STL "soup" + 0.01 mm crack | unwelded points, small crack | 146 open edges, 11 shells left | fixed | fixed |
| f6 two overlapping boxes | parts pass through each other | unchanged (2 shells) | **fixed: merged into one solid** (volume 15,000 = true union) | unchanged |
| f7 two cubes on one edge + loose fin | non-manifold edge, dangling face | **worse**: adds 3 flipped pairs, fixes nothing | fixed (fin removed, edge split) | not fixed |
| f8 cube + duplicate faces | duplicates | fixed | fixed | fixed |
| f9 sphere inside-out | whole model inverted | flips it, **but health calls the inside-out model "valid", so the app never offers Repair** | fixed | fixed |

Score: ours 3 of 10 fixed and 3 made worse. Windows 10 of 10. Blender 7 of 10
(no overlap merge, no non-manifold fix).

Speed on the book (334,180 faces): ours 2.2 s, Blender 2.1 s, Windows 21.5 s.

What Windows' engine does differently (worth knowing before adopting it):

- It **never deletes thin triangles**; it merges very short edges (shortest
  edge on the book went from 0.0001 mm to 0.002 mm) so the surface stays sealed.
- It **moves points slightly**: about 0.003 mm on most models, 0.027 mm at
  worst on the book. The book's volume changed 0.02%.
- It **re-cuts flat faces** into more triangles (a 12-face box comes back with
  96 faces; the book gained 2,472 faces). The shape is unchanged.
- It **merges overlapping parts** into one solid and removes loose fins. This
  is its "Wrap" step and is most of why 3D Builder's repair feels so good on
  messy downloads.
- It runs on every mesh it is given, even a perfect one (the clean box came
  back re-cut). 3D Builder only offers Repair when a model is flagged, which
  our `nothing_to_fix()` already copies.

PrusaSlicer and OrcaSlicer ship the same idea ("Fix model by Windows 10",
`FixModelByWin10.cpp`), so calling this engine is established practice. The
API has no deprecation notice on Microsoft Learn and works on this Windows 11
machine.

## Root causes (each confirmed on the test models)

1. **Hole closing turns itself off.** `boundary_loops` (`repair.rs` ~line 846)
   returns *no loops at all* if any single vertex touches two holes
   (`outgoing.insert(...).is_some()` → `return Vec::new()`). The holey sphere
   has 19 such points and the torn book has 25, so not even their 71 / 49
   single-triangle holes were closed.
2. **Holes over 4 edges are never closed.** `close_small_holes` skips loops
   outside 3..=4. Both `CloseTrivialHoles` and `CloseAllHoles` call it. There
   is no real hole filler anywhere in core.
3. **Thin triangles are deleted, not collapsed.** `remove_degenerate_faces`
   drops faces with cross-product² ≤ 1e-6 (and `PrepareForExport` does it
   again). Every deleted face leaves a hole, and it runs *after* the hole
   steps, so nothing patches them. Health uses 1e-24, so the faces Repair
   deletes are not ones the user was warned about.
4. **"Wrap" is a no-op.** `wrap_mesh` is compact + fix winding. The real step
   merges overlapping parts and strips inside/loose geometry. Ours leaves
   overlaps and fins untouched.
5. **"Stitch gaps" cannot stitch.** `weld_vertices` rounds each point to a
   0.001 mm grid. Two points 0.0002 mm apart on either side of a grid line
   never merge, and real cracks (0.01 mm in f5) are far beyond it. The Python
   build used `trimesh.repair.stitch`, which at least fan-closed loops; the
   Rust port dropped that, so on this point Rust is weaker than Python.
6. **Non-manifold edges are not handled**, and winding repair across them
   adds flips (f7 got worse).
7. **The planar clean-up step adds faces for no benefit.**
   `regularize_planar_regions` replaces one triangle of a flat patch with a
   3-triangle fan around its centre. It fixes nothing.
8. **No "is it better?" gate.** `can_apply = candidate_safe` only checks
   numbers and indices. Desktop and MCP commit and say "Repair complete" even
   when the result has more problems than the input.
9. **Health misses two real problems**, so the app never offers Repair for
   them: an inside-out model (`is_volume` uses `abs(volume)`), and
   self-intersections (`self_intersections: None`, "not checked").

Side note: health counts "2 disconnected shells" as a problem, so a model
meant to be two pieces is flagged and Repair runs on it. Harmless once Repair
cannot damage; it matters today because it can.

## Other tools looked at

| Tool | Useful for us | Licence fit (we are Apache-2.0) |
|---|---|---|
| Windows `Printing3DModel.RepairAsync` | Exactly 3D Builder's result. Windows only. | OS API, fine |
| Manifold (`manifold-csg`, already a dependency) | Union of closed shells = the "merge overlapping parts" half of Wrap | Apache-2.0, fine |
| Blender Make Manifold | Shows the right *order*: weld → collapse degenerates → fill all holes → normals | GPL, reference only |
| MeshFix (Attene) | Best open-source all-in-one, including self-intersections | GPL/commercial, reference only |
| CGAL Polygon Mesh Processing | Liepa hole filling with fairing, stitch borders, orient soup | GPL/commercial, reference only |
| Liepa 2003, "Filling holes in meshes" | Published algorithm (minimum-weight triangulation + refine + fair) we can write ourselves | algorithm, fine |

# Fix plan

Goal: Repair never makes a model worse, and on Windows gives the same result
as 3D Builder; on Mac/Linux (and as the Windows fallback) our own engine gets
as close as possible. Desktop and MCP share every step (full parity rule).

## Owner decisions needed first

1. **Use the Windows engine on Windows?** Recommended: yes, as the main path,
   with our own engine as fallback and for Mac/Linux.
2. **Is a tiny shape change OK?** Windows moves points ~0.003 mm (0.027 mm
   worst on the book) and adds triangles on flat sides. Recommended: yes, it
   is what 3D Builder does.
3. **Merge overlapping parts into one solid during Repair?** Recommended: yes,
   like 3D Builder.
4. **Refuse a repair that does not help?** Recommended: yes, show "Repair
   couldn't improve this model. Nothing was changed." and add no undo entry.

## Step 1: stop the damage (small, ship first)

- Add a before/after score in core: open edges, non-manifold edges, flipped
  pairs, degenerate faces, inside-out shells, shell count. `can_apply` only
  when the result is safe **and** no count got worse **and** at least one got
  better. Desktop (`desktop_jobs.rs` ~149) and MCP (`server.rs` ~2387) both
  use it; refused = `Unchanged` with the decision-4 message.
- Stop deleting slivers: `RemoveDegeneratedFaces` and `PrepareForExport` only
  drop faces with a repeated vertex index (true zero area after welding). One
  threshold shared with `inspect_mesh`.
- Fix `boundary_loops` so a vertex shared by two holes splits into two loops
  instead of aborting (choose the next boundary edge by turning angle).
- Health: flag an inside-out shell (negative signed volume) as a problem.
- Tests: f10 (sealed + sliver) stays sealed; a sealed sliver mesh like the
  book stays sealed (generated fixture, not the owner's file); f3 closes its
  single-triangle holes; a refused repair leaves no history entry; MCP gives
  the same answer.

## Step 2: Windows engine route (gives 3D Builder results)

- New core module behind `cfg(windows)` (capability check in
  `modeler3d-platform` per the adapter rules): build a `Printing3DModel` and
  `Printing3DMesh` straight from our buffers (no 3MF file), `RepairAsync`,
  read the buffers back. Needs `windows` features `Graphics_Printing3D`,
  `Storage_Streams`, `Foundation_Collections` (the probe used these plus
  `Storage`; reading buffers back went through `DataReader`). Run
  `cargo deny` and record the size change.
- Run on the job worker thread with progress, cancel through the async
  action's `Cancel()`, and a time limit. The book takes ~21 s, so the
  progress dialog must stay honest.
- Always re-check the result with `inspect_mesh` and the Step 1 gate. If the
  API is missing, fails, times out, or the gate says worse, fall back to the
  native engine, and say which engine ran in the report and the log.
- One object at a time (each object is its own mesh, as now).
- MCP `repair` uses the same function, so desktop and MCP match on the same
  machine. Add `engine: "windows" | "native"` to the MCP result.

## Step 3: native engine rebuilt (Mac/Linux and fallback), in this order

1. **Weld** by true distance (spatial hash checking neighbour cells), 0.001 mm.
2. **Stitch** open edges only: snap boundary points to nearby boundary
   points/edges within a larger gap tolerance (start 0.05 mm, tune on the
   fixtures), splitting edges at T-junctions.
3. **Duplicates** removed (as now).
4. **Non-manifold**: remove dangling fins; split edges shared by more than two
   faces into separate sheets.
5. **Degenerates**: collapse the short edge of needles and flip the long edge
   of caps, with the link-condition check so the surface stays sealed. Never
   delete and leave a hole.
6. **Holes of any size**: Liepa minimum-weight triangulation for loops up to
   a few hundred edges, ear-clipping fallback for larger, then refine and
   smooth the patch to match the surrounding triangle size. Flat holes get a
   flat triangulation.
7. **Orientation**: consistent winding per shell, outward by volume (current
   code is fine once non-manifold edges are gone).
8. **Wrap**: union all closed shells with Manifold (already a dependency) so
   overlapping parts merge and inside shells disappear. Add a BVH
   triangle-triangle self-intersection check to `inspect_mesh` at the same
   time, so health stops saying "not checked".
9. **Tiny shells** removed (as now).
10. Remove `regularize_planar_regions` from Repair.
11. Step 1 gate.

Target: all 10 fixtures fixed, except possibly a self-intersection inside a
single shell; book under 5 s.

## Step 4: tests and evidence

- Turn the `genfx.py` fixtures into core tests (generated in code, no binary
  files), each checking: watertight, one consistent outward shell per
  intended part, distance from the original under a bound.
- Windows-only test: native and Windows engines both pass the same fixtures.
- MCP golden test for the `engine` field and the refused-repair message.
- Gate: `cargo test --workspace`, clippy, fmt, `cargo deny`, then
  `Update-CurrentBuild.ps1` before telling the owner it is done.

## Step 5: what the owner sees

- The dialog says what was fixed in plain words, e.g. "Repaired: closed 87
  holes, fixed 1,858 flipped faces, merged 2 overlapping parts." instead of
  "12 repair stage(s)".
- A refused repair shows the decision-4 message and changes nothing.
- The "needs repair" marker also appears for inside-out models and (after
  Step 3.8) self-intersecting ones.

## Optional: side-by-side with 3D Builder

3D Builder 20.0.4.0 is installed. If the owner repairs the book and one or
two fixtures (the `.3mf` files `genfx.py` writes) in 3D Builder, we can
confirm it matches the Windows engine numbers above. It should, since it is
the same engine, so this is a nice-to-have.
