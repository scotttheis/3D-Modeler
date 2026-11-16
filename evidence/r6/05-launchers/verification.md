# R6-05 snapshot verification

Verified 2026-10-29 from `E:\3D-Modeler` after the R6-05 source,
launcher, evidence, and `WORK_LOG.md` changes.

The manifest includes every visible and hidden file below the repository root
except files below `target\` and this self-referential verification file. Each
included source file was compared with the corresponding snapshot file by
relative path, byte length, and SHA-256 digest. The aggregate digest is
SHA-256 over sorted UTF-8 manifest lines in the form
`path|bytes|sha256`.

- Source files: 341
- Snapshot files before copying this verification artifact: 341
- Source bytes: 5,515,698
- Snapshot bytes before copying this verification artifact: 5,515,698
- Hidden files in both manifests: 0
- Missing paths: 0
- Extra paths: 0
- Byte-length mismatches: 0
- SHA-256 mismatches: 0
- Aggregate manifest SHA-256:
  `ffc190c0573c6595e8aae959f3238f1a79bebae081b8f33dbf450b816f3b464f`
- `target\` in source: present for local builds; excluded from the snapshot
- `target\` in snapshot: absent

The verification artifact was copied into the snapshot after the comparison so
it does not create a self-referential digest.

Snapshot:

`G:\Builder3D-Rust_snapshot_20261029-212252_R6-05-launchers`
