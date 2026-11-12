# Checkpoint: RB-02 First Working Application

**Status:** RB-02 implementation and evidence are complete. The owner accepted the current Light/Dark visual baseline and allowed fine-tuning later. RB-03 has not started.

## Objective

Complete the bounded workflow in the existing eframe app: New, Cube, native import, real shaded viewport, camera controls, exact pick, gizmo move, undo/redo, and project save/reopen. Keep L:\Builder3D read-only and stop before RB-03.

## Completed

- Integrated the R5 shaded renderer into the existing eframe window through eframe's adapter/device/queue and egui-wgpu texture presentation.
- Added live physical-pixel resizing, orbit/pan/zoom, geometry ray picking to stable IDs, and translation-gizmo preview/commit/cancel/no-op behavior.
- Added the native import picker, supported format filters, explicit unit selection (default mm), and path preservation on picker cancellation.
- Exercised New, Cube, fixture import, camera gestures, exact pick, gizmo move, undo/redo, save, and reopen in Light and Dark.
- Captured and inspected 25 foreground client screenshots at 1280 x 840 logical/physical pixels, 96 DPI / 100%.
- Recorded fixture provenance, runtime adapter, per-image metadata, workflow observations, verification, and one RB-02 parity feature/audit entry.

## Changed project evidence and implementation

RB-02-edited source/manifests: `Cargo.toml`, `Cargo.lock`, `crates/builder3d-app/Cargo.toml`, `crates/builder3d-app/src/shell.rs`, `crates/builder3d-app/src/dialogs.rs`, `crates/builder3d-app/src/commands.rs`, `crates/builder3d-viewport/src/renderer.rs`, `crates/builder3d-viewport/src/math.rs`, `crates/builder3d-viewport/src/picking.rs`, and `crates/builder3d-viewport/src/adaptive.rs`.

Evidence files include `report.md`, `walkthrough.md`, `capture-board.md`, `capture-metadata.toml`, `fixture-manifest.toml`, `runtime-adapter.txt`, `verification.md`, `checkpoint.md`, 25 PNGs, and two saved `.3dp` projects under this directory. The RB-02 feature/audit entry is appended to `evidence/rebuild/parity-matrix.toml`; the RB-01 audit record is unchanged. `WORK_LOG.md` has the RB-02 entry and will receive the snapshot receipt after copying.

## Verification and runtime

`cargo fmt --all -- --check`, `cargo test --workspace --locked` (203 passed), `cargo check --workspace --locked`, strict workspace Clippy, `cargo deny check`, TOML parsing, and release `cargo bloat` all passed. Cargo Deny reported existing duplicate transitive-version warnings; advisories, bans, licenses, and sources passed. `cargo audit` is unavailable because cargo-audit is not installed. Release executable size is 14,135,808 bytes; `.text` is 11.0 MiB. Source art plus the release executable totals 57,476,773 bytes (54.79 MiB) before installer/runtime sidecars.

The live adapter was Quadro RTX 4000 / Vulkan / NVIDIA driver 581.42. The primary display was 2560 x 1440; the app client was 1280 x 840 at 96 DPI / 100% scale.

## Recovery snapshot

Immutable additive snapshot: `G:\Builder3D-Rust_snapshot_20261031-111700_RB-02-03-complete-workflow`. Verified 2026-10-31 at 11:17:52 MDT. Robocopy exit code 1; source and snapshot each contain 619 files and 100,355,053 bytes. Relative path, size, and per-file SHA-256 differences were 0 at copy time. The verification walk skipped 0 reparse entries; only root target/, root .cache/, and reparse-point entries were excluded. The owner-acceptance updates to WORK_LOG.md, report.md, capture-board.md, verification.md, the parity ledger, and this checkpoint postdate the snapshot; the snapshot contains the preacceptance review state. No application code changed after the snapshot. Existing G: snapshots and `L:\Builder3D` are unchanged.

## Remaining review boundary

The empty-scene floor line crosses the prompt. The live walkthrough exercised STL only and one Windows scale. Selection, camera, and undo history do not persist through project reopen. These are recorded boundaries, not claims of complete application parity.

**Exact next action:** Continue with all of RB-03 by following
[`docs/rb-03-lifecycle-and-file-completion-handoff.md`](../../../docs/rb-03-lifecycle-and-file-completion-handoff.md).
RB-02 visual fine-tuning may happen later, as the owner directed.

No files were staged or committed.

### RB-03 handoff baseline snapshot receipt

Additive snapshot: G:\Builder3D-Rust_snapshot_20261031-115217_RB-03-handoff-baseline. Verified 2026-10-31T11:54:22-06:00 with Robocopy exit code 1. Source and snapshot each contain 620 files and 100,376,505 bytes; relative-path, size, and per-file SHA-256 differences are 0. The verification walk found 0 reparse entries. Repository-root target and .cache were excluded. Recovery-critical hashes are recorded in WORK_LOG.md. This receipt postdates the snapshot.
