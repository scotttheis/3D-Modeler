# Rotation ring depth and finish

Goal: make Rotate rings feel spatial by hiding ring strokes and curved grips
where their screen projection crosses the selected object's oriented bounds.
Keep visible arcs and picking in front of the object unchanged. Rotation math,
history and MCP behavior remain unchanged.

Implementation: app-painted ring lines and curved grips use the live rotation
selection frame for silhouette clipping. Renderer-neutral gizmo rings also use
selected bounds for clearance and suppress segments occluded by selected
objects.

Verify: Fast gate for `modeler3d-app` and `modeler3d-viewport`; regression for
visible and hidden projected segments; Release build and `--smoke`; before and
after review captures in both themes.

Review pack: [`../rebuild/captures/2026-11-18-rotate-occlusion/`](../rebuild/captures/2026-11-18-rotate-occlusion/).
