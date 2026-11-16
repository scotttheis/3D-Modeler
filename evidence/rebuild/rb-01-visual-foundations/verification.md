# RB-01 Verification

**Date:** 2026-10-31  
**Tests:** None run.

## Build and capture verification

| Check | Result |
| --- | --- |
| Rust app build | cargo build --package modeler3d-app --locked; exit 0 |
| Rust executable | target/debug/modeler3d-app.exe; SHA-256 D1639A6561A3E73A0C3D5F85D4076BAA51DA760A9800E866D2B9613BE759719B |
| Python runtime | Python 3.12.10 from the existing L:\Builder3D venv |
| Python source identity | main.py, main_window.py, style.py, toolbar.py, and icon_assets.py hashes recorded in capture-metadata.toml |
| Client-area measurements | All four windows measured at 1280 × 840 logical and physical pixels |
| Display scaling | System and per-window DPI 96; 100% scale |
| Screenshots | Four inspected 1280 × 840 client captures and two 2560 × 840 1:1 comparison images |
| Capture serialization | Python Dark, Python Light, Rust Dark, Rust Light; each application closed before the next session |
| Python side effects | Isolated QSettings, app-data, local-app-data, temp, and runtime paths; original L:\Builder3D files and user settings were not changed |
| Rust side effects | Isolated app-data paths; the Light selection was contained in that isolated profile |
| Gateway | Left running and untouched; it had no target UI window during capture |
| App shutdown | Python and Rust target windows were closed after their captures; no target app process remained afterward |
| RB-02 | Not started; awaiting owner review and explicit approval |

No application tests were run. The build and live captures do not verify New, Cube creation, native import, viewport rendering, picking, gizmo movement, undo, or save/reopen.

## Capture provenance

The Python and Rust live window identifiers, capture timestamps, exact client origins and window bounds, image dimensions, byte lengths, hashes, executable identity, source hashes, isolation details, and prior historical gate records are in [capture-metadata.toml](capture-metadata.toml). The four images and no-scale side-by-side comparisons are linked from [capture-board.md](capture-board.md).

## Scope and limitations

The matched images cover the empty Welcome screen with the Insert ribbon active in Light and Dark. They visibly show differences in Welcome placement, Recent presentation, the right-side panel, the hamburger glyph, and ribbon proportions. Owner visual review remains pending; no regional tolerance or parity pass is claimed.

No scene, object list, selected object, transform strip, viewport interaction, or import dialog was exercised. Theme setup was isolated: Python Light was configured in a temporary QSettings profile and Rust Light was selected through Settings. Theme persistence across restart was not tested. No package staging was attempted. The read-only reference at L:\Builder3D was not modified.

The temporary capture profile and dialog screenshots remain confined to the ignored project target directory; the immutable recovery snapshot excludes that directory.