# Thin rotation rings with front and rear visibility

Owner correction: undo the Resize appearance change and apply thin lines to
Rotate, crossing in front of the object and hidden behind visible surfaces.

Read shell.rs, shell/rotate_controls.rs and viewport renderer.rs/cpu.rs.
Implementation: restore Resize, use one-pixel rotation ring strokes and open
arrowheads in the GPU/CPU depth-tested viewport. Retain local-axis picking,
live preview, command history and MCP semantics.
Verify Fast gate for app/viewport, Release smoke and both-theme captures of
upright, tilted and dragging Rotate plus restored Resize.
Baseline: G:\3D-Modeler_snapshot_20261118-100611_rotate-thin-baseline | 13391 files | 1926567768 bytes verified.
Review pack: evidence/rebuild/captures/2026-11-18-rotate-thin/.

Verified: Full gate `logs/gate/20261118-100850-full` passed (270 app,
57 viewport tests, workspace Clippy/fmt/tests, deny/audit, catalog parity,
smoke, 92 review captures). Release build and smoke passed. Resize source
matches `fc7fd04^`; captures confirm the restored controls. Release size
23,387,136 bytes. Visual acceptance awaits the owner.
Recovery: `G:\3D-Modeler_snapshot_20261118-101504_rotate-thin` (counts logged in logs/rotate-thin-recovery.txt).
