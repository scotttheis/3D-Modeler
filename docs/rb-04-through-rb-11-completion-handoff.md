# 3D Modeler Rust continuation handoff: RB-04 through project completion

> Current continuation (2026-11-02): [complete RB-09 Shared AI and Regression](rb-09-shared-ai-and-regression-handoff.md), then stop before RB-10. The RB-04 assignment below is historical; the product plan and current RB-09 handoff own execution.

> Execution update: RB-04 implementation, automated gates and sequential Light/Dark
> native evidence are complete. Visual/owner acceptance remains pending; all32
> RB-03 status rows are preserved. See the current
> [RB-04 report](../evidence/rebuild/rb-04-scene-and-precise-transforms/report.md)
> and [checkpoint/snapshot receipt](../evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md).
> This task stopped before RB-05. The prepared starting state below is historical;
> retain the ordered RB-05 through RB-11 scope for the next task.

Prepared 2026-10-31 for a fresh implementation session.

## Immediate objective

Complete the entire **RB-04 — Scene and precise transforms** slice in the
existing `E:\3D-Modeler` workspace. RB-04 is the next full slice in the
current product rebuild plan. After RB-04 and its evidence, verification, and
recovery snapshot are complete, stop before implementing RB-05. The rest of
the roadmap is included below so the project can continue to RB-11 without
restarting the audit or treating RB-04 as product completion.

The owner has asked that work proceed without extra approval stops. Make
routine design and implementation choices from the documented behavior and
tests, and keep moving through RB-04. Preserve actual owner-review gates in the
project plan; do not invent additional approval gates or mark pending review as
accepted.

## Current state

- Rust workspace: `E:\3D-Modeler`; Python product/reference:
  `L:\Builder3D`. Keep the Python tree read-only. The Python application remains
  the temporary shipping fallback until the project replacement gate passes.
- RB-03 lifecycle, import/export, units, recents, and previews have their
  implementation and engineering record. Its checkpoint and results are in
  `evidence/rebuild/rb-03-lifecycle-and-file-completion/`.
- RB-03 ledger: implementation and automated status pass on all 32 rows;
  native interaction passes on 30 and remains partial on two; visual review and
  owner acceptance remain pending. The paired board has 26 old-Python/Rust
  action pairs, Python on the left, with recorded appearance differences.
  Windows locked before a second Python export-picker replay could cover each
  format. Do not represent that replay or the pending owner review as complete.
- The additive RB-03 review snapshot is
  `G:\Builder3D-Rust_snapshot_20261031-165222_RB-03-review`. Its pre-receipt
  tree had 901 files / 139,803,911 bytes and zero path, size, or SHA-256
  differences. The final post-receipt comparison found only `WORK_LOG.md` and
  the RB-03 checkpoint changed after the copy. Use the working tree; do not
  restore or overwrite it from a snapshot.
- No RB-04 implementation has started. The parity matrix currently assigns 24
  stable requirement rows to RB-04 with implementation partial and automated,
  interactive, visual, and owner states pending. Reconcile those rows with the
  complete RB-04 scope below; add a stable row only for a real uncovered
  contract.
- The Rust repository currently shows its files as untracked. Preserve the
  existing tree; do not reset, stage, or commit.

## Plan authority and scope

Use `docs/product-rebuild-plan.md` as the current product acceptance order:
RB-00 through RB-11. The current task is RB-04, followed by RB-05 through RB-11
in that order. The old R0-R8 technical work is historical conversion evidence;
the work log and `evidence/r3-final/` record the completed R3 mesh-operation
stage. Do not restart R3 or treat its completion as completion of the desktop
product. Likewise, do not jump to the legacy R4 MCP label in place of the
current RB-04 slice.

The product plan's RB-04 exit evidence is: live grouped and hidden-object
cases, stable object IDs, exact transform values, and one history entry per
action or completed drag. Its required behavior includes:

- Items rows and thumbnails, selection and sticky selection, visibility,
  naming, duplicate/delete, grouping and ungrouping, and clipboard actions.
- Move, Rotate, and Resize through the existing numeric and viewport paths;
  preserve mm as the internal unit, inch display conversion, degree rotations,
  resize basis and proportional-lock behavior.
- Settle, mirror, measure, align, and arrange with world-space bounds and the
  reference's axis, landmark, ordering, and history semantics.
- Correct behavior with groups, hidden objects, mixed selection, text fields
  with focus, undo/redo, and stable identity. Selection-only actions must not
  create geometry history entries; each committed model action or completed
  drag must create one appropriate history entry.

The existing RB-04 ledger includes duplicate, copy, cut, paste, delete,
group/ungroup, row selection, select/deselect/invert, mesh-health/Repair
reachability, visibility, settle, mirror, measure, align, arrange, numeric
rotation/resize, units, resize basis, proportional lock, and Delete shortcut
focus behavior. Verify the stable IDs and Python contract before implementation.
The product plan also calls out naming, sticky selection, Items thumbnails, and
Move; make sure each is represented by an existing ledger row or add a focused
row rather than letting it disappear between the plan and tests.

## How to resume

1. From `E:\3D-Modeler`, read `E:\AGENTS.md`, `E:\RustDeveloper.md`,
   `AGENTS.md`, `WORK_LOG.md`, this handoff, and the RB-03 checkpoint. Check
   `git status`; preserve the current untracked files.
2. Read `docs/product-rebuild-plan.md` sections 4, 6, 8, 9, 10, and 11;
   `docs/component-catalog.toml`; `evidence/rebuild/parity-matrix.toml`; the
   RB-02 checkpoint; and the RB-03 report, checkpoint, walkthrough, and
   visual-comparison board. Use the current product plan, not stale status
   text in older R-stage documents, for slice order.
3. Trace each RB-04 action in the read-only Python source and its focused tests
   before editing. Relevant starting points include `L:\Builder3D\src\main_window.py`,
   `model_actions.py`, `application_services.py`, `mesh_ops.py`, the object
   panel and viewport handlers, and focused tests for desktop actions,
   transforms, grouping, selection, and keyboard focus. Follow references to
   their authoritative implementations rather than guessing from names.
4. Inspect the existing Rust command, scene/history, app shell, Items panel,
   viewport input, transform controls, and tests. Keep scene mutations in the
   command/core path; widgets should dispatch semantic commands.
5. Update or create the focused requirement-to-test map, then implement and
   verify all RB-04 behavior. Work autonomously through routine choices; do not
   stop for approval after a small patch or at the end of an individual
   control.

## Required reference-first evidence

The owner specifically directed that comparison focus on what the old
application looked like while each operation was performed. For every relevant
RB-04 action:

- Operate the Python reference and Rust application sequentially, never with
  both target windows open together. Capture the old action and its result,
  then the corresponding Rust action and result using the same scene state
  where practical.
- Capture the affected surfaces in Light and Dark at the plan's 1280 x 840
  logical-pixel size. Record actual client size, DPI/scale, theme, fixture,
  operation, and build identity. Put Python on the left in the comparison
  board and annotate visible layout, dialog, selection, transform, and result
  differences.
- Include actual grouped and hidden-object actions, keyboard/text-focus cases,
  exact numeric transforms, selection modifiers, undo/redo, and canceled or
  invalid operations. Do not substitute a source inspection or a test result
  for a live UI capture where the plan requires visible evidence.
- Keep `L:\Builder3D` unchanged. Never send input to a locked/obscured desktop;
  if the desktop is unavailable, continue independent code and test work and
  record only the specific live evidence that could not be collected.

## RB-04 implementation and exit work

Follow the existing code architecture and the contracts in the Python
reference. Cover both the core state transition and the visible route for each
action. At minimum, verify:

- Selection: row click, Ctrl/Command toggle, Shift range from the anchor,
  select all, deselect all, invert, sticky selection, empty-space clicks, and
  selection of grouped objects. Selection changes do not dirty geometry or
  add history.
- Objects and clipboard: duplicate produces independent identities and the
  reference world-space offset; copy leaves the scene untouched; cut is one
  undoable removal; paste creates durable new IDs and remains available after
  selection changes; delete is one undoable action and does not steal typing
  focus. Group/ungroup preserves child meshes, transforms, metadata, identity
  rules, and selection behavior.
- Visibility and Items: hidden objects remain represented correctly in rows,
  selection, exports, and print-related paths; thumbnails and health warnings
  correspond to the actual object. Inspect the existing visibility-gap row
  and resolve the recorded Show/Eye-control decision from the reference and
  current owner direction; do not silently change Python behavior.
- Transforms: exact world-space Move values; Rotate Roll/Pitch/Yaw in degrees
  without changing scale; Resize in mm/in with the correct conversion, basis,
  alignment, and proportional ratio. Numeric editing and viewport drags share
  the semantic command path and commit once at the right boundary.
- Other operations: settle to the build plane from world bounds; mirror using
  the reference axis rule; measure selected world dimensions; align by the
  reference axis/landmark; arrange in deterministic order with the configured
  gap. Each operation preserves unaffected object state and undoes/redoes as
  one grouped change.
- Failure/no-op paths: invalid values, canceled drags, empty selections,
  hidden or grouped objects, and resource limits leave the scene and history
  in the documented state. Avoid introducing a new behavior merely because it
  is convenient to implement.

Add focused Rust tests for observable scene/identity/transform/history results
and relevant native input paths. Run the matching focused Python reference
tests read-only. At the completed RB-04 boundary run the applicable integrated
workspace gates from the product plan, record exact results, validate the
parity/evidence links, and create one additive verified G: recovery snapshot
for the completed slice as required by the existing workspace rules. Do not
add process overhead beyond those existing project requirements.

## Evidence and handoff updates

At RB-04 completion:

- Update the existing RB-04 parity rows with separate implementation,
  automated, interactive, visual, and owner-review states. Keep uncovered or
  unavailable work visible; never promote a status from a planned action.
- Add an RB-04 report, verification record, walkthrough, comparison board,
  capture metadata, and concise checkpoint under
  `evidence/rebuild/rb-04-scene-and-precise-transforms/`.
- Update `WORK_LOG.md` with the concrete changes, focused and integrated
  checks, Python reference results, visual observations, limits, and verified
  snapshot path. Update the checkpoint with the exact RB-05 next action.
- Make a new timestamped, additive G: snapshot. Never overwrite or modify an
  existing snapshot. Verify counts, bytes, relative paths, sizes, and SHA-256;
  exclude repository-root `target/`, `.cache/`, and reparse entries.
- Do not stage or commit. Do not claim RB-04 visual acceptance or overall
  product completion. Stop after the RB-04 evidence and snapshot; do not begin
  RB-05 in this handoff's execution.

## Remaining completion route

RB-04 is one full slice, not the product finish line. Continue later in the
current plan's order:

| Slice | Remaining scope |
|---|---|
| RB-05 | Complete viewport gestures, presets, gizmos, snapping, floor/grid, lighting, colors, wireframe/X-Ray, and resize interaction. |
| RB-06 | Complete primitives/torus, repair/simplify/subdivide/smooth, booleans, emboss/hollow, brushes, and density workflows. Reuse completed core R3 work; verify desktop reachability and parity. |
| RB-07 | Complete appearance/material/image workflows, background jobs, print checks, slicer staging, progress, failure, and cancellation. |
| RB-08 | Complete preferences and shell: favorites/order/pin/reset, menus/dialogs/settings migration, shortcuts/accessibility, and welcome fallbacks. |
| RB-09 | Verify desktop and MCP operate on the same live document, with revisions, replay, file/cancel behavior, and optional-MCP reliability. |
| RB-10 | Measure representative workloads and resource routing; verify memory/cache lifetime, clean install/launch, package size, and host coverage. |
| RB-11 | Fix remaining parity gaps, complete the full live walkthrough and matched visual review, run final gates, obtain explicit owner acceptance, and make the verified final recovery snapshot. |

Only RB-11 can establish final product acceptance. The current RB plan
supersedes the older R0-R8 conversion order for product acceptance; completed
R-stage code remains reusable evidence and must not be repeated without a
specific gap.

## Fresh-session prompt

Read and follow `E:\3D-Modeler\docs\rb-04-through-rb-11-completion-handoff.md`.
Complete the full RB-04 slice from `docs/product-rebuild-plan.md`: Items,
selection/sticky selection, visibility, naming, duplicate/delete, grouping,
clipboard, precise Move/Rotate/Resize, settle, mirror, measure, align, and
arrange. Continue autonomously through the slice; do not ask for routine
confirmation or stop after representative controls. Compare the Python app at
`L:\Builder3D` with Rust while performing each action, using paired captures
and recording visible differences. Keep Python read-only, preserve the
existing untracked Rust tree, follow the required tests/evidence/snapshot
workflow, and do not stage or commit. RB-03 visual/owner review and two partial
native rows remain pending; do not claim them accepted, but do not let that
status block RB-04 work. Finish RB-04, write its report/checkpoint, verify the
additive G: snapshot, then stop before RB-05 and report the remaining roadmap
through RB-11.
