# RB-06 native walkthrough

The walkthrough ran on the Windows desktop with one visible application at a
time. The Python reference ran first and was closed before the Rust target was
launched. Accepted captures were made at a 1280 x 840 client size, 96 DPI, and
100% display scale.

## Python reference - first

Command:

```powershell
$env:PYTHONDONTWRITEBYTECODE = '1'
& 'L:\Builder3D\venv\Scripts\python.exe' -B evidence\rebuild\rb-06-modelling-tools-and-brushes\rb06_python_native.py
```

The visible reference window used actual OS Insert/Cube input for the primitive
case. For each Light/Dark theme it captured empty/rest, primitive Cube, and the
modelling fixture. The six records are in
[python-native-results.json](python-native-results.json). The reference process
was closed before the Rust replay.

## Rust target - second

Command:

```powershell
& .\evidence\rebuild\rb-06-modelling-tools-and-brushes\native-rust-rb06.ps1
```

The harness launched `target\debug\modeler3d-app.exe`, changed theme through the
native UI, and sent actual OS pointer, keyboard, Shift-drag, and Escape input.
For each theme it exercised the Torus editor; all five persistent brushes;
Simplify, Subdivide, Smooth, and Normalize; Boolean picker/cancel and exact-two
Merge/Intersect/Subtract; Emboss and Hollow dialogs; Repair; Paint/Material;
View toggles; and Items actions. It accepted 36 records per theme and closed the
target after the replay.

## Acceptance guards

Every Rust record contains client dimensions, DPI, screen origin, input route,
PNG SHA-256, and the executable SHA-256. The final run contains one executable
identity only. The accepted image set contains 78 PNGs: six Python reference
images and 72 Rust images. No `modeler3d-app` process remained after capture.

The first two boolean replays are retained only in the rejection history: they
selected three visible objects in the evidence harness, and one later Subtract
fixture produced an empty result. The final harness selects the target and the
new overlapping Cube; both final Subtract captures report a committed result.
