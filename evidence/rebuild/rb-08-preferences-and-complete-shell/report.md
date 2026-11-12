# RB-08 Preferences and Complete Shell Report

**Date:** 2026-11-02  
**Status:** RB-08 implementation and automated verification pass. The Rust
interactive matrix is waiting for the Windows desktop to be unlocked; no
Rust native captures from the locked session are accepted. Visual review and
owner acceptance remain pending. The owner later authorized proceeding to
RB-09; RB-08's native matrix and completion snapshot remain open.

## Delivered

- Reused the existing Rust shell, settings dialog, preview worker, ribbon
  configuration, and shortcuts. Added independent per-field JSON recovery,
  preservation of unknown settings, Python key aliases, and a read-only
  Windows QSettings/registry importer. A current JSON value wins over legacy
  data, including an intentionally empty recent list. Malformed settings are
  preserved before the first recovery write.
- Added Settings controls for preferred slicer selection/browse/clear,
  non-destructive Clear Recent Files, and About. Print Prep can remember a
  slicer only by explicit selection. A manually selected executable is
  canonicalized and labeled without launching it.
- Completed the missing file/exit shortcuts, dirty-document guards, text and
  modal focus precedence, dialog Escape ordering, accessible names and
  keyboard focus feedback. Enabled the eframe AccessKit Windows provider.
- Reused the bounded asynchronous recent-preview worker and its existing
  missing, corrupt and unsupported fallbacks. Clear Recent Files invalidates
  stale preview results before they can repopulate the list.
- Preserved the Python light accent and adjusted the filled Rust light-theme
  Print control to meet the tested 4.5:1 normal/hover text contrast. Dark
  palette values remain unchanged.

## Verification

- Python focused reference: **12 passed**. The corrected isolated reference
  run has **50 accepted PNGs** under
  `native/python-live-baseline-20261102-094708/captures/`.
- Rust focused tests: **178 passed** (app 159, platform 19). Workspace:
  **314 passed**, zero failures.
- All ten documented Rust commands exited successfully. Release binary:
  15,653,888 bytes; SHA-256
  `db406708cf19efe9ed62c3c53d02684dc49c4458d44c59bd940e6bc45ca81f75`.
  `cargo audit` is unavailable; the `cargo bloat --workspace` CLI warning is
  recorded in `verification.md`.
- The final Rust process imported Dark mode, snapping, and ribbon preferences
  into the isolated JSON file. Its AccessKit/UIA tree reports 38 descendants,
  36 with names, and all 27 buttons named. The only screenshot attempt was
  black because `LockApp` owned the foreground; it is recorded as rejected.

See `verification.md`, `requirement-test-map.md`, `paired-comparison-board.md`,
`initial-button-ledger.md`, and `rejected-attempts.md` for test symbols,
route-level evidence, and capture adjudication.

## Limits and recovery

- The Rust live Light/Dark, size/scale, restart, keyboard, text-field, modal,
  focus, picker and preview-fallback matrix is pending desktop unlock. No
  locked-session capture is counted as native product evidence.
- Only Windows 11 at 96 DPI / 100% is currently available. No alternate
  display scale or external screen reader has been tested.
- The accepted Rust release process is identified by the hash above. The
  additive RB-08 baseline is
  `G:\Builder3D-Rust_snapshot_20261102-085830_RB-08-baseline`; the final
  completion snapshot and receipt remain pending.
- `L:\Builder3D` remains read-only. The product has not been promoted or
  accepted. RB-09 implementation is the owner-authorized next slice; this
  does not close RB-08 or provide its missing native evidence.
