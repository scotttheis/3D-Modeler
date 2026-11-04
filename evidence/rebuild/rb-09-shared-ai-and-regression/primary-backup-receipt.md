# RB-09 Primary G: Backup Receipt

- Source, active Rust worktree: `E:\Builder3D-Rust`.
- Destination, new additive snapshot: `G:\Builder3D-Rust_snapshot_20261103-111729_RB-09-paused-e-primary`.
- Backup policy: `G:\` is the primary recovery-backup destination. Removable USB copies are occasional one-offs unless the owner requests otherwise. The current one-off copy is `L:\Builder3D-Rust_snapshot_20261103-103227_RB-09-paused`; see `usb-backup-receipt.md`. It is not a working tree.
- `L:\Builder3D` remains the separate read-only Python reference/shipping build.
- Copy command:
  `robocopy E:\Builder3D-Rust G:\Builder3D-Rust_snapshot_20261103-111729_RB-09-paused-e-primary /E /COPY:DAT /DCOPY:DAT /XJ /XD E:\Builder3D-Rust\target E:\Builder3D-Rust\.cache E:\Builder3D-Rust\logs /R:1 /W:1 /NP /NFL /NDL`.
- Scope: project source, documentation, evidence, lockfiles, and Git metadata. The repository-root `target`, `.cache`, and `logs` directories and reparse traversal are excluded. Nested RB-09 evidence logs are included; no `*.log` exclusion was used.
- The destination did not exist before this copy. No existing G: snapshot was replaced or changed.

## Copy and verification

- Robocopy exit code: `1` (files copied; no failures).
- Source and G: snapshot included files / bytes: `3291` / `584916627` each.
- Missing / extra relative paths: `00` / `00`.
- Size or SHA-256 mismatches: `0`.
- Reparse entries in source included tree / G: snapshot: `0` / `0`.
- Independent verification used `verify_rb09_snapshot.ps1`, which compares every included relative path, byte length, and SHA-256.

This snapshot captures the paused RB-09 state. The native live-client gate remains unverified, the interface and floor gaps remain open, the separate RB-08 native evidence remains pending, and RB-10 has not started. `E:\Builder3D-Rust` remains the only active Rust worktree.
