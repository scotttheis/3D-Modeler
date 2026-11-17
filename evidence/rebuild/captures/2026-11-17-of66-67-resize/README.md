# Resize review: OF-66 / OF-67

- OF-66: compare `before-resize-{light,dark}.png` with `after-resize-{light,dark}.png`; the grey size label above the model is removed.
- OF-67: compare `before-resize-drag-{light,dark}.png` with `after-resize-drag-{light,dark}.png`; the same held +X drag now shows X = 52.706 mm below the model instead of the stale 40.000 mm. Y and Z stay 40.000 mm.
- `release-resize-*` and `release-resize-drag-*` show the rebuilt Release app in both themes.

The existing `resize` / `resize-drag` harness states paint the real window framebuffer with isolated settings. The drag state stages the ordinary gesture each frame without committing; these captures prove painted output, not physical pointer input or owner acceptance.

Try in Release:

1. Resize with each axis handle, making the model smaller and larger. Watch the existing XYZ boxes before releasing; no size number should appear above the model.
2. Drag a corner to resize proportionally; all three sizes should follow. Try inches and Object basis on a rotated model as well.
3. Press Escape during a drag: the model and displayed sizes should return. Release another drag, then Undo once and Redo once.
4. Enter a size in the existing box and Apply (or Enter); numeric entry should still work.

Machine evidence: `resize_boxes_follow_pointer_preview_cancel_and_one_step_commit` covers X/Y/Z and corner gestures, both units, grid/object basis, three successive shrinking/expanding proposals, Escape/focus-loss/blocked-input cancellation, unchanged model/history during preview, no jump on release, one Undo/Redo, and idle numeric drafts. `resize_overlay_never_paints_a_size_number` checks idle and active overlays. Existing numeric/proportional, snapping, anchoring and no-op tests remain in the app suite. Gesture rays are supplied directly; no physical pointer check is claimed.

Independent source review found no blocking issues. No MCP interface change: only Resize feedback painting changes; existing `set_transform` / `transform_selection` resize semantics remain shared and unchanged. OF-68 through OF-70 are outside this pack.

Gate logs: `logs/gate/20261117-155839-fast/` and `logs/gate/20261117-155921-full/`. Release build log: `logs/of66-67-release.log`. Current acceptance and recovery state: `docs/STATUS.md`.
