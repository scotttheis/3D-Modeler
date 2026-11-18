# Rotation grips from rotate.png

Use the owner's `erros/rotate.png` reference: solid double-ended arrowheads
joined by a narrower solid stem. Follow each rotation arc with a 32-point
curved stem, 6-point stem width, and filled tips 14 points long by 16 high.
Keep thin ring lines and GPU/CPU mesh occlusion, with local-axis picking,
preview and history.
Read shell/rotate_controls.rs, shell.rs and viewport renderer.rs/cpu.rs.
Verify Fast/Full gates, Release smoke and both-theme upright/tilted/drag
captures against the reference. Resize retains its original appearance.
Reference and review pack: erros/rotate.png;
evidence/rebuild/captures/2026-11-18-rotate-reference/.

Baseline: G:\3D-Modeler_snapshot_20261118-104339_rotate-reference-baseline | 13616 files | 1966406600 bytes verified.
Verified: Full gate `logs/gate/20261118-104915-full` passed, including
271 app and 57 viewport tests, catalog parity and 92 captures. Regression
checks opposite arrow tips in edge-on views and mesh occlusion for both
strokes and fills. Release build/smoke and both-theme captures passed.
Release EXE: 23,393,792 bytes. Owner visual acceptance pending.
Recovery: `G:\3D-Modeler_snapshot_20261118-105552_rotate-reference` (counts in logs/rotate-reference-recovery.txt).
