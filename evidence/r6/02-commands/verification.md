# R6-02 snapshot verification

Verified 2026-10-29 from `E:\Builder3D-Rust` after the R6-02 source,
evidence, and `WORK_LOG.md` changes.

The manifest includes every visible and hidden file below the repository root
except files below `target\` and this self-referential verification file. Each
included source file was compared with the corresponding snapshot file by
relative path, byte length, and SHA-256 digest. The aggregate digest is
SHA-256 over sorted UTF-8 manifest lines in the form
`path|bytes|sha256`.

- Source files: 283
- Snapshot files before copying this verification artifact: 283
- Source bytes: 5,460,005
- Snapshot bytes before copying this verification artifact: 5,460,005
- Hidden files in both manifests: 0
- Missing paths: 0
- Extra paths: 0
- Byte-length mismatches: 0
- SHA-256 mismatches: 0
- Aggregate manifest SHA-256:
  `bcfae2220c48f76d6e7525653612141926eeced490154a4f837a3c9959a9174b`
- `target\` in source: present for local builds; excluded from the snapshot
- `target\` in snapshot: absent

The verification artifact was copied into the snapshot after the comparison so
it does not create a self-referential digest.

Snapshot:

`G:\Builder3D-Rust_snapshot_20261029-211441_R6-02-commands`
