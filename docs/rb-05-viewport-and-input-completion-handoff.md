# RB-05 handoff: complete the entire viewport and input slice

Prepared 2026-11-01 for the next implementation session.

## Objective and stop boundary

Complete **all of RB-05 - Viewport and input completion** in the existing
`E:\Builder3D-Rust` worktree. Implement the full slice, run its focused and
integrated checks, complete the required sequential Python/Rust native
walkthrough and paired evidence, update the ledger and documentation, and
create a verified additive G: recovery snapshot. Then **stop before RB-06**.

This is a full-slice assignment. Do not stop after one camera control, a
representative gizmo, a passing test batch, a plan, or a progress-only handoff.
Continue through every RB-05 requirement and its verification. Make routine
implementation decisions without asking for confirmation. Preserve actual
owner-review gates; successful tests or an agent's visual inspection are not
owner approval.

The owner asked which slices cover unfinished buttons for personal information,
then explicitly clarified that troubleshooting can wait. **Do not turn that
conversation into a button inventory, general troubleshooting campaign, or
RB-04 reopening task.** Continue the planned work as it stands: the next full
slice is RB-05. Complete RB-05's own controls and fix regressions introduced by
that work; leave unrelated modeling/appearance/shell completion to its owning
later slices. The owner's comments did not accept RB-03 or RB-04 visuals.

## Authority and non-negotiable rules

Read these before implementation:

1. `E:\AGENTS.md` and `E:\RustDeveloper.md`.
2. Project [AGENTS.md](../AGENTS.md) and [WORK_LOG.md](../WORK_LOG.md).
3. [Product rebuild plan](product-rebuild-plan.md), especially sections 4-6
   and 8-12. It owns the RB-00 through RB-11 order and acceptance process.
4. [Component catalog](component-catalog.toml),
   [parity matrix](../evidence/rebuild/parity-matrix.toml), and
   [build prerequisites](building.md).
5. The RB-02 accepted baseline and the RB-03/RB-04 evidence listed below.

Keep `L:\Builder3D` read-only. It is the Python behavioral/visual reference
and remains the shipping fallback until the replacement gate passes. Do not
restore, reset, replace, stage, or commit the Rust worktree. Its files were
already untracked; preserve that work. Use additive snapshots, never overwrite
an existing G: backup, and do not touch H: backups.

Keep geometry, scene/history, file contracts, camera/gesture mathematics and
semantic commands separate from toolkit/platform/render adapters. Desktop
and MCP retain the same services; application automation must not be built
from physical input replay. Physical input is appropriate for the separate
native verification harness. Model values remain millimetres. A completed
model edit has one history entry; a view/selection/no-op action does not create
geometry history. Keep stable identities and bounded resource usage.

No new toolkit migration, unrelated dependency upgrade, speculative rewrite,
OS policy change, staging, commit, or PR is part of this task. If an MCP
interface must actually change, first follow the workspace MCP design
prerequisite; ordinary RB-05 viewport work should not require that scope.

## Starting state and evidence to retain

RB-02's current Light/Dark visual baseline was accepted on 2026-10-31, with
fine-tuning deferred. RB-03 implementation/engineering evidence is recorded,
but its remaining native/visual/owner reviews are still pending. RB-04
implementation, engineering gates and required native evidence were completed;
its visual/owner acceptance is also pending. RB-05 has not been implemented.

Read the following records rather than restarting the earlier conversion:

- [RB-02 checkpoint](../evidence/rebuild/rb-02-first-working-application/checkpoint.md).
- [RB-03 checkpoint](../evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md).
- [RB-04 checkpoint](../evidence/rebuild/rb-04-scene-and-precise-transforms/checkpoint.md),
  [report](../evidence/rebuild/rb-04-scene-and-precise-transforms/report.md),
  [verification](../evidence/rebuild/rb-04-scene-and-precise-transforms/verification.md),
  [walkthrough](../evidence/rebuild/rb-04-scene-and-precise-transforms/walkthrough.md),
  [paired board](../evidence/rebuild/rb-04-scene-and-precise-transforms/visual-comparison.html),
  [visual inspection](../evidence/rebuild/rb-04-scene-and-precise-transforms/visual-inspection.md),
  [viewport implementation](../evidence/rebuild/rb-04-scene-and-precise-transforms/viewport-transforms.md)
  and [numeric/selection contracts](../evidence/rebuild/rb-04-scene-and-precise-transforms/numeric-selection.md).

The last verified RB-04 application boundary has:

| Evidence | Recorded result; revalidate freshness after changes |
|---|---|
| Workspace tests | 261 passed; zero failed/ignored |
| Gates | fmt/check/strict workspace all-target all-feature Clippy/deny/metadata/debug and release smoke/release/bloat passed |
| Audit | Cargo audit was unavailable; no audit pass claimed |
| Native cases | 74 scene cases, 22 viewport cases, four Settle cases, eight paired physical-pick states, plus modal controls |
| Comparison | 386 paired action states / 812 PNGs; historical intermediate frames are explicitly classified |
| Independent Python oracles | 122 gesture, 32 multi-numeric, 42 placement, nine Euler/polar cases |
| Preservation | 108 asset hash pairs unchanged; 142 unique feature IDs; 33 catalog IDs; all 32 RB-03 review rows preserved |

Build identities and exact commands are in
[final-native-build-artifacts.json](../evidence/rebuild/rb-04-scene-and-precise-transforms/final-native-build-artifacts.json)
and [final-native-gate-results.json](../evidence/rebuild/rb-04-scene-and-precise-transforms/final-native-gate-results.json).
The debug SHA-256 is
`82fd07e7691d76cb6f348ba6aafd86d91c9f9b94b56ee72e7de398a70477dfa1`.
The ordinary release is 14,972,928 bytes, SHA-256
`fb5c4d3427b73607378af8c74ccd83dc6428d01d1e6b934765971aaa855d1c87`.
These are previous verified artifacts, not proof for new source changes and
not a complete installed-package measurement.

The completed RB-04 recovery snapshot is
`G:\Builder3D-Rust_snapshot_20261101-001205_RB-04-completed`:
2,501 files / 355,033,106 bytes, zero path/size/SHA-256 differences before
receipt writing. Its post-receipt comparison had only WORK_LOG.md and the
RB-04 checkpoint changed. Subsequent handoff documentation is a later change;
do not misapply that historical two-file comparison to the current tree.
An earlier `001137` copy remains immutable; its receipt helper had an encoding
error and the `001205` snapshot supersedes it as the completed RB-04 receipt.
Do not restore either copy over the working tree.

## Full RB-05 implementation checklist

Reconcile the existing RB-05 ledger with the entire scope below. Rows assigned
to earlier slices may already cover a basic path; reuse them and their tests.
Add a stable row only when a real required contract is absent. Do not let
orbit, lighting, floor behavior, authored colors or resize interaction vanish
because a corresponding row currently lives elsewhere.

### Camera and navigation

- Complete the reference's orbit, middle/right pan, wheel zoom and supported
  gesture/pinch paths. Establish exact buttons, modifiers, sensitivity, axis
  conventions and distance limits from Python, not from UI labels.
- Preserve scene geometry, IDs and geometry history during navigation. Verify
  orbit consistency at different distances and with differently scaled scenes.
- Complete Center View, Frame Selection, Reset View, Top and Side, plus any
  other actual reference presets in this slice. Frame world bounds using the
  reference's visibility/selection rules. Establish empty, hidden-only,
  grouped, multiple-selection and degenerate-bounds behavior.
- Double-click must frame the intended picked object. Avoid an unintended
  second selection toggle or model mutation. Follow the real reference's
  reset/framing transitions and view-state persistence contract.
- Exercise resize/maximize/restore, viewport aspect changes and available
  display scales without using old fixed-coordinate assumptions.

### Picking, gizmos and complete resize interaction

- Retain exact full-geometry/frontmost-face picking, group-root identity,
  visibility filtering, modifier/sticky selection and gizmo hit priority.
  Add focused RB-05 regressions where navigation or rendering changes interact
  with these existing paths; do not restart the RB-04 work wholesale.
- Match Move gizmo placement and style, including the reference's
  camera-nearest-face rule. RB-04's centered Move gizmo was a documented gap.
- Complete Rotate ring and Resize box/handle presentation, scale with camera,
  hover/active feedback, visibility, occlusion/depth behavior and hit geometry.
  The visible controls and the geometry that receives input must agree.
- Preserve all-axis movement and degree rotation mathematics, signed stretch,
  object/group identity, hidden selected targets and one commit on release.
- Complete Resize Grid/Object basis, face opposite-side anchoring, uniform
  corner resizing and proportional-lock interaction in the real viewport.
  The observed reference has **six face and eight corner controls**. RB-04
  tests also exercise internal edge mathematics; that does not authorize
  exposing twelve additional edge grips as original visible controls.
- Motion changes a preview; release commits once. No-motion, invalid rays,
  Escape, focus loss, modal capture, selection/document replacement and stale
  revisions must follow the recorded safe Rust contract without partial edits.
  Preserve exact untouched objects and exact native undo/redo results.

### Snapping

- Complete ribbon Snap state, persistence and its connection to the actual
  viewport. Parse persisted false values correctly; a checked icon alone is
  not a functioning snapping implementation.
- Match lower/upper/center object landmarks, floor capture, tolerances,
  candidate/anchor rules, exclusions and feedback from the actual reference.
- Verify Shift bypass during a live drag. Distinguish landmark snapping from
  fixed increments: the current ledger explicitly forbids inventing open-space
  fixed increments as a substitute.
- Cover spheres/curved geometry, groups, mixed visible/hidden selection and
  measured object centers, with snapping both on and off. Preserve one-action
  history and the existing text/modifier/modal boundaries.

### Floor, grid, lighting and display modes

- Complete build-plane/floor and metric grid appearance, world-axis placement,
  camera-relative behavior and toggles. Match the functioning reference's
  colors, spacing, line/depth treatment and any actual floor/shadow behavior.
- Match surface lighting and authored object/vertex colors. Preserve authored
  color through selection tint and deselection. Keep display shading separate
  from RB-07 material/image editing workflows.
- Wireframe keeps opaque shading with the reference's unlit black,
  depth-tested edges. X-Ray is an independent translucent mode. Implement their
  real combinations, toggles, visibility and selection feedback, including
  hidden/grouped scenes; do not treat one as an alias for the other.
- Verify Light/Dark, rest and interaction, CPU/GPU paths and available
  fallback behavior. Keep actual mesh presentation throughout the product
  path: **no projected-rectangle or bounding-box substitute for mesh preview**.
- Reuse immutable geometry and invalidate only derived data that actually
  changed. Bound geometry, uploads, cache growth and temporary allocation;
  preserve full resolution at rest and a faithful adaptive interaction tier.
  Measure changed behavior honestly; full performance/installed-size/host
  acceptance still belongs to RB-10.

## Source and implementation starting points

Read Python first, following imports to the authoritative camera/renderer/
floor owners. Main starting points under `L:\Builder3D`:

- `src/viewport.py`, `src/gizmo.py`, `src/snapping.py`.
- `src/main_window.py`, `src/application_services.py`, `src/style.py`.
- `tests/test_live_viewport_input.py`, `tests/test_gizmo.py`,
  `tests/test_snapping.py`, `tests/test_viewport.py`,
  `tests/test_renderer_math.py`, and relevant focused cases in
  `tests/test_adapter_boundaries.py` / `tests/test_desktop_actions.py`.

Rust starting points:

- `crates/builder3d-app/src/shell.rs`: actual window/view/input routing,
  viewport controls, selection, modal/text containment and gizmo painting.
- `crates/builder3d-app/src/view_state.rs`, `commands.rs`,
  `transform_gesture.rs`, `gesture_tests.rs`, `transforms.rs`, `rotation.rs`
  and `lib.rs`: semantic state, exact transform contracts and native wiring.
- `crates/builder3d-viewport/src/camera.rs`, `gizmo.rs`, `picking.rs`,
  `snapping.rs`, `floor.rs`, `scene.rs`, `renderer.rs`, `cpu.rs`, `adaptive.rs`
  and `math.rs`: portable math, exact geometry, rendering and fallback.
- `builder3d-platform` and existing resource-policy modules own platform
  capability/budget seams. Do not introduce a separate vendor-name policy.

Existing RB-05 ledger groups are gizmo Move/Rotate/Resize, Snap preference and
landmarks, camera pan/zoom, double-click frame, Center/Frame/Reset/Top/Side,
Grid/Wireframe/X-Ray. Move remains partial; base Rotate/Resize mathematics and
native paths passed RB-04 but their broader RB-05 presentation/input work is
still required. Inspect actual rows rather than blindly promoting them.

## Reference-first native and visual work

Create RB-05 evidence under
`evidence/rebuild/rb-05-viewport-and-input-completion/`.

1. Record a bounded requirement-to-test/walkthrough map for the entire slice.
   Capture Python actions/results before implementing the corresponding Rust
   repair, using shared scene fixtures with known IDs, dimensions and colors.
2. Run Python and Rust target windows sequentially, one target app at a time.
   A background MCP gateway without a target UI is not itself a blocker.
3. Match fixture, camera, selection, mode, theme, size and scale where practical.
   Capture Light/Dark at 1280x840 logical client pixels and record actual
   physical size, DPI/scale, operation, fixture and genuine build identity.
   Also exercise ordinary/maximized layouts and available display scales.
4. Exercise actual OS pointer/key input for pick/drag/cancel/navigation, not
   just semantic handlers or injected final matrices. Label any source
   widget/handler setup and any simulated display/hardware evidence separately.
5. Include empty/single/multiple/grouped/hidden scenes; on/off mode pairs;
   before/during/after rest/interaction; all relevant axes/bases; release,
   no-motion, invalid/cancel/focus/modal/stale paths and exact undo/redo.
6. Put Python left in the paired board. Inspect every accepted capture, reject
   stale/partial/blank/obscured/wrong-target/lock-screen frames, and describe
   visible differences in the affected controls and mesh results. Add focused
   crops/differences when useful; preserve original client-sized frames.
7. Update implementation, automated, native, visual and owner states separately.
   Do not claim owner acceptance from agent inspection or a successful launch.

Do not change corporate locking/sleep/group policy or the owner's video/browser
setup. Never send input to a locked/secure/obscured desktop. If it locks, do
independent code/tests and keep the specific native work outstanding until it
can actually be completed. Do not report unavailable live evidence as passed.

## Existing harness lessons and known differences

The RB-04 evidence directory contains reusable native guards, saved-state
verifiers, geometry helpers and capture journals. Adapt copies for RB-05;
do not overwrite the RB-04 fixtures, captures, journals or accepted historical
build metadata. Its validators are scoped to that boundary and need a separate
RB-05 counterpart rather than hardcoded new numbers in old records.

- Check the input desktop, exact PID/window, actual client geometry and
  foreground ownership before input. Re-query after window moves/scaling.
  The later user-review launch was maximized at 2560x1369 client pixels; the
  1280x840 macro's path-dialog check correctly refused to type there. Do not
  infer that the review fixture opened, or reuse those coordinates blindly.
- The review launch used `target/debug/builder3d-app.exe` with isolated
  `LOCALAPPDATA=E:\Builder3D-Rust\target\rb04-current-native-profile`.
  A disposable copy named `target/RB04-Visual-Review-20261101-043423.3dp`
  was made; it is not an authoritative fixture. Discover current processes
  and unsaved user state rather than reusing old PID files or closing a
  potentially edited user window.
- Clicking an active ribbon tab can collapse it. Check the painted active
  tab and expanded ribbon before using row/control coordinates. Park an
  idle automation cursor on blank navigation, not in the viewport.
- Dark dialog automation once lost selection or changed the camera while
  normalizing tabs. Those attempts were rejected. A queued-pointer theory
  passed on unchanged code; its experimental test was removed and the original
  source hash verified. Do not assume a confirmed product defect or restart
  that investigation from this informational handoff. Use the guarded verified
  routes and investigate only an actual failure of the current RB-05 task.
- Wait for real repaint and completed asynchronous open/save/operation state.
  Python dialog values were refreshed with an explicit Qt event/repaint wait.
  Preserve original `.3dp` inputs; use uniquely named output/review copies.
- Python's observed Shift selection picks endpoints; Rust's required contiguous
  range was explicitly recorded. Python Escape/focus-loss drag behavior also
  differs from Rust's safe preview cancellation. Preserve these documented
  contracts; do not silently change them or infer owner approval.
- Source naming/Eye comparison states were prepared because equivalent source
  controls were absent. The hidden source multi-selection lock widget was set
  explicitly. Do not relabel setup as observed physical interaction.
- Repair's fan point/report differences remain RB-06. Larger numeric strips,
  shaded Items rows and other recorded visual differences are evidence, not
  acceptance; fix the ones owned by RB-05 without a broad unrelated shell pass.

## Build, tests and integrated exit checks

Use the pinned toolchain. Initialize build-only libclang in the same shell:

```powershell
cd E:\Builder3D-Rust
. ./scripts/Initialize-BuildClang.ps1
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo fmt --all -- --check
cargo deny check
cargo metadata --locked --format-version 1 --no-deps
cargo build --locked -p builder3d-app --bin builder3d-app
```

Run focused Rust and matching read-only Python tests for each changed contract.
Use Python's existing `L:\Builder3D\venv\Scripts\python.exe`, `-B` /
`PYTHONDONTWRITEBYTECODE=1`, isolated E: profiles/output and no L: pytest cache.
Do not install or modify packages in the reference to make a test pass. Keep
real native GL/input tests distinct from pure/offscreen math tests.

At the completed boundary run applicable integrated release/smoke/size gates:

```powershell
cargo build --release --locked -p builder3d-app --bin builder3d-app
cargo bloat --release --workspace
cargo bloat --release -p builder3d-app --bin builder3d-app
```

Run isolated debug and ordinary-release `--smoke` as well as the native
walkthrough. Preserve/hash the ordinary release before bloat replaces it with
a symbol-enabled artifact. Workspace bloat may ignore `--workspace`; do not
call its default-binary result a total package measurement. Run Cargo audit
when installed, otherwise record unavailable. Required checks must not be
replaced by a test count copied from RB-04.

Add meaningful outcomes-based tests for camera invariants, exact picking,
gizmo hit/render agreement, snapping targets/bypass, independent display modes,
colors/visibility, resize anchors/ratios, preview/commit/cancel and resource/cache
lifetime where changed. Reuse existing fixtures and regression tests; do not
weaken the original oracle matrices or manufacture passing screenshot masks.

Use explicit UTF-8/UTF-8-with-BOM decoding for text and detect UTF-16 receipts.
Windows Python's default encoding previously corrupted a BOM in a receipt
helper. Check native process exit codes with a reliable subprocess runner;
do not turn a missing PowerShell Process.ExitCode into a reported pass. None
of these tooling details belongs in the product's user-facing flows.

## Completion deliverables

Finish all of the following before stopping:

- All RB-05 behavior and visible routes above implemented, with no temporary
  debug/placeholder/unfinished paths introduced into accepted behavior.
- Focused reference/Rust tests, applicable integrated checks and real native
  pick/drag/cancel/rest/interaction/hidden/grouped workflows recorded.
- RB-05 report, verification, requirement-to-test map, walkthrough, paired
  comparison board, capture/build metadata and concise continuation checkpoint.
- Ledger/catalog/evidence links, symbols, unique IDs, source/packaged asset
  hashes, preserved source status and current executable/source identities
  validated. Record actual remaining visual/owner review without promotion.
- WORK_LOG.md and continuation pointers updated with exact results, visible
  differences, limitations and the RB-06 next scope.
- New additive timestamped G: completed-RB-05 snapshot. Exclude root target/,
  .cache/ and reparse entries; include source/manifests/tests/docs/Git metadata
  and required evidence. Verify copy result, file/byte counts, relative paths,
  sizes and SHA-256. Record exact receipt and any documented post-copy receipt
  differences. Keep old snapshots immutable. Follow the project's bounded
  subtask/checkpoint rules before context compaction.
- Stop before RB-06. Final reply reports what changed, checks/native evidence,
  snapshot verification, still-pending reviews and RB-06 next. Do not merely
  say the next step is to continue RB-05 if required available work remains.

## Reviews and later slices that remain separate

Preserve all 32 RB-03 status rows: implementation/automated pass, native
30 pass/two partial, visual/owner pending. Its saved-checkpoint undo/redo/branch
native replay, full filesystem-failure native matrix, extra Python per-format
export-picker replay and visual/owner review remain pending. RB-04 visual/owner
acceptance remains pending. Neither is an invented routine approval stop for
this authorized RB-05 work; neither may be silently marked accepted.

After the full RB-05 boundary, the subsequent task is RB-06: primitives/torus,
repair/simplify/subdivide/smooth, booleans, emboss/hollow and persistent
brush/density workflows. Reuse the historical completed R3 core work; do not
restart that stage or confuse legacy R labels with current RB slice numbers.
Then retain RB-07 appearance/jobs/print, RB-08 preferences/shell, RB-09 live
desktop/MCP consistency, RB-10 performance/package/host verification, and
RB-11 final parity/live/visual/owner acceptance, in that order. Only the final
acceptance gate establishes the completed product.

## Simple activation prompt

Read and follow `E:\Builder3D-Rust\docs\rb-05-viewport-and-input-completion-handoff.md`.
Complete the entire RB-05 slice, including tests, paired native Python/Rust
evidence, documentation and a verified additive G: snapshot. Then stop before
RB-06. Continue the planned work; defer the unrelated button troubleshooting.
Keep Python read-only, preserve pending reviews and the Rust worktree, and do
not stage or commit.

Planned additive RB-05 handoff documentation recovery snapshot (RB-05 implementation has not started): `G:\Builder3D-Rust_snapshot_20261101-051329_RB-05-handoff`. Root target/.cache and reparse entries excluded; exact receipt follows after verification.


## Verified handoff documentation snapshot

- Snapshot: `G:\Builder3D-Rust_snapshot_20261101-051329_RB-05-handoff`; verified 2026-11-01T05:13:37-07:00; Robocopy exit 1.
- Source/copy: 2,502 files / 355,076,335 bytes each; zero missing/extra paths and zero size/SHA-256 differences; zero reparse entries. Root target/.cache/reparse entries excluded.
- This is a documentation-only continuation snapshot. RB-05 implementation has not started. Links and reference paths validate; application source hashes and the entire parity ledger are unchanged. No implementation tests or native walkthrough were rerun for this documentation task. Full RB-05 work is the next-session assignment, followed by a stop before RB-06; unrelated button troubleshooting is deferred. No staging/commit or Python edits.
- The receipt postdates the immutable copy; only WORK_LOG.md and this RB-05 handoff are expected to differ afterward.

Post-receipt comparison: 2502 included paths in each tree; no missing or extra paths; exactly WORK_LOG.md and the RB-05 handoff differ from their post-copy receipts; no other size/SHA-256 differences; reparse entries 0 in each.
