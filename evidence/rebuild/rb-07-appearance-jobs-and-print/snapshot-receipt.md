# RB-07 snapshot receipt

## Final immutable snapshot

- Path: `G:\Builder3D-Rust_snapshot_20261101-162515_RB-07-complete`
- First copy and comparison: Robocopy exit code 1; 2,873 included files / 574,292,879 bytes in source and snapshot; zero missing/extra paths, size differences, or SHA-256 differences; zero reparse entries. This was the copy before the receipt was written.
- Final receipt-inclusive synchronization: Robocopy exit code 1; 2,874 included files / 574294626 bytes in both source and snapshot; zero missing/extra paths, size differences, or SHA-256 differences; zero reparse entries. The final per-file SHA-256 comparison ran after the receipt was synchronized.
- Exclusions: directories named target, .cache, and logs; files matching *.log; reparse points/junctions.
- Copy method: Robocopy /E /COPY:DAT /DCOPY:DAT /XJ /XD target .cache logs /XF *.log. The final sync copied the receipt and did not mirror or delete snapshot content.

## Baseline references

- Verified RB-06 snapshot: `G:\Builder3D-Rust_snapshot_20261101-095500_RB-06-complete`.
- Additive RB-07 baseline: `G:\Builder3D-Rust_snapshot_20261101-132912_RB-07-baseline`; 2,533 files / 396,966,329 bytes; zero missing/extra paths, size/SHA-256 differences, or reparse entries.

The final snapshot is the stopping checkpoint for RB-07. No RB-08 implementation, staging, or commit is included.
