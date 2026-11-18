# Resize -> Settle without distortion
Goal: place resized/rotated selections on Y=0 with rigid world-space motion.
Files: core placement.rs/lib.rs; app transforms.rs, shell.rs, mcp_bridge.rs;
review_capture.rs; MCP r4/server.rs; scripts/Invoke-Gate.ps1.
Cause: Python-compatible local post-rotation sheared nonuniformly resized
objects. Shared core settle_on_floor pre-rotates in world space, centers X/Z
and places the support face on the floor through one grouped command.
Desktop and MCP both delegate to it. MCP previously used local translation
and the wrong floor axis. Schemas, output shape and replay rules are unchanged.
Frozen Python fixtures remain unchanged; the single-object bug is an
intentional owner-requested correction. Group fixture comparison is retained.
Verify: regression failed before the fix; distance preservation in both Resize
bases, single/multi selection, floor contact, no-op, undo/redo; MCP exact
single/multi desktop parity and grouped history; invalid-target atomicity.
First Fast gate: logs/gate/20261118-115242-fast (275 app tests).
Independent diff reviews found no blockers; cross-family review unavailable.
Official MCP tools specification and Rust SDK releases checked 2026-11-18;
no protocol/SDK changes. See E:\MCP_DESIGN_GUIDE.md; follow E:\RustDeveloper.md.
Pack: evidence/rebuild/captures/2026-11-18-settle-claw/README.md.
Baseline: G:\3D-Modeler_snapshot_20261118-114840_settle-claw-baseline
(13,956 files / 2,076,032,925 bytes verified).
First recovery: G:\3D-Modeler_snapshot_20261118-115425_resize-settle
(13,991 files / 2,079,052,511 bytes verified). Full gate: logs/gate/20261118-120020-full passed (521 tests, 104 captures).
Release build/smoke/captures passed; final recovery: `G:\3D-Modeler_snapshot_20261118-122146_settle-claw`.
