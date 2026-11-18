# Resize -> Settle without distortion
Goal: settle resized/rotated selections on Y=0 with rigid world-space motion.
Files: transforms.rs, shell.rs, review_capture.rs, scripts/Invoke-Gate.ps1.
Remove the single-object local-rotation special case; preserve centering and
one grouped undo entry. Retain historical Python fixtures as evidence while
recording the intentional correction to its single-object rotation bug.
Verify: shape-distance regression after numeric Resize in both bases,
single/multi selection, floor contact, repeat no-op, undo/redo, Fast/Full gate,
both-theme before/after captures. Review existing MCP settle behavior.
Baseline: see logs/settle-claw-baseline.txt (copy and counts verified).
Follow E:\RustDeveloper.md and AGENTS.md. Owner acceptance remains pending.

Checkpoint: regression failed before the fix (distance changed); all four
Settle tests and Fast gate passed (275 app tests), independent diff review
found no blockers. Pack: evidence/rebuild/captures/2026-11-18-settle-claw/.
Both-theme before/after states captured; initial scene PNGs match exactly.
MCP already lowers by translation and cannot introduce this local-rotation
bug; its existing behavior is unchanged. Full gate follows with cursor task.
Recovery: `G:\3D-Modeler_snapshot_20261118-115425_resize-settle` (additive copy; counts verified by logs/resize-settle-recovery.txt).
Next: implement shared open/closed robot-claw grab feedback.
