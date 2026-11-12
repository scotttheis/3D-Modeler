# Builder3D Rust R8 Execution Brief

> RETIRED 2026-10-30 by owner direction. Historical record only; do not execute
> this brief or its fresh-session prompt. Use
> [the product rebuild plan](product-rebuild-plan.md). The R8 product-completion
> claim is withdrawn: original artwork, real viewport integration, functional
> parity and owner acceptance were not established. Keep valid component tests
> as scoped evidence; they do not establish application readiness.

This document is the working brief for a fresh session that completes the
post-conversion R8 phase of the Builder3D Rust rewrite. It is intentionally
self-contained so the session prompt can stay short and the context can be
reconstructed from one file.

## 1. Objective

Complete R8 as the native product-polish and UI-parity phase after R7's
conversion-first coverage work. The result should be a usable native Builder3D
application whose shell and workflows are recognizably equivalent to the
original Python application, while continuing to use the Rust core, command,
viewport, platform, and MCP seams already implemented.

The current Rust app is functional but still visually and ergonomically thin.
R8 is specifically where the ribbon, object panel, welcome/recent surface,
dialogs, settings, interaction cues, feedback, accessibility, and presentation
quality are completed. Do not spend R8 replacing the already-converted core or
re-implementing R1–R7 behavior.

`L:\Builder3D\docs\rust-rewrite.md` does not define a numbered R8 section. Its
authoritative post-R6 order is conversion coverage first and product polish
after coverage. This brief operationalizes that post-coverage phase as R8. If
the owner adds a newer explicit R8 scope, that scope takes precedence and this
brief should be updated before implementation continues.

## 2. Workspace and behavioral oracle

- Rust rewrite: `E:\Builder3D-Rust`
- Python reference/fallback: `L:\Builder3D`
- Rewrite plan: `L:\Builder3D\docs\rust-rewrite.md`
- Rust component catalog: `E:\Builder3D-Rust\docs\component-catalog.toml`
- Rust progress log: `E:\Builder3D-Rust\WORK_LOG.md`
- R7 evidence: `E:\Builder3D-Rust\evidence\r7\`
- Recovery snapshots: `G:\Builder3D-Rust_snapshot_*`

The Python tree is a read-only behavioral and visual oracle. It remains the
shipping fallback until the owner-approved Rust stage gate passes. Never edit
Python source, Python tests, Python assets, or existing snapshots as part of
R8.

## 3. Required instructions

Before taking action, read these completely:

1. `E:\AGENTS.md`
2. `E:\RustDeveloper.md`
3. `E:\Builder3D-Rust\AGENTS.md`
4. `L:\Builder3D\AGENTS.md`, if present
5. `L:\Builder3D\docs\rust-rewrite.md`
6. This brief
7. `E:\Builder3D-Rust\docs\component-catalog.toml`
8. The latest R7 section in `WORK_LOG.md` and the latest applicable evidence
   checkpoint

Use the pinned Rust toolchain. Prefer PowerShell-friendly commands. Use
`apply_patch` for source and documentation edits. Do not commit.

## 4. R7 baseline — do not redo it

R7 conversion coverage was completed on 2026-10-30. The final recovery point
is:

`G:\Builder3D-Rust_snapshot_20261030-101500_R7-final`

The R7 result included:

- 29 catalog IDs.
- All 41 Python modules under `L:\Builder3D\src` mapped to catalog entries.
- All 19 Python support/reference scripts under `L:\Builder3D\scripts` mapped
  to the reference-only catalog entry.
- Catalog statuses limited to `validated` and `ported`.
- 173 locked workspace tests passed, plus doctest runs.
- `cargo fmt`, workspace check, Clippy with `-D warnings`, cargo-deny,
  locked metadata, catalog/coverage validation, and native smoke passed.
- Native smoke output:
  `Builder3D shell smoke: objects=2 selected=1 tabs=5`
- `cargo audit` was not available on that machine and was recorded as skipped.
- No Python source, existing snapshot, or commit was changed.

The R7 entries most directly targeted by R8 are currently `ported` because
their semantic shell contracts exist but their native presentation is not yet
at parity:

- `builder3d.app.shell`
- `builder3d.app.commands`
- `builder3d.app.desktop-adapters`
- `builder3d.mcp.desktop-bridge`

Do not blindly promote these entries. Promote them only after the corresponding
R8 workflow and evidence are real.

## 5. R8 non-negotiable constraints

- Do not modify Python source, tests, assets, or configuration in
  `L:\Builder3D`.
- Do not modify, delete, rotate, or overwrite an existing `G:\` snapshot.
- Do not use `git reset --hard`, `git checkout --`, or equivalent destructive
  recovery commands.
- Do not commit or create a pull request.
- Do not redo R1–R7 core, file-format, mesh, MCP, viewport, platform, or
  conversion work unless an R8 UI defect requires a narrow compatible seam.
- Keep model mutation in Rust core/application commands; widgets must not edit
  meshes or scene collections directly.
- Keep the desktop shell, MCP adapter, platform code, and core geometry
  independently testable and toolkit-neutral where the architecture requires
  it.
- Avoid adding dependencies unless they are necessary, compatible with the
  pinned toolchain, and justified in the evidence.
- No temporary debugging paths, fake product behavior, or unfinished controls
  may be accepted as completed R8 behavior.
- Preserve existing public file, unit, object-ID, selection, undo, export,
  operation, and MCP contracts.

## 6. Required R8 workflow

### Phase 0 — establish the R8 baseline

1. Close any previously running Rust or Python app before changing code.
2. Inspect `git status --short`; preserve all existing work.
3. Inspect the R7 catalog, evidence, work log, and current shell source.
4. Launch the current native app once and record the visible gaps.
5. Run the existing focused shell tests and native smoke.
6. Create a new verified baseline snapshot on `G:\` before the first R8 code
   change.
7. Record the exact snapshot path, file count, byte count, and unresolved UI
   gaps in the first R8 checkpoint.

### Phase 1 — shell structure and navigation

Bring the native eframe shell close to the original information architecture:

- ribbon tabs and command grouping;
- command labels, icons, tooltips, separators, and selected states;
- favorites, pinning, reorder, reset, and overflow behavior;
- file/application menu behavior;
- object panel and viewport proportions;
- status bar, title, dirty marker, and empty-scene states;
- light/dark theme surfaces and consistent spacing/typography.

The shell should remain a presentation layer over `CommandState` and the
existing typed dialog/action boundaries.

### Phase 2 — object panel and scene interaction

Complete the UI for the already-ported selection and object semantics:

- select, deselect, invert, multi-select, and selection feedback;
- visibility toggles and object names;
- grouping, ungrouping, cut/copy/paste, duplicate, and delete;
- object-panel context actions and keyboard shortcuts;
- dirty-state and undo/redo feedback;
- accessible labels, predictable focus, and clear disabled/error states.

Add focused tests for command routing and UI state transitions without making
the tests depend on a GPU context.

### Phase 3 — welcome, recent, and file workflows

Complete the user-facing project/model lifecycle:

- new/open/save/save-as/import/export surfaces;
- recent-project list using the bounded Rust preferences policy;
- recent-entry fallback presentation for missing/unsupported files;
- empty, loading, success, and failure states;
- file path validation and clear error messages;
- unit selection and authoritative-unit format messaging;
- no accidental scene mutation when a dialog is cancelled or a file fails.

Use the existing Rust file and command services. Do not duplicate format logic
inside the shell.

### Phase 4 — settings, theme, branding, and preferences

Finish the native replacement for the Python adapter/presentation modules:

- persisted theme mode and display preferences;
- import-unit defaults and recent-file persistence;
- ribbon favorites, pinning, and order persistence;
- application icon/branding treatment where native assets support it;
- consistent light/dark icon and artwork selection;
- settings validation, migration defaults, and corrupt-settings recovery.

Keep the settings file bounded and typed. Add recovery behavior for malformed
or missing settings rather than panicking or silently accepting invalid data.

### Phase 5 — numeric controls, shortcuts, and accessibility

Make editing behavior match the original workflow rather than only matching
static screenshots:

- position, size, rotation, scale, gap, wall-thickness, and print-bed values;
- suffix-aware numeric editing and sensible select-all behavior;
- bounded ranges, finite-value validation, and clear invalid-input feedback;
- Ctrl/Meta shortcut behavior appropriate to the host platform;
- shortcuts yielding to text fields and dialogs;
- keyboard navigation, focus indication, labels, and sensible tab order;
- tooltips and status messages for unfamiliar controls.

Use the existing transform/preferences/print-prep services as the source of
truth.

### Phase 6 — viewport and visual interaction parity

Improve presentation and interaction without weakening the existing renderer
contracts:

- build bed, grid, floor contact cues, object colors, selection outline/cues;
- camera reset/top/front/side/frame behavior;
- transform gizmo presentation and snap feedback;
- wireframe and X-ray toggles;
- object-panel/viewport selection synchronization;
- responsive layout at the supported minimum window size;
- stable behavior with empty, hidden, grouped, and high-object-count scenes.

Do not replace the authoritative full-resolution picking/rendering paths with a
preview-only shortcut. Preserve the R5 resource-aware renderer behavior.

### Phase 7 — operation feedback and advanced workflows

Expose the already-ported background and print capabilities coherently:

- progress, indeterminate state, cancellation, completion, failure, and
  bounded error messages;
- print-preparation report with actionable warnings/errors;
- known-slicer discovery and selection;
- 3MF staging and Bambu STL fallback handoff;
- image-color sampling and material/color controls;
- operation history/status without blocking the shell unnecessarily.

No generic shell command execution may be introduced. Use the constrained
platform slicer seam.

### Phase 8 — parity, repair, and final R8 acceptance

1. Compare all major shell workflows against the Python reference behavior and
   relevant tests.
2. Launch both applications as needed for visual comparison; use screenshots
   or other local evidence when useful.
3. Fix regressions in focused slices rather than adding broad speculative
   rewrites.
4. Update catalog statuses, contracts, symbols, tests, evidence, and
   conversion notes.
5. Ensure no R8 catalog entry is left `planned` or `polish`. Use `ported` only
   when a toolkit-specific replacement is intentionally complete enough for
   the documented R8 boundary; explain any remaining post-R8 visual work.
6. Run the complete verification matrix and create the final R8 recovery
   snapshot.

## 7. Slice protocol

Every R8 slice must follow this loop:

1. State the slice objective in commentary.
2. Read the relevant Python implementation and tests as a read-only oracle.
3. Create a pre-slice recovery snapshot on `G:\` if this is the first slice;
   otherwise use the immediately preceding verified R8 final snapshot as the
   next baseline and document it.
4. Implement one cohesive Rust change.
5. Add focused tests before moving on.
6. Run `cargo fmt`, focused check/test, and Clippy for the changed crate.
7. Update `docs/component-catalog.toml`, R8 evidence, and `WORK_LOG.md`.
8. Run the required full gates at the end of the slice when practical.
9. Create and verify the post-slice snapshot before beginning another slice.
10. Record unresolved risks and the exact next slice in the checkpoint.

Use additive, recoverable snapshots. Exclude `target`, caches, logs, and
generated output unless a specific evidence task requires them. Verify each
snapshot using the copy result plus source/snapshot file-count and byte-count
comparisons.

## 8. Catalog and evidence requirements

For every new or materially changed component:

- keep the stable catalog ID;
- list the Rust paths and public symbols;
- list the Python oracle paths without editing them;
- list focused tests;
- add report, verification, and checkpoint evidence under
  `E:\Builder3D-Rust\evidence\r8\<slice>\`;
- describe contracts and user-visible behavior;
- record intentional toolkit substitutions or post-R8 boundaries explicitly;
- never mark a component validated solely because it compiles.

The final R8 catalog audit must verify:

- unique component IDs;
- all referenced Rust/test/evidence paths exist;
- every product Python module and reference script is accounted for;
- no stale R7 `next_patch_ids` claim remains for work completed in R8;
- no `planned` or `polish` status remains in the R8 scope.

## 9. Verification matrix

Run from `E:\Builder3D-Rust`:

```powershell
cargo fmt --all -- --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo deny check
cargo metadata --locked --format-version 1 --no-deps
cargo run -p builder3d-app --bin builder3d-app -- --smoke
```

Also run, when installed and relevant:

```powershell
cargo audit --locked
cargo bloat --release --workspace
```

The final verification record must state which optional checks were unavailable
and why. A manual native launch is required for R8; a smoke run alone cannot
prove visual parity.

## 10. R8 completion criteria

R8 is complete only when all of the following are true:

- the native shell presents the major original workflows coherently;
- ribbon/navigation, object panel, welcome/recent, dialogs, settings, theme,
  shortcuts, viewport cues, progress/errors, print handoff, and color/material
  paths are implemented or explicitly bounded in evidence;
- focused tests cover new UI state and routing logic without requiring a GPU;
- the full verification matrix passes;
- catalog/evidence/work log are current and internally consistent;
- all R8 snapshots are verified and recoverable;
- no Python source, existing snapshot, or commit was changed;
- the final handoff identifies any remaining work as post-R8 owner visual
  acceptance or a separately scoped stage, never as silently unfinished R8
  coverage.

## 11. Fresh-session prompt

Use this short prompt after opening a new session:

```text
Complete all of R8 for the Builder3D Rust rewrite.

First read and follow the workspace/project instructions, then read the complete
R8 execution brief at:

E:\Builder3D-Rust\docs\r8-execution-brief.md

Work in E:\Builder3D-Rust using L:\Builder3D only as a read-only behavioral and
visual oracle. R7 conversion coverage is complete; do not redo R1–R7. Follow
the brief's snapshot, catalog, evidence, testing, UI-parity, and no-commit rules.
Complete the native eframe shell and user workflows, not just a demo. Launch the
app for manual inspection, implement R8 in small tested slices, update the
catalog/evidence/WORK_LOG, run every required verification gate, create the
final verified G:\ recovery snapshot, and stop only when all R8 work is
genuinely complete. Do not modify Python source or existing snapshots.
```
