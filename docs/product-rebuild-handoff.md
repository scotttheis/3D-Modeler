# Historical handoff: RB-00 reference and reuse audit

Prepared: 2026-10-30. **Superseded for current execution:** RB-00 is complete.
This file preserves the audit-era scope and findings; it is not the current
next-session handoff. The current task is RB-02 under
[`rb-02-first-working-application-handoff.md`](rb-02-first-working-application-handoff.md),
as authorized by the owner on 2026-10-31. The complete scope and stage gates
remain in [`product-rebuild-plan.md`](product-rebuild-plan.md).

## Historical RB-00 task (completed)

RB-00 established a map of the original 3D Modeler's appearance and behavior,
recorded which Rust code can be reused, and identified implementation gaps.
The following requirements describe that completed audit and are retained as
historical context, not as the current session's task.

RB-00 was complete when the original app's user-visible actions had individual
ledger entries, important dated design choices were reconciled with current
source, valid reference captures or explicit unknowns were recorded, owner
artwork had a provenance manifest, Rust reuse decisions had source/test
anchors, and the report proposed a small first visual and working prototype.
If Windows blocked a real reference capture, the audit recorded what was
unavailable instead of substituting a blank or obscured screenshot.

## Read in this order

First read the complete current files, not just this handoff:

1. `E:\AGENTS.md` and `E:\RustDeveloper.md`.
2. `E:\3D-Modeler\AGENTS.md`.
3. This handoff and `E:\3D-Modeler\docs\product-rebuild-plan.md`.
4. `L:\Builder3D\AGENTS.md` and only the relevant original product material:
   `docs/ui-style-guide.md`, `docs/rust-rewrite.md`, `docs/architecture.md`,
   `docs/project-format.md`, `docs/ai-mcp.md`, `docs/testing.md`,
   `docs/mcp-modern-design.md`, `TODO.md`'s current task and work order,
   `issues.md`, `COMPLETED.md`, `src/icon_assets.py`, and the relevant source
   and test files listed below.
5. `E:\3D-Modeler\WORK_LOG.md`'s latest plan-reset entry and
   `evidence/rebuild/00-plan-reset/`.
6. The Rust catalog entries, implementations, tests and R8 records listed in
   “Known current state” below. Treat R8 completion language as superseded.

The shared Rust guide still applies. The original project and every existing
`G:\` snapshot are read-only. Keep the work in `E:\3D-Modeler`; do not
modify Python, reset or clean the untracked worktree, commit, or open a pull
request. Do not redo R1-R7 conversion work. Do not use the retired R8 brief as
an execution plan. Do not run a broad Python desktop suite under its current
Windows containment. Read relevant tests and existing results to map contracts;
run no test suite merely to inventory it. Follow all product and file safety
rules if opening the reference app to observe it.

## Known current state

- Current execution authority: `docs/product-rebuild-plan.md`, slices RB-00 to
  RB-11. The prior R8 brief is retired and the R8 product acceptance claim was
  withdrawn.
- The Rust workspace is the project under repair. Its working files were
  untracked in the last observed Git status; preserve every file as existing
  owner work. Inspect `git status --short` again before any action.
- The original Python application at `L:\Builder3D` is the behavioral and
  visual reference and temporary fallback. Do not write to its source, tests,
  assets, settings or project files.
- A fresh pre-handoff recovery point was verified:
  `G:\Builder3D-Rust_snapshot_20261030-134045_RB-00-handoff-baseline`.
  It contains 447 files / 50,518,780 bytes, with all 447 SHA-256 hashes
  matching. `target` and `.cache` were excluded; junction traversal was
  disabled. Recheck status and make a fresh additive baseline before RB-00
  changes, following the current snapshot rules.
- Catalog: 30 IDs. Core/file/mesh/MCP/renderer contracts retain their scoped
  historical tests; `modeler3d.app.shell` and
  `modeler3d.app.desktop-adapters` are `polish`, and
  `modeler3d.app.commands` is `ported`. Do not promote them from inventory or
  smoke results.
- `crates/modeler3d-app/src/shell.rs` paints rectangle previews and stores
  rectangle hit targets. `modeler3d-viewport` has a separate R5 renderer.
  Their actual integration is open work.
- Original owner art exists: eight Insert images; 44 action images in each
  theme; root `Modeler3D.png`, `Modeler3D.ico`, `pin.jpg`, and `snap.png`.
  Verify the exact current manifest and mapping. Do not mistake Microsoft
  reference-app artwork for 3D Modeler's own assets.
- `evidence/r8/00-baseline/python-reference.png` is a blank client area. It is
  not an acceptable appearance baseline. Inspect every capture before relying
  on it. Historical R8 tests and logs describe what they actually exercised;
  they do not prove UI or workflow acceptance.
- Original product goals still include shared GUI/MCP scene state, file/unit/
  undo compatibility, resource-aware performance, Windows/Linux/macOS support,
  40 MB installed and 15 MB compressed size targets, and the measured Quadro
  viewport target. RB-00 records them as requirements; it does not retest or
  declare them passed.

## RB-00 work sequence

### 1. Record the starting state

Inspect `git status --short`, relevant running 3D Modeler processes and the
current working files. Preserve existing changes and sessions. Read the latest
verified recovery receipt. Before any RB-00 file changes, create a new additive
`G:\` snapshot with source, tests, fixtures, artwork, manifests, catalog,
documentation, evidence, work log and Git metadata. Exclude only generated
outputs that the instructions allow; verify copied relative paths, file and
byte counts, and recovery-critical hashes. Do not alter any previous snapshot.

Keep the original checkout read-only. If a real Python app inspection is
appropriate, use it only as an oracle: prefer a temporary known fixture, do not
save or overwrite a source project, and do not change persistent preferences.
Avoid disturbing an already-running owner session. If reliable visual
inspection is unavailable, finish independent source/test/asset inventory and
mark live captures pending. Never fake completion with synthetic app input or
a blank capture.

### 2. Enumerate the product from its real entry points

Inspect the Python source and tests, not just labels or TODO prose. Build an
inventory from `MainWindow` construction, ribbon/menu specifications and
handlers, `_build_right_panel`, `_register_shortcuts`, dialogs, `TransformBar`,
viewport event handlers, gizmo/snapping code, preferences, operation feedback,
print handoff and accessibility labels. Follow each user action into
`ApplicationServices` or its owning service and note the actual result,
preconditions, errors, state/history effects, and known limitations.

Cross-check every tab command and utility, context-menu action, welcome/recent
card, file dialog choice, transform mode, viewport gesture, persistent tool,
setting and keyboard route. Compare the public 60-tool MCP catalog and its
project lifecycle/view/print behavior with the same services. The catalog's
41-module and 19-script coverage counts are file-conversion inventory, not a
feature checklist. Split actions with different user outcomes into separate
stable requirement IDs; do not use one broad “ribbon works” row.

Read the closest focused tests to discover expected details. Useful Python
targets include `test_adapter_boundaries.py`, `test_desktop_actions.py`,
`test_transform_bar.py`, `test_live_viewport_input.py`,
`test_core_workflow.py`, `test_project_files.py`, `test_project_format.py`,
`test_icon_assets.py`, the mesh-operation tests, and the focused MCP tests.
Record test names and assertions; do not claim an assertion proves more than
it covers. Inventory Rust unit/contract coverage from the catalog and source.

### 3. Reconcile historical design and behavior decisions

Use `docs/ui-style-guide.md` as the present visual contract. Reconcile it with
current source and dated records in `TODO.md`, `issues.md`, `COMPLETED.md`,
`WORK_LOG.md`, focused tests and screenshot references. Specifically record the
accepted 2026-10-02 floor treatment, orbit/pan gestures and Shift-left editing,
snapping bypass, resize basis/anchoring, transform strip units, persistent
brush popovers, icon mappings/transparency/theme swaps, high-poly display tiers,
and any later owner correction.

For each disagreement record the date/source, what current code does, whether
the decision remains accepted, and the evidence. Distinguish owner-approved
behavior from AI screenshot captions, tentative ideas, known defects and
unverified old claims. Do not revive wishlist items or Microsoft's appearance
as requirements. Leave unresolved material choices as questions with the
evidence needed to answer them.

### 4. Establish a trustworthy appearance baseline

Inventory available screenshots before use. For each, record full path,
origin/date if known, actual visible app/window, theme, scene, size, scale and
limitations. Open it and confirm it is the original product, fully painted,
unobscured and relevant. Historical captures may be useful but do not relabel
them as live verification.

When live reference inspection is available and safe, capture the original in
both themes at the same logical client size intended for the Rust comparison.
Cover welcome with and without recents; each Insert/Object/Edit/Paint/View tab;
menu and settings; empty, single, multiple, grouped and hidden objects; Move,
Rotate and Resize strips/gizmos; import unit choice; a brush-settings popover;
and representative progress, error and print-prep states. Record per-image
state, dimensions, theme and scale. Use known fixtures or temporary models and
do not save changes back into `L:\Builder3D`.

If a state cannot be reached or the desktop is intercepted, capture the reason
and leave that reference requirement pending. Note expected review sizes and
scales from the style guide. The full matched scale review belongs to later
implementation gates; RB-00 must supply a trustworthy representative
reference set for RB-01/RB-02.

### 5. Record the supplied art and branding

Read `src/icon_assets.py` and the loader/style code. Enumerate each file from
the four theme icon directories and the four root assets. For every file
record: stable logical ID, exact original path, theme, command/family mapping,
dimensions, pixel format, alpha/outer-edge condition, byte length, SHA-256,
and whether the live reference actually uses it. Include intentional generated
icons and missing dedicated artwork as such; do not invent files to fill gaps.

The future Rust app must package its own approved copy and have no runtime
dependency on `L:\Builder3D`. RB-00 only records provenance and mapping; do not
copy, resize, retouch or regenerate art in this inventory slice.

### 6. Decide what Rust can reuse

For each requirement, identify exact Rust catalog ID, file/symbol, focused
test, UI route (if any), and current observed limitation. Mark the recommendation
`reuse`, `repair`, `replace`, or `unknown`, with a short evidence-based reason
and estimated integration risk. These are RB-00 recommendations, not approval
to delete or refactor code.

Use these preliminary hypotheses to guide inspection, not as predetermined
results:

- Preserve proven core geometry, file readers/writers, history, operations,
  typed commands, MCP contracts, resource planning, camera and renderer.
- Keep eframe as the first UI candidate. Test its ability to present the
  original art and layout and host the existing renderer before proposing a
  toolkit change.
- Inspect `ViewportRenderer` and native eframe/wgpu context ownership. The
  backend is not reusable by the desktop until actual shaded geometry, camera
  orbit, picking and edit feedback work in the same app window.
- Retire the rectangle preview/hit target from any accepted product path once
  an integrated renderer replaces it; preserve it only if explicitly useful
  for non-product tests.
- Inspect the 1,900-line-class shell for focused component repair seams. Do
  not assume either “keep all” or “rewrite all.” Decide per component after
  looking at dependencies, behavior and test cost.
- Treat `ThemeArtwork` as a substitute implementation that does not contain
  the original owner-art mappings; it is not an authoritative asset source.

If the exact issue is unknown, report it as `unknown` and say which small,
reversible RB-02 experiment would settle it. Do not prototype a second UI
framework in RB-00.

## Required RB-00 outputs

Keep one feature ledger at `evidence/rebuild/parity-matrix.toml`. Add new IDs
under `[feature]` records without changing stable component IDs. Required
fields for each feature:

```toml
[[feature]]
id = "modeler3d.ui.example.action"
area = "example"
behavior = "Observable result and preconditions"
python_paths = ["src/main_window.py"]
python_symbols = ["MainWindow.example"]
python_tests = ["tests/test_example.py::test_contract"]
rust_component_ids = ["modeler3d.app.example"]
rust_paths = ["crates/modeler3d-app/src/example.rs"]
rust_symbols = ["ExampleCommand"]
desktop_entry_points = ["ribbon/menu/shortcut/viewport as applicable"]
contracts = ["result", "units", "history", "failure"]
fixture = "evidence/rebuild/rb-00-reference-audit/fixtures/example"
implementation = "pending" # pending | partial | complete | blocked
automated = "pending" # pending | pass | fail | not_applicable
interactive = "pending" # pending | pass | fail | not_applicable
visual = "pending" # pending | pass | fail | not_applicable
owner_acceptance = "pending" # pending | accepted | rejected
reuse = "unknown" # reuse | repair | replace | unknown
evidence = []
known_limits = []
next_slice = "RB-02"
```

Use real values and stable IDs, not the placeholder example. Do not mark an
existing component `complete` merely because its Rust module exists. Do not
mark UI interaction or owner acceptance `pass` without observing it.

Create:

- `evidence/rebuild/rb-00-reference-audit/report.md`: scope, summary, source
  map, decision reconciliation, confirmed and pending observations, top gaps,
  recommended reuse, risks and exact RB-01/RB-02 proposal.
- `evidence/rebuild/rb-00-reference-audit/asset-manifest.toml`: hashes,
  dimensions, mapping and alpha notes for all original supplied artwork; no
  copied resources in this slice.
- `evidence/rebuild/rb-00-reference-audit/reference/`: only inspected captures,
  using filenames that name screen, theme and size. Do not check in a blank,
  unrelated or unreviewed capture as evidence.
- `evidence/rebuild/rb-00-reference-audit/checkpoint.md`: verified status,
  completed outputs, unresolved items, recovery point and exact next slice.
- Update `WORK_LOG.md` with the RB-00 result and snapshot receipt. Update
  `docs/component-catalog.toml` only if the audit discovers a factual path or
  scope correction; do not promote maturity statuses during inventory.

## RB-00 completion review

Before calling RB-00 complete, answer all of these from the artifacts:

1. Is every original interactive surface/action mapped to a distinct feature
   record, with its handler, relevant tests and expected result?
2. Are accepted design decisions separated from historical suggestions and
   defects? Are material conflicts and unknowns explicit?
3. Has every claimed screenshot been opened and checked? Is the old blank
   Python capture explicitly disqualified?
4. Does the owner-art manifest cover both themes and branding with hashes,
   dimensions, transparency and logical mappings?
5. Does every major workflow map to Rust commands/services and shared MCP
   capability where required? Are hardcoded desktop-only workflows visible?
6. Does every reuse recommendation point to code and tests? Are gaps between
   module tests, UI wiring and actual live behavior stated?
7. Is the first prototype small enough to show the owner the real design and
   a working model edit before building more chrome?
8. Are statuses pending where the owner or live host has not accepted them?
9. Does the checkpoint provide an exact next action and a verified new
   snapshot, with no source changes or lost existing work?

RB-00 authorizes the audit artifacts only. Its exit is a reviewable decision
package. RB-01 then prepares a representative visual direction for owner
review; RB-02 proves the same window can perform a real model workflow. Do not
silently roll into those implementation slices in the RB-00 handoff.

## Ready-to-use prompt

Copy this prompt to start the next session:

```text
Execute RB-00 only from the 3D Modeler Rust product rebuild plan.

First read E:\AGENTS.md, E:\RustDeveloper.md,
E:\3D-Modeler\AGENTS.md, then the complete
E:\3D-Modeler\docs\product-rebuild-handoff.md and
E:\3D-Modeler\docs\product-rebuild-plan.md. Continue from the latest
WORK_LOG.md entry and verified checkpoint.

Audit the original 3D Modeler at L:\Builder3D as a read-only behavior and visual
reference. Produce the RB-00 feature ledger, decision reconciliation, inspected
reference baseline, original asset provenance manifest, evidence-based Rust
reuse/gap map, report, checkpoint and verified G:\ recovery snapshot. Inspect
the real sources and focused tests. Preserve all existing untracked Rust work.

Do not edit application code, Python files/assets/settings/projects, or existing
snapshots. Do not run broad Python desktop suites, redo R1-R7, start RB-01/RB-02,
commit, or claim unobserved UI behavior. If a live capture is blocked, record
the reason and leave that evidence pending. Reuse current Rust/eframe by default;
make no toolkit change without the evidence and owner decision required by the
plan. Finish with the RB-00 evidence and exact next step, then stop.
```
