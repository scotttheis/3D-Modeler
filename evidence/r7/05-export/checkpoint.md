# R7-05 checkpoint

- **Objective:** complete the application export selection and format-routing
  seam without beginning visual polish.
- **Completed:** implemented `builder3d.app.export`, updated command routing,
  catalog, and evidence.
- **Changed source:** `crates/builder3d-app/src/export.rs`,
  `crates/builder3d-app/src/commands.rs`, and
  `crates/builder3d-app/src/lib.rs`.
- **Changed documentation/evidence:**
  `docs/component-catalog.toml` and `evidence/r7/05-export/`.
- **Checks:** format, locked workspace check/tests (147 tests plus doctests),
  clippy, cargo-deny, locked metadata, catalog parse/path validation, and
  native smoke all pass. See `verification.md` for exact gate results.
- **Baseline recovery snapshot:**
  `G:\Builder3D-Rust_snapshot_20261030-092520_R7-05-export-baseline`
  — 265 files and 1,687,078 bytes, excluding `target`, `evidence`, and
  `WORK_LOG.md`; source and snapshot counts/bytes match.
- **Final recovery snapshot:**
  `G:\Builder3D-Rust_snapshot_20261030-092745_R7-05-export-final`
  — 158 files and 1,531,806 bytes, excluding `target`, `evidence`, and
  `WORK_LOG.md`; source and snapshot counts/bytes match.
- **Next conversion slice:** `builder3d.app.print-prep`.
- **Recovery/continuation:** reread this checkpoint, `WORK_LOG.md`, current Git
  status, and the workspace instructions after the final snapshot. Do not
  modify the Python checkout or existing snapshots.
