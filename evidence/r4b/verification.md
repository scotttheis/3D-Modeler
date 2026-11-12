# R4b snapshot verification

Snapshot: `G:\Builder3D-Rust_snapshot_20261029-155700_R4b-catalog-final`
Source: `E:\Builder3D-Rust`
Excluded: every `target\` directory

| Check | Source | Snapshot |
| --- | ---: | ---: |
| Files excluding target | 00000125 | 00000125 |
| Bytes excluding target | 000002183210 | 000002183210 |
| Relative paths | 00000125 | 00000125 |
| Missing/extra paths | 00000000 | 00000000 |
| SHA-256 mismatches | 00000000 | 00000000 |

Manifest SHA-256 excluding all `verification.md` files:

- source: `c26807adcc6dc41aefed5ccea9254fcfeab358663d6837a85495d537d470aa70`
- snapshot: `c26807adcc6dc41aefed5ccea9254fcfeab358663d6837a85495d537d470aa70`

Verification command: recursive source/snapshot relative-path, byte-total,
per-file SHA-256, and target-exclusion comparison after the additive copy.
