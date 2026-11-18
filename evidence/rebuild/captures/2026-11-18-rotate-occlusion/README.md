# Rotate ring occlusion

Before/after in light and dark themes. Rear ring segments and curved arrow
grips are clipped to the rotated selection silhouette, so the controls read as
wrapping around the object instead of drawing across its faces. Axis colors,
visible grips, picking and transform behavior are retained.

The before images are the OF-68 Release captures. The after images are from the
Release app after this change. These captures show painting; they do not prove
physical pointer behavior or owner approval.

Validation: Fast gate passed for `modeler3d-app` and `modeler3d-viewport`;
Release build and `--smoke` passed. Details in
[`docs/tasks/rotation-occlusion.md`](../../../../docs/tasks/rotation-occlusion.md).
