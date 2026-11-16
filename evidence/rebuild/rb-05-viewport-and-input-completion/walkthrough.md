# RB-05 native walkthrough

The walkthrough was performed on the default Windows desktop only. The Python reference ran first; the Rust target was not open concurrently. Every accepted frame was captured after a settle wait and recorded its actual client size, DPI, theme, action, route, and image hash.

## Python reference — first

Command:

```powershell
$env:PYTHONDONTWRITEBYTECODE = '1'
& 'L:\Builder3D\venv\Scripts\python.exe' -B evidence\rebuild\rb-05-viewport-and-input-completion\rb05_python_native.py
```

For each Light/Dark theme the visible Qt window was set to 1280 x 840, then actual Win32 input exercised New/empty, a single scene, exact double-click frame, left orbit, middle pan, right pan, wheel zoom, View/Grid toggle, Wireframe, X-Ray, Snap, Move rest/interaction, multiple objects, grouped objects, and hidden objects. The accepted result is [python-native-results.json](python-native-results.json), 30 records.

## Rust target — second

Command:

```powershell
& .\evidence\rebuild\rb-05-viewport-and-input-completion\native-rust-rb05.ps1
```

The harness launched `target\debug\modeler3d-app.exe`, opened each immutable `.3dp` fixture through the application menu and native file dialog, and sent guarded OS pointer/button/wheel/keyboard events. It replayed both themes in this order: empty and single resting scenes; orbit, middle/right pan, wheel, and double-click frame; Move/Rotate/Resize; no-op, Escape, focus-loss, grouped, and hidden Move; then a resting floor/grid render. It accepted 52 records, 26 per theme, against debug SHA-256 `4a42ec94338baf986806740e7a59c7fc7db2faa9bdf2027af4d8f6160ac69232`.

## Inspection and rejection

Accepted PNG paths were checked for existence, 1280 x 840 dimensions, 96 DPI capture conditions, theme pixel guard, and journal hash. The accepted image set has 82 files and zero missing paths. The supplemental Rust View-control experiment is retained but not accepted for Grid-off because the captured Grid-off frame was identical to its resting frame; see [rejected-attempts.md](rejected-attempts.md). No rejected frame is used to claim visual parity or owner acceptance.
