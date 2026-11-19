//! Object-oriented rotation geometry shared by painting and pointer picking.
use modeler3d_viewport::{
    Aabb, Camera, GIZMO_TARGET_PIXELS, GizmoGeometry, GizmoLine, GizmoMode, Ray, add,
    build_gizmo_with_bounds, dot, scale, subtract,
};

use crate::transform_gesture::ResizeFrame;

impl ResizeFrame {
    /// Shared object-oriented selection frame for Rotate and Resize.
    pub(crate) fn rotation_selection(
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

/// Screen reach of a rotate grip, in points: about twice the drawn arrow, so
/// the arrow is easy to catch without touching the object behind it.
const GRIP_HIT_RADIUS: f32 = 34.0;

/// One grip per ring: axis index, colour and screen path with depths.
type Grip = (usize, [f32; 4], Vec<(eframe::egui::Pos2, f32)>);

/// The grip chosen for each ring. Painting and picking both use this, so what
/// is drawn is what can be grabbed.
fn chosen_grips(
    geometry: &GizmoGeometry,
    camera: &Camera,
    frame: ResizeFrame,
    matrix: modeler3d_viewport::Mat4,
    rect: eframe::egui::Rect,
) -> Vec<Grip> {
    let segments = modeler3d_viewport::ROTATE_RING_SEGMENTS as usize;
    let mut chosen = Vec::new();
    let mut occupied = Vec::<eframe::egui::Rect>::new();
    for (axis, ring) in geometry.lines.chunks_exact(segments).enumerate() {
        let preferred = (axis * segments / 3 + segments / 8) % segments;
        let candidate = ring
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                let path = grip_path(ring, index, camera, frame, matrix, rect)?;
                let mut bounds = eframe::egui::Rect::NOTHING;
                for (point, _) in &path {
                    bounds.extend_with(*point);
                }
                let bounds = bounds.expand(22.0);
                let overlap = occupied
                    .iter()
                    .filter(|previous| previous.intersects(bounds))
                    .count();
                let d = index.abs_diff(preferred);
                Some((
                    overlap * segments * 4 + d.min(segments - d),
                    line.color,
                    path,
                    bounds,
                ))
            })
            .min_by_key(|(score, _, _, _)| *score);
        let Some((_, color, path, bounds)) = candidate else {
            continue;
        };
        occupied.push(bounds);
        chosen.push((axis, color, path));
    }
    chosen
}

impl ResizeFrame {
    /// The ring whose drawn grip is within reach of `pointer` (viewport
    /// points), nearest first.
    pub(super) fn rotation_grip_hit(
        self,
        camera: &Camera,
        matrix: modeler3d_viewport::Mat4,
        rect: eframe::egui::Rect,
        pointer: eframe::egui::Pos2,
    ) -> Option<usize> {
        let geometry = self.rotation_geometry(camera)?;
        chosen_grips(&geometry, camera, self, matrix, rect)
            .into_iter()
            .filter_map(|(axis, _, path)| {
                let distance = path
                    .windows(2)
                    .map(|pair| distance_to_segment(pointer, pair[0].0, pair[1].0))
                    .fold(f32::INFINITY, f32::min);
                (distance <= GRIP_HIT_RADIUS).then_some((distance, axis))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, axis)| axis)
    }
}

fn distance_to_segment(
    point: eframe::egui::Pos2,
    start: eframe::egui::Pos2,
    end: eframe::egui::Pos2,
) -> f32 {
    let span = end - start;
    let length_squared = span.length_sq();
    if !length_squared.is_finite() || length_squared <= 1.0e-6 {
        return point.distance(start);
    }
    let along = ((point - start).dot(span) / length_squared).clamp(0.0, 1.0);
    point.distance(start + span * along)
}

/// Thin rings and filled double-ended grips share the mesh depth test.
/// Grips follow the owner's rotate.png proportions in viewport points.
pub(super) fn guide_geometry(
    frame: ResizeFrame,
    camera: &Camera,
    matrix: modeler3d_viewport::Mat4,
    rect: eframe::egui::Rect,
    emphasized: Option<usize>,
) -> GizmoGeometry {
    let Some(mut geometry) = frame.rotation_geometry(camera) else {
        return GizmoGeometry::default();
    };
    let Ok(inverse) = matrix.inverse() else {
        return GizmoGeometry::default();
    };
    let segments = modeler3d_viewport::ROTATE_RING_SEGMENTS as usize;
    for (axis, ring) in geometry.lines.chunks_exact_mut(segments).enumerate() {
        for line in ring {
            line.width_pixels = 1.0;
            if emphasized == Some(axis) {
                for value in &mut line.color[..3] {
                    *value += (1.0 - *value) * 0.35;
                }
            }
        }
    }
    for (_, color, path) in chosen_grips(&geometry, camera, frame, matrix, rect) {
        let world = |point: eframe::egui::Pos2, depth: f32| {
            inverse
                .transform_point([
                    (point.x - rect.left()) / rect.width() * 2.0 - 1.0,
                    1.0 - (point.y - rect.top()) / rect.height() * 2.0,
                    depth,
                ])
                .ok()
        };
        let mut triangle = |points: [(eframe::egui::Pos2, f32); 3]| {
            if let [Some(first), Some(second), Some(third)] =
                points.map(|(point, depth)| world(point, depth))
            {
                geometry.triangles.push(modeler3d_viewport::GizmoTriangle {
                    first,
                    second,
                    third,
                    color,
                });
            }
        };
        let edges: Vec<_> = path
            .iter()
            .enumerate()
            .map(|(i, (point, depth))| {
                let before = path[i.saturating_sub(1)].0;
                let after = path[(i + 1).min(path.len() - 1)].0;
                let across = (after - before).normalized().rot90() * 3.0;
                ((*point + across, *depth), (*point - across, *depth))
            })
            .collect();
        for pair in edges.windows(2) {
            triangle([pair[0].0, pair[0].1, pair[1].0]);
            triangle([pair[1].0, pair[0].1, pair[1].1]);
        }
        let last = path.len() - 1;
        for (end, neighbor) in [(0, 1), (last, last - 1)] {
            let (base, depth) = path[end];
            let outward = (base - path[neighbor].0).normalized();
            let across = outward.rot90() * 8.0;
            triangle([
                (base + outward * 14.0, depth),
                (base + across, depth),
                (base - across, depth),
            ]);
        }
    }
    geometry
}

/// A short visible arc with consistent projected direction avoids folded grips
/// when a ring is seen edge-on. Each node retains its own scene depth.
fn grip_path(
    ring: &[GizmoLine],
    index: usize,
    camera: &Camera,
    frame: ResizeFrame,
    matrix: modeler3d_viewport::Mat4,
    rect: eframe::egui::Rect,
) -> Option<Vec<(eframe::egui::Pos2, f32)>> {
    let mut path = Vec::new();
    let mut remaining = 32.0;
    for step in 0..ring.len() {
        let line = ring[(index + ring.len() - step) % ring.len()];
        if ring_segment_is_occluded(camera, &line, frame) {
            return None;
        }
        let [start, end] = super::project_gizmo_line(matrix, rect, line.start, line.end)?;
        if !rect.shrink(24.0).contains(start) || !rect.shrink(24.0).contains(end) {
            return None;
        }
        let first = matrix.transform_point(line.start).ok()?;
        let second = matrix.transform_point(line.end).ok()?;
        let length = start.distance(end);
        if length <= 0.01 {
            continue;
        }
        if path.is_empty() {
            path.push((end, second[2]));
        }
        let fraction = (remaining / length).min(1.0);
        path.push((
            end.lerp(start, fraction),
            second[2] + (first[2] - second[2]) * fraction,
        ));
        remaining -= length;
        if remaining <= 0.0 {
            break;
        }
    }
    if remaining > 0.0 || path.len() < 2 {
        return None;
    }
    let overall = (path[0].0 - path.last()?.0).normalized();
    path.windows(2)
        .all(|pair| (pair[0].0 - pair[1].0).normalized().dot(overall) > 0.7)
        .then_some(path)
}

/// Rejects rear arrow placements while allowing front arcs across the object.
/// Actual stroke visibility is determined per pixel by the scene depth buffer.
pub(super) fn ring_segment_is_occluded(
    camera: &Camera,
    line: &GizmoLine,
    frame: ResizeFrame,
) -> bool {
    let midpoint = [
        (line.start[0] + line.end[0]) * 0.5,
        (line.start[1] + line.end[1]) * 0.5,
        (line.start[2] + line.end[2]) * 0.5,
    ];
    let camera_position = camera.position();
    let direction = subtract(midpoint, camera_position);
    let Ok(world_ray) = Ray::new(camera_position, direction) else {
        return false;
    };
    let local_origin = subtract(world_ray.origin, frame.center);
    let local_ray = Ray::new(
        frame.axes.map(|axis| dot(local_origin, axis)),
        frame.axes.map(|axis| dot(world_ray.direction, axis)),
    );
    let half_extents = frame.extents.map(|extent| extent.abs() * 0.5);
    let Ok(bounds) = Aabb::new(half_extents.map(|extent| -extent), half_extents) else {
        return false;
    };
    let Ok(local_ray) = local_ray else {
        return false;
    };
    let distance = dot(direction, direction).sqrt();
    bounds
        .ray_entry(local_ray)
        .is_some_and(|entry| entry < distance - 1.0e-3)
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
    fn rotate_ring_rear_segments_hide_but_front_segments_cross_the_selection() {
        let mut camera = Camera::new();
        camera.set_view(0.0, 0.0).expect("front view");
        let frame = ResizeFrame {
            center: [0.; 3],
            axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            extents: [4.; 3],
        };
        let line = |z| GizmoLine {
            start: [0., 0., z],
            end: [0.1, 0., z],
            color: [1.; 4],
            width_pixels: 3.,
        };

        assert!(ring_segment_is_occluded(&camera, &line(-3.), frame));
        assert!(!ring_segment_is_occluded(&camera, &line(3.), frame));
    }

    #[test]
    fn thin_rotation_strokes_keep_front_and_rear_arcs_for_mesh_depth_testing() {
        let mut camera = Camera::new();
        camera.set_viewport_size(1280, 720).expect("viewport");
        camera
            .frame_bounds(&[Aabb::new([-20.; 3], [20.; 3]).expect("bounds")])
            .expect("frame");
        let frame = ResizeFrame {
            center: [0.; 3],
            axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            extents: [40.; 3],
        };
        let matrix = camera.view_projection(35.).expect("matrix");
        let rect = eframe::egui::Rect::from_min_size(
            eframe::egui::Pos2::ZERO,
            eframe::egui::vec2(1280., 720.),
        );
        let original = frame.rotation_geometry(&camera).expect("geometry");
        let guides = guide_geometry(frame, &camera, matrix, rect, None);
        let lines = guides.lines;
        assert_eq!(lines.len(), original.lines.len());
        assert!(!guides.triangles.is_empty(), "filled grips are present");
        assert!(lines.iter().all(|line| line.width_pixels == 1.0));
        for (actual, expected) in lines.iter().zip(&original.lines) {
            assert_eq!((actual.start, actual.end), (expected.start, expected.end));
        }
    }

    #[test]
    fn filled_rotation_grips_point_in_opposite_directions_when_rings_turn_edge_on() {
        let mut camera = Camera::new();
        camera.set_viewport_size(1280, 720).expect("viewport");
        camera
            .frame_bounds(&[Aabb::new([-20.; 3], [20.; 3]).expect("bounds")])
            .expect("frame");
        let frame = ResizeFrame {
            center: [0.; 3],
            axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            extents: [40.; 3],
        };
        let rect = eframe::egui::Rect::from_min_size(
            eframe::egui::Pos2::ZERO,
            eframe::egui::vec2(1280., 720.),
        );
        for (yaw, pitch) in [(0., 35.), (45., 25.), (90., 0.)] {
            camera.set_view(yaw, pitch).expect("view");
            let geometry = guide_geometry(
                frame,
                &camera,
                camera.view_projection(35.).expect("matrix"),
                rect,
                None,
            );
            for axis in [
                modeler3d_viewport::GizmoAxis::X,
                modeler3d_viewport::GizmoAxis::Y,
                modeler3d_viewport::GizmoAxis::Z,
            ] {
                let tips: Vec<_> = geometry
                    .triangles
                    .iter()
                    .filter(|triangle| triangle.color == axis.color())
                    .rev()
                    .take(2)
                    .collect();
                assert_eq!(
                    tips.len(),
                    2,
                    "both filled tips for {axis:?}, view {yaw}/{pitch}"
                );
                let outward = |triangle: &modeler3d_viewport::GizmoTriangle| {
                    let base = scale(add(triangle.second, triangle.third), 0.5);
                    modeler3d_viewport::normalize(subtract(triangle.first, base), "tip")
                        .expect("tip")
                };
                assert!(
                    dot(outward(tips[0]), outward(tips[1])) < -0.5,
                    "opposite tips for {axis:?}, view {yaw}/{pitch}"
                );
            }
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
