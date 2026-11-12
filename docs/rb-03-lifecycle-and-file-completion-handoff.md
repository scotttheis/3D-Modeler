# RB-03 Lifecycle and File Completion Handoff

**Execution update (2026-10-31):** RB-03 implementation, automated coverage,
integrated Rust gates, native workflow evidence, and its paired visual review
package are recorded. Owner visual acceptance remains pending; two native
interaction rows are partial. This RB-03 implementation handoff is complete.
The current continuation is RB-04 under
[`rb-04-through-rb-11-completion-handoff.md`](rb-04-through-rb-11-completion-handoff.md);
the owner has directed that pending RB-03 review status not block that work.
Do not claim RB-03 owner acceptance. See the RB-03 report and checkpoint for
the evidence boundary.

**Owner-approved starting point:** RB-02 is complete. On 2026-10-31, the owner accepted its current Light/Dark visual baseline and said fine-tuning can happen later. The RB-02 checkpoint records the walkthrough, verification, known limits, approval, and prior G: snapshot.

**Objective:** Finish the full RB-03 lifecycle and file scope in the existing Rust eframe application: project lifecycle, every reference-supported import and export format, unit handling, recent files and previews, and failure/cancel behavior. Do not mark RB-03 complete after implementing only representative formats or a partial workflow.

This handoff follows [the product rebuild plan](product-rebuild-plan.md), [the Rust workspace rules](../AGENTS.md), [the RB-02 checkpoint](../evidence/rebuild/rb-02-first-working-application/checkpoint.md), and the Python reference at L:\Builder3D. That reference remains read-only.

## Step 1: Resume from the accepted RB-02 state

Read these files before changing code:

- E:\AGENTS.md, E:\RustDeveloper.md, and E:\Builder3D-Rust\AGENTS.md
- docs/product-rebuild-plan.md and this handoff
- evidence/rebuild/rb-02-first-working-application/checkpoint.md, report.md, verification.md, and capture-board.md
- evidence/rebuild/parity-matrix.toml and WORK_LOG.md
- The relevant reference docs listed under Step 2

Check the current Git status and preserve the existing tree. At handoff creation, the Rust repository files were untracked; do not stage or commit them, as the owner directed. Keep L:\Builder3D read-only.

Use the verified RB-03 handoff baseline snapshot recorded at the end of this document. It includes the owner-acceptance records and this handoff as they stood before its post-copy receipt. After each bounded implementation patch is complete, make and verify the additive G: recovery snapshot required by the workspace rules before starting the next patch.

Treat the owner-approved RB-02 Light/Dark appearance as the current baseline. Fine-tuning can happen later. RB-03 still needs its own evidence for any lifecycle or file UI it changes.

## Step 2: Map the Python contract and current Rust gaps

Use the rebuild plan as the scope authority. Read the Python project-format, architecture, testing, and Rust rewrite documents, and TODO section R. Trace behavior in the current Python source and its focused tests before copying any behavior:

- L:\Builder3D\docs\project-format.md
- L:\Builder3D\docs\architecture.md
- L:\Builder3D\docs\testing.md
- L:\Builder3D\docs\rust-rewrite.md and L:\Builder3D\TODO.md section R
- L:\Builder3D\src\main_window.py, mesh_ops.py, project_io.py, application_services.py, three_mf.py, gltf_io.py, and wrl_io.py
- Relevant tests, including test_project_files.py, test_project_format.py, test_desktop_actions.py, test_application_services.py, and the format-specific tests

Trace the reference handlers for replacement prompts, New/Open/Save/Save As/exit, recent dispatch, model import, export selection, and recent thumbnails. In particular, check _confirm_replace_scene, _on_new_scene, _on_open, _on_save, _on_save_as, closeEvent, _open_recent, _import_model_file, build_export_objects, _load_recent_thumbnail_payload, and the recent-thumbnail worker.

Then inspect the current Rust seams and tests rather than assuming they already satisfy the contract:

- crates/builder3d-core/src/project.rs and file_io.rs
- crates/builder3d-core/src/three_mf.rs, gltf.rs, and wrl.rs
- crates/builder3d-app/src/commands.rs, dialogs.rs, preferences.rs, ui_state.rs, and shell.rs
- crates/builder3d-core/tests/file_io_golden.rs, gltf_golden.rs, three_mf_golden.rs, and wrl_golden.rs

Write a small requirement-to-test map before implementation. Check existing stable requirement IDs first; update their rows or add only missing rows in evidence/rebuild/parity-matrix.toml. Keep project lifecycle, each file-format contract, units, export selection, recent behavior, and failure behavior distinct.

The reference contract to verify is:

| Contract | Required behavior |
|---|---|
| Project files | .3dp is the project format: a ZIP containing scene.json and one STL mesh entry per scene object or group child. New/Open/Save/Save As operate on projects; Open selects .3dp. Preserve the current schema and migration rules in L:\Builder3D\docs\project-format.md. |
| Import formats | Add imports STL, OBJ, PLY, 3MF, glTF, and GLB. WRL is an export format in the reference, not an import format. |
| Import units | STL/OBJ/PLY use an explicit or saved default unit. The reference choices and scale-to-mm factors are um 0.001, mm 1, cm 10, in 25.4, ft 304.8, and m 1000. The default is mm. 3MF/glTF/GLB carry authoritative units and must not be scaled again using the user override. |
| Imported scene | Preserve the reference's supported names, colors, transforms, and multiple objects. Check placement on Y=0 and framing using the mandatory walkthrough in the rebuild plan. A failed or cancelled import must not leave a partial scene or history entry. |
| Export formats | STL, OBJ, PLY, 3MF, glTF, GLB, and WRL. Verify the output of each format independently. |
| Export contents | Match the reference selection rule: when objects are selected, export selected visible objects; when none are selected, export all visible objects. If no visible object remains, return the reference-style error. Preserve world transforms and color wherever the selected format supports it, and match the reference behavior where a format is lossy. |
| Recent files | Keep the reference's ten-entry ordering, deduplication, persistence, and project/model dispatch. Check successful record points in the Python handlers. Display a real preview when one is available and the reference fallback for missing, unsupported, or corrupt entries. |
| Failure/cancel | Failed or cancelled replacement, save, import, or export preserves the current scene, project path, dirty state, and history. A cancelled overwrite leaves the destination untouched; a failed .3dp save preserves its previous valid file. Match any recent-list change to the reference. |

The current Rust ImportUnit list is known to lack the reference's um choice. Treat this as a known gap to resolve, then continue checking for other gaps. The Rust recent-entry status and ten-entry preference helpers already exist; verify the complete persisted and visible behavior, including thumbnails and dispatch.

Do not widen RB-03 into RB-04 scene selection/organization or later slices.

## Step 3: Complete project lifecycle and .3dp safety

Implement and verify the full New, Open, Save, and Save As lifecycle in the existing app:

1. New creates a clean scene and clears the project path and dirty state.
2. Open loads only a .3dp project and replaces the current scene only after the new file has been validated successfully.
3. Save writes to the current project path; Save As selects a destination and uses .3dp as the project extension according to the reference.
4. The title and dirty indicator follow the current project path and edits.
5. Before replacing a dirty scene or exiting, exercise Save, Discard, and Cancel. A cancelled Save As or failed Save must abort the replacement/exit.
6. Verify native dialog cancellation, overwrite decisions, invalid/corrupt projects, read/write errors, and a prior valid destination file.

Preserve the project contract: current schema 1; unversioned schema 0 migration in memory; reject future schemas with an explanatory error; validate archive, JSON, object data, transforms, colors, IDs, mesh references and mesh contents before replacing the live scene. Preserve the resource limits documented in project-format.md. Save through a temporary archive beside the destination and replace it only after the complete write, so a failed save retains the prior valid file.

Round-trip the scene fields in the project format, including object IDs, transforms, RGBA colors, visibility, metadata, mesh references, and the project/revision fields described by the reference. Test legacy and current fixtures, a future schema, and malformed or incomplete archives. Keep these tests with the existing Rust project tests and fixtures.

## Step 4: Complete model imports and unit handling

Finish the Add/Open model path and native picker for every supported import format: .stl, .obj, .ply, .3mf, .gltf, and .glb.

1. Match the reference's Add behavior and Add-with-units behavior. Keep .3dp project Open distinct from model Add/import.
2. Add the missing um option and all six reference conversions. Keep the default and persisted ambiguous-import unit at mm.
3. Apply a chosen/default unit exactly once to unitless formats. Use the units declared by 3MF, glTF, and GLB without applying the user unit override.
4. Exercise a unitless fixture in both millimetres and inches; check actual dimensions.
5. Exercise multi-object colored 3MF and glTF/GLB fixtures. Check object count, names, colors, transforms, Y=0 placement, and framing against the reference.
6. Cancel the picker and the unit choice. Try malformed and unsupported input. In every failure/cancel case, verify no partial geometry, selection/history, project path, dirty-state, or incorrect recent entry is left behind.

Imports that do succeed should behave as one user operation for the existing command/undo model.

## Step 5: Complete exports

Finish native export dialogs and the seven reference-supported output formats: .stl, .obj, .ply, .3mf, .gltf, .glb, and .wrl.

For each format, export meaningful scene data, then independently reopen or inspect the result. The reference does not import WRL; use a compatible independent reader for that output and record the method. Verify mesh geometry, world-space transforms, and color for formats that support it. Check file naming and extension behavior, overwrite confirmation, and cancellation.

Exercise the reference's selected-visible rule with: no selection, one selected object, multiple selected objects, hidden selected objects, mixed visible/hidden scene, and no visible objects. Confirm export failure or cancellation leaves the scene, selection, project path, and dirty state intact. Confirm overwrite cancellation leaves the destination untouched, and match the reference's destination behavior for failed writes.

## Step 6: Finish recent files and previews

Complete the recent-file experience on the welcome surface:

- Ten entries maximum, newest-first order, deduplication, and persistence after restarting the app.
- Successful project and model operations appear at the same points as in the Python reference.
- A .3dp recent opens as a project; a model recent follows the reference import path. Match the reference's recent-entry and error behavior for WRL, but do not claim that WRL import succeeds.
- Show a valid thumbnail/preview when the reference can produce one. Match the documented fallback for missing files, unsupported formats, corrupt files, and unavailable previews.
- Exercise removal or warning behavior for missing entries as implemented by the reference.
- Opening a failed or stale recent entry must not replace or dirty the current scene.

Keep preview loading responsive using the existing Rust app structure. Match user-visible behavior; do not port Python/Qt implementation details as a runtime dependency.

## Step 7: Add focused contract tests, then run the full engineering gates

Add tests for the contracts changed in Steps 3-6. Assert user-visible data and state, not only helper structure. Cover at least:

- Current and legacy .3dp round trips; rejection of a future or corrupt project; failed-save atomicity.
- Each import format; all six unit conversions; unitless imports in mm and inches; authoritative 3MF/glTF/GLB units; multi-object names/colors/transforms.
- Each of the seven export formats; selected/visible rules; transformed geometry and supported colors; independent output validation.
- Save/discard/cancel and picker cancellation; failed operations preserving scene, path, dirty state, and history; failed .3dp saves preserving a prior valid project and overwrite cancellation preserving its destination.
- Recent-list ordering, deduplication, ten-entry bound, persistence, project/model dispatch, preview success, and fallback states.

For each patch, follow the plan's format, focused check/test, changed-crate Clippy, native exercise, and evidence update gates. At the integrated RB-03 milestone, run from E:\Builder3D-Rust:

~~~powershell
cargo fmt --all -- --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo deny check
cargo metadata --locked --format-version 1 --no-deps
cargo run -p builder3d-app --bin builder3d-app -- --smoke
cargo build --release --locked -p builder3d-app --bin builder3d-app
cargo bloat --release --workspace
cargo bloat --release -p builder3d-app --bin builder3d-app
~~~

Run cargo audit --locked if installed and record if it is unavailable. Validate the parity ledger and any manifests as TOML, check requirement IDs and evidence links, and record exact commands, exit results, skipped work, and failures. A passing smoke check does not replace interactive verification.

## Step 8: Complete the native walkthrough and relevant Light/Dark evidence

Use the live Rust app for the mandatory file journey in section 9 of the rebuild plan: import a unitless model in mm and inches; import multi-object colored 3MF and glTF/GLB; check dimensions, Y=0 placement, names, colors, and framing; edit the imported scene; export every supported output; then independently verify the results.

Also exercise the project lifecycle, dirty prompts, failures/cancellations, restart-persisted recents, previews, and missing/unsupported/corrupt recent entries in the visible native app. Record actual steps, before/after state, build identity, and results. Keep automated tests and live GUI results distinct.

For RB-03 UI changes, capture the affected surfaces in Light and Dark and compare against the read-only Python reference with the same fixture and UI state. Use the plan's 1280 x 840 logical-pixel comparison size and record actual client size, physical size, DPI/display scale, theme, and build identity. Capture the Python and Rust applications sequentially: open one, take the captures, close it, then open the other. Inspect each saved image and record differences; do not claim visual pass from a smoke test or an unreviewed capture.

Follow L:\Builder3D\docs\testing.md; use focused reference tests where applicable. Do not run the broad Python desktop suite or run its OpenGL widget with QT_QPA_PLATFORM=offscreen.

## Step 9: Close RB-03 evidence and hand off to RB-04

Store RB-03 results under evidence/rebuild/rb-03-lifecycle-and-file-completion/. Include a concise report, verification record, checkpoint, relevant capture metadata/images, and a format/unit results table. Update:

- evidence/rebuild/parity-matrix.toml with distinct lifecycle, import/unit, export, recent/preview, failure, automated, interactive, visual, and owner-review states. Keep pending or unverified work visible.
- docs/component-catalog.toml only when changed component IDs or contracts require it.
- WORK_LOG.md with actual changes, commands/results, live walkthrough, known limits, and recovery snapshot.
- The checkpoint with completed work, changed files, verification evidence, remaining defects, snapshot path, and the exact next RB-04 action.

Make and verify an additive final G: snapshot after RB-03 evidence is complete. Include the Git metadata, source, manifests, tests, docs, and evidence; exclude the repository-root target, repository-root .cache, and reparse-point entries as specified by the rebuild plan. Record the absolute path, timestamp, copy result, file count, byte count, relative-path/size/SHA-256 comparison, exclusions, and any recovery-critical hashes. Do not overwrite or modify an existing G: snapshot.

RB-03 is complete only when all the lifecycle/import/export/recent/failure requirements above have implementation, focused automated coverage, required native walkthrough evidence, relevant visual evidence, an accurate parity ledger, and a verified recovery snapshot. Then stop at RB-03 and report the exact next slice as RB-04; do not pull RB-04 implementation into this work.

Do not stage or commit.

## Handoff baseline snapshot

Path: G:\Builder3D-Rust_snapshot_20261031-115217_RB-03-handoff-baseline

Verified 2026-10-31T11:54:22-06:00. Robocopy exit code 1; source and snapshot each contain 620 files and 100,376,505 bytes. Relative-path, file-size, and per-file SHA-256 differences are all 0. The copy excluded repository-root target, repository-root .cache, and reparse-point entries; the verification walk found 0 reparse entries. The snapshot includes the pre-receipt handoff state and intentionally predates this receipt.

Recovery-critical hashes at copy time are recorded in WORK_LOG.md.
