# RB-01 Visual Foundations Handoff

Prepared: 2026-10-30  
Authority: `docs/product-rebuild-plan.md`, RB-01  
State: RB-01 matched captures and owner review remain pending

**Current status note (2026-10-31):** Matched live Light/Dark captures and
explicit owner approval remain pending. The owner's task instruction requires
both before RB-02; until then, do safe RB-01 capture/evidence work only and
stop. Capture original and Rust apps sequentially, one target window at a time.
A background MCP gateway without a target UI window is not by itself a
blocker. After the gate passes, follow the
[`RB-02 execution handoff`](rb-02-first-working-application-handoff.md).

## 1. Goal and boundaries

RB-01 establishes the original 3D Modeler visual direction in the current Rust application and gets the owner’s approval before broader application work. It covers original artwork and branding, theme tokens, fonts and control states, ribbon and panel proportions, Welcome cards, and the transform strip in Light and Dark.

Keep the existing Rust workspace and eframe as the first implementation path. The workspace pins eframe 0.37.0. Do not prototype another UI toolkit. A toolkit change requires a measured comparison and the owner decision described by the rebuild plan.

RB-01 is a visual slice. Do not begin the RB-02 New/Cube/import/edit/save workflow, integrate the production viewport renderer, or claim model-workflow parity. The current Rust shell draws a rectangle preview; the real renderer remains separate. RB-02/RB-05 own that integration.

## 2. Required read order

Before making changes, read:

1. `E:\AGENTS.md` and `E:\RustDeveloper.md`.
2. `E:\3D-Modeler\AGENTS.md`.
3. `docs/product-rebuild-plan.md` and `docs/product-rebuild-handoff.md`.
4. The latest `WORK_LOG.md` entry, this document, and:
   - `evidence/rebuild/rb-00-reference-audit/checkpoint.md`
   - `evidence/rebuild/rb-00-reference-audit/report.md`
   - `evidence/rebuild/parity-matrix.toml`
   - `evidence/rebuild/rb-00-reference-audit/asset-manifest.toml`
5. The original read-only reference material in `L:\Builder3D`, especially its `AGENTS.md`, `docs/ui-style-guide.md`, `src/main_window.py`, `src/style.py`, `src/icon_assets.py`, and the dated owner decisions in `WORK_LOG.md`.
6. Rust presentation seams and their tests: `crates/modeler3d-app/src/shell.rs`, `theme.rs`, `artwork.rs`, `Cargo.toml`, relevant component catalog entries, and focused app/theme/artwork tests.

The shared Rust practices in `E:\RustDeveloper.md` remain in force. Preserve all existing untracked Rust work. Do not clean, reset, stage, or commit the workspace.

## 3. RB-00 handoff facts

RB-00 produced 114 feature records, a 108-file original asset manifest, six inspected 1440×138 historical ribbon crops, a source/test and Rust reuse map, and a verified G: recovery snapshot:

`G:\Builder3D-Rust_snapshot_20261030-142550_RB-00-final`

The receipt records 460 files / 51,075,936 bytes and matching file sets and SHA-256 hashes at copy time. The work-log receipt was appended after that snapshot. Existing G: snapshots must remain unchanged.

The six historical ribbon crops do not establish the current full-window layout. The old blank image at `evidence/r8/00-baseline/python-reference.png` is explicitly disqualified. The RB-00 report records why current live full-window reference captures remain pending. Recheck processes and desktop availability at the start of RB-01; do not assume the prior blocker still exists.

At the RB-00 inventory, `L:\Builder3D` was a dirty checkout with 69 tracked changes and 154 untracked paths. Its audited HEAD and key source hashes are in the report. Leave all of that source and user work untouched. No original product art has yet been copied into the Rust application.

## 4. Comparison size and capture rules

The rebuild plan sets **1280×840 logical pixels** as the initial comparison target. Use that target for the matched comparison board.

The Python source calls `MainWindow.resize(1280, 820)` at startup. Record that as the original’s startup size. It is a separate source fact from the plan’s comparison target; do not replace the plan target with 1280×820. Record the actual logical client area, physical image dimensions, display scale, and window state for each capture. Capture original and Rust with the same logical client size, display scale, font setup, fixture, camera, selection, theme, and UI state. If the target cannot be matched on the current display, state the measured difference and leave that comparison pending instead of cropping or rescaling product content.

Capture and inspect the original in both themes for these representative states:

- Welcome with no recents and with recents.
- Insert, Object, Edit, Paint, and View ribbon tabs.
- Application menu and Settings.
- Empty, single, multiple, grouped, and hidden-object Items states.
- Move, Rotate, and Resize strips and representative gizmos.
- Brush settings popover and import-unit choice.
- Operation progress and error, Print Prep Check, and print handoff.

Record each capture’s source, date, window/surface, state, theme, logical dimensions, physical dimensions, scale, and limitations. Open every image. Reject blank, stale, occluded, partially painted, lock-screen, or wrong-window captures. Keep historical crops labeled as historical. Do not save reference changes back into `L:\Builder3D`.

The plan lists 100%, 125%, 150%, and 200% scaling, ordinary/maximized layouts, and monitor moves. Check configurations available without changing machine-wide settings. Record unavailable configurations as untested; RB-01 does not close the later full scale review.

When safe live capture is impossible, do not stop or kill an existing Modeler3D/MCP process and do not launch a concurrent target. Finish independent source/artwork work, record the process and reason, and leave live capture, matching, and acceptance pending. No historical or model-generated image can substitute for a live capture.

## 5. Visual work in the Rust app

Implement only the visual foundations in the existing eframe app:

- Package the original 3D Modeler branding and theme artwork needed for the selected screens.
- Match Light and Dark color tokens, typography, spacing, border treatment, and enabled/hover/pressed/disabled/focus states.
- Match the top utility row, icon-only menu trigger, ribbon tabs and command row, pin/collapse and favorites presentation, and panel proportions.
- Design the Welcome cards and recent-file presentation, Items-panel appearance, and Move/Rotate/Resize strip.
- Render the original artwork live in the Rust app and capture it; label design mockups separately from running-app captures.

Use the RB-00 asset manifest as provenance. Add an RB-01 packaged-art manifest recording the Rust destination, source and packaged SHA-256, theme, logical action, dimensions, format, and alpha treatment. If extending the RB-00 manifest, make destination fields additive and retain its original-source facts. Do not edit, retouch, resize, or regenerate the source art. The Rust product must not load resources from `L:\Builder3D`. If a mapped command lacks dedicated art, preserve the documented mapping or record the missing-art decision; do not invent a replacement asset.

Important source facts to reconcile in the design board:

- Python defaults to Dark; Rust preferences currently default to Light.
- The known Light tokens are close; the Rust Dark palette differs materially from Python’s Dark palette.
- The source maps Subdivide to `Simplify.png`, X-Ray to `ExRay.png`, and Subtract to `Subtract.png`.
- The source maps Undo/Redo PNGs while older text describes generated arrows. The root `pin.jpg` and `snap.png` fallback paths are shadowed by the mapped themed Pin/Snap PNGs.
- The owner accepted the default metric floor without the projected contact cue on 2026-10-02. RB-01 records the shell appearance; it does not integrate the viewport renderer or change this decision.

Do not promote `modeler3d.app.shell` or `modeler3d.app.desktop-adapters` beyond their current catalog state. Update catalog facts only if source inspection proves a factual correction.

## 6. Comparison and verification

Build a review board with original and Rust captures side by side. Use overlays/difference views where they help locate layout shifts. State tolerances per region using capture repeatability; do not rely on one global similarity score or mask product regions. Mask only justified operating-system decoration or font rasterization differences.

Run only focused checks needed for the changed Rust visual components and asset loading. Do not run broad Python desktop suites. Do not claim that a capture proves functionality, or that module tests prove visual fidelity. Update each ledger status separately:
- automated verification only when a focused test was run;
- visual status only for inspected, matched evidence;
- interactive status only for actions actually exercised;
- owner acceptance only after explicit owner review.

## 7. Deliverables and exit

Create `evidence/rebuild/rb-01-visual-foundations/` with:

- `report.md`: design decisions, captured states, known differences, blocker status, risk, verification and owner-review questions.
- `capture-board.md` plus inspected original/Rust captures and per-image metadata.
- `asset-manifest.toml`: packaged destination and hash provenance for Rust artwork/branding.
- `verification.md` and `checkpoint.md`.

Update the RB-00 feature ledger with RB-01 evidence without erasing history, append the result and exact next work to `WORK_LOG.md`, and create a new additive G: recovery snapshot. Verify Robocopy result, relative file sets, file count, byte count, and recovery-critical SHA-256 hashes. Exclude only the plan-approved reproducible outputs; do not traverse junctions or modify an existing snapshot. Append the snapshot receipt after copy and state that it postdates the snapshot.

RB-01 exits when the matched board and live original-art rendering are ready for review and the owner explicitly approves a representative visual direction. If capture or review is unavailable, record the blocker and keep the slice pending. Do not treat silence, a successful launch, or a model-generated image as approval. Present the evidence and ask for the owner’s direction after the work is reviewable. Stop before RB-02.

