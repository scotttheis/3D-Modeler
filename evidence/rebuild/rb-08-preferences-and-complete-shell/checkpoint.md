# RB-08 In-Progress Checkpoint

**Date:** 2026-11-02  
**Status:** implementation and automated verification pass; RB-08 native Rust
interaction and final snapshot remain pending. By the owner's later
2026-11-02 direction, work may proceed to RB-09; RB-08 remains open and is not
accepted.

## Current verified state

- Source remains the current `E:\Builder3D-Rust` worktree. No files are staged,
  committed, reset, restored from RB-07, or discarded.
- The pre-edit RB-08 snapshot on G: is
  `G:\Builder3D-Rust_snapshot_20261102-085830_RB-08-baseline`; its receipt
  records 3,082 files / 577,354,485 bytes and zero path, size, SHA-256 or
  reparse-point differences.
- `L:\Builder3D` is read-only. Its source hash is unchanged and its short Git
  status remains 116 records. The accepted Python comparison is the corrected
  isolated run `native/python-live-baseline-20261102-094708/` with 50 captures.
- RB-08 code is in place. Rust focused tests pass 178 (app 159, platform 19);
  workspace tests pass 314. Twelve focused Python reference tests pass.
  Verification records all ten documented gate command results and the
  `cargo audit` availability limit.
- The Release app is `target\release\builder3d-app.exe`, SHA-256
  `db406708cf19efe9ed62c3c53d02684dc49c4458d44c59bd940e6bc45ca81f75`,
  15,653,888 bytes. Its isolated settings file imported Dark theme, snapping,
  and ribbon preferences from the existing registry values.

## Native status and next action

The Release window is PID 27176 in
`native/rust-live-final-20261102-101724/`. Windows `LockApp` PID 9808 owns the
foreground. The runner guard was updated and verified to refuse input/capture
while the lock screen is active. The one black diagnostic screenshot and its
hash are excluded from accepted evidence. AccessKit/UIA exposes 38 welcome
descendants, 36 named, including 27 named buttons, but keyboard focus and
visible focus were not verified in the locked session.

The original stop-before-RB-09 instruction was superseded by the owner. Do not
report RB-08 as closed or use RB-09 evidence as a substitute for its missing
native matrix or completion snapshot. If native work resumes, continue with
paired Light/Dark shell captures
at 980×640, 1280×840, 1600×900 and maximized at 96 DPI / 100%; verify restart,
settings/migration, ribbon/menu/dialog, keyboard/text-field/modal, accessibility
focus and welcome-preview cases. Then complete the route ledger, parity matrix,
component catalog, report, verification, walkthrough, work log and checkpoint;
create and independently verify the additive RB-08 completion snapshot on G:.
The current implementation continuation is RB-09 under
`docs/rb-09-shared-ai-and-regression-handoff.md`.
