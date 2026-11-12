use crate::{
    ViewportError,
    camera::Camera,
    lighting::ViewportLighting,
    math::{Aabb, Vec3, normalize},
    scene::RenderScene,
};

/// Default build-window floor extent in millimetres.
pub const DEFAULT_FLOOR_SIZE: f32 = 200.0;
/// Authored major metric spacing in millimetres.
pub const FLOOR_MAJOR_GRID_SPACING: f32 = 100.0;
/// Desired screen spacing for adaptive floor grid major lines.
pub const FLOOR_GRID_TARGET_PIXELS: f32 = 80.0;
/// Maximum grid line count on each axis.
pub const MAX_FLOOR_GRID_LINES: usize = 128;
/// Surface subdivisions used to blend the floor gradually into the viewport.
pub const FLOOR_SURFACE_SUBDIVISIONS: usize = 64;
/// Checker size used by the 3D Builder style floor, in millimetres.
pub const FLOOR_CHECKER_CELL_SIZE: f32 = 10.0;
/// Maximum cached contact-cue polygons.
pub const MAX_CONTACT_CUES: usize = 256;
/// Expanding translucent polygons that soften the cached floor-shadow edge.
pub(crate) const CONTACT_SHADOW_LAYERS: [(f32, f32); 12] = [
    (1.900, 0.050),
    (1.818, 0.053),
    (1.736, 0.059),
    (1.655, 0.067),
    (1.573, 0.076),
    (1.491, 0.087),
    (1.409, 0.098),
    (1.327, 0.111),
    (1.245, 0.125),
    (1.164, 0.139),
    (1.082, 0.154),
    (1.000, 0.170),
];

/// A renderer-neutral floor/grid line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorLine {
    /// Start point on y=0.
    pub start: Vec3,
    /// End point on y=0.
    pub end: Vec3,
    /// RGBA line color.
    pub color: [f32; 4],
    /// Fade factor at the start point.
    pub start_alpha: f32,
    /// Fade factor at the end point.
    pub end_alpha: f32,
    /// Width in pixels.
    pub width_pixels: f32,
}

/// Camera-relative floor surface bounded to the build plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorSurface {
    /// Minimum x/z corner.
    pub minimum: Vec3,
    /// Maximum x/z corner.
    pub maximum: Vec3,
    /// RGBA surface color.
    pub color: [f32; 4],
    /// Point on the build plane where the floor is most visible.
    pub fade_center: Vec3,
    /// Unit direction across the build plane toward the far edge of the view.
    pub fade_direction: Vec3,
    /// Size of the alternating floor squares in millimetres.
    pub checker_cell_size: f32,
    /// Distance toward the far edge where the floor fade begins.
    pub fade_start_distance: f32,
    /// Distance toward the far edge where the floor reaches the viewport background.
    pub fade_end_distance: f32,
}

impl FloorSurface {
    /// Returns the unbounded fade position, where 0 is fully visible and 1 is
    /// the viewport background.
    #[must_use]
    pub fn fade_parameter_at(&self, point: Vec3) -> f32 {
        let dx = point[0] - self.fade_center[0];
        let dz = point[2] - self.fade_center[2];
        let distance = dx * self.fade_direction[0] + dz * self.fade_direction[2];
        (distance - self.fade_start_distance)
            / (self.fade_end_distance - self.fade_start_distance).max(f32::EPSILON)
    }

    /// Returns the smooth opacity of the floor at a build-plane point.
    #[must_use]
    pub fn fade_alpha_at(&self, point: Vec3) -> f32 {
        smooth_fade(self.fade_parameter_at(point), 0.0, 1.0)
    }
}

/// A distance label placed on a world-axis line on the floor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorScaleLabel {
    /// Non-negative distance along a floor axis from the fixed world origin.
    pub value_mm: u32,
    /// World-space position on the build plane, stable while the camera pans.
    pub position: Vec3,
    /// Opacity inherited from the floor's far-edge fade.
    pub alpha: f32,
}

/// A bounded, camera-relative directional shadow cue on the floor.
#[derive(Clone, Debug, PartialEq)]
pub struct ContactCue {
    /// Source object ID.
    pub object_id: String,
    /// Fallback footprint polygon on y=0.
    pub polygon: Vec<Vec3>,
    /// RGBA cue color.
    pub color: [f32; 4],
    /// World-space bounds projected in the current light direction.
    pub bounds: Aabb,
}

impl ContactCue {
    /// Returns a floor shadow projected opposite the current key light.
    pub(crate) fn shadow_polygon(&self, light_world: Option<Vec3>) -> Vec<Vec3> {
        let Some(light_world) = light_world else {
            return self.polygon.clone();
        };
        if light_world[1] <= 0.05 {
            return self.polygon.clone();
        }
        let extent = [
            self.bounds.maximum[0] - self.bounds.minimum[0],
            self.bounds.maximum[1] - self.bounds.minimum[1],
            self.bounds.maximum[2] - self.bounds.minimum[2],
        ];
        let maximum_shadow_distance =
            (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt() * 4.0
                + 10.0;
        let points = self
            .bounds
            .corners()
            .into_iter()
            .map(|point| {
                let distance = (point[1].max(0.0) / light_world[1]).min(maximum_shadow_distance);
                [
                    point[0] - light_world[0] * distance,
                    0.004,
                    point[2] - light_world[2] * distance,
                ]
            })
            .collect();
        let polygon = convex_hull_xz(points);
        if polygon.len() >= 3 {
            polygon
        } else {
            self.polygon.clone()
        }
    }
}

/// Converts the active view-space key light to world space once per frame.
pub(crate) fn shadow_light_direction_world(
    camera: &Camera,
    lighting: ViewportLighting,
) -> Option<Vec3> {
    let view_matrix = camera.view_matrix().ok()?;
    let view_to_world = view_matrix.inverse().ok()?;
    let view_light = lighting.direction_in_view(view_matrix);
    let light_world4 = view_to_world.transform4([view_light[0], view_light[1], view_light[2], 0.0]);
    normalize(
        [light_world4[0], light_world4[1], light_world4[2]],
        "floor shadow light direction",
    )
    .ok()
}

/// Expands a shadow polygon around its center to create one soft alpha layer.
pub(crate) fn expand_shadow_polygon(polygon: &[Vec3], scale: f32) -> Vec<Vec3> {
    if polygon.is_empty() {
        return Vec::new();
    }
    let mut center = [0.0_f32, 0.004, 0.0];
    for point in polygon {
        center[0] += point[0] / polygon.len() as f32;
        center[2] += point[2] / polygon.len() as f32;
    }
    polygon
        .iter()
        .map(|point| {
            [
                center[0] + (point[0] - center[0]) * scale,
                point[1],
                center[2] + (point[2] - center[2]) * scale,
            ]
        })
        .collect()
}

/// Floor and contact geometry emitted for one viewport frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloorGeometry {
    /// Whether the floor is below the camera and may be drawn.
    pub visible: bool,
    /// Adaptive minor spacing in world units.
    pub minor_spacing: f32,
    /// Adaptive major spacing in world units.
    pub major_spacing: f32,
    /// Grid and axis lines.
    pub lines: Vec<FloorLine>,
    /// Actual bounded build-plane surface.
    pub surface: Option<FloorSurface>,
    /// +Z front orientation cue.
    pub front_cue: Vec<FloorLine>,
    /// Cached/settled contact cues.
    pub contact_cues: Vec<ContactCue>,
}

/// Incremental cache of object bounds used for floor-shadow projection.
#[derive(Clone, Debug, Default)]
pub struct ContactCueCache {
    cues: Vec<ContactCue>,
    scene_revision: u64,
}

impl ContactCueCache {
    /// Returns an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Rebuilds the bounded cache after interaction settles.
    pub fn settle(
        &mut self,
        scene: &RenderScene,
        scene_revision: u64,
    ) -> Result<(), ViewportError> {
        if self.scene_revision == scene_revision && !self.cues.is_empty() {
            return Ok(());
        }
        let mut cues = Vec::new();
        for object in scene.objects().iter().take(MAX_CONTACT_CUES) {
            let bounds = object.world_bounds()?;
            let polygon = footprint(bounds);
            if polygon.len() >= 3 {
                cues.push(ContactCue {
                    object_id: object.object_id().to_owned(),
                    polygon,
                    color: [0.05, 0.08, 0.12, 0.28],
                    bounds,
                });
            }
        }
        self.cues = cues;
        self.scene_revision = scene_revision;
        Ok(())
    }

    /// Returns cached cues during camera motion and defers scene-bound work
    /// until interaction settles when the cache is stale.
    pub fn cues_for_frame(
        &self,
        camera: &Camera,
        scene: &RenderScene,
        active_interaction: bool,
        scene_revision: u64,
    ) -> Result<Vec<ContactCue>, ViewportError> {
        if self.scene_revision == scene_revision && !self.cues.is_empty() {
            return Ok(self.cues.clone());
        }
        if active_interaction {
            return Ok(Vec::new());
        }
        let mut fallback = Self::default();
        fallback.settle(scene, scene_revision)?;
        let _ = camera;
        Ok(fallback.cues)
    }
}

/// Builds camera-relative adaptive floor/grid geometry.
pub fn build_floor(
    camera: &Camera,
    bed_size: f32,
    show_grid: bool,
    show_orientation: bool,
) -> Result<FloorGeometry, ViewportError> {
    if !bed_size.is_finite() || bed_size <= 0.0 {
        return Err(ViewportError::InvalidInput(String::from(
            "floor size must be positive and finite",
        )));
    }
    let visible = camera.position()[1] >= -0.001;
    if !visible {
        return Ok(FloorGeometry {
            visible: false,
            ..FloorGeometry::default()
        });
    }
    let major_spacing = FLOOR_MAJOR_GRID_SPACING;
    let minor_spacing = adaptive_minor_spacing(camera.distance());
    let center = camera.center();
    let half_extent = (bed_size * 0.5).max(camera.distance() * 8.0).max(1_000.0);
    let center_x = snap(center[0], major_spacing);
    let center_z = snap(center[2], major_spacing);
    let yaw = camera.yaw_degrees().to_radians();
    let pitch = camera.pitch_degrees().to_radians();
    let fade_direction = [-yaw.sin(), 0.0, -yaw.cos()];
    let top_ray_angle = pitch - camera.vertical_fov_degrees().to_radians() * 0.5;
    let horizontal_eye_distance = camera.distance() * pitch.cos().abs();
    let visible_far_distance = if top_ray_angle > 0.001 {
        (camera.position()[1] / top_ray_angle.tan() - horizontal_eye_distance).max(50.0)
    } else {
        half_extent * 0.92
    };
    let fade_end_distance = visible_far_distance.min(half_extent * 0.98).max(50.0);
    let fade_start_distance = (fade_end_distance * 0.4).min(fade_end_distance - 25.0);
    let surface = FloorSurface {
        minimum: [center_x - half_extent, -0.002, center_z - half_extent],
        maximum: [center_x + half_extent, -0.002, center_z + half_extent],
        color: [0.74, 0.74, 0.74, 1.0],
        fade_center: [center[0], 0.0, center[2]],
        fade_direction,
        checker_cell_size: FLOOR_CHECKER_CELL_SIZE,
        fade_start_distance,
        fade_end_distance,
    };
    let minimum_x = center_x - half_extent;
    let maximum_x = center_x + half_extent;
    let minimum_z = center_z - half_extent;
    let maximum_z = center_z + half_extent;
    let mut lines = Vec::new();
    if show_grid {
        for x in grid_positions(minimum_x, maximum_x, minor_spacing, major_spacing) {
            let major = (x / major_spacing).round().abs() * major_spacing;
            let is_major = (x - major).abs() <= minor_spacing * 0.1;
            let color = if is_major {
                [0.20, 0.26, 0.34, 0.55]
            } else {
                [0.16, 0.20, 0.27, 0.30]
            };
            let width_pixels = if is_major { 1.5 } else { 1.0 };
            push_faded_line(
                &mut lines,
                [x, 0.0, minimum_z],
                [x, 0.0, center_z],
                color,
                width_pixels,
                &surface,
            );
            push_faded_line(
                &mut lines,
                [x, 0.0, center_z],
                [x, 0.0, maximum_z],
                color,
                width_pixels,
                &surface,
            );
        }
        for z in grid_positions(minimum_z, maximum_z, minor_spacing, major_spacing) {
            let major = (z / major_spacing).round().abs() * major_spacing;
            let is_major = (z - major).abs() <= minor_spacing * 0.1;
            let color = if is_major {
                [0.20, 0.26, 0.34, 0.55]
            } else {
                [0.16, 0.20, 0.27, 0.30]
            };
            let width_pixels = if is_major { 1.5 } else { 1.0 };
            push_faded_line(
                &mut lines,
                [minimum_x, 0.0, z],
                [center_x, 0.0, z],
                color,
                width_pixels,
                &surface,
            );
            push_faded_line(
                &mut lines,
                [center_x, 0.0, z],
                [maximum_x, 0.0, z],
                color,
                width_pixels,
                &surface,
            );
        }
    }
    let front_cue = if show_orientation {
        orientation_cue(camera, half_extent.min(32.0))
    } else {
        Vec::new()
    };
    Ok(FloorGeometry {
        visible,
        minor_spacing,
        major_spacing,
        lines,
        surface: Some(surface),
        front_cue,
        contact_cues: Vec::new(),
    })
}

/// Builds readable metric labels at fixed world-space floor-axis coordinates.
///
/// Panning changes each label's screen projection but does not move its world
/// position, keeping the text attached to the floor grid.
#[must_use]
pub fn build_floor_scale_labels(camera: &Camera) -> Vec<FloorScaleLabel> {
    if camera.position()[1] < -0.001 {
        return Vec::new();
    }
    let Ok(floor) = build_floor(camera, DEFAULT_FLOOR_SIZE, false, false) else {
        return Vec::new();
    };
    let Some(surface) = floor.surface else {
        return Vec::new();
    };
    let interval = floor_label_interval(camera.distance());
    let label_range = (surface.fade_end_distance * 1.05).max(interval * 2.0);
    let steps = (label_range / interval).ceil().clamp(1.0, 32.0) as i32;
    let mut labels = Vec::with_capacity((steps as usize * 2).saturating_add(1));
    labels.push(FloorScaleLabel {
        value_mm: 0,
        position: [0.0, -0.01, 0.0],
        alpha: surface.fade_alpha_at([0.0, 0.0, 0.0]),
    });

    // Keep labels on the two grid axes, on the side leading away from the
    // camera. When the camera looks straight down an axis, use its positive
    // direction as a stable tie-breaker.
    let x_direction = if surface.fade_direction[0].abs() > f32::EPSILON {
        surface.fade_direction[0].signum()
    } else {
        1.0
    };
    let z_direction = if surface.fade_direction[2].abs() > f32::EPSILON {
        surface.fade_direction[2].signum()
    } else {
        1.0
    };

    for step in 1..=steps {
        let offset = step as f32 * interval;
        for position in [
            [x_direction * offset, -0.01, 0.0],
            [0.0, -0.01, z_direction * offset],
        ] {
            let alpha = surface.fade_alpha_at(position);
            if alpha <= 0.04 {
                continue;
            }
            labels.push(FloorScaleLabel {
                value_mm: offset.round() as u32,
                position,
                alpha,
            });
        }
    }
    labels
}

fn floor_label_interval(camera_distance: f32) -> f32 {
    if camera_distance <= 1_500.0 {
        50.0
    } else if camera_distance <= 3_500.0 {
        100.0
    } else {
        500.0
    }
}

fn push_faded_line(
    lines: &mut Vec<FloorLine>,
    start: Vec3,
    end: Vec3,
    color: [f32; 4],
    width_pixels: f32,
    surface: &FloorSurface,
) {
    let start_alpha = surface.fade_alpha_at(start);
    let end_alpha = surface.fade_alpha_at(end);
    if start_alpha <= f32::EPSILON && end_alpha <= f32::EPSILON {
        return;
    }
    lines.push(FloorLine {
        start,
        end,
        color,
        start_alpha,
        end_alpha,
        width_pixels,
    });
}

fn smooth_fade(distance: f32, start: f32, end: f32) -> f32 {
    if distance <= start {
        return 1.0;
    }
    if distance >= end {
        return 0.0;
    }
    let range = (end - start).max(f32::EPSILON);
    let t = ((distance - start) / range).clamp(0.0, 1.0);
    1.0 - t * t * (3.0 - 2.0 * t)
}

fn snap(value: f32, spacing: f32) -> f32 {
    if spacing <= f32::EPSILON {
        value
    } else {
        (value / spacing).round() * spacing
    }
}

fn adaptive_minor_spacing(distance: f32) -> f32 {
    [
        (0.8, 0.5),
        (2.5, 1.0),
        (7.0, 2.0),
        (20.0, 5.0),
        (50.0, 10.0),
        (120.0, 50.0),
    ]
    .into_iter()
    .find(|(maximum, _)| distance <= *maximum)
    .map_or(50.0, |(_, spacing)| spacing)
}

fn grid_positions(minimum: f32, maximum: f32, minor: f32, major: f32) -> Vec<f32> {
    let major_count = ((maximum - minimum) / major).ceil() as usize + 1;
    let mut positions = (0..major_count)
        .map(|index| (minimum / major).ceil() * major + index as f32 * major)
        .filter(|value| *value <= maximum + minor * 0.1)
        .collect::<Vec<_>>();
    let remaining = MAX_FLOOR_GRID_LINES.saturating_sub(positions.len());
    if remaining > 0 {
        for index in 0..=remaining {
            let value = minimum + (maximum - minimum) * index as f32 / remaining.max(1) as f32;
            if !positions
                .iter()
                .any(|existing| (*existing - value).abs() <= minor * 0.1)
            {
                positions.push(value);
            }
        }
    }
    positions.sort_by(f32::total_cmp);
    positions.truncate(MAX_FLOOR_GRID_LINES);
    positions
}

fn orientation_cue(camera: &Camera, length: f32) -> Vec<FloorLine> {
    let center = camera.center();
    let start = [center[0], 0.004, center[2]];
    let end = [center[0], 0.004, center[2] + length.max(8.0)];
    let head = length.max(8.0) * 0.25;
    vec![
        FloorLine {
            start,
            end,
            color: [0.24, 0.33, 0.42, 0.75],
            start_alpha: 1.0,
            end_alpha: 1.0,
            width_pixels: 2.0,
        },
        FloorLine {
            start: [end[0] - head, end[1], end[2] - head],
            end,
            color: [0.24, 0.33, 0.42, 0.75],
            start_alpha: 1.0,
            end_alpha: 1.0,
            width_pixels: 2.0,
        },
        FloorLine {
            start: [end[0] + head, end[1], end[2] - head],
            end,
            color: [0.24, 0.33, 0.42, 0.75],
            start_alpha: 1.0,
            end_alpha: 1.0,
            width_pixels: 2.0,
        },
    ]
}

fn footprint(bounds: Aabb) -> Vec<Vec3> {
    vec![
        [bounds.minimum[0], 0.004, bounds.minimum[2]],
        [bounds.maximum[0], 0.004, bounds.minimum[2]],
        [bounds.maximum[0], 0.004, bounds.maximum[2]],
        [bounds.minimum[0], 0.004, bounds.maximum[2]],
    ]
}

fn convex_hull_xz(mut points: Vec<Vec3>) -> Vec<Vec3> {
    points.sort_by(|left, right| {
        left[0]
            .total_cmp(&right[0])
            .then_with(|| left[2].total_cmp(&right[2]))
    });
    points.dedup_by(|left, right| {
        (left[0] - right[0]).abs() <= 0.0001 && (left[2] - right[2]).abs() <= 0.0001
    });
    if points.len() < 3 {
        return points;
    }
    let mut lower = Vec::new();
    for point in points.iter().copied() {
        while lower.len() >= 2
            && cross_xz(lower[lower.len() - 2], lower[lower.len() - 1], point) <= 0.0
        {
            lower.pop();
        }
        lower.push(point);
    }
    let mut upper = Vec::new();
    for point in points.iter().rev().copied() {
        while upper.len() >= 2
            && cross_xz(upper[upper.len() - 2], upper[upper.len() - 1], point) <= 0.0
        {
            upper.pop();
        }
        upper.push(point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn cross_xz(first: Vec3, second: Vec3, third: Vec3) -> f32 {
    (second[0] - first[0]) * (third[2] - first[2]) - (second[2] - first[2]) * (third[0] - first[0])
}

#[cfg(test)]
mod tests {
    use builder3d_core::{Scene, SceneObject, make_box};

    use super::*;
    use crate::scene::RenderScene;

    #[test]
    fn floor_spacing_is_adaptive_and_hidden_below_camera() {
        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(800, 600).is_ok());
        let floor = build_floor(&camera, DEFAULT_FLOOR_SIZE, true, true);
        assert!(floor.is_ok());
        let Some(floor) = floor.map_err(|_| ()).ok() else {
            return;
        };
        assert!(floor.visible);
        assert!(floor.major_spacing >= floor.minor_spacing);
        assert_eq!(floor.major_spacing, FLOOR_MAJOR_GRID_SPACING);
        assert!(floor.surface.is_some());
        assert!(!floor.lines.is_empty());
        let Some(surface) = floor.surface else {
            return;
        };
        assert_eq!(surface.fade_alpha_at(surface.fade_center), 1.0);
        assert_eq!(
            surface.fade_alpha_at([
                surface.fade_center[0] + surface.fade_direction[0] * surface.fade_end_distance,
                0.0,
                surface.fade_center[2] + surface.fade_direction[2] * surface.fade_end_distance,
            ]),
            0.0
        );
        let lateral = [
            surface.fade_center[0] - surface.fade_direction[2] * surface.fade_end_distance,
            0.0,
            surface.fade_center[2] + surface.fade_direction[0] * surface.fade_end_distance,
        ];
        assert_eq!(surface.fade_alpha_at(lateral), 1.0);
        let halfway = [
            surface.fade_center[0]
                + surface.fade_direction[0]
                    * (surface.fade_start_distance + surface.fade_end_distance)
                    * 0.5,
            0.0,
            surface.fade_center[2]
                + surface.fade_direction[2]
                    * (surface.fade_start_distance + surface.fade_end_distance)
                    * 0.5,
        ];
        let halfway_alpha = surface.fade_alpha_at(halfway);
        assert!(halfway_alpha > 0.0 && halfway_alpha < 1.0);
        assert!(
            floor
                .lines
                .iter()
                .any(|line| line.start_alpha < 1.0 || line.end_alpha < 1.0)
        );
        camera.reset_view();
        camera.set_view(0.0, 89.0).ok();
        let top = build_floor(&camera, DEFAULT_FLOOR_SIZE, true, true);
        assert!(top.is_ok_and(|value| value.visible));
        let top_labels = build_floor_scale_labels(&camera);
        assert!(top_labels.iter().any(|label| label.value_mm == 0));
        assert!(top_labels.iter().any(|label| label.value_mm == 50));
        assert!(top_labels.iter().any(|label| label.value_mm == 100));
        camera.set_view(0.0, -89.0).ok();
        let hidden = build_floor(&camera, DEFAULT_FLOOR_SIZE, true, true);
        assert!(hidden.is_ok_and(|value| !value.visible));
        assert!(build_floor_scale_labels(&camera).is_empty());
    }

    #[test]
    fn floor_scale_labels_remain_world_anchored_when_camera_pans() {
        let mut camera = Camera::new();
        camera.reset_view();
        assert!(camera.set_viewport_size(800, 600).is_ok());

        let before = build_floor_scale_labels(&camera);
        assert!(!before.is_empty());
        assert!(
            before
                .iter()
                .any(|label| { label.value_mm == 0 && label.position == [0.0, -0.01, 0.0] })
        );
        let before_positions = before
            .iter()
            .map(|label| (label.value_mm, label.position))
            .collect::<Vec<_>>();

        let before_center = camera.center();
        assert!(camera.pan(20.0, 0.0).is_ok());
        assert_ne!(camera.center(), before_center);
        let after = build_floor_scale_labels(&camera);
        let after_positions = after
            .iter()
            .map(|label| (label.value_mm, label.position))
            .collect::<Vec<_>>();

        assert_eq!(before_positions, after_positions);
    }

    #[test]
    fn cached_contact_shadows_remain_available_during_camera_motion() {
        let mesh = make_box([2.0, 2.0, 2.0]).map_err(|_| ()).ok();
        let Some(mesh) = mesh else {
            return;
        };
        let mut core_scene = Scene::new();
        assert!(core_scene.add_object(SceneObject::new(mesh)).is_ok());
        let render_scene = RenderScene::from_core(&core_scene).map_err(|_| ()).ok();
        let Some(render_scene) = render_scene else {
            return;
        };
        let mut cache = ContactCueCache::new();
        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(800, 600).is_ok());
        let active = cache.cues_for_frame(&camera, &render_scene, true, 1);
        assert!(active.is_ok_and(|value| value.is_empty()));
        assert!(cache.settle(&render_scene, 1).is_ok());
        let moving = cache.cues_for_frame(&camera, &render_scene, true, 1);
        assert!(moving.is_ok_and(|value| value.len() == 1));
        let settled = cache.cues_for_frame(&camera, &render_scene, false, 1);
        assert!(settled.is_ok_and(|value| value.len() == 1));
    }

    #[test]
    fn screen_aligned_shadow_direction_tracks_the_camera_orbit() {
        let mesh = make_box([2.0; 3]).expect("box");
        let mut core_scene = Scene::new();
        core_scene
            .add_object(SceneObject::new(mesh))
            .expect("add box");
        let render_scene = RenderScene::from_core(&core_scene).expect("render scene");
        let mut cache = ContactCueCache::new();
        cache.settle(&render_scene, 1).expect("settle cues");
        let cue = &cache.cues[0];
        let mut first_camera = Camera::new();
        first_camera.set_viewport_size(800, 600).expect("viewport");
        let first_light = shadow_light_direction_world(&first_camera, ViewportLighting::default());
        let first = cue.shadow_polygon(first_light);
        let first_center = first.iter().fold([0.0_f32; 3], |mut center, point| {
            for axis in [0, 2] {
                center[axis] += point[axis] / first.len() as f32;
            }
            center
        });
        first_camera.set_view(90.0, 35.0).expect("orbit");
        let rotated_light =
            shadow_light_direction_world(&first_camera, ViewportLighting::default());
        let rotated = cue.shadow_polygon(rotated_light);
        let rotated_center = rotated.iter().fold([0.0_f32; 3], |mut center, point| {
            for axis in [0, 2] {
                center[axis] += point[axis] / rotated.len() as f32;
            }
            center
        });
        assert!(first.len() >= 3);
        assert!(rotated.len() >= 3);
        assert_ne!(first_center, rotated_center);
    }
}
