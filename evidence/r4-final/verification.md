# R4 final snapshot verification

Snapshot: `G:\Builder3D-Rust_snapshot_20261029-155958_R4-final`
Source: `E:\Builder3D-Rust`
Excluded: every `target\` directory

| Check | Source | Snapshot |
| --- | ---: | ---: |
| Files excluding target | 00000131 | 00000131 |
| Bytes excluding target | 000002197215 | 000002197215 |
| Relative paths | 00000131 | 00000131 |
| Missing/extra paths | 00000000 | 00000000 |
| SHA-256 mismatches | 00000000 | 00000000 |

Manifest SHA-256 excluding all `verification.md` files:

- source: `a1d7db148d6ceada9323c31981758d081c051b5e5b2b0b53fe4bb3cd3d9b6ced`
- snapshot: `a1d7db148d6ceada9323c31981758d081c051b5e5b2b0b53fe4bb3cd3d9b6ced`

Verification command: recursive source/snapshot relative-path, byte-total,
per-file SHA-256, and target-exclusion comparison after the additive copy.
The manifest hash is SHA-256 of sorted UTF-8 lines in the form
`relative/path|bytes|lowercase-file-sha256`, excluding every
`verification.md`, with a final newline.
