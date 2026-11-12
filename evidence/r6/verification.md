# R6 final snapshot verification

Verified 2026-10-29 from `E:\Builder3D-Rust` after the complete R6 source,
packaging, evidence, checkpoint, and `WORK_LOG.md` changes.

The manifest includes every visible and hidden file below the repository root
except files below `target\` and this self-referential verification file. Each
included source file was compared with the corresponding final snapshot file
by relative path, byte length, and SHA-256 digest. The aggregate digest is
SHA-256 over sorted UTF-8 manifest lines in the form
`path|bytes|sha256` without a trailing newline.

- Source files: 355
- Snapshot files before copying this verification artifact: 355
- Source bytes: 5,549,105
- Snapshot bytes before copying this verification artifact: 5,549,105
- Hidden files in both manifests: 0
- Missing paths: 0
- Extra paths: 0
- Byte-length mismatches: 0
- SHA-256 mismatches: 0
- Aggregate manifest SHA-256:
  `3694d31e13859a085c79e0534ebc4ee4e14b9bb53d8dadd677a5d7bf39de71f7`
- `target\` in source: present for local builds; excluded from the snapshot
- `target\` in snapshot: absent

The verification artifact was copied into the snapshot after the comparison so
it does not create a self-referential digest.

Final snapshot:

`G:\Builder3D-Rust_snapshot_20261029-214752_R6-final`
