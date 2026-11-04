# RB-09 Source Preservation

## Working state

- Working source: `E:\Builder3D-Rust`, branch `main` in an unborn repository. `git status --short --branch` showed the project files as untracked. Every pre-existing file is treated as owner work.
- Python behavior reference: `L:\Builder3D`, read-only throughout RB-09.
- No files have been staged, committed, reset, discarded, or restored from a snapshot.
- Baseline copy: `G:\Builder3D-Rust_snapshot_20261102-141606_RB-09-baseline`; see `baseline-snapshot-receipt.md`.

## Starting processes and desktop state

The starting process inspection found the existing Python MCP gateway at `L:\Builder3D\scripts\builder3d_mcp.py` (PIDs 24452 and 20392), Windows `LockApp.exe` (PID 9808), and unrelated Team Scheduler and Blender MCP processes. They were left running. No Rust Builder3D application or `r4_mcp` process was present. The lock screen owned the foreground, so no input was sent and no window was closed. Paired live desktop/MCP evidence is therefore unverified until an interactive session is available.

## Reference protection

No command writes to `L:\Builder3D`. Reference reads and focused tests use the existing Python 3.12 environment and project-provided isolation. Do not install packages, write logs, change settings, or create fixtures in the reference tree.

## Snapshot limits

The baseline snapshot excludes generated `target`, `.cache`, and `logs` directories, `*.log` files, and reparse points. The independent included-tree manifest comparison matched every relative path, byte length, and SHA-256 hash, with no included reparse points. RB-09 evidence, later source edits, and the later completion snapshot are not part of this pre-edit baseline.

## Post-verification reference check

After the focused Python MCP/catalog/gateway tests, `git -c
safe.directory=L:/Builder3D -C L:\Builder3D status --short` returned the same
116 records noted in the RB-08 checkpoint. The behavior-critical
`src/main_window.py` SHA-256 remains
`3fc2b1c25a53f6f7d040d3321d331a22d7b8a462f8cb4fcd18a9631b960a7c41`. The
command-scoped Git safety override did not change global configuration. Test
bytecode was disabled, pytest cache was disabled, and temporary files/logs were
placed under this RB-09 evidence directory on E:. No command wrote to
`L:\Builder3D`.

## Owner path and backup direction (2026-11-03)

The active Rust project remains `E:\Builder3D-Rust` for source, builds, and
launches. `G:\` is the primary additive recovery-backup destination. The
current G: copy is
`G:\Builder3D-Rust_snapshot_20261103-111729_RB-09-paused-e-primary`.
Removable USB is for occasional one-off copies only; the current USB copy is
`L:\Builder3D-Rust_snapshot_20261103-103227_RB-09-paused` and its receipt is
`usb-backup-receipt.md`. The USB snapshot is not the active project. The
`L:\Builder3D` Python reference remains read-only and is separate from both
Rust backups.

The Rust Release app and MCP gateway were built after Rust source changes were
complete. Later edits are evidence, documentation, catalog, parity, and
work-log updates; Rust source was unchanged. The additive staging copy is
`G:\Builder3D-Rust_snapshot_20261102-152935_RB-09-handoff-staging`; the final
RB-09 handoff copy is
`G:\Builder3D-Rust_snapshot_20261102-152935_RB-09-handoff`. Their independent
path/size/SHA/reparse verification is recorded in `snapshot-receipt.md`. The
final copy is a handoff copy because mandatory native evidence is still
unverified, not an RB-09 completion snapshot.
