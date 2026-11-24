# Task card R7: what the owner sees (plain-words Repair result, marker tests, fixtures in code, MCP golden)

Read first: `docs/tasks/repair-plan.md` (phase R7) and this card. Where this
card is more exact than the plan, the card wins.

Edit only:

- `crates/modeler3d-core/src/repair.rs` (new summary function), `lib.rs`
  (re-export it),
- new test file `crates/modeler3d-core/tests/repair_fixtures.rs`,
- `crates/modeler3d-app/src/commands.rs` (`repair_selected` and its tests),
  `crates/modeler3d-app/src/desktop_jobs.rs` (Repair branch only),
  `crates/modeler3d-app/src/dialogs.rs` (`repair_dialog` text only),
  `crates/modeler3d-app/src/shell.rs` (review state setup for Repair, tests
  for the Items marker), `crates/modeler3d-app/src/review_capture.rs`
  (one new state name and its doc row),
- `crates/modeler3d-mcp/src/r4/server.rs` (Repair success result gets
  `message`; tests),
- new golden file `crates/modeler3d-mcp/tests/fixtures/repair_open_box.json`
  and its test (in `server.rs` tests or a new `crates/modeler3d-mcp/tests/`
  file, whichever reaches the server more simply).

No new dependencies. No `unwrap`/`expect`/panics in library code. Do not
change `repair_mesh*`, `inspect_mesh` or the repair gate. Do not commit. Do
not run the Full gate. Before cargo, run `scripts/Initialize-BuildClang.ps1`
in the same PowerShell session, then set `$ErrorActionPreference = 'Continue'`.

Finish with `cargo fmt`,
`cargo clippy -p modeler3d-core -p modeler3d-app -p modeler3d-mcp --all-targets -- -D warnings`,
`cargo test -p modeler3d-core -p modeler3d-app -p modeler3d-mcp`.

## Part A: plain-words result (core, shared by desktop and MCP)

Add `pub fn repair_summary(report: &RepairReport) -> String` in `repair.rs`
and re-export it from `lib.rs`. For a report that `nothing_to_fix()` return
`REPAIR_NOTHING_TO_FIX_MESSAGE`; for `refused()` return
`REPAIR_REFUSED_MESSAGE`. Otherwise build
`"Repaired: <part>, <part> and <part>."` from `before`/`after` health and
`score_before`/`score_after`, listing only the parts whose count went down,
in this order (numbers with a comma every three digits, e.g. `1,858`;
singular when the count is 1):

1. holes: `before.boundary_edge_loops - after.boundary_edge_loops` →
   `closed 87 holes` / `closed 1 hole`
2. inside-out parts: `score.inside_out_shells` drop →
   `turned 1 inside-out part right side out`
3. flipped faces: `score.flipped_edges` went down (count is of edges, not
   faces, so no number) → `turned flipped faces the right way`
4. overlapping parts: `score.shells` went from b to a (a < b) →
   `merged 2 overlapping parts into 1` (only when
   `score_before.tiny_shells == score_after.tiny_shells`; otherwise the drop
   is reported by item 7 only)
5. `score.duplicate_faces` → `removed 2 duplicate faces`
6. `score.degenerate_faces` → `removed 1 zero-size triangle`
7. `score.tiny_shells` → `removed 3 tiny loose pieces`
8. `score.non_manifold_edges` → `fixed 4 edges shared by more than two faces`
9. `score.self_intersections` → `fixed 12 places where the model passed through itself`

If none of these went down, return `"Repaired."`. Then append the engine,
as a second sentence: if `engine_note` is `Some(note)`, append `" " + note`;
else if the engine is Windows append `" Used the Windows repair engine."`;
else append nothing. Example: `"Repaired: closed 87 holes and removed 1
zero-size triangle. Used the Windows repair engine."`

Unit tests in `repair.rs` (or `tests/mesh_ops_repair.rs`) that build
reports by running real repairs: open box → `Repaired: closed 1 hole.`
(native engine forced by `MODELER3D_REPAIR_ENGINE=native` is not needed:
call `repair_mesh`, the native-only API), plural/thousands formatting checked
on a hand-made `RepairReport` clone with edited counts.

## Part B: desktop and MCP use it

- `commands.rs` `repair_selected`: the success message is
  `repair_summary(&result.report)` (replace the `Repair: N stage(s)` /
  `Repair: Windows repair engine` strings). Keep `"Repair: no changes"` only
  for the case where `replace_mesh` reports no change.
- `desktop_jobs.rs` Repair branch: the `DesktopJobResult::Mesh` message for
  Repair is exactly `repair_summary(&result.report)` (no
  `"Repair complete: N face(s); …"` wrapper). Simplify, Subdivide and Smooth
  are unchanged. Check that the Repair dialog / status shows this message
  after the job applies; if the dialog does not show job messages, show it
  the same way the refused message is shown today.
- `dialogs.rs` `repair_dialog`: replace the label text with
  `"Repair closes holes, turns flipped faces the right way and joins overlapping parts. If it can't make the model better, nothing is changed."`
- MCP `server.rs` repair success: add `"message": repair_summary(...)` to the
  result JSON (refused and nothing-to-fix results already have it; keep them
  using the same constants).
- Tests: desktop and MCP give the same message for the same model (an open
  cube): extend the existing `commands.rs` repair tests.

## Part C: needs-repair marker

The Items marker uses `item_is_healthy` (`shell.rs`), which is
`inspect_mesh(..).valid`. Add tests that prove the marker shows for an
inside-out cube (all faces reversed) and for a self-crossing model (box with
one corner pushed through its bottom, as in `model_with_unrepairable_hole`
in `commands.rs`), and not for a clean cube or a hollow cube with a correct
inward cavity. Do not change the marker code unless a test fails.

## Part D: review capture states

- State `repair` (existing): instead of a clean cube, the selected cube has
  its last face removed before Repair runs (an open box), so the capture
  shows `Repaired: closed 1 hole.`. Make sure the capture is taken after the
  background job finished (look at how other job states wait; if the harness
  does not wait for jobs, run the repair synchronously in the review setup
  through the same job function so the painted message is the real one).
- New state `repair-refused`: the self-crossing hollow box from
  `model_with_unrepairable_hole`, so the capture shows the refused message.
  Add it to the module doc table in `review_capture.rs`.

## Part E: fixtures as core tests generated in code

New `crates/modeler3d-core/tests/repair_fixtures.rs`. Port the generator
`evidence/repair-eval-2026-11-22/scripts/genfx.py` (f2-f10; f1 is the
owner's book and is not included) and `genfx_open500.py` (500-edge hole) to
Rust functions that build each mesh in code. No binary or data files. Where
the Python uses `numpy` random numbers, use a small fixed-seed generator in
the test (e.g. a 64-bit LCG or xorshift); the exact faces chosen may differ
from the Python files, that is fine. Each fixture runs through
`repair_mesh` (native, every OS) and checks:

| Fixture | Expected after native repair |
|---|---|
| f2 open cylinder (64 edges) | valid, 0 open edges, no new points |
| f3 holey sphere (3 % faces removed) | valid, closed, same vertex count |
| f4 sphere with 20 % flipped faces | valid, 0 flipped edges, volume > 0 |
| f5 unwelded jittered cylinder with a 0.01 mm crack | valid, one part |
| f6 two overlapping boxes | valid, one part, volume 15,000 ± 0.01 mm³ |
| f7 two cubes sharing an edge plus a fin | 0 non-manifold edges, fin gone, total volume 2,000 ± 0.01 mm³, two parts |
| f8 cube with duplicate faces | valid, 12 faces |
| f9 inside-out sphere | valid, volume > 0 |
| f10 sealed cube with a sliver | stays sealed (0 open edges) |
| open500 | valid, closed |

For each also assert the verdict is `Improved` (except f10 where the input
is already valid: assert it is not made worse), and that
`repair_summary` is not the refused message. Keep the file under ~400 lines;
reuse helpers from `tests/repair_native.rs` by copying the few needed (test
files cannot share modules unless a `tests/common/mod.rs` exists; check).

## Part F: MCP golden test

A golden JSON file for MCP `repair` on an open 20 mm box (stock cube with
its last face removed), native engine forced (set
`MODELER3D_REPAIR_ENGINE=native` in a child process like
`disabled_windows_and_native_override_use_native` in
`tests/repair_windows.rs` does, or call with Windows not allowed if the
server path permits). Golden content: the result with volatile fields
(`operation_id`, timestamps, `scene_revision`, object ids) removed or
replaced by fixed placeholders, keys sorted, floats as produced. The test
fails if the result differs from the file; setting
`MODELER3D_UPDATE_GOLDEN=1` rewrites the file. Write the file once, read it
and check it is sensible (`changed` true, `message` "Repaired: closed 1
hole.", `verdict` "Improved", `engine` "native").

## Report back

List changed files, the test names added, and the exact fmt / clippy / test
results. Do not claim anything not run.
