# RB-07 checkpoint

**Slice:** RB-07 Appearance, Jobs, and Print Handoff  
**Boundary status:** Implementation and automated verification pass; native interaction pass with the documented Light-theme Rust cancellation limitation; visual review and owner acceptance pending.  
**Date:** 2026-11-01

- Rust workspace tests: 301 passed. Focused counts: app 150, core 88, platform 15, viewport 34.
- Python focused reference: 103 passed. Full suite: 785 passed, 1 pre-existing read-only machine-path failure, two existing trimesh warnings.
- Documented Rust quality gates: 10/10 passed. cargo audit is not installed.
- Accepted native evidence: 117 PNGs: Python 34, Rust 82, Bambu Studio 1. Rust debug/release executable hashes and per-frame image hashes are recorded in this evidence directory.
- Actual installed Bambu Studio opened the independently validated staged 12-triangle STL.
- Additive RB-07 baseline: G:\Builder3D-Rust_snapshot_20261101-132912_RB-07-baseline.
- Final verified snapshot: G:\Builder3D-Rust_snapshot_20261101-162515_RB-07-complete; see snapshot-receipt.md.
- L:\Builder3D remained read-only; its 116 pre-existing status records and recorded source hashes are preserved. Nothing was staged or committed.

Known limits: Rust Light Cancel did not reach a confirmed native cancelled frame; Python both-theme and Rust Dark native cancellation plus Rust shell cancellation tests pass. Missing/unreadable/oversized/unsupported image errors, deliberate stale-result and busy cases, and slicer launch failure are automated-only. Python native Print Prep reports were captured manually after the runner's QAction dispatch failed; Python external slicer launch has automated adapter coverage only. Visual review and owner acceptance remain pending.

**Next slice: RB-08.** Stop at this boundary; no RB-08 implementation has started.

