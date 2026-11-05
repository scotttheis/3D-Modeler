use crate::{
    ViewportError,
    camera::Camera,
    math::{Aabb, Vec3},
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
/// Maximum cached contact-cue polygons.
pub const MAX_CONTACT_CUES: usize = 256;

/// A renderer-neutral floor/grid line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorLine {
    /// Start point on y=0.
    pub start: Vec3,
    /// End point on y=0.
    pub end: Vec3,
    /// RGBA line color.
    pub color: [f32; 4],
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
}

/// A bounded translucent footprint cue on the floor.
#[derive(Clone, Debug, PartialEq)]
pub struct ContactCue {
    /// Source object ID.
    pub object_id: String,
    /// Convex footprint polygon on y=0.
    pub polygon: Vec<Vec3>,
    /// RGBA cue color.
    pub color: [f32; 4],
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

/// Incremental cache of settled contact cues.
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
                });
            }
        }
        self.cues = cues;
        self.scene_revision = scene_revision;
        Ok(())
    }

    /// Returns cached cues for a settled frame. Active interaction deliberately
    /// emits no cues so camera motion cannot hitch on derived work.
    pub fn cues_for_frame(
        &self,
        camera: &Camera,
        scene: &RenderScene,
        active_interaction: bool,
        scene_revision: u64,
    ) -> Result<Vec<ContactCue>, ViewportError> {
        if active_interaction {
            return Ok(Vec::new());
        }
        if self.scene_revision == scene_revision {
            return Ok(self.cues.clone());
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
    let minimum_x = center_x - half_extent;
    let maximum_x = center_x + half_extent;
    let minimum_z = center_z - half_extent;
    let maximum_z = center_z + half_extent;
    let mut lines = Vec::new();
    if show_grid {
        for x in grid_positions(minimum_x, maximum_x, minor_spacing, major_spacing) {
            let major = (x / major_spacing).round().abs() * major_spacing;
            let is_major = (x - major).abs() <= minor_spacing * 0.1;
            lines.push(FloorLine {
                start: [x, 0.0, minimum_z],
                end: [x, 0.0, maximum_z],
                color: if is_major {
                    [0.20, 0.26, 0.34, 0.55]
                } else {
                    [0.16, 0.20, 0.27, 0.30]
                },
                width_pixels: if is_major { 1.5 } else { 1.0 },
            });
        }
        for z in grid_positions(minimum_z, maximum_z, minor_spacing, major_spacing) {
            let major = (z / major_spacing).round().abs() * major_spacing;
            let is_major = (z - major).abs() <= minor_spacing * 0.1;
            lines.push(FloorLine {
                start: [minimum_x, 0.0, z],
                end: [maximum_x, 0.0, z],
                color: if is_major {
                    [0.20, 0.26, 0.34, 0.55]
                } else {
                    [0.16, 0.20, 0.27, 0.30]
                },
                width_pixels: if is_major { 1.5 } else { 1.0 },
            });
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
        surface: Some(FloorSurface {
            minimum: [minimum_x, -0.002, minimum_z],
            maximum: [maximum_x, -0.002, maximum_z],
            color: [0.025, 0.04, 0.075, 1.0],
        }),
        front_cue,
        contact_cues: Vec::new(),
    })
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
            width_pixels: 2.0,
        },
        FloorLine {
            start: [end[0] - head, end[1], end[2] - head],
            end,
            color: [0.24, 0.33, 0.42, 0.75],
            width_pixels: 2.0,
        },
        FloorLine {
            start: [end[0] + head, end[1], end[2] - head],
            end,
            color: [0.24, 0.33, 0.42, 0.75],
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
        camera.set_view(0.0, -89.0).ok();
        let hidden = build_floor(&camera, DEFAULT_FLOOR_SIZE, true, true);
        assert!(hidden.is_ok_and(|value| !value.visible));
    }

    #[test]
    fn contact_cues_are_skipped_during_camera_motion_and_cached_at_rest() {
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
        let settled = cache.cues_for_frame(&camera, &render_scene, false, 1);
        assert!(settled.is_ok_and(|value| value.len() == 1));
    }
}
