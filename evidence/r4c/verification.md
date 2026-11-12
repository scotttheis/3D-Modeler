# R4c snapshot verification

Snapshot: `G:\Builder3D-Rust_snapshot_20261029-155900_R4c-behavior-final`
Source: `E:\Builder3D-Rust`
Excluded: every `target\` directory

| Check | Source | Snapshot |
| --- | ---: | ---: |
| Files excluding target | 00000128 | 00000128 |
| Bytes excluding target | 000002189406 | 000002189406 |
| Relative paths | 00000128 | 00000128 |
| Missing/extra paths | 00000000 | 00000000 |
| SHA-256 mismatches | 00000000 | 00000000 |

Manifest SHA-256 excluding all `verification.md` files:

- source: `099e62fefd2a59a2992aa4106d5a513cf485ccb48ff8e536b33048237113a7d4`
- snapshot: `099e62fefd2a59a2992aa4106d5a513cf485ccb48ff8e536b33048237113a7d4`

Verification command: recursive source/snapshot relative-path, byte-total,
per-file SHA-256, and target-exclusion comparison after the additive copy.
