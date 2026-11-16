# RB-02 Capture Board

All images below are inspected direct desktop captures from the visible Rust `3D Modeler` window. Each PNG is the 1280 x 840 client rectangle, captured with GDI `CopyFromScreen`; each window reported 96 DPI / 100% scale, was restored and visible, and had foreground title `3D Modeler`. Full timestamps, process IDs, file sizes, and SHA-256 values are in [capture-metadata.toml](capture-metadata.toml).

The interactive workflow was captured Light first and its processes were closed before the Dark workflow was opened. The separate startup Welcome captures were collected earlier in the order Dark then Light, with each process closed before the next was opened. No target app windows overlapped. These images document the Rust RB-02 workflow, while the matched Python/Rust theme comparisons remain in the separate [RB-01 capture board](../rb-01-visual-foundations/capture-board.md).

| State | Light | Dark |
|---|---|---|
| Welcome | [PNG](captures/rust-light-welcome-client-1280x840.png) | [PNG](captures/rust-dark-welcome-client-1280x840.png) |
| New empty scene | [PNG](captures/rust-light-new-empty-client-1280x840.png) | [PNG](captures/rust-dark-new-empty-client-1280x840.png) |
| Cube added | [PNG](captures/rust-light-cube-client-1280x840.png) | [PNG](captures/rust-dark-cube-client-1280x840.png) |
| Native picker cancellation | [PNG](captures/rust-light-native-picker-cancel-client-1280x840.png) | Not separately captured; cancel preservation is also covered by app tests. |
| Fixture imported at default placement | [PNG](captures/rust-light-cube-fixture-imported-client-1280x840.png) | [PNG](captures/rust-dark-cube-fixture-imported-client-1280x840.png) |
| Orbit, pan, and zoom | [PNG](captures/rust-light-camera-orbit-pan-zoom-client-1280x840.png) | [PNG](captures/rust-dark-camera-orbit-pan-zoom-client-1280x840.png) |
| Exact geometry pick | [PNG](captures/rust-light-exact-pick-client-1280x840.png) | [PNG](captures/rust-dark-exact-pick-client-1280x840.png) |
| Gizmo before move | [PNG](captures/rust-light-gizmo-before-client-1280x840.png) | [PNG](captures/rust-dark-gizmo-before-client-1280x840.png) |
| Gizmo after move | [PNG](captures/rust-light-gizmo-after-client-1280x840.png) | [PNG](captures/rust-dark-gizmo-after-client-1280x840.png) |
| Undo | [PNG](captures/rust-light-undo-client-1280x840.png) | [PNG](captures/rust-dark-undo-client-1280x840.png) |
| Redo | [PNG](captures/rust-light-redo-client-1280x840.png) | [PNG](captures/rust-dark-redo-client-1280x840.png) |
| Saved before reopen | [PNG](captures/rust-light-saved-before-reopen-client-1280x840.png) | [PNG](captures/rust-dark-saved-before-reopen-client-1280x840.png) |
| Reopened project | [PNG](captures/rust-light-reopened-client-1280x840.png) | [PNG](captures/rust-dark-reopened-client-1280x840.png) |

The initial import captures show the 40 mm cube occluding the much smaller fixture. Later camera, pick, transform, undo/redo, and save/reopen captures show the fixture after the cube was reduced to 5 mm and moved to X = -6 mm.

## Reopened result

| Light | Dark |
|---|---|
| ![Light project reopened in 3D Modeler](captures/rust-light-reopened-client-1280x840.png) | ![Dark project reopened in 3D Modeler](captures/rust-dark-reopened-client-1280x840.png) |

Owner accepted this Light/Dark baseline for now; visual fine-tuning may happen later.
