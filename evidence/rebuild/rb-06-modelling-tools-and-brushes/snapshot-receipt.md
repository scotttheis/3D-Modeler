# RB-06 verified snapshot receipt

Snapshot target: `G:\Builder3D-Rust_snapshot_20261101-095500_RB-06-complete`  
Source: `E:\Builder3D-Rust`  
Scope: additive RB-06 completion snapshot; existing G: snapshots were not modified.

## Copy contract

Robocopy used `/E /COPY:DAT /DCOPY:DAT /R:1 /W:1 /XJ`, excluding the repository
`target`, `.cache`, and `logs` directories plus `*.log` files. Reparse points
were not followed. A Robocopy exit code of 1 means files were copied with no
failure for this snapshot operation.

## Final verification

The staging copy was measured before this receipt was finalized; the final copy
will synchronize this receipt and the matching completion-record edits. The
final state must show equal source and snapshot file counts/bytes, zero missing
or extra relative paths, zero file-size differences, zero SHA-256 differences,
and zero reparse entries. No source changes follow the final receipt/copy pair.

- Robocopy exit code: 1 (normal copy with no failures)
- Included files: 2,532 source / 2,532 snapshot
- Included bytes: 396,942,394 source / 396,942,394 snapshot
- Missing / extra / size differences: 0 / 0 / 0
- SHA-256 differences: 0
- Reparse entries: 0 source / 0 snapshot
- Excluded content: `target`, `.cache`, `logs`, `*.log`

The final comparison was repeated after the receipt synchronization copy. The
pre-receipt staging comparison was also zero-difference at 396,941,767 bytes;
the finalized receipt copy is the immutable result recorded here.

This receipt is part of the RB-06 evidence package and is not a Git commit or
staging action.
