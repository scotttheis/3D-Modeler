# Rotation ring finish

Goal: make the Rotate rings read as precise, three-dimensional controls while
preserving their axis colors, silhouette clipping, picking, hover/drag feedback
and transform behavior.

Implementation: shade continuous visible arc paths with a soft cast edge, dark
contour, shaded body and narrow glint; give curved grips matching edge
definition; increase ring tessellation from 48 to 96 segments. The app painter
joins visible segments into paths so the contour stays smooth between clipped
gaps.

Verify: Fast gate for `modeler3d-app` and `modeler3d-viewport`; Release build
and `--smoke`; before/after review captures in both themes.

Review pack: [`../rebuild/captures/2026-11-18-rotate-finish/`](../rebuild/captures/2026-11-18-rotate-finish/).

Recovery snapshot: `G:\3D-Modeler_snapshot_20261118-085850_rotation-finish`.
