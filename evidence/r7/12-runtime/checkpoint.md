# R7-12 checkpoint

- **Objective:** convert the platform runtime path, process, and startup-guard
  seams from the Python reference into testable Rust.
- **Completed:** added `builder3d-platform::RuntimePaths`, descriptor
  resolution, read-only process liveness, and a typed offscreen guard.
- **Changed source:** `crates/builder3d-platform/src/runtime.rs`,
  `crates/builder3d-platform/src/lib.rs`, and the component catalog/evidence.
- **Recovery baseline:**
  `G:\Builder3D-Rust_snapshot_20261030-100300_R7-12-runtime-baseline`
  — 164 files and 1,605,212 bytes, excluding `target`, `evidence`, and
  `WORK_LOG.md`; source and snapshot counts/bytes match.
- **Final recovery snapshot:**
  `G:\Builder3D-Rust_snapshot_20261030-100600_R7-12-runtime-final`
  — 165 files and 1,616,429 bytes, excluding `target`, `evidence`, and
  `WORK_LOG.md`; source and snapshot counts/bytes match.
- No Python source, R1–R6 evidence, or existing snapshot was modified. No
  commit was created.
