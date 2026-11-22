# Paste at the original's spot, and Settle straight down

ID: OWNER-2026-11-21-PASTE-SETTLE
Owner request 2026-11-21. Two small fixes. Done 2026-11-21 (committed on rebuild/work). Cause of Settle: `settle_on_floor` recentred X/Z on the world origin; it now keeps the current X/Z.

1. **Paste goes where the original was.** Today Paste puts the copy beside the
   original (keep that: the owner likes it). New: if the original has been
   moved since it was copied, the paste lands at the spot the original was
   copied from (its position at copy time), not beside its new position.
   Unmoved original: unchanged behaviour (beside it).
2. **Settle drops straight down from where the object is now.** After an
   object is moved, Settle currently puts it back at its original location.
   It must keep the object's current X/Z (horizontal position) and only change
   height until it rests on the floor, or on whatever is below it as today.

Files: `crates/modeler3d-core/src/scene.rs` (`paste_beside`, `beside_offset`,
clipboard), `crates/modeler3d-core/src/placement.rs` (`settle_on_floor`),
`crates/modeler3d-app/src/commands.rs` (`CommandId::Paste` ~956,
`settle_selected` ~2203). MCP must match the desktop (shared core call; run the
parity check) and the MCP result for paste/settle may change.
Verify: tests for both (move original then paste; move then settle keeps X/Z;
one undo entry each; no-op makes none), workspace tests, fmt, Clippy, deny,
audit, parity. Then `Update-CurrentBuild.ps1`.
Investigate first: why Settle returns to the original spot (is it using the
object's stored original placement instead of its current transform?).
Out: file formats, other tools.
Tier: Implementer
