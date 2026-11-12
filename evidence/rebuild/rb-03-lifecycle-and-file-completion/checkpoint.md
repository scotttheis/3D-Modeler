# Checkpoint: RB-03 Lifecycle, Imports, and Exports

**Status:** RB-03 implementation and engineering work are ready for owner visual review. Rust native lifecycle/import/export/recent workflows, independent format checks, and paired Python-reference operation captures are recorded. Owner visual acceptance is pending; no visual pass is claimed. RB-04 implementation has not started.

## Completed work

- `.3dp` saves use a temporary archive beside the target and atomically replace a prior valid file. Current and legacy schema, future-schema rejection, malformed archives, transforms, meshes, IDs, colors, metadata, and round-trips have focused coverage.
- Save/Open/New/Save As/Exit update path/title/dirty state only after success. Dirty replacement prompts support Save/Discard/Cancel. Stable saved-history checkpoints are covered by a regression for undo, redo, and branching.
- STL/OBJ/PLY/3MF/glTF/GLB imports are implemented. Unitless formats use the six reference units; 3MF and glTF/GLB use authoritative units. Multi-object names, colors, transforms, placement, and grouped undo are covered.
- STL/OBJ/PLY/3MF/glTF/GLB/WRL exports are implemented with selection/visibility behavior, world transforms, supported colors, cancellation preservation, and independent output checks.
- Recent files persist with ten-entry ordering/deduplication, project/model dispatch, previews, fallbacks, and missing/corrupt/unsupported handling.
- Fixed the recent-card column allocation so every entry gets a visible width. Added `recent_grid_allocates_a_visible_width_for_every_column`.

## Verification and native evidence

- Integrated Rust workspace: fmt, check, 228 tests, strict Clippy, deny, metadata, smoke, release build, and both bloat commands passed. `cargo audit` is not installed. The `--workspace` bloat warning and measured default/app targets are documented in `verification.md`.
- Focused Rust app run: 92 tests passed; changed-crate strict Clippy, isolated debug build, native seven-format output verification, and selection matrix passed.
- Focused read-only Python tests: 35 format/project tests and 3 unitless-import cases passed. `L:\Builder3D` Git status matches the saved 116-line baseline. Original QSettings were restored.
- Rust native walkthrough covered lifecycle prompts, all imports/units, all seven exports, selection/visibility, cancel/error preservation, restart-persisted recents, previews, and recent failure states.
- Python reference captures were taken during lifecycle, import, export, cancel, recent, and error actions. The complete review board and individual full-size images are in `visual-comparison.html` and `captures/`.
- The extra old-app per-format export replay could not begin when Windows locked the desktop; no input was sent. The old picker/3MF/overwrite interactions already captured remain in the board. This does not affect the Rust seven-format export verification.

## Current review state

- 32 RB-03 parity rows retain implementation and automated `pass`.
- 30 rows have native interactive `pass`; saved-checkpoint history marker and full filesystem-failure audit remain `partial` because their complete live sequences were not captured.
- Visual and owner-acceptance fields remain `pending` until the owner reviews the paired screenshots. Observed visual differences are recorded in the board and report.
- `L:\Builder3D` was not modified, nothing was staged or committed, and no RB-04 implementation started.

## Recovery snapshot and next action

The verified saved-marker interim snapshot remains unchanged:
`G:\Builder3D-Rust_snapshot_20261031-143631_RB-03-saved-marker`.

The additive RB-03 review snapshot receipt is recorded below. Owner visual
review remains pending, but the latest owner direction is to continue with
**RB-04 — Scene and precise transforms** without waiting on that review. The
scope is selection/sticky selection, visibility, naming, groups, clipboard,
Move/Rotate/Resize, settle, mirror, measure, align, and arrange. The current
RB-04 handoff is `docs/rb-04-through-rb-11-completion-handoff.md`.

## RB-03 additive review snapshot receipt

- Immutable snapshot: `G:\Builder3D-Rust_snapshot_20261031-165222_RB-03-review`.
- Robocopy exit code 1 at 2026-10-31 16:52:25 -06:00. Source and snapshot each contain 901 files / 139,803,911 bytes; relative-path, size, and per-file SHA-256 differences were all 0 before this receipt. Reparse entries: 0 in both trees.
- Repository-root `target/` was present and excluded; `.cache/` was absent and explicitly excluded; reparse entries were excluded with `/XJ`. The complete recovery-critical hash list is in the matching `WORK_LOG.md` receipt.
- This receipt postdates the immutable snapshot. The next implementation slice is **RB-04 — Scene and precise transforms** under the owner's latest direction; its work does not wait on the pending RB-03 visual review. No RB-04 implementation has started.
- Final post-receipt tree comparison: no included relative paths are missing or extra. Exactly `WORK_LOG.md` and `evidence/rebuild/rb-03-lifecycle-and-file-completion/checkpoint.md` differ in size and SHA-256 because their receipts were appended after the copy; no other included file differs. Reparse entries remain 0 in both trees.
