# RB-06 handoff: complete modelling tools, mesh operations, and persistent brushes

Prepared 2026-11-01 for a fresh implementation session.

## Objective and stop boundary

Complete the entire **RB-06 — Modelling tools** slice in the existing
`E:\3D-Modeler` worktree. Finish the implementation, repair every broken
button and control discovered in the affected shell, run focused and full
tests, perform sequential paired Python/Rust native verification, update the
feature ledger and documentation, and create a verified additive recovery
snapshot on `G:`. Then stop before **RB-07**.

This is a full-slice assignment. Do not stop after proving that the core mesh
algorithms compile, after adding one primitive, after opening a dialog, after
one representative brush stroke, or after a test-only dispatch. A button is
not complete merely because it has a label, icon, handler, `CommandId`, or
unit test. It is complete only when a user can activate it in the native
application and observe the correct operation, state, history, error and
cancel behavior.

The previous RB-05 handoff is now historical. RB-05 implementation, automated
verification and native interaction passed; its visual review and owner
acceptance remain pending. Its immutable completed snapshot is
`G:\Builder3D-Rust_snapshot_20261101-064700_RB-05-complete`. Do not restore or
overwrite it. RB-06 starts from the current worktree and ends before RB-07;
do not reopen RB-05 except to repair a regression caused by RB-06.

## Mandatory rule: repair non-working functionality in all buttons

Treat this task as both feature completion and a button-functionality repair
pass. A fresh session must inventory and exercise every visible command button
in the current shell, not just the buttons that appear in the RB-06 feature
list. At minimum this includes the Insert, Object, Edit, Paint and View ribbon
commands, the file-menu Repair entry, favorites/overflow copies, Items-panel
health or context actions, dialog Apply/Cancel/close controls, and any button
that appears after an RB-06 command is activated. The complete feature contract
for a later slice may remain later-slice work, but a visible button may not be
dead, incorrectly disabled, routed to the wrong command, silently no-op, crash,
or claim success without producing its documented result.

For each visible button/control, verify all applicable cases:

- the control is visible, has the correct label/icon/state, and is enabled only
  when its preconditions are satisfied;
- a real native click reaches the intended semantic command and the same
  command is used from favorites, menus, Items actions and other duplicate
  entry points;
- dialogs open with the right object and current values, every field changes
  the operation, Apply commits, Cancel/Escape/close preserves the original
  state, and invalid values produce an honest error without scene loss;
- a successful model action changes the intended geometry or metadata, keeps
  stable identity/selection/appearance/transform as required, marks the
  document dirty, reports a useful status, and creates exactly one undo entry;
- Undo and Redo restore the exact before/after scene, including object count,
  mesh statistics, colors, transforms, metadata, selection and visibility;
- no-selection, wrong-selection, empty, hidden, grouped, multiple-selection,
  degenerate, invalid, disjoint, stale-revision, focus-loss, modal and cancel
  paths are safe for that control; and
- repeated activation, no-op input, switching tabs, closing a popover, and
  reopening the same workflow do not leave stale check states, stale dialogs,
  partial meshes, orphaned jobs or an inconsistent history.

If any check fails, repair the root cause in the appropriate semantic,
operation, shell or platform layer and add a regression test. Do not record a
known broken button as a harmless limitation while it is part of the active
RB-06 surface. Do not replace a missing behavior with a success message or a
center-point demo. If a control's full feature belongs to RB-07 or RB-08,
record that boundary honestly, but still make its basic reachability and
state/error behavior real and non-misleading.

## Authority and non-negotiable rules

Read these before implementation:

1. `E:\AGENTS.md` and `E:\RustDeveloper.md`.
2. The project [AGENTS.md](../AGENTS.md) and [WORK_LOG.md](../WORK_LOG.md).
3. The [product rebuild plan](product-rebuild-plan.md), especially its
   reference hierarchy, functional inventory, architecture rules, ordered
   slices, evidence requirements and exit gates.
4. The [RB-05 report](../evidence/rebuild/rb-05-viewport-and-input-completion/report.md),
   [RB-05 verification](../evidence/rebuild/rb-05-viewport-and-input-completion/verification.md),
   [RB-05 requirement map](../evidence/rebuild/rb-05-viewport-and-input-completion/requirement-test-map.md),
   [RB-05 walkthrough](../evidence/rebuild/rb-05-viewport-and-input-completion/walkthrough.md),
   and the prior [RB-05 handoff](rb-05-viewport-and-input-completion-handoff.md).
5. The [component catalog](component-catalog.toml), the
   [parity matrix](../evidence/rebuild/parity-matrix.toml), and
   [build prerequisites](building.md).
6. The read-only Python reference sources and tests named below.

The reference product is `L:\Builder3D`. Keep that entire tree read-only. It
is the behavioral and visual oracle and remains the shipping fallback until a
later stage gate replaces it. Do not edit, format, install into, regenerate
cache files in, or otherwise mutate the Python tree. Do not stage or commit
the Rust worktree. Do not modify `G:` or `H:` backups except to create the
new additive snapshot required by this handoff. Do not overwrite an existing
snapshot.

Keep the existing Cargo workspace and architecture. Geometry, scene state,
commands, history, project formats and operation scheduling remain independent
of egui/eframe, wgpu, Windows input and MCP. The desktop and MCP adapters must
use the same authoritative scene and semantic services. Physical input is for
native verification only; application behavior must not be implemented as a
macro of physical clicks.

Model values remain millimetres. Every model mutation is one command or one
grouped command. No-op actions do not create history. Long or cancellable
operations compute from an isolated snapshot and commit once only if the
document revision and source identity are still current. Errors and
cancellation preserve the source scene.

If the session changes an MCP tool, schema or protocol boundary, first read
`E:\MCP_DESIGN_GUIDE.md`, then the owning MCP/architecture/testing documents
and current official MCP sources. Ordinary RB-06 desktop work should reuse the
existing MCP surface and should not expand protocol scope.

## Current starting point and audit warnings

The current Rust surface is intentionally not proof of completion. Start with
read-only inspection of `ribbon_spec()` and `file_menu_spec()` in
`crates/modeler3d-app/src/commands.rs`, the shell dispatch in
`crates/modeler3d-app/src/shell.rs`, dialog actions in
`crates/modeler3d-app/src/dialogs.rs`, and the operation modules in
`crates/modeler3d-core`.

The current visible RB-06 inventory includes these semantic commands:

| Surface | Visible controls | Semantic command(s) | Required audit focus |
|---|---|---|---|
| Insert | Add/Open, Cube, Cylinder, Pyramid, Cone, Sphere, Torus, Hexagon, Wedge | `ImportModel`, `AddPrimitive(PrimitiveKind)` | Creation, reference dimensions, selection, color, identity, one history entry; Torus opens its parameter workflow. |
| Object | Edit Torus, Repair, plus duplicate entry points in the file menu and Items health row | `EditTorus`, `Repair` | Parametric-only Torus editing; health detection; safe repair; dialog and cancel behavior. |
| Edit | Simplify, Subdivide, Smooth | `Simplify`, `Subdivide`, `Smooth` | Actual selected-mesh mutation, bounded work, reports, cancellation, undo/redo. |
| Edit | Grab, Crease, Smooth Brush, Blob | `Grab`, `Crease`, `SmoothBrush`, `Blob` | Persistent checkable tools, remembered settings, real surface strokes, navigation/modifier precedence, one commit per stroke. |
| Edit | Normalize | `Normalize` | Selected-region density operation, boundary preservation, progress/cancel, before/after report, not a center-only dab. |
| Edit | Merge, Intersect, Subtract | `Merge`, `Intersect`, `Subtract` | Real boolean union/intersection/difference, exact operand rules, picker/cancel, result ownership and undo. |
| Edit | Emboss, Hollow | `Emboss`, `Hollow` | Working parameter controls, safe validation, geometry result, bounded failure and one undo. |
| Shared | Favorites, overflow, file menu Repair, Items-panel health/context actions, dialog Apply/Cancel/close buttons | Same semantic commands as above | Duplicate entry points must not diverge or silently no-op. |

The core `PrimitiveKind` enum also contains `Tetrahedron`, but it is not in
the current visible reference/Rust Insert ribbon. Do not invent a new visible
Tetrahedron button as part of RB-06; check that a core-only value does not
create an accidental UI or ledger gap.

There are two immediate audit flags in the current code. They are not a final
diagnosis, but they must be resolved by reference comparison and native
interaction:

- The current `CommandId::Merge` dispatch visibly routes through
  `scene.group_selected()` while the product contract calls for a boolean
  union. This is a likely non-working or wrong-function path and must not be
  accepted without proving the resulting solid geometry against the Python
  Merge workflow.
- The current direct command paths for the brush family and Normalize use a
  center sample through `apply_center_brush`. The reference requires persistent
  settings and real Shift-left surface strokes. A center sample is not proof of
  a working brush; repair the complete viewport tool route and test it with
  actual pointer movement.

Use the exact current source as evidence, not assumptions. If another button
has the same pattern—handler present but behavior is placeholder, wrong,
unreachable or disconnected—repair it and add it to the button audit.

## Reference-first investigation

Read the Python implementation before deciding what the Rust behavior should
be. Follow the actual handlers and services, not just labels or old notes.
Relevant starting points are:

- `L:\Builder3D\src\main_window.py` for `RIBBON_SPEC`, menu entries,
  primitive/torus handlers, dialog behavior, brush popovers, mesh-operation
  launch/commit paths and boolean picking;
- `L:\Builder3D\src\application_services.py` for isolated mesh snapshots,
  repair, simplify, subdivide, smooth, density and boolean service contracts;
- `L:\Builder3D\src\mesh_ops.py`, `repair.py`, `tools.py`, `scene.py` and
  `operation_manager.py` for geometry, reports, cancellation, history and
  commit rules;
- `L:\Builder3D\src\viewport.py`, `gizmo.py` and the relevant input helpers
  for surface picking, Shift-left brush gestures, camera-navigation
  precedence and tool cancellation; and
- `L:\Builder3D\docs\ui-style-guide.md`, `docs\project-format.md`,
  `docs\architecture.md`, `docs\testing.md`, `TODO.md`, `issues.md` and
  `COMPLETED.md` for accepted presentation and historical boundaries.

The Rust starting points are:

- `crates/modeler3d-app/src/commands.rs`, `shell.rs`, `dialogs.rs`,
  `operations.rs` and `ui_state.rs` for semantic dispatch, enabled-state
  logic, dialogs, status/errors, favorites and duplicate entry points;
- `crates/modeler3d-core/src/primitives.rs`, `scene.rs`, `mesh_ops.rs`,
  `repair.rs`, `simplify.rs`, `subdivide.rs`, `smooth.rs`, `brushes.rs`,
  `density.rs` and `shell_ops.rs` for portable geometry and history; and
- the matching integration tests under
  `crates/modeler3d-core/tests/`, plus the existing native harness patterns in
  `evidence/rebuild/rb-05-viewport-and-input-completion/`.

The old R3 mesh implementation is reusable evidence, not a new stage to
restart. Repair desktop reachability, contracts and parity gaps in the
current workspace. Do not create a second scene, a second command registry,
or a separate brush implementation used only by screenshots.

## RB-06 functional requirements

### 1. Primitives and Torus

Exercise every visible Insert primitive on a clean scene and compare the
result with the Python reference. For each of Cube, Cylinder, Pyramid, Cone,
Sphere, Hexagon and Wedge verify the reference dimensions/bounds, center or
apex rule, default authored color, origin placement, name, stable object ID,
automatic selection and exactly one creation history action. Verify that
adding another primitive clears the old selection according to the current
reference contract and that undo/redo restores the complete scene.

Torus is a parameterized workflow, not a generic primitive shortcut. Verify:

- insertion creates durable parametric metadata and opens the edit workflow;
- Solid/Hole mode, Radius, Tube, Sides and Steps are present where the
  reference presents them, with the reference defaults, units and bounded
  ranges;
- changing each control updates the live preview and metadata without
  corrupting the source object;
- invalid combinations are rejected without a partial mesh;
- Cancel, Escape and dialog close restore the exact original mesh and
  metadata; and
- Apply/OK commits the final mesh and metadata in exactly one undoable grouped
  update, preserves object identity/appearance/transform/selection, and
  survives save, reopen and export.

`Edit Torus` must work only for a selected parametric torus. A non-torus,
non-parametric mesh, empty selection, hidden object or stale selection must
receive a clear safe response and must not open a misleading editor or mutate
the scene.

### 2. Mesh health and Repair

Use the reference health checks and a permanent broken-mesh corpus. Cover
degenerate faces, duplicate faces/vertices, non-manifold edges, holes or
boundaries, inconsistent or reversed winding, disconnected/tiny shells,
self-intersections where the reference attempts them, flat/zero-volume input,
already-valid input and meshes too large for the bounded profile.

Health must be visible at the correct object/Items entry point and must lead to
an actual `Repair` workflow. Repair must:

- report the detected conditions and staged before/after counts;
- perform the safe best-effort stages that the reference contract promises;
- replace the source only when the candidate is safe and valid;
- preserve object ID, name, transform, color, visibility, group membership and
  selection on a successful replacement;
- use one grouped history entry for a successful replacement, no history for a
  no-op or rejected candidate, and exact Undo/Redo;
- support preview/report, cancellation and typed failure without losing the
  source scene; and
- keep the original mesh available when a candidate is unsafe, rather than
  claiming that a report alone was a repair.

Compare the known RB-04 fan-point difference and any other tessellation
difference honestly. Equal bounds or face counts do not by themselves prove
mesh-operation parity. State the geometric tolerance and report the remaining
intentional limitation only if it is genuinely outside the RB-06 contract.

### 3. Simplify, Subdivide, Smooth and density

For `Simplify`, `Subdivide` and `Smooth`, use meaningful meshes rather than a
single trivial box. Verify selected-target rules, hidden/grouped/multiple
selection behavior, mesh face/vertex counts, bounds and acceptable geometric
deviation, object state, authored color, transforms, stable identity,
selection, progress and status. `Subdivide` must meet the reference face-count
contract; `Smooth` must produce the reference geometry effect without
unexpected unbounded shrinkage; `Simplify` must stay within its bounded
minimum/maximum and quality rules.

For every operation cover valid success, no selection, empty/degenerate input,
already-minimal or no-op input, invalid parameters, cancellation, an
intervening document edit, a hidden or grouped target and a large but bounded
input. Compute from a snapshot, reject stale results, commit once and leave
the source unchanged on error or cancellation. Export and reopen at least one
result from each operation and verify that the saved geometry is the geometry
shown after the operation.

`Normalize` is a persistent density tool as well as a mesh operation. The
reference exposes a drawn region, region radius, target edge length (including
the automatic-from-boundary mode) and bounded pass/iteration controls. Verify
that the selected region changes, its boundary is preserved, density seams are
controlled, face counts and geometric deviation are reported, progress is
observable, cancellation is honored and only one final history entry is
created. A command that applies a default sample at the mesh center is not an
acceptable replacement for the region workflow.

### 4. Persistent brushes

`Grab`, `Crease`, `Smooth Brush` and `Blob` are persistent checkable tools.
Their settings popover remains available without reopening a modal dialog,
remembers edits while the tool remains active, and can be hidden/reopened
without losing the active tool. Verify the reference fields and defaults:

| Tool | Required setting families | Observable surface behavior |
|---|---|---|
| Grab | Radius, Strength, Falloff, Symmetry | Pulls a local region with bounded elastic falloff. |
| Crease | Radius, Strength, Direction, Falloff | Pinches a drawn path while preserving the supported topology/direction contract. |
| Smooth Brush | Radius, Strength, Shrinkage, Falloff | Smooths a drawn path with explicit volume/shrinkage behavior. |
| Blob | Radius, Strength, Direction, Falloff | Inflates or deflates a drawn path along surface normals within bounds. |

Use a fixture with enough surface resolution to make a local deformation
visible. With the tool active, perform an actual OS Shift-left click/dab and a
multi-point drag across the surface. Verify that ordinary navigation drags
remain camera navigation, that the modifier is required and respected, that
the picked surface is the intended frontmost object, and that a stroke affects
the local path rather than applying one fixed center operation.

For each brush verify preview/active feedback, pointer capture and release,
settings changes, boundary behavior, radius/strength/falloff/direction or
symmetry effects, cancellation with Escape or focus loss, modal precedence,
empty/wrong/hidden/grouped selection, a no-motion stroke, a valid stroke,
one undo entry per committed stroke, exact Undo/Redo and safe repeated use.
The source mesh, transform, color, identity and unrelated objects must remain
unchanged except for the intended local geometry. Deactivate the tool cleanly
and confirm that the check state and popover do not become stale.

### 5. Booleans

Implement and verify actual solid `Merge`/union, `Intersect`/intersection and
`Subtract`/difference behavior. Read the Python operand-selection and picker
rules directly; do not infer them from the Rust button order. Cover selected
operand order, viewport cutter picking, retry after picking the excluded
object, Escape cancellation, one-vs-two selection modes, hidden/grouped
objects, empty selection, duplicate operands, disjoint solids, touching or
coplanar faces, overlapping solids, non-manifold/self-intersecting inputs and
kernel failure.

On success verify result volume/mesh, target/tool retention or deletion,
selection, name/identity rule, transform and color/material provenance,
status, dirty state, one grouped history entry and exact Undo/Redo. On failure
or cancellation, preserve both source operands and their selection. The Rust
`Merge` button must not remain a grouping action or a success-looking shortcut
for boolean union. Any asynchronous boolean must protect against intervening
scene edits and must never commit a stale result.

### 6. Emboss and Hollow

Match the reference parameter and failure contracts. Emboss must support the
reference text/font/size/depth/raised-or-engraved/placement controls and must
produce visible, bounded surface relief at the expected location. Test empty
text, unsupported or invalid values, excessive depth, concave or too-small
targets, cancel, preview/error behavior, and the valid result. Preserve
identity, transform, color and selection and commit one undoable mesh change.

Hollow must accept bounded wall thickness, create a validated constant-
thickness shell where the reference supports it, and select/report the result
correctly. Test invalid or excessive thickness, open/flat/concave/degenerate
inputs, cancellation, bounded concave fallback, no-op and kernel failure.
Never replace the source with an invalid shell or claim success when only a
dialog opened. Preserve state on every failure and verify Undo/Redo and an
export/reopen round trip for a valid shell.

## Button-by-button audit procedure

Build a ledger before implementation and keep it through final evidence. The
ledger must include at least these columns:

| Field | What to record |
|---|---|
| Surface and label | Ribbon tab, file menu, favorite, Items row, context menu or dialog; exact label and icon. |
| Reference route | Python handler/test and expected enabled/checkable/modal state. |
| Rust route | `CommandId`, shell handler, dialog action and core/operation service. |
| Preconditions | Selection count/type, active tool, modal/focus state, valid geometry and any modifier. |
| Native action | Exact click, key, pointer path or dialog edit; target window, theme, fixture and build identity. |
| Result | Status, scene revision, object IDs, selection, mesh counts/bounds, colors/transforms/metadata and history delta. |
| Failure/cancel | Invalid, empty, cancel, Escape, focus loss, stale revision and error result. |
| Regression | Rust/Python test, evidence record and fix commit-free source location. |
| Status | Implementation, automated, native, visual and owner fields separately. |

Use `ribbon_spec()` and `file_menu_spec()` as the initial Rust inventory, but
also inspect the actual rendered widgets. Include hidden favorites, overflow,
Items-panel rows and buttons created dynamically by dialogs. Search for every
`ui.button`, `QPushButton`, `QToolButton`, `CommandId`, handler and dialog
action under the touched surfaces. Reconcile the Rust inventory against
Python `RIBBON_SPEC` and tests. A command present in source but absent from
the native surface is not complete; a button present in the native surface
without a working semantic route is a defect.

When a defect is found, first reproduce it on the smallest fixture, then trace
the semantic route to the root cause. Repair the narrowest correct layer,
add a regression test that asserts the user-visible result, and rerun the
native case. Do not hide a broken control by disabling it unless the Python
reference is also genuinely unavailable for the same precondition; use an
honest status/error and preserve the scene.

## Implementation and recovery sequence

Work in bounded subtasks. Before the first implementation subtask, create a
new timestamped G: baseline snapshot containing the source, manifests, tests,
documentation and Git metadata, excluding `target`, caches, logs and
reproducible generated output. Verify its copy result, path list, byte count,
SHA-256 comparison, reparse count and excluded paths. After every completed
subtask, write a checkpoint and create another additive snapshot before moving
on or compacting context.

A useful order is:

1. Inventory the current Python/Rust controls, read the reference contracts,
   create the baseline snapshot and write the initial RB-06 requirement/test
   map. Do not change behavior until the known dispatch gaps are recorded.
2. Complete primitive insertion, Torus metadata/parameter editing and all
   primitive button routes. Add focused unit and app tests and verify the
   basic native creation/dialog flows.
3. Complete health detection, Repair, Simplify, Subdivide, Smooth and
   Normalize with isolated operation snapshots, reports, cancellation and
   history. Build the broken-mesh and density fixtures.
4. Complete the persistent brush shell and viewport routes. Verify actual
   surface strokes, remembered settings, modifier precedence and safe cancel
   behavior with OS input.
5. Complete Merge/Intersect/Subtract and Emboss/Hollow. Audit source/tool
   retention, operand order, errors, dialog buttons and save/export behavior.
6. Run the full button audit, focused/full tests, sequential native evidence,
   documentation and final preservation checks. Update the ledger only from
   observed evidence, write the final checkpoint and create the verified
   completed RB-06 snapshot. Stop before RB-07.

Do not start the next subtask while the previous subtask lacks its checkpoint
and recovery snapshot. Checkpoints must state objective, changed files,
tests/checks, native/evidence results, snapshot path, unresolved risks and the
exact next subtask.

## Tests and integrated gates

Run the relevant Python tests read-only with the existing environment. Keep
Python bytecode and pytest cache out of `L:\Builder3D`:

```powershell
cd L:\Builder3D
$env:PYTHONDONTWRITEBYTECODE = "1"
L:\Builder3D\venv\Scripts\python.exe -B -m pytest -q -p no:cacheprovider tests/test_mesh_ops.py tests/test_desktop_actions.py tests/test_ai_commands.py tests/test_adapter_boundaries.py
```

At minimum, exercise the named reference cases already recorded in the parity
matrix: primitive dimensions/centering/apex/volume, the parametric Torus
contract, `test_desktop_repair_applies_safe_best_effort_and_undo`,
`test_remaining_core_mesh_actions_change_the_scene`, the boolean workflow and
picker cases, `test_ai_simplify_and_smooth_preserve_object_state_and_undo`,
`test_ai_subdivide_doubles_faces_and_is_undoable`, the AI Emboss/Hollow
identity/undo cases, the brush settings-popover cases, the brush topology/
bounds cases, and the Normalize progress/cancellation case. Run the full
read-only relevant Python suite if the focused list does not cover a changed
route. Never change a Python test or source to make it pass.

Add Rust regressions for every repaired button and every discovered routing
defect. Run focused tests in the owning crates, including the mesh operation
integration tests listed by `docs/component-catalog.toml`:

```text
crates/modeler3d-core/tests/mesh_ops_booleans.rs
crates/modeler3d-core/tests/mesh_ops_repair.rs
crates/modeler3d-core/tests/mesh_ops_simplify.rs
crates/modeler3d-core/tests/mesh_ops_subdivide.rs
crates/modeler3d-core/tests/mesh_ops_smooth.rs
crates/modeler3d-core/tests/mesh_ops_brushes.rs
crates/modeler3d-core/tests/mesh_ops_density.rs
crates/modeler3d-core/tests/mesh_ops_shell.rs
```

Use assertions about user outcomes: geometry and report values, selection,
identity, history length, cancellation, errors, dialog state and stale-result
rejection. A test that only checks that a handler returns `Ok` is insufficient.

Then run the locked integrated gates from a fresh PowerShell session or record
the exact environment used:

```powershell
cd E:\3D-Modeler
. .\scripts\Initialize-BuildClang.ps1
cargo fmt --all
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all -- --check
cargo deny check
cargo metadata --locked --format-version 1 --no-deps
cargo build --locked -p modeler3d-app --bin modeler3d-app
cargo build --locked -p modeler3d-app --bin modeler3d-app --release
cargo bloat --release --bin modeler3d-app -n 20
```

Run debug and release `--smoke` checks with isolated writable application
profiles, then close both targets and confirm no target process remains. If a
tool is unavailable, record it as unavailable and do not silently call the
gate passed. `cargo audit` may be reported separately if it is not installed;
that does not replace the required `cargo deny check`.

Do not accept `unwrap()`, `expect()`, `todo!()` or `dbg!()` in accepted
application behavior. Keep public Rust seams typed and documented, keep
library code panic-free across crate boundaries, and remove temporary
diagnostics before final verification.

## Native fixtures and paired evidence

Create a dedicated evidence tree at
`evidence/rebuild/rb-06-modelling-tools-and-brushes/`. Keep fixtures and
metadata there or in another explicitly documented project-local location.
Do not overwrite RB-05 evidence.

The fixture manifest must include, with stable names and hashes:

- an empty scene and clean Cube/Cylinder/Pyramid/Cone/Sphere/Hexagon/Wedge
  primitives;
- several Torus parameter combinations, including Solid/Hole and boundary
  values that remain valid;
- valid, already-repaired, degenerate, duplicate, open/hole, non-manifold,
  reversed-winding, disconnected/tiny-shell, flat/zero-volume and bounded
  self-intersecting meshes;
- low-density and dense meshes for Simplify/Subdivide/Smooth/Normalize and
  brush strokes;
- single, multiple, grouped, hidden and mixed-visible selections;
- overlapping, disjoint, touching/coplanar and invalid boolean operands; and
- a surface fixture with a clearly measurable brush path away from its center.

Run the Python reference first, then close it. Run one Rust target at a time,
then close it. Use actual OS clicks, key presses, pointer drags, Shift
modifiers, dialog edits and Escape/cancel actions for native claims. Do not
use a background MCP gateway, semantic dispatch injection or a screenshot
alone as proof of desktop interaction.

Capture both Light and Dark at a 1280x840 logical client area, 96 DPI / 100%
scale, with matching fixture, camera, selection and operation state. Record
the actual client/physical dimensions, Windows build, display scale, theme,
fixture hash, source revision, executable path/hash and process identity. The
desktop guard must verify the correct foreground window, target PID and
unlocked input surface before sending input. Never send automation input to a
locked, secure, obscured or unrelated window.

For each operation and button, capture the relevant sequence rather than only
the final resting image:

1. clean before state and the visible button/dialog/tool state;
2. opening or activation, including enabled/checked/focus feedback;
3. parameter edits and live preview where applicable;
4. during/progress/stroke state where the reference exposes it;
5. successful after state with status, selection and geometry evidence;
6. Undo and Redo states;
7. invalid/error, Escape/cancel, focus-loss and stale-result states where
   applicable; and
8. export/reopen or save/reopen state for at least one valid result in each
   operation family.

Every accepted record needs a sidecar containing semantic state, not only a
PNG: object IDs/names, count, selection, visibility/group membership,
revision/history IDs, mesh counts/bounds/volume where available, color,
transform, metadata, status text, active tool/settings, operation progress,
cancel result, image SHA-256 and executable SHA-256. Inspect every accepted
frame. Reject blank, stale, partial-paint, wrong-window, obscured, lock-screen,
wrong-theme, wrong-fixture and duplicate frames. Keep rejected attempts and
explain why they were rejected.

The paired board puts Python on the left and Rust on the right, with one row
per equivalent operation/state. State whether each comparison is directly
observed or an inference from source/test evidence. Do not turn an agent's
visual inspection into owner acceptance; keep implementation, automated,
native, visual-review and owner-acceptance statuses separate.

## Required evidence and documentation deliverables

The final `evidence/rebuild/rb-06-modelling-tools-and-brushes/` directory must
contain, as applicable:

- `report.md` with scope, implementation summary, button audit outcome,
  known limits, status table and exact stop boundary;
- `checkpoint.md` with the final recovery state and next slice `RB-07`;
- `verification.md` with commands, versions, exit codes, test counts, build
  identities, smoke results and preservation checks;
- `requirement-test-map.md` mapping every RB-06 requirement and visible button
  to Python source/tests, Rust symbols/tests, native records and captures;
- `walkthrough.md` describing the sequential Python-then-Rust native replay;
- `paired-comparison-board.md` or an equivalent inspectable HTML board with
  Python-left/Rust-right pairs;
- `capture-metadata.toml` or JSON with dimensions, DPI, theme, fixture,
  operation, input method, status and hashes;
- fixture/operation manifests, accepted and rejected native journals, and
  source-preservation/hash records; and
- any focused mesh-health, boolean, density or brush reports needed to make
  the result independently auditable.

Update `evidence/rebuild/parity-matrix.toml` for every RB-06 row. Update
`docs/component-catalog.toml` only when the named component contract and its
evidence justify the status. Add the final work and snapshot receipt to
`WORK_LOG.md`. Preserve RB-03, RB-04 and RB-05 visual/owner-pending states;
do not promote them from silence or from successful tests. Use separate
fields for implementation, automated, native/interactive, visual and owner
acceptance. The final RB-06 records must point to `RB-07` and must not claim
RB-07 work has started.

## Final snapshot and exit checklist

After the final code, tests, native evidence and documentation are complete,
create a new additive snapshot such as
`G:\Builder3D-Rust_snapshot_YYYYMMDD-HHMMSS_RB-06-complete`. Exclude
`target`, caches, logs and reparse entries unless a specific evidence file
requires them. Verify the exact source/snapshot file list, file count, byte
count, path differences, size differences, SHA-256 differences and reparse
counts. Confirm the final snapshot contains the handoff/evidence/docs and
does not contain an unintended Python edit, secret, build output or stale
temporary capture. Write the receipt after the copy and run a final read-only
post-receipt comparison; do not edit source after the receipt without making
the receipt stale and repeating it.

Before stopping, all of these must be true:

- every visible RB-06 button and every affected duplicate entry point has been
  exercised natively and either works or has been repaired;
- all primitives, Torus editing, health/Repair, Simplify, Subdivide, Smooth,
  Normalize, all four persistent brushes, three booleans, Emboss and Hollow
  meet their reference contracts and safe error/cancel rules;
- every repaired non-working button has a regression test and evidence record;
- focused Python/Rust tests and all applicable locked workspace gates have
  recorded results with no unreported failures;
- paired Python-left/Rust-right Light/Dark native records use actual OS input,
  matching fixtures and verified executable/image identities;
- the parity matrix, component catalog where justified, work log, report,
  verification, walkthrough, comparison board and checkpoint agree;
- the final additive G: snapshot and exact receipt are verified; and
- the session has stopped before RB-07.

Do not begin appearance/material/image workflows, background jobs, print or
slicer completion (RB-07), preference/shell/accessibility completion (RB-08),
desktop/MCP live consistency (RB-09), performance/package/host acceptance
(RB-10), or final owner acceptance (RB-11). Only fix a later-slice surface
when it is necessary to remove a broken shared button route, and document the
boundary rather than silently expanding the slice.

## Short activation prompt

Read and follow `E:\3D-Modeler\docs\rb-06-modelling-tools-and-brush-completion-handoff.md`. Complete all RB-06 work, repair every non-working button, run tests and paired native evidence, update docs, make a verified additive G: snapshot, and stop before RB-07. Keep Python read-only and do not stage or commit.
