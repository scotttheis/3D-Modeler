# RB-07 source preservation

## Recovery point

- Starting Rust worktree: `E:\Builder3D-Rust` (all files remain uncommitted and
  unstaged; no reset/checkout was used).
- Verified RB-06 source snapshot: `G:\Builder3D-Rust_snapshot_20261101-095500_RB-06-complete`.
- Additive RB-07 baseline: `G:\Builder3D-Rust_snapshot_20261101-132912_RB-07-baseline`.
- Baseline verification: Robocopy exit 1; 2,533 files / 396,966,329 bytes on
  source and snapshot; zero missing/extra paths; zero size/SHA-256 differences;
  zero reparse entries. `target`, `.cache`, `logs`, and `*.log` were excluded.
- The only file present in the current worktree but absent from the RB-06
  snapshot is this slice handoff, `docs/rb-07-appearance-jobs-and-print-handoff.md`.

## Python reference checkout

`L:\Builder3D` is read-only. Its initial Git status contains 116 pre-existing
records: 69 modified tracked paths and 47 untracked paths. These changes are
preserved; no Python source, tests, docs, settings, or generated output was
written by this RB-07 work. Reference tests use the existing venv with
`PYTHONDONTWRITEBYTECODE=1`, `-B`, and pytest cache disabled.

SHA-256 values recorded before testing for behavior-critical reference files:

| File | SHA-256 |
|---|---|
| `src/main_window.py` | `3fc2b1c25a53f6f7d040d3321d331a22d7b8a462f8cb4fcd18a9631b960a7c41` |
| `src/image_color.py` | `ccdf8c5a31040cbdbf4113fcd8f9e91b12fb6a3c0d4b623514b3963788053e8b` |
| `src/print_prep.py` | `7e3ea72e2063ceec7a6b785ddefb427f5c54ddbd52a5952f136a1c0cc8412fa7` |
| `src/slicer_launcher.py` | `dafa677ae8848974a0a95b5f8d2eda2619ee8d3a1a37c0048c3dee5bd529b3a3` |
| `src/operation_manager.py` | `c9c2316b6c3a2005a35f96262316f9be9867a0cf6a2162a0468d42de81ae7244` |
| `tests/test_paint.py` | `3ea8d93d9927310db21f97e9e430836fda4f21a0c8966bbaa712c1705f5fb009` |
| `tests/test_chunk7.py` | `c9d1bd04e087ec3406c10463cb380fabfccc97c8b31d2b317a0b9ebc46c4ec65` |
| `tests/test_slicer_launcher.py` | `1a99867da7ae80446e7ea82850a65e814d7db012a5f8462a2923bdec57f75f46` |
| `tests/test_desktop_actions.py` | `ac3748932f1669496dbdbc5810248ec3837799329bd27017c0cc01e141318cf` |
| `tests/test_adapter_boundaries.py` | `f3bfa2376fb41560d325b8e1b644d97a770e496fc9139c5dc82ae35a6a94adee` |

Python MCP gateway processes observed at baseline are unrelated and were left
running.

## Reused implementation

Rust `image_color`, `operations`, `print_prep`, `print_handoff`, and `slicer`
remain the starting points. Legacy R7 code and evidence are retained as
technical references only; current native reachability and failure behavior
are verified separately in the RB-07 journal.


## RB-07 completion verification

- The verified RB-06 snapshot remained unchanged. The RB-07 additive baseline is `G:\Builder3D-Rust_snapshot_20261101-132912_RB-07-baseline` (2,533 files / 396,966,329 bytes; zero missing/extra paths, size/hash differences, or reparse entries).
- The Python reference `L:\Builder3D` remained read-only. Its Git status still contains 116 pre-existing records. Every behavior-critical file hash above matches its pre-test value. The full test failure is an existing assertion that expects the old `K:\builder3D.png` path; no reference edit was made.
- Accepted and rejected native frames are separated. The accepted set has 117 PNGs; 142 excluded/rejected PNGs and raw attempts remain under `rejected/`.
- Debug executable SHA-256: `0BF7753FE460A6A854906457F86363DDC905B455FA790760B6D9122F6F9CB0D6`. Release SHA-256: `E26C37DE652B6434727AE7F6396012CDF4AA911299857B621D336AFB4DA38448`.
- No files were staged or committed. No changes followed into RB-08.
- Final immutable snapshot: `G:\Builder3D-Rust_snapshot_20261101-162515_RB-07-complete`; receipt is `snapshot-receipt.md`.
