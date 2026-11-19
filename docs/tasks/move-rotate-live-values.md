# OF-70 - Live Move and Rotate numbers

Goal: the Move position boxes and Rotate angle boxes follow the object while
a viewport drag is in progress, like the Resize sizes (OF-67).
Files: `shell.rs` (`sync_move_rotate_fields`, `stage_review_move_drag`),
`review_capture.rs` (state `move-drag`), `scripts/Invoke-Gate.ps1`, feedback
and STATUS.
Do: compute the boxes from the same uncommitted gesture delta the commit
applies (`delta * transform`): position for one object, bounds centre for
several; angles from the first selected object. Restore scene values when the
drag ends without a commit (Escape, focus loss, blocked input). Leave idle
text alone so numeric edits survive repainting.
Reproduced first: board `gate-20261118-120020`, state `rotate-drag`, shows a
cube turned 40 degrees while the boxes still read 25 / 35 / 20 deg.
Verify: regression `move_and_rotate_boxes_follow_the_drag_cancel_and_commit_without_a_jump`
(mm and inches, one and two objects, X/Y/Z, Move and Rotate, Escape / focus
loss / blocked / release, scene and history untouched mid-drag, no jump on
release, one-step undo/redo, numeric edit kept); Full gate; both-theme
before/after captures of `move-drag` and `rotate-drag`; Release build and smoke.
MCP: no change. MCP has no drag; `transform_selection` and friends already
return the committed values, which the boxes now match on release.
Out: Resize (done in OF-67), any gesture math, snapping or history change.
Review pack: `evidence/rebuild/captures/2026-11-18-of70-live-values/`.
