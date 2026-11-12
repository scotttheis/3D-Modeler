# RB-06 checkpoint — inventory and recovery baseline

Date: 2026-11-01  
State: inventory complete; implementation ready to begin  
Next bounded subtask: repair command routing and persistent modelling-tool interaction.

## Recovery baseline

- Snapshot: `G:\Builder3D-Rust_snapshot_20261101-082535_RB-06-baseline`
- Source included files: 2,433
- Snapshot included files: 2,433
- Source included bytes: 376,187,014
- Snapshot included bytes: 376,187,014
- Missing / extra / different: 0 / 0 / 0
- Reparse points: 0 in source and snapshot
- Excluded and absent from snapshot: `target`, `.cache`, `logs`, `*.log`
- Robocopy status: 1 (files copied, no failures)

## Scope guard

- `L:\Builder3D` was read for behavior and test contracts only; no Python file
  was written, generated, or formatted.
- RB-07 work is out of scope. Paint's basic reachability is included only
  because its buttons are visible in the shell; richer paint behavior remains
  a later boundary.
- No Git staging or commit is permitted for this run.

## Evidence rule

Each implementation subtask will receive focused Rust tests, native OS-input
evidence where applicable, and an additive `G:\Builder3D-Rust_snapshot_*`
checkpoint before the next subtask. The final snapshot will be created only
after all verification artifacts are complete.

## Completion checkpoint

Implementation, automated verification, paired native evidence, and the full
button audit are now complete. Primitive/Torus, Repair, Simplify, Subdivide,
Smooth, Normalize, persistent Grab/Crease/Smooth Brush/Blob strokes, Boolean
Union/Intersect/Subtract, Emboss, Hollow, and the visible Object/Paint/View/
Items/dialog routes are covered by the implementation and evidence package.

- Rust workspace: 271 locked tests passed; format, check, all-feature clippy,
  deny, metadata, release build, bloat, and debug/release smoke passed.
- Python reference: 272 tests passed with two existing trimesh warnings; Python
  remained read-only.
- Native evidence: 6 Python baseline records and 72 Rust records, 36 Light /
  36 Dark, at 1280 x 840 and 96 DPI; 78 accepted PNGs total.
- Final Rust native debug SHA-256:
  `82284b6fd85221df3a804790cafb1171c469c5cb614460cb73199068ac551d14`.
- Final Light/Dark Subtract frames report `Subtract: 10496 face(s)`; no app
  process remained after replay.
- Audit and disposition: [initial-button-ledger.md](initial-button-ledger.md).
- Report, map, walkthrough, pairing, and rejection history are in this
  directory. No Python source was written and no Git staging or commit was
  performed.

Final additive snapshot:
`G:\Builder3D-Rust_snapshot_20261101-095500_RB-06-complete`. The staging
measurement is 2,532 included files / 396,942,394 bytes on both sides, with zero
missing, extra, size, SHA-256, and reparse differences. The finalized receipt
is copied once more before the final zero-difference verification; no source
changes follow that copy. RB-07 is explicitly out of scope.
