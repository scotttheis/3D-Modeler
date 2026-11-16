# R4a snapshot verification

Snapshot: `G:\Builder3D-Rust_snapshot_20261029-155500_R4a-protocol-final`
Source: `E:\3D-Modeler`
Excluded: every `target\` directory

The numeric fields use fixed-width values so this record can be updated after
the additive copy without changing the measured byte total.

| Check | Source | Snapshot |
| --- | ---: | ---: |
| Files excluding target | 00000122 | 00000122 |
| Bytes excluding target | 000002177988 | 000002177988 |
| Relative paths | 00000122 | 00000122 |
| Missing/extra paths | 00000000 | 00000000 |
| SHA-256 mismatches | 00000000 | 00000000 |

Manifest SHA-256 excluding all `verification.md` files:

- source: `dc32ee41868d8daadd37d104924eac8c3761e02a3ede4b258b72f41073ff5a4d`
- snapshot: `dc32ee41868d8daadd37d104924eac8c3761e02a3ede4b258b72f41073ff5a4d`

Verification command: recursive source/snapshot relative-path, byte-total,
per-file SHA-256, and target-exclusion comparison after the copy.
