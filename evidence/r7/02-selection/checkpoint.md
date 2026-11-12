# R7-02 checkpoint

- **Objective:** continue the owner-approved conversion-first plan with the
  next command-surface follow-up, without entering shell polish.
- **Completed:** implemented and tested
  `builder3d.app.commands.selection`; updated the component catalog and R7
  evidence.
- **Changed source:** `crates/builder3d-app/src/commands.rs` and
  `crates/builder3d-app/src/shell.rs`.
- **Changed documentation/evidence:**
  `docs/component-catalog.toml` and `evidence/r7/02-selection/`.
- **Checks:** formatting, workspace check/tests, clippy, cargo-deny, metadata,
  catalog parse/path validation, and native smoke all pass. See
  `verification.md` for the exact commands.
- **Baseline recovery snapshot:**
  `G:\Builder3D-Rust_snapshot_20261030-090808_R7-02-selection-baseline`
  — 263 files and 1,632,683 bytes, with excluded `target`, `evidence`, and
  `WORK_LOG.md`; source and snapshot counts/bytes match.
- **Final recovery snapshot:**
  `G:\Builder3D-Rust_snapshot_20261030-091210_R7-02-selection-final`
  — 263 files and 1,639,456 bytes, created after this checkpoint and
  verified with matching source/snapshot counts and bytes; `target`,
  `evidence`, and `WORK_LOG.md` were excluded.
- **Next conversion slice:** `builder3d.app.commands.transforms`, covering
  the remaining object-transform workflow before print-preparation and
  background workflow gaps.
- **Recovery/continuation:** after the final snapshot, reread this checkpoint,
  `WORK_LOG.md`, current Git status, and the workspace instructions before the
  next R7 slice. Do not modify the Python checkout or existing snapshots.
