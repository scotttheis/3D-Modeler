# R5 snapshot verification

Verified 2026-10-29 from `E:\3D-Modeler` after the final R5 source and
evidence changes.

The manifest includes every visible and hidden file below the repository root
except files below `target\` and this self-referential verification file. Each
included source file was compared with the corresponding snapshot file by
relative path, byte length, and SHA-256 digest. The aggregate digest is
SHA-256 over the sorted UTF-8 manifest lines in the form
`path|bytes|sha256`.

Source manifest:

- files: 274
- bytes: 5,341,286
- aggregate SHA-256:
  `97729e626d223d866da108632218fc2dc373555b94abdec537e1265ed806d096`

All ten additive snapshots produced the same result:

| Snapshot | Files | Bytes | Missing | Extra | SHA-256 mismatches | Aggregate manifest SHA-256 | `target\` present |
| --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| `20261029-190500_R5-01-wgpu-renderer` | 274 | 5,341,286 | 0 | 0 | 0 | `97729e626d223d866da108632218fc2dc373555b94abdec537e1265ed806d096` | no |
| `20261029-190600_R5-02-camera` | 274 | 5,341,286 | 0 | 0 | 0 | `97729e626d223d866da108632218fc2dc373555b94abdec537e1265ed806d096` | no |
| `20261029-190700_R5-03-gizmo` | 274 | 5,341,286 | 0 | 0 | 0 | `97729e626d223d866da108632218fc2dc373555b94abdec537e1265ed806d096` | no |
| `20261029-190800_R5-04-picking` | 274 | 5,341,286 | 0 | 0 | 0 | `97729e626d223d866da108632218fc2dc373555b94abdec537e1265ed806d096` | no |
| `20261029-190900_R5-05-floor` | 274 | 5,341,286 | 0 | 0 | 0 | `97729e626d223d866da108632218fc2dc373555b94abdec537e1265ed806d096` | no |
| `20261029-191000_R5-06-contact-cues` | 274 | 5,341,286 | 0 | 0 | 0 | `97729e626d223d866da108632218fc2dc373555b94abdec537e1265ed806d096` | no |
| `20261029-191100_R5-07-adaptive-tiers` | 274 | 5,341,286 | 0 | 0 | 0 | `97729e626d223d866da108632218fc2dc373555b94abdec537e1265ed806d096` | no |
| `20261029-191200_R5-08-resource-ledger` | 274 | 5,341,286 | 0 | 0 | 0 | `97729e626d223d866da108632218fc2dc373555b94abdec537e1265ed806d096` | no |
| `20261029-191300_R5-09-workload-planner` | 274 | 5,341,286 | 0 | 0 | 0 | `97729e626d223d866da108632218fc2dc373555b94abdec537e1265ed806d096` | no |
| `20261029-191500_R5-final` | 274 | 5,341,286 | 0 | 0 | 0 | `97729e626d223d866da108632218fc2dc373555b94abdec537e1265ed806d096` | no |

The verification file was then copied into each snapshot; it is excluded from
the manifest only to avoid a self-referential hash. The final snapshot is:

`G:\Builder3D-Rust_snapshot_20261029-191500_R5-final`
