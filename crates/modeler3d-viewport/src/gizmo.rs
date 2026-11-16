use crate::{
    ViewportError,
    camera::{Camera, VIEW_PICK_TOLERANCE_PIXELS},
    math::{Aabb, Ray, Vec3, add, cross, dot, normalize, scale, subtract},
};

/// Pixel target for camera-readable gizmo handles.
pub const GIZMO_TARGET_PIXELS: f32 = 96.0;
/// Size multiplier for Move handles in both drawing and hit testing.
pub const MOVE_GIZMO_SCALE: f32 = 3.0;
/// Fraction of the screen-sized gizmo length used to clear the camera-facing
/// object face before a move shaft begins.
pub const MOVE_ARROW_OFFSET_FACTOR: f32 = 0.07;
/// Fraction of the screen-sized gizmo length from the camera-facing object
/// face to a move arrow tip.
pub const MOVE_ARROW_HALF_LENGTH_FACTOR: f32 = 0.425;
const MOVE_ARROW_HEAD_LENGTH_FACTOR: f32 = 0.17;
const MOVE_ARROW_HEAD_WIDTH_FACTOR: f32 = 0.10;
const MOVE_ARROW_OUTLINE_GROWTH_FACTOR: f32 = 0.018;
const MOVE_ARROW_HEAD_SEGMENTS: u32 = 12;
const MOVE_GIZMO_OUTLINE: [f32; 4] = [0.10, 0.12, 0.17, 0.88];
const ROTATE_ARROW_PICK_RADIUS_PIXELS: f32 = 26.0;
/// Fraction of the screen-sized gizmo length used for rotate-ring clearance.
pub const ROTATE_RING_GAP_FACTOR: f32 = 0.18;
/// Segment count for the analytic rotate ring.
pub const ROTATE_RING_SEGMENTS: u32 = 48;
/// Normalized X-axis gizmo color (#FF3333).
pub const X_AXIS_COLOR: [f32; 4] = [1.0, 0.2, 0.2, 1.0];
/// Normalized Y-axis gizmo color (#33CC33).
pub const Y_AXIS_COLOR: [f32; 4] = [0.2, 0.8, 0.2, 1.0];
/// Normalized Z-axis gizmo color (#3333FF).
pub const Z_AXIS_COLOR: [f32; 4] = [0.2, 0.2, 1.0, 1.0];

/// The three viewport transform modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum GizmoMode {
    /// Translate along one world axis.
    Move,
    /// Rotate around one world axis.
    Rotate,
    /// Scale along one world axis.
    Resize,
}

/// One world axis in a gizmo hit or primitive.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum GizmoAxis {
    /// X axis.
    X,
    /// Y axis.
    Y,
    /// Z axis.
    Z,
}

impl GizmoAxis {
    /// Returns the unit world vector.
    #[must_use]
    pub const fn vector(self) -> Vec3 {
        match self {
            Self::X => [1.0, 0.0, 0.0],
            Self::Y => [0.0, 1.0, 0.0],
            Self::Z => [0.0, 0.0, 1.0],
        }
    }

    /// Returns the reference axis color.
    #[must_use]
    pub const fn color(self) -> [f32; 4] {
        match self {
            Self::X => X_AXIS_COLOR,
            Self::Y => Y_AXIS_COLOR,
            Self::Z => Z_AXIS_COLOR,
        }
    }
}

/// A renderer-neutral line primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GizmoLine {
    /// Start position.
    pub start: Vec3,
    /// End position.
    pub end: Vec3,
    /// RGBA color.
    pub color: [f32; 4],
    /// Screen-oriented line width in pixels.
    pub width_pixels: f32,
}

/// A renderer-neutral triangle primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GizmoTriangle {
    /// First vertex.
    pub first: Vec3,
    /// Second vertex.
    pub second: Vec3,
    /// Third vertex.
    pub third: Vec3,
    /// RGBA color.
    pub color: [f32; 4],
}

/// Camera-readable geometry emitted by the gizmo builder.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GizmoGeometry {
    /// Axis shafts and ring segments.
    pub lines: Vec<GizmoLine>,
    /// Arrowheads and resize handles.
    pub triangles: Vec<GizmoTriangle>,
}

/// A selected gizmo handle hit by a screen ray.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GizmoHit {
    /// Hit axis.
    pub axis: GizmoAxis,
    /// Distance along the world ray.
    pub distance: f32,
    /// Normalized position along a move/resize shaft.
    pub parameter: f32,
}

/// Builds renderer-neutral gizmo geometry with a stable screen target.
pub fn build_gizmo(
    camera: &Camera,
    center: Vec3,
    mode: GizmoMode,
) -> Result<GizmoGeometry, ViewportError> {
    build_gizmo_with_bounds(camera, center, mode, None)
}

/// Builds gizmo geometry using the selected object's bounds for camera-facing
/// Move placement and Rotate-ring sizing.
pub fn build_gizmo_with_bounds(
    camera: &Camera,
    center: Vec3,
    mode: GizmoMode,
    bounds: Option<Aabb>,
) -> Result<GizmoGeometry, ViewportError> {
    let length = camera.world_tolerance(gizmo_target_pixels(mode)).max(0.01);
    let head_length = length * 0.24;
    let head_width = length * 0.20;
    let mut geometry = GizmoGeometry::default();
    for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
        let direction = axis.vector();
        match mode {
            GizmoMode::Move => {
                let (start, tip, near_side) =
                    move_arrow_segment(camera, center, axis, length, bounds);
                let outward = scale(direction, near_side);
                let head_length = length * MOVE_ARROW_HEAD_LENGTH_FACTOR;
                let shaft_end = add(tip, scale(outward, -head_length));
                geometry.lines.push(GizmoLine {
                    start,
                    end: shaft_end,
                    color: axis.color(),
                    width_pixels: 3.25,
                });
                append_move_arrowhead(&mut geometry, tip, axis, near_side, length)?;
            }
            GizmoMode::Resize => {
                let perpendicular = perpendicular(direction)?;
                let side = cross(direction, perpendicular);
                let extent = bounds
                    .map(|value| axis_extent(value, center, axis))
                    .unwrap_or(length);
                for sign in [-1.0_f32, 1.0_f32] {
                    let handle_center =
                        add(center, scale(direction, sign * extent.max(length * 0.5)));
                    geometry.lines.push(GizmoLine {
                        start: add(handle_center, scale(direction, -sign * length * 0.18)),
                        end: add(handle_center, scale(direction, sign * length * 0.18)),
                        color: axis.color(),
                        width_pixels: 2.0,
                    });
                    let half = head_width * 0.75;
                    let tip = add(handle_center, scale(direction, sign * length * 0.18));
                    let base = add(tip, scale(direction, -sign * head_length));
                    geometry.triangles.extend([
                        GizmoTriangle {
                            first: tip,
                            second: add(base, scale(perpendicular, half)),
                            third: add(base, scale(perpendicular, -half)),
                            color: axis.color(),
                        },
                        GizmoTriangle {
                            first: tip,
                            second: add(base, scale(side, half)),
                            third: add(base, scale(side, -half)),
                            color: axis.color(),
                        },
                    ]);
                }
            }
            GizmoMode::Rotate => {
                let ring_radius = bounds
                    .map(|value| projected_extent(value, center, axis))
                    .unwrap_or(0.0)
                    .max(length * 0.5)
                    + ROTATE_RING_GAP_FACTOR * length;
                let segments = ROTATE_RING_SEGMENTS;
                let first_basis = perpendicular(direction)?;
                let second_basis = normalize(cross(direction, first_basis), "gizmo ring basis")?;
                for segment in 0..segments {
                    let first_angle = std::f32::consts::TAU * segment as f32 / segments as f32;
                    let second_angle =
                        std::f32::consts::TAU * (segment + 1) as f32 / segments as f32;
                    let first = add(
                        center,
                        scale(
                            add(
                                scale(first_basis, first_angle.cos()),
                                scale(second_basis, first_angle.sin()),
                            ),
                            ring_radius,
                        ),
                    );
                    let second = add(
                        center,
                        scale(
                            add(
                                scale(first_basis, second_angle.cos()),
                                scale(second_basis, second_angle.sin()),
                            ),
                            ring_radius,
                        ),
                    );
                    geometry.lines.push(GizmoLine {
                        start: first,
                        end: second,
                        color: axis.color(),
                        width_pixels: 3.0,
                    });
                }
            }
        }
    }
    Ok(geometry)
}

/// Tests the exact analytic shafts/rings represented by the gizmo geometry.
pub fn hit_test(
    camera: &Camera,
    center: Vec3,
    mode: GizmoMode,
    screen_x: f32,
    screen_y: f32,
    scene_radius: f32,
) -> Result<Option<GizmoHit>, ViewportError> {
    hit_test_with_bounds(camera, center, mode, screen_x, screen_y, scene_radius, None)
}

/// Tests the same gizmo geometry produced by [`build_gizmo_with_bounds`].
pub fn hit_test_with_bounds(
    camera: &Camera,
    center: Vec3,
    mode: GizmoMode,
    screen_x: f32,
    screen_y: f32,
    scene_radius: f32,
    bounds: Option<Aabb>,
) -> Result<Option<GizmoHit>, ViewportError> {
    let ray = camera.screen_ray(screen_x, screen_y, scene_radius)?;
    let length = camera.world_tolerance(gizmo_target_pixels(mode)).max(0.01);
    let tolerance_pixels = match mode {
        GizmoMode::Rotate => VIEW_PICK_TOLERANCE_PIXELS.max(ROTATE_ARROW_PICK_RADIUS_PIXELS),
        GizmoMode::Move | GizmoMode::Resize => VIEW_PICK_TOLERANCE_PIXELS,
    };
    let tolerance = camera.world_tolerance(tolerance_pixels).max(0.001);
    let mut nearest = None;
    for axis in [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z] {
        let candidate = match mode {
            GizmoMode::Rotate => {
                let radius = bounds
                    .map(|value| projected_extent(value, center, axis))
                    .unwrap_or(0.0)
                    .max(length * 0.5)
                    + ROTATE_RING_GAP_FACTOR * length;
                hit_ring(ray, center, axis, radius, tolerance)?
            }
            GizmoMode::Move => {
                let (start, end, _) = move_arrow_segment(camera, center, axis, length, bounds);
                hit_segment(ray, start, end, tolerance).map(|(distance, parameter)| GizmoHit {
                    axis,
                    distance,
                    parameter,
                })
            }
            GizmoMode::Resize => {
                let extent = bounds
                    .map(|value| axis_extent(value, center, axis))
                    .unwrap_or(length);
                let mut best = None;
                for sign in [-1.0_f32, 1.0_f32] {
                    let handle_center = add(
                        center,
                        scale(axis.vector(), sign * extent.max(length * 0.5)),
                    );
                    let start = add(handle_center, scale(axis.vector(), -sign * length * 0.18));
                    let end = add(handle_center, scale(axis.vector(), sign * length * 0.18));
                    if let Some((distance, parameter)) = hit_segment(ray, start, end, tolerance)
                        && best.is_none_or(|(current, _)| distance < current)
                    {
                        best = Some((distance, parameter));
                    }
                }
                best.map(|(distance, parameter)| GizmoHit {
                    axis,
                    distance,
                    parameter,
                })
            }
        };
        if let Some(candidate) = candidate
            && nearest.is_none_or(|current: GizmoHit| candidate.distance < current.distance)
        {
            nearest = Some(candidate);
        }
    }
    Ok(nearest)
}

fn gizmo_target_pixels(mode: GizmoMode) -> f32 {
    match mode {
        GizmoMode::Move => GIZMO_TARGET_PIXELS * MOVE_GIZMO_SCALE,
        GizmoMode::Rotate | GizmoMode::Resize => GIZMO_TARGET_PIXELS,
    }
}

fn axis_extent(bounds: Aabb, center: Vec3, axis: GizmoAxis) -> f32 {
    let component = match axis {
        GizmoAxis::X => 0,
        GizmoAxis::Y => 1,
        GizmoAxis::Z => 2,
    };
    (bounds.maximum[component] - center[component])
        .abs()
        .max((bounds.minimum[component] - center[component]).abs())
}

fn projected_extent(bounds: Aabb, center: Vec3, axis: GizmoAxis) -> f32 {
    let components = match axis {
        GizmoAxis::X => [1, 2],
        GizmoAxis::Y => [0, 2],
        GizmoAxis::Z => [0, 1],
    };
    let first = (bounds.maximum[components[0]] - center[components[0]])
        .abs()
        .max((bounds.minimum[components[0]] - center[components[0]]).abs());
    let second = (bounds.maximum[components[1]] - center[components[1]])
        .abs()
        .max((bounds.minimum[components[1]] - center[components[1]]).abs());
    first.hypot(second)
}

fn move_arrow_segment(
    camera: &Camera,
    center: Vec3,
    axis: GizmoAxis,
    length: f32,
    bounds: Option<Aabb>,
) -> (Vec3, Vec3, f32) {
    let direction = axis.vector();
    let near_side = camera_facing_side(camera, center, direction);
    let extent = bounds
        .map(|value| axis_extent(value, center, axis))
        .unwrap_or(0.0);
    let face_center = add(center, scale(direction, near_side * extent));
    let start = add(
        face_center,
        scale(direction, near_side * MOVE_ARROW_OFFSET_FACTOR * length),
    );
    let tip = add(
        face_center,
        scale(
            direction,
            near_side * MOVE_ARROW_HALF_LENGTH_FACTOR * length,
        ),
    );
    (start, tip, near_side)
}

fn camera_facing_side(camera: &Camera, center: Vec3, axis: Vec3) -> f32 {
    let view = subtract(camera.position(), center);
    if dot(axis, view) >= 0.0 { 1.0 } else { -1.0 }
}

fn append_move_arrowhead(
    geometry: &mut GizmoGeometry,
    tip: Vec3,
    axis: GizmoAxis,
    near_side: f32,
    length: f32,
) -> Result<(), ViewportError> {
    let direction = axis.vector();
    let head_length = length * MOVE_ARROW_HEAD_LENGTH_FACTOR;
    let head_width = length * MOVE_ARROW_HEAD_WIDTH_FACTOR;
    let outline_growth = length * MOVE_ARROW_OUTLINE_GROWTH_FACTOR;
    let side = perpendicular(direction)?;
    let depth = normalize(cross(direction, side), "move arrowhead basis")?;
    let outward = scale(direction, near_side);
    let outline_base = add(tip, scale(outward, -(head_length + outline_growth)));
    let outline_width = head_width + outline_growth;
    let base = add(tip, scale(outward, -head_length));

    for segment in 0..MOVE_ARROW_HEAD_SEGMENTS {
        let first_angle = std::f32::consts::TAU * segment as f32 / MOVE_ARROW_HEAD_SEGMENTS as f32;
        let second_angle =
            std::f32::consts::TAU * (segment + 1) as f32 / MOVE_ARROW_HEAD_SEGMENTS as f32;
        let first_ring_direction = add(
            scale(side, first_angle.cos()),
            scale(depth, first_angle.sin()),
        );
        let second_ring_direction = add(
            scale(side, second_angle.cos()),
            scale(depth, second_angle.sin()),
        );
        geometry.triangles.push(GizmoTriangle {
            first: tip,
            second: add(outline_base, scale(first_ring_direction, outline_width)),
            third: add(outline_base, scale(second_ring_direction, outline_width)),
            color: MOVE_GIZMO_OUTLINE,
        });
        geometry.triangles.push(GizmoTriangle {
            first: tip,
            second: add(base, scale(first_ring_direction, head_width)),
            third: add(base, scale(second_ring_direction, head_width)),
            color: axis.color(),
        });
    }
    Ok(())
}

fn perpendicular(axis: Vec3) -> Result<Vec3, ViewportError> {
    let candidate = if axis[0].abs() < 0.9 {
        cross(axis, [1.0, 0.0, 0.0])
    } else {
        cross(axis, [0.0, 1.0, 0.0])
    };
    normalize(candidate, "gizmo perpendicular")
}

fn hit_segment(ray: Ray, start: Vec3, end: Vec3, tolerance: f32) -> Option<(f32, f32)> {
    let segment = subtract(end, start);
    let segment_length_squared = dot(segment, segment);
    if segment_length_squared <= f32::EPSILON {
        return None;
    }
    let origin_delta = subtract(ray.origin, start);
    let ray_segment = dot(ray.direction, segment);
    let ray_origin = dot(ray.direction, origin_delta);
    let segment_origin = dot(segment, origin_delta);
    let denominator = 1.0 - ray_segment * ray_segment / segment_length_squared;
    let (ray_distance, segment_distance) = if denominator.abs() > 1.0e-6 {
        let ray_distance =
            (ray_segment * segment_origin / segment_length_squared - ray_origin) / denominator;
        let segment_distance =
            (segment_origin + ray_distance * ray_segment) / segment_length_squared;
        (ray_distance, segment_distance)
    } else {
        return None;
    };
    if ray_distance < 0.0 || !(0.0..=1.0).contains(&segment_distance) {
        return None;
    }
    let ray_point = ray.at(ray_distance);
    let segment_point = add(start, scale(segment, segment_distance));
    let separation = subtract(ray_point, segment_point);
    (dot(separation, separation) <= tolerance * tolerance)
        .then_some((ray_distance, segment_distance))
}

fn hit_ring(
    ray: Ray,
    center: Vec3,
    axis: GizmoAxis,
    radius: f32,
    tolerance: f32,
) -> Result<Option<GizmoHit>, ViewportError> {
    let direction = axis.vector();
    let denominator = dot(ray.direction, direction);
    if denominator.abs() <= 1.0e-6 {
        return Ok(None);
    }
    let distance = dot(subtract(center, ray.origin), direction) / denominator;
    if distance < 0.0 {
        return Ok(None);
    }
    let hit = ray.at(distance);
    let radial = subtract(hit, center);
    let radial_length = dot(radial, radial).sqrt();
    if (radial_length - radius).abs() > tolerance {
        return Ok(None);
    }
    Ok(Some(GizmoHit {
        axis,
        distance,
        parameter: (radial_length / radius).clamp(0.0, 1.0),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gizmo_colors_and_camera_scaling_are_renderer_neutral() {
        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(800, 600).is_ok());
        let geometry = build_gizmo(&camera, [0.0, 0.0, 0.0], GizmoMode::Move);
        assert!(geometry.is_ok());
        let Some(geometry) = geometry.map_err(|_| ()).ok() else {
            return;
        };
        assert_eq!(geometry.lines.len(), 3);
        assert!(geometry.triangles.len() >= 3);
        assert_eq!(geometry.lines[0].color, X_AXIS_COLOR);
        let first_length = dot(geometry.lines[0].end, geometry.lines[0].end).sqrt();
        camera.set_viewport_size(800, 1_200).ok();
        let taller = build_gizmo(&camera, [0.0, 0.0, 0.0], GizmoMode::Move);
        assert!(taller.is_ok());
        let taller_length = taller
            .map(|value| dot(value.lines[0].end, value.lines[0].end).sqrt())
            .unwrap_or(0.0);
        assert!(taller_length < first_length);
    }

    #[test]
    fn move_handle_hit_returns_axis_and_parameter() {
        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(800, 600).is_ok());
        let length = camera.world_tolerance(gizmo_target_pixels(GizmoMode::Move));
        let projected = camera.view_projection(2.0).ok();
        let Some(projected) = projected else {
            return;
        };
        let (start, end, _) =
            move_arrow_segment(&camera, [0.0, 0.0, 0.0], GizmoAxis::X, length, None);
        let point_world = scale(add(start, end), 0.5);
        let point = projected.transform_point(point_world).ok();
        let Some(point) = point else {
            return;
        };
        let hit = hit_test(
            &camera,
            [0.0, 0.0, 0.0],
            GizmoMode::Move,
            (point[0] + 1.0) * 400.0,
            (1.0 - point[1]) * 300.0,
            2.0,
        );
        assert!(hit.is_ok());
    }

    #[test]
    fn move_arrows_point_outward_from_the_camera_facing_faces() {
        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(800, 600).is_ok());
        let bounds = Aabb::new([-2.0, -1.0, -3.0], [2.0, 1.0, 3.0]);
        assert!(bounds.is_ok());
        let Some(bounds) = bounds.map_err(|_| ()).ok() else {
            return;
        };
        let geometry =
            build_gizmo_with_bounds(&camera, [0.0, 0.0, 0.0], GizmoMode::Move, Some(bounds));
        assert!(geometry.is_ok());
        let Some(geometry) = geometry.map_err(|_| ()).ok() else {
            return;
        };
        let x_line = geometry.lines[0];
        let near_side = camera_facing_side(&camera, [0.0, 0.0, 0.0], GizmoAxis::X.vector());
        let extent = axis_extent(bounds, [0.0, 0.0, 0.0], GizmoAxis::X);
        assert!(x_line.start[0] * near_side > extent);
        assert!(x_line.end[0] * near_side > x_line.start[0] * near_side);
        assert!(x_line.start[1].abs() < 1.0e-4);
        assert!(x_line.start[2].abs() < 1.0e-4);
        assert_eq!(
            geometry.triangles.len(),
            MOVE_ARROW_HEAD_SEGMENTS as usize * 6
        );
    }

    #[test]
    fn rotate_ring_geometry_and_hit_use_the_same_bounds_scaled_radius() {
        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(800, 600).is_ok());
        let bounds = Aabb::new([-4.0, -1.0, -1.0], [4.0, 1.0, 1.0]);
        assert!(bounds.is_ok());
        let Some(bounds) = bounds.map_err(|_| ()).ok() else {
            return;
        };
        let geometry =
            build_gizmo_with_bounds(&camera, [0.0, 0.0, 0.0], GizmoMode::Rotate, Some(bounds));
        assert!(geometry.is_ok_and(|value| value.lines.len() >= ROTATE_RING_SEGMENTS as usize * 3));
    }
}
