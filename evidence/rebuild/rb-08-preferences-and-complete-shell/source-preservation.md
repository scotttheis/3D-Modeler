# RB-08 Source Preservation

## Scope and authority

- Working source: `E:\Builder3D-Rust`, branch `main` (unborn repository).
- Python behavior reference: `L:\Builder3D`, read-only throughout this slice.
- RB-07 is complete; RB-08 is the authorized implementation slice. Stop before RB-09.
- No Git files were staged, committed, reset, restored, or discarded.
- The accepted Python reference run uses a corrected adapter-level `IniFormat` override and stores its INI/runtime under this evidence folder. An earlier exploratory launcher used `setDefaultFormat`/`setPath` but still constructed `QSettings(organization, application)`, which selected the Windows registry. During that exploratory run, the reference theme was toggled and `HKCU\Software\Builder3D\Builder3D\theme_mode` changed to `light`; it was restored to `dark` before Rust preference migration was tested. The current registry values are `theme_mode=dark`, `object_snapping_enabled=true`, and an empty `ribbon_favorites` list. No Python source, reference project, or file under `L:\Builder3D` was written.

## Pre-edit recovery copy

Before changing application implementation, created the additive snapshot
`G:\Builder3D-Rust_snapshot_20261102-085830_RB-08-baseline`.

Robocopy returned `1` (files copied, no failure). An independent recursive
manifest comparison found **3,082 files / 577,354,485 bytes** in both source and
snapshot. Every relative path, file size, and SHA-256 matched. Missing paths,
extra paths, size mismatches, hash mismatches, and reparse points: **0**.
Generated `target\` and `.cache\` outputs were excluded.

The source worktree remains the working copy. This snapshot is additive and was
not used to restore or overwrite any source.

## Existing process state

At inspection, the unrelated Team Scheduler, MCP, and Python service processes
were left alone. The RB-07 Rust UI baseline was closed with verified foreground
Alt+F4 before implementation. Native Python and Rust reference sessions used
isolated paths below this evidence directory and were closed before builds.

## Native reference identity

- Python: `L:\Builder3D\venv\Scripts\python.exe` (Python 3.12.10); the source
  `src/main_window.py` SHA-256 was
  `3fc2b1c25a53f6f7d040d3321d331a22d7b8a462f8cb4fcd18a9631b960a7c41`.
- Rust pre-edit Release executable SHA-256:
  `e26c37de652b6434727ae7f6396012cdf4aa911299857b621d336afb4da38448`.
- Native baseline client area: 1280×840 at 96 DPI (100% scale).

## QSettings harness correction

The first pre-edit Python observations were collected before Rust implementation
and remain in their original run folders, but the launcher did not isolate
Windows `QSettings` as intended. Those frames are diagnostic only and are not
counted as accepted captures. The accepted comparison run is
`native/python-live-baseline-20261102-094708/`; it replaces
`desktop_adapters.QSettings` with an explicit INI-format constructor and writes
settings under the evidence run directory. The subsequent Rust launch read the
three registry values above and migrated them into its isolated JSON runtime;
the Rust registry adapter is read-only.

At final source inspection, `git -c safe.directory=L:/Builder3D -C
L:\Builder3D status --short` still reported 116 records, matching the RB-07
report. Git required only a command-scoped safe-directory setting because the
volume does not record ownership; no global Git configuration was changed.
The Python `src/main_window.py` SHA-256 remains
`3fc2b1c25a53f6f7d040d3321d331a22d7b8a462f8cb4fcd18a9631b960a7c41`.

The rejected Python run `native/python-live-baseline-20261102-094527/` was
observed using the registry theme even though its metadata requested another
theme, and is not counted. Its record and the registry restoration are listed
in `rejected-attempts.md`.

## Limits

These snapshots and captures establish byte preservation and observed behavior;
they do not establish owner visual acceptance. RB-08 owner acceptance remains
pending.
