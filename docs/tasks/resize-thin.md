# Thin resize arrows and front guides

Goal: thin open arrowheads with guides across the front of the selection,
hidden by visible scene surfaces.

Read: resize_overlay.rs, shell.rs, viewport renderer.rs and cpu.rs.
Implementation: place guides on camera-facing selection bounds and render
strokes through the shared depth-tested GPU/CPU viewport paths.
Verify: Fast gate for app and viewport, Release build and smoke, both-theme
resize and drag captures with oblique camera checks.
Review pack: evidence/rebuild/captures/2026-11-18-resize-thin/.
Baseline: G:\3D-Modeler_snapshot_20261118-094301_resize-baseline
(13,158 files, 1,885,043,314 bytes verified).

Verification: Fast gate `logs/gate/20261118-095012-fast` passed. Full run
`logs/gate/20261118-094632-full` passed every step except the initial Clippy
check (test-code chunk iterator lint). Fixed it and reran workspace Clippy
successfully. All 92 board captures exist. Release and Release smoke passed.
An additional cargo-bloat rebuild was cancelled; the verified Release EXE
measures 23,394,304 bytes. Installer size was not measured in this UI task.
Final recovery: `G:\3D-Modeler_snapshot_20261118-095859_resize-thin`.
File/byte verification is recorded in `logs/resize-thin-recovery.txt`.

Owner correction, 2026-11-18: the intended control was Rotate. Resize appearance
changes are reverted by the rotate-thin task.
