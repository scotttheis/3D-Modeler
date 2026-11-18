# Owner feedback — 2026-11-17 (transform feedback)

New transform requests from the owner. Work one item at a time in order,
following the review and verification workflow in
[`owner-feedback-2026-11-14.md`](owner-feedback-2026-11-14.md).

| # | Request | Notes | Status |
|---|---|---|---|
| OF-66 | Remove the number displayed above the model while resizing. | Removed the size label from the Resize overlay, idle and dragging. Existing size boxes retained. Captures: `evidence/rebuild/captures/2026-11-17-of66-67-resize/`. No MCP change: feedback painting only. | Done, awaiting owner review |
| OF-67 | Show the live size in the existing number box below the model while resizing. | All XYZ size boxes follow the same uncommitted delta as the mesh, in display units. Cancel restores scene sizes; numeric entry and the existing axis, snap, proportional, anchor and one-command history paths are retained. Regression: `resize_boxes_follow_pointer_preview_cancel_and_one_step_commit`. No MCP change: existing resize-by-value semantics retained. | Done, awaiting owner review |
| OF-68 | Redesign the rotation arrows to point both ways. | Original curved grips with flat solid heads at both ends, inspired by `rotate.png` and Resize. Axis colors, rings and picking retained. Captures: `evidence/rebuild/captures/2026-11-17-of68-rotation/`. No MCP change: painting only; existing rotation semantics retained. | Done, awaiting owner review |
| OF-69 | Restyle the Move arrows to feel more like the Resize handles. | Preserve the existing axis meaning and interaction. | Queued |
| OF-70 | Update Move and Rotate numbers live during a drag. | The displayed values should track the object's current movement or rotation while the pointer is moving. | Queued |
