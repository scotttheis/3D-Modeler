# R6-03 snapshot verification

Verified 2026-10-29 from `E:\Builder3D-Rust` after the R6-03 source,
evidence, and `WORK_LOG.md` changes.

The manifest includes every visible and hidden file below the repository root
except files below `target\` and this self-referential verification file. Each
included source file was compared with the corresponding snapshot file by
relative path, byte length, and SHA-256 digest. The aggregate digest is
SHA-256 over sorted UTF-8 manifest lines in the form
`path|bytes|sha256`.

- Source files: 287
- Snapshot files before copying this verification artifact: 287
- Source bytes: 5,478,276
- Snapshot bytes before copying this verification artifact: 5,478,276
- Hidden files in both manifests: 0
- Missing paths: 0
- Extra paths: 0
- Byte-length mismatches: 0
- SHA-256 mismatches: 0
- Aggregate manifest SHA-256:
  `ad3bcc6e0a06f123cb5a3c429f138f3647e866765c137e25cb74cb36bc1aa7f7`
- `target\` in source: present for local builds; excluded from the snapshot
- `target\` in snapshot: absent

The verification artifact was copied into the snapshot after the comparison so
it does not create a self-referential digest.

Snapshot:

`G:\Builder3D-Rust_snapshot_20261029-211751_R6-03-dialogs`
