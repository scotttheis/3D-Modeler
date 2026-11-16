# R6-01 snapshot verification

Verified 2026-10-29 from `E:\3D-Modeler` after the R6-01 source,
evidence, and `WORK_LOG.md` changes.

The manifest includes every visible and hidden file below the repository root
except files below `target\` and this self-referential verification file. Each
included source file was compared with the corresponding snapshot file by
relative path, byte length, and SHA-256 digest. The aggregate digest is
SHA-256 over sorted UTF-8 manifest lines in the form
`path|bytes|sha256`.

- Source files: 280
- Snapshot files before copying this verification artifact: 280
- Source bytes: 5,442,199
- Snapshot bytes before copying this verification artifact: 5,442,199
- Hidden files in both manifests: 0
- Missing paths: 0
- Extra paths: 0
- Byte-length mismatches: 0
- SHA-256 mismatches: 0
- Aggregate manifest SHA-256:
  `785238a1cc43e9fb42537c817a6244785d979b0f8642b9de2c01c85c0553e520`
- `target\` in source: present for local builds; excluded from the snapshot
- `target\` in snapshot: absent

The verification artifact was copied into the snapshot after the comparison so
it does not create a self-referential digest.

Snapshot:

`G:\Builder3D-Rust_snapshot_20261029-211137_R6-01-ribbon`
