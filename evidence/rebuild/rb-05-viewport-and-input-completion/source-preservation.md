# RB-05 source preservation

The Python product at `L:\Builder3D` was used as a read-only reference. The native script imported source modules and created only RB-05-owned fixture/capture output under `E:\Builder3D-Rust\evidence\rebuild\rb-05-viewport-and-input-completion`.

The reference repository already had pre-existing unrelated modifications and untracked recovery files. A status check with `git -c safe.directory=L:/Builder3D -C L:\Builder3D status --short` reported 116 existing lines; RB-05 did not edit or stage that tree. Relevant reference source hashes observed during the replay were:

| Reference file | SHA-256 |
|---|---|
| `src/viewport.py` | `7e2af3293072f865a011b787d05e3c9afc426e5bf2072b3cd7590acda79df033` |
| `src/gizmo.py` | `9ea7850c6737be5b4609f986d2ad021bccb26f8e6f73dbad9b406e33013a9fd2` |
| `src/snapping.py` | `17dfde9e69677b57c0a69792193b52be2b1c8881469fa5e64bdeeafb197c8886` |
| `src/main_window.py` | `3fc2b1c25a53f6f7d040d3321d331a22d7b8a462f8cb4fcd18a9631b960a7c41` |
| `src/application_services.py` | `1c9724616ad20924a44a720176967c264fecd942622e04d37b548b9c332684d8` |

Rust changes are confined to the Builder3D-Rust recovery tree and RB-05 evidence/docs. No staging or commit was performed. G-drive snapshots are additive; existing snapshot contents were not modified.
