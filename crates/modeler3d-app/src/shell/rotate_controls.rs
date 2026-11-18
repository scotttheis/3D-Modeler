//! Object-oriented rotation geometry shared by painting and pointer picking.
use modeler3d_viewport::{
    Aabb, Camera, GIZMO_TARGET_PIXELS, GizmoGeometry, GizmoMode, Ray, add, build_gizmo_with_bounds,
    dot, scale, subtract,
};

use crate::transform_gesture::ResizeFrame;

impl ResizeFrame {
    pub(super) fn rotation_selection(
        scene: &modeler3d_core::Scene,
        indices: &[usize],
    ) -> Result<Self, crate::transforms::TransformError> {
        let mut frame = Self::from_scene(scene, indices, true)?;
        if indices.len() <= 1 {
            return Ok(frame);
        }
        // The first selected object's orientation gives a group a stable
        // frame that also follows group commits, undo and redo.
        let basis = crate::rotation::basis(scene.object(indices[0])?.transform())?;
        frame.axes = std::array::from_fn(|i| std::array::from_fn(|j| basis[j][i]));
        let origin = frame.center;
        let mut minimum = [f32::INFINITY; 3];
        let mut maximum = [f32::NEG_INFINITY; 3];
        for &index in indices {
            let object = scene.object(index)?;
            for &vertex in &object.mesh().vertices {
                let offset = subtract(object.transform().transform_point(vertex), origin);
                for axis in 0..3 {
                    let value = dot(offset, frame.axes[axis]);
                    minimum[axis] = minimum[axis].min(value);
                    maximum[axis] = maximum[axis].max(value);
                }
            }
        }
        let bounds = Aabb::new(minimum, maximum).map_err(|_| {
            crate::transforms::TransformError::InvalidValue {
                name: "rotation bounds",
                reason: "bounds exceed finite millimetres",
            }
        })?;
        frame.extents = std::array::from_fn(|i| bounds.maximum[i] - bounds.minimum[i]);
        frame.center = frame.rotation_point(std::array::from_fn(|i| {
            bounds.minimum[i] * 0.5 + bounds.maximum[i] * 0.5
        }));
        Ok(frame)
    }

    pub(super) fn rotation_geometry(self, camera: &Camera) -> Option<GizmoGeometry> {
        let half = self.extents.map(|extent| extent.abs() * 0.5);
        let bounds = Aabb::new(half.map(|v| -v), half).ok()?;
        let mut geometry =
            build_gizmo_with_bounds(camera, [0.; 3], GizmoMode::Rotate, Some(bounds)).ok()?;
        for line in &mut geometry.lines {
            line.start = self.rotation_point(line.start);
            line.end = self.rotation_point(line.end);
        }
        Some(geometry)
    }

    fn rotation_point(self, local: [f32; 3]) -> [f32; 3] {
        let mut point = self.center;
        for (axis, value) in self.axes.into_iter().zip(local) {
            point = add(point, scale(axis, value));
        }
        point
    }

    pub(super) fn rotation_hit(self, camera: &Camera, ray: Ray) -> Option<usize> {
        // Same pixel tolerance as the original analytic rotate-ring picker.
        let tolerance = camera.world_tolerance(26.).max(0.001);
        let length = camera.world_tolerance(GIZMO_TARGET_PIXELS).max(0.01);
        let mut nearest: Option<(f32, usize)> = None;
        for (index, normal) in self.axes.into_iter().enumerate() {
            let denominator = dot(ray.direction, normal);
            if denominator.abs() <= 1.0e-6 {
                continue;
            }
            let distance = dot(subtract(self.center, ray.origin), normal) / denominator;
            if distance < 0. {
                continue;
            }
            let radius = (0..3)
                .filter(|axis| *axis != index)
                .map(|axis| (self.extents[axis] * 0.5).powi(2))
                .sum::<f32>()
                .sqrt()
                .max(length * 0.5)
                + modeler3d_viewport::ROTATE_RING_GAP_FACTOR * length;
            let radial = subtract(ray.at(distance), self.center);
            if (dot(radial, radial).sqrt() - radius).abs() <= tolerance
                && nearest.is_none_or(|(best, _)| distance < best)
            {
                nearest = Some((distance, index));
            }
        }
        nearest.map(|(_, index)| index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        commands::CommandId,
        shell::{Modeler3dApp, TransformPanelMode, ViewportDrag},
        transform_gesture::TransformGesture,
    };
    use modeler3d_core::PrimitiveKind;
    use modeler3d_viewport::ROTATE_RING_SEGMENTS;

    fn close(a: [f32; 3], b: [f32; 3]) {
        for i in 0..3 {
            assert!((a[i] - b[i]).abs() < 0.001, "{a:?} != {b:?}");
        }
    }

    #[test]
    fn rotated_rings_pick_their_local_axes_and_preview_survives_commit_and_undo() {
        for multiple in [false, true] {
            let mut app = Modeler3dApp::default();
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("cube");
            if multiple {
                app.commands
                    .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                    .expect("second cube");
                app.commands.set_selection_indices([0, 1]);
                let id = app.commands.scene().objects()[1].object_id().to_owned();
                app.commands
                    .set_object_position_by_id(&id, [35., 10., 0.])
                    .expect("move second cube");
            }
            app.commands
                .rotate_selected([25., 35., 20.])
                .expect("rotate");
            app.transform_panel_mode = TransformPanelMode::Rotate;
            app.update_gizmo_frame();
            let original = app.rotation_control_frame().expect("frame");
            assert!(original.axes[0][1].abs() > 0.1);
            let geometry = original.rotation_geometry(&app.camera).expect("geometry");
            for (index, ring) in geometry
                .lines
                .as_chunks::<{ ROTATE_RING_SEGMENTS as usize }>()
                .0
                .iter()
                .enumerate()
            {
                let mut hits = 0;
                for line in ring {
                    close(
                        [
                            dot(subtract(line.start, original.center), original.axes[index]),
                            0.,
                            0.,
                        ],
                        [0.; 3],
                    );
                    let ray = Ray::new(
                        add(line.start, scale(original.axes[index], 100.)),
                        scale(original.axes[index], -1.),
                    )
                    .expect("ray");
                    hits += usize::from(original.rotation_hit(&app.camera, ray) == Some(index));
                }
                assert!(hits > 0, "local axis {index} must be pickable");
            }
            let snapshot = app
                .commands
                .capture_transform_selection()
                .expect("snapshot");
            let point = add(original.center, scale(original.axes[1], 50.));
            let start = Ray::new(
                add(point, scale(original.axes[0], 100.)),
                scale(original.axes[0], -1.),
            )
            .expect("ray");
            let mut drag =
                TransformGesture::rotate(snapshot, original.center, original.axes[0], start)
                    .expect("drag");
            let point = add(original.center, scale(original.axes[2], 50.));
            drag.update(
                Ray::new(
                    add(point, scale(original.axes[0], 100.)),
                    scale(original.axes[0], -1.),
                )
                .expect("ray"),
            )
            .expect("update");
            app.viewport_drag = Some(ViewportDrag::Transform(Box::new(drag.clone())));
            assert_eq!(app.active_gizmo_axis_index(), Some(0));
            let preview = app.rotation_control_frame().expect("preview");
            close(preview.axes[0], original.axes[0]);
            close(preview.axes[1], original.axes[2]);
            let preview_geometry = preview
                .rotation_geometry(&app.camera)
                .expect("preview geometry");
            let delta = modeler3d_viewport::Mat4::from_rows(drag.delta.rows()).expect("matrix");
            for (before, after) in geometry.lines.iter().zip(&preview_geometry.lines) {
                close(
                    delta.transform_point(before.start).expect("point"),
                    after.start,
                );
            }
            // Cancellation returns to the exact committed frame.
            app.viewport_drag = None;
            close(
                app.rotation_control_frame().expect("cancelled").axes[1],
                original.axes[1],
            );
            app.commands
                .commit_transform_snapshot(&drag.snapshot, drag.delta)
                .expect("commit");
            app.update_gizmo_frame();
            let committed = app.rotation_control_frame().expect("committed");
            for i in 0..3 {
                close(committed.axes[i], preview.axes[i]);
            }
            app.commands.dispatch(CommandId::Undo).expect("undo");
            app.update_gizmo_frame();
            close(
                app.rotation_control_frame().expect("undone").axes[1],
                original.axes[1],
            );
            app.commands.dispatch(CommandId::Redo).expect("redo");
            app.update_gizmo_frame();
            close(
                app.rotation_control_frame().expect("redone").axes[1],
                preview.axes[1],
            );
        }
    }
}
