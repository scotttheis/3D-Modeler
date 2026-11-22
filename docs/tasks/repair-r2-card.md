# Task card R2: stop the damage (Repair never makes a model worse)

Read first: `docs/tasks/repair-plan.md` (phase R2), `docs/tasks/repair-spec.md`
section 3 and "Rules for the R3 Windows route". Edit only the files named here.
Do not commit. Do not run the Full gate; run `cargo fmt`, then
`cargo clippy -p modeler3d-core -p modeler3d-app -p modeler3d-mcp --all-targets -- -D warnings`
and `cargo test -p modeler3d-core` (run `scripts/Initialize-BuildClang.ps1`
first in the same PowerShell session). No `unwrap`/`expect` in library code.

## 1. Health (`crates/modeler3d-core/src/mesh_ops.rs`, `inspect_mesh`)

- One shared degenerate rule: add `pub(crate) fn is_degenerate_triangle(a, b, c) -> bool`
  (cross-product squared <= 1e-24 or non-finite, today's `inspect_mesh` rule) and
  use it in `inspect_mesh` and in Repair. Delete Repair's own `tolerance * tolerance`
  threshold.
- Add to `MeshHealthReport`: `flipped_edges: usize` (shared edges with two owners whose
  directions agree) and `inside_out_shells: usize` (shells with negative signed volume
  when NO shell has positive volume, i.e. the whole model is inside out; only when
  winding is consistent). Existing `cavity_shells` logic stays as is.
- When `inside_out_shells > 0` push the issue `"{n} inside-out shells"` so
  `valid` is false and the app offers Repair. Do not change any other issue text.
  Do not add the new fields to MCP JSON output that has golden tests unless a test
  needs it; check `repair_report`/`normalize_report` in
  `crates/modeler3d-mcp/src/r4/server.rs` and keep existing keys unchanged.

## 2. Gate (`crates/modeler3d-core/src/repair.rs`)

- Add `pub struct RepairScore { open_edges, non_manifold_edges, flipped_edges,
  degenerate_faces, duplicate_faces, inside_out_shells, tiny_shells, shells }` built
  from a `MeshHealthReport` (`shells` = `body_count - cavity_shells`, clamped to >= 0).
  `RepairScore::from_health`, and `fn gate(before, after) -> RepairVerdict`:
  `Improved` only if no field got worse and at least one got strictly better;
  otherwise `NoBetter` (nothing better) or `Worse`.
- `RepairReport` gets `pub score_before`, `pub score_after`, `pub verdict`.
  `can_apply = candidate_safe && verdict == Improved`.
- Add `pub const REPAIR_REFUSED_MESSAGE: &str = "Repair couldn't improve this model. Nothing was changed.";`
  and `RepairReport::refused(&self) -> bool` (= `!nothing_to_fix() && !can_apply`).
  Export both from `lib.rs`.
- `nothing_to_fix()` stays `before.valid`.

## 3. Stop deleting slivers (`repair.rs`)

- `remove_degenerate_faces` and `prepare_for_export` drop only faces with a repeated
  vertex index (true zero area after welding). Drop the `tolerance` parameter.
  Keep `is_degenerate_triangle` for the report only.

## 4. `boundary_loops` (`repair.rs`)

- A vertex that starts two boundary edges must split into two loops, not abort.
  Collect all outgoing boundary edges per vertex (`BTreeMap<u32, Vec<u32>>`); when
  walking a loop, at a vertex with several outgoing edges choose the one that turns
  most sharply back toward the loop (smallest turning angle relative to the incoming
  edge, measured with the vertex positions; ties by lower index) so each simple hole
  becomes its own loop. Keep `collect_loops` for `component_boundary_loop`.
  `close_small_holes` still closes only 3..=4 edge loops (R5 does larger holes).

## 5. Adapters apply the gate

- `crates/modeler3d-app/src/desktop_jobs.rs` (~line 150): after the `nothing_to_fix`
  check, if `result.report.refused()` return
  `DesktopJobResult::Unchanged { message: REPAIR_REFUSED_MESSAGE }`.
- `crates/modeler3d-app/src/commands.rs` `repair_selected` (~1230): same, return the
  refused message with no scene mutation (no history entry).
- `crates/modeler3d-mcp/src/r4/server.rs` (~2387): same, return
  `{"action":..,"changed":false,"message":REPAIR_REFUSED_MESSAGE,"report":..}` with
  no history entry, shaped like the nothing-to-fix branch. `repair_report` JSON gets
  `"verdict"` and the before/after score only if that does not break a golden test.
- `mesh_ops.rs:698` calls `repair_mesh_with_options` internally; check that caller
  still works when the repair is refused (use the original mesh then).

## 6. Tests (generated in code, no binary files)

In `crates/modeler3d-core/tests/mesh_ops_repair.rs` (or a new `repair_gate.rs`):
- f10: sealed 10 mm box with a sealed sliver (point near an edge, sliver height 1e-5):
  after Repair the result is still watertight (0 boundary edges) or the repair is refused.
- book-like: a sealed sphere/grid mesh with ~50 sealed slivers: stays sealed.
- f3: sphere with a few single-triangle holes removed (non-adjacent, include two holes
  sharing a vertex): all single-triangle holes close.
- f9: sphere with all faces flipped: `inspect_mesh` is not valid and reports
  `inside_out_shells == 1`; Repair fixes it (outward volume, valid).
- A repair that would make things worse or no better returns `refused()` (use a
  mesh where the pipeline cannot help, e.g. a 6-edge hole, or f7 two cubes sharing an
  edge plus fin if it is refused) and `can_apply == false`.
- App test (`crates/modeler3d-app/src/commands.rs` tests): refused repair leaves
  `history_state_id` unchanged. MCP test next to
  `mcp_repair_of_a_clean_model_matches_the_desktop_and_changes_nothing`: refused repair
  returns `changed:false` with `REPAIR_REFUSED_MESSAGE` and unchanged history.
- Update existing tests that asserted the old behaviour; do not weaken them.

Report at the end: files changed, test counts, anything you could not do.
