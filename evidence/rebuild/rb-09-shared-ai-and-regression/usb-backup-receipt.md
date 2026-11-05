# RB-09 USB Backup Receipt

- Source: `E:\Builder3D-Rust`.
- Destination: `L:\Builder3D-Rust_snapshot_20261103-103227_RB-09-paused`.
- Target device: USB `Generic STORAGE DEVICE`, volume `16GBMICROSD`, FAT32.
- Free space before copy: 4,755,218,432 bytes. The included source tree was
  584,907,453 bytes before this receipt was added; its largest included file was
  67,108,865 bytes, below FAT32's per-file limit.
- Copy command:
  `robocopy E:\Builder3D-Rust L:\Builder3D-Rust_snapshot_20261103-103227_RB-09-paused /E /COPY:DAT /DCOPY:DAT /XJ /XD E:\Builder3D-Rust\target E:\Builder3D-Rust\.cache E:\Builder3D-Rust\logs /R:1 /W:1 /NP /NFL /NDL`.
- Exclusions: repository-root `target`, `.cache`, and `logs`, plus reparse
  points/junction traversal. Nested RB-09 evidence logs are included. No
  `*.log` exclusion was used.
- The existing `L:\Builder3D` directory and all existing USB contents are left
  untouched; the backup is a new sibling directory.
- Independent check: `verify_rb09_snapshot.ps1` compares every included
  relative path, byte length, and SHA-256 and counts reparse entries.

## Copy and verification

- Robocopy exit code: 1 (files copied; no failures).
- Source and USB snapshot included files / bytes: `3290` / `584909079` each.
- Missing / extra relative paths: 0 / 0.
- Size or SHA-256 mismatches: 0.
- Reparse entries in source included tree / USB snapshot: 0 / 0.

This is a paused-state backup. The Rust project remains on E: as the active
working copy. The backup does not include the generated `target` build tree;
the Release binaries can be rebuilt from the backed-up source and lockfile.
