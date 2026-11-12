# R8 checkpoint — presentation parity polish

- Baseline: `G:\Builder3D-Rust_snapshot_20261030-122209_R8-01-shell-final`
  (169 files, 1,726,142 bytes; source and snapshot totals matched).
- Objective: remove the remaining small native presentation mismatches found
  during the first R8 live inspection.
- Completed: disabled object-panel actions, visible camera-mode cue, compact
  status transform summary, and a real unsupported-recent-path test.
- Focused verification: format, locked app check/tests, app Clippy, and native
  launch all passed; 61 app tests remain green.
- Remaining: run the complete locked workspace verification matrix, complete
  the catalog/coverage audit, update the final R8 evidence/log, and create the
  verified additive final G: recovery snapshot.
- Constraints: Python source, existing snapshots, and commits remain
  untouched.
