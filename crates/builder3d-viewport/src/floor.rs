use crate::{
    ViewportError,
    camera::Camera,
    lighting::ViewportLighting,
    math::{Aabb, Mat4, Vec3, add, cross, dot, normalize, scale},
    scene::{RenderObject, RenderScene},
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
/// Mesh vertices sampled across the whole scene when a shadow outline is
/// rebuilt. Shadows are rebuilt only when the scene changes, never while the
/// camera moves, so this bounds the one-off cost on very dense scenes.
const SILHOUETTE_SCENE_VERTEX_BUDGET: usize = 200_000;
/// Minimum vertices sampled from one object, however crowded the scene.
const SILHOUETTE_OBJECT_VERTEX_FLOOR: usize = 2_048;
/// Azimuth steps per elevation ring of the silhouette support directions.
const SILHOUETTE_AZIMUTH_STEPS: usize = 24;
/// Elevations, in degrees, of the silhouette support-direction rings.
const SILHOUETTE_ELEVATIONS: [f32; 5] = [-60.0, -25.0, 0.0, 25.0, 60.0];
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

/// A distance label lying flat on the build plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorScaleLabel {
    /// Positive distance along a floor axis from the fixed world origin.
    pub value_mm: u32,
    /// World-space anchor of the text, stable while the camera pans.
    pub position: Vec3,
    /// Unit direction the text block extends in from `position`, or zero
    /// when `position` is the centre of the text.
    pub extend: Vec3,
    /// Unit world direction the text reads along.
    pub baseline: Vec3,
    /// Unit world direction from the top of a glyph to its bottom.
    pub down: Vec3,
    /// Em height of the text on the floor, in millimetres.
    pub text_height_mm: f32,
    /// Opacity from the floor's far-edge fade and the projected text size.
    pub alpha: f32,
}

/// A bounded, camera-relative directional shadow cue on the floor.
#[derive(Clone, Debug, PartialEq)]
pub struct ContactCue {
    /// Source object ID.
    pub object_id: String,
    /// Top-down outline of the object on y=0 (convex).
    pub polygon: Vec<Vec3>,
    /// World-space extreme points of the mesh: projecting these along any
    /// light direction and taking the hull gives the object's convex shadow.
    pub silhouette: Vec<Vec3>,
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
        let corners;
        let source: &[Vec3] = if self.silhouette.len() >= 3 {
            &self.silhouette
        } else {
            corners = self.bounds.corners();
            &corners
        };
        let points = source
            .iter()
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
        let object_count = scene.objects().len().clamp(1, MAX_CONTACT_CUES);
        let vertex_budget =
            (SILHOUETTE_SCENE_VERTEX_BUDGET / object_count).max(SILHOUETTE_OBJECT_VERTEX_FLOOR);
        for object in scene.objects().iter().take(MAX_CONTACT_CUES) {
            let bounds = object.world_bounds()?;
            let silhouette = silhouette_points(object, vertex_budget)?;
            let outline = convex_hull_xz(
                silhouette
                    .iter()
                    .map(|point| [point[0], 0.004, point[2]])
                    .collect(),
            );
            let polygon = if outline.len() >= 3 {
                outline
            } else {
                footprint(bounds)
            };
            if polygon.len() >= 3 {
                cues.push(ContactCue {
                    object_id: object.object_id().to_owned(),
                    polygon,
                    silhouette,
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

/// Spacing of the floor distance labels in millimetres. Like 3D Builder, the
/// spacing is fixed: zooming out shrinks the labels instead of thinning them.
pub const FLOOR_LABEL_SPACING_MM: f32 = 50.0;
/// Em height of the floor distance text, in millimetres on the build plane.
pub const FLOOR_LABEL_TEXT_HEIGHT_MM: f32 = 5.5;
/// Most labels laid along each floor axis.
pub const MAX_FLOOR_LABELS_PER_AXIS: usize = 40;
/// Projected em heights, in pixels, over which a distant label fades in
/// instead of shimmering as an unreadable speck.
const FLOOR_LABEL_FADE_PIXELS: (f32, f32) = (1.0, 3.5);
/// Distance of each label row from its axis, in millimetres: the X row runs
/// just in front of a default 40 mm primitive and the Z row just right of
/// it, as in 3D Builder.
const FLOOR_LABEL_ROW_OFFSET_MM: f32 = 24.0;

/// Builds the floor distance labels: text lying flat on the build plane at
/// fixed 50 mm steps along the world +X and -Z axes.
///
/// Every label is a world-space quad, so perspective foreshortens distant
/// labels instead of stacking upright screen text. Each label's baseline
/// follows the world axis closest to the camera's right-hand direction, so
/// the text reads left to right from any orbit. Positions never move while
/// the camera pans. The list is empty when the floor itself is hidden
/// (camera below the build plane).
#[must_use]
pub fn build_floor_scale_labels(camera: &Camera) -> Vec<FloorScaleLabel> {
    let Ok(floor) = build_floor(camera, DEFAULT_FLOOR_SIZE, false, false) else {
        return Vec::new();
    };
    let Some(surface) = floor.surface else {
        return Vec::new();
    };
    let yaw = camera.yaw_degrees().to_radians();
    let right = [yaw.cos(), 0.0, -yaw.sin()];
    let baseline = if right[0].abs() >= right[2].abs() {
        [right[0].signum(), 0.0, 0.0]
    } else {
        [0.0, 0.0, right[2].signum()]
    };
    // Glyph "down" on the floor points toward the viewer, so the text is not
    // mirrored when seen from above.
    let down = cross(baseline, [0.0, 1.0, 0.0]);
    let height = FLOOR_LABEL_TEXT_HEIGHT_MM;
    let projection = camera.view_projection(1.0).ok();
    let (width_px, height_px) = camera.viewport_size();
    let mut labels = Vec::with_capacity(MAX_FLOOR_LABELS_PER_AXIS * 2);
    for step in 1..=MAX_FLOOR_LABELS_PER_AXIS {
        let value = step as f32 * FLOOR_LABEL_SPACING_MM;
        // X labels are centred on their distance; Z labels start beside the
        // Z axis and extend to the right of it.
        for (position, extend) in [
            ([value, 0.0, FLOOR_LABEL_ROW_OFFSET_MM], [0.0; 3]),
            ([FLOOR_LABEL_ROW_OFFSET_MM, 0.0, -value], [1.0, 0.0, 0.0]),
        ] {
            let floor_alpha = surface.fade_alpha_at(position);
            // A label seen at a grazing angle is squashed flat but still
            // wide, and reads as a faint dash (as in 3D Builder), so the
            // larger projected em extent decides the fade.
            let pixel_alpha = projection.map_or(1.0, |matrix| {
                let em = |direction: Vec3| {
                    projected_length(
                        matrix,
                        position,
                        add(position, scale(direction, height)),
                        width_px,
                        height_px,
                    )
                };
                let pixels = em(down).max(em(baseline));
                1.0 - smooth_fade(pixels, FLOOR_LABEL_FADE_PIXELS.0, FLOOR_LABEL_FADE_PIXELS.1)
            });
            let alpha = floor_alpha * pixel_alpha;
            if alpha <= 0.02 {
                continue;
            }
            labels.push(FloorScaleLabel {
                value_mm: value.round() as u32,
                position,
                extend,
                baseline,
                down,
                text_height_mm: height,
                alpha,
            });
        }
    }
    labels
}

/// Pixel distance between two projected world points, or 0 when either lies
/// behind the camera.
fn projected_length(matrix: Mat4, first: Vec3, second: Vec3, width: u32, height: u32) -> f32 {
    let project = |point: Vec3| {
        let clip = matrix.transform4([point[0], point[1], point[2], 1.0]);
        (clip[3] > 1.0e-4 && clip.iter().all(|value| value.is_finite())).then(|| {
            [
                clip[0] / clip[3] * width as f32 * 0.5,
                clip[1] / clip[3] * height as f32 * 0.5,
            ]
        })
    };
    match (project(first), project(second)) {
        (Some(a), Some(b)) => ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt(),
        _ => 0.0,
    }
}

/// One glyph-mesh vertex of laid-out text, in the text's own flat
/// coordinates: `position` in points with y down and the origin at the top
/// left of the text block, `uv` normalised into the glyph atlas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanarTextVertex {
    /// Point position, y down.
    pub position: [f32; 2],
    /// Normalised atlas coordinate.
    pub uv: [f32; 2],
    /// Coverage multiplier from the text layout (1 for plain glyphs).
    pub alpha: f32,
}

/// Toolkit-neutral glyph mesh for one label string, produced by the UI
/// adapter's text layout and placed on the floor by [`build_floor_text`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlanarText {
    /// Width and height of the text block in points.
    pub size: [f32; 2],
    /// Em size the text was laid out at, in points.
    pub font_size: f32,
    /// Glyph vertices.
    pub vertices: Vec<PlanarTextVertex>,
    /// Triangle-list indices into `vertices`.
    pub indices: Vec<u32>,
}

/// One vertex of floor text in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FloorTextVertex {
    /// World position on the build plane.
    pub position: Vec3,
    /// Normalised atlas coordinate.
    pub uv: [f32; 2],
    /// sRGB-encoded colour with straight alpha.
    pub color: [f32; 4],
}

/// World-space text mesh drawn on the build plane, depth-tested against the
/// scene so objects standing on a label hide it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloorTextMesh {
    /// Vertices.
    pub vertices: Vec<FloorTextVertex>,
    /// Triangle-list indices into `vertices`.
    pub indices: Vec<u32>,
}

impl FloorTextMesh {
    /// Whether there is nothing to draw.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

/// Maps each label's laid-out glyph mesh onto the build plane.
///
/// `text_for` returns the glyph mesh for a label value, or `None` to skip
/// it. `color` is the sRGB-encoded label colour; each label's fade alpha
/// multiplies its alpha.
#[must_use]
pub fn build_floor_text<'a>(
    labels: &[FloorScaleLabel],
    mut text_for: impl FnMut(u32) -> Option<&'a PlanarText>,
    color: [f32; 4],
) -> FloorTextMesh {
    let mut mesh = FloorTextMesh::default();
    for label in labels {
        let Some(text) = text_for(label.value_mm) else {
            continue;
        };
        if text.font_size <= 0.0 || !text.font_size.is_finite() {
            continue;
        }
        let Ok(base) = u32::try_from(mesh.vertices.len()) else {
            break;
        };
        let world_per_point = label.text_height_mm / text.font_size;
        let half = [text.size[0] * 0.5, text.size[1] * 0.5];
        let center = add(
            label.position,
            scale(
                label.extend,
                (dot(label.baseline, label.extend).abs() * half[0]
                    + dot(label.down, label.extend).abs() * half[1])
                    * world_per_point,
            ),
        );
        for vertex in &text.vertices {
            let across = (vertex.position[0] - half[0]) * world_per_point;
            let along = (vertex.position[1] - half[1]) * world_per_point;
            let position = add(
                center,
                add(scale(label.baseline, across), scale(label.down, along)),
            );
            let mut vertex_color = color;
            vertex_color[3] *= label.alpha * vertex.alpha;
            mesh.vertices.push(FloorTextVertex {
                position,
                uv: vertex.uv,
                color: vertex_color,
            });
        }
        mesh.indices
            .extend(text.indices.iter().map(|index| base.saturating_add(*index)));
    }
    mesh
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

/// Unit support directions: rings around the vertical axis plus straight up
/// and down. Each keeps the mesh point furthest along it, so the kept points
/// approximate the mesh's convex hull from every side the light can come.
fn silhouette_directions() -> Vec<Vec3> {
    let mut directions = vec![[0.0, 1.0, 0.0], [0.0, -1.0, 0.0]];
    for elevation in SILHOUETTE_ELEVATIONS {
        let (sin_e, cos_e) = elevation.to_radians().sin_cos();
        for step in 0..SILHOUETTE_AZIMUTH_STEPS {
            let azimuth = std::f32::consts::TAU * step as f32 / SILHOUETTE_AZIMUTH_STEPS as f32;
            let (sin_a, cos_a) = azimuth.sin_cos();
            directions.push([cos_e * cos_a, sin_e, cos_e * sin_a]);
        }
    }
    directions
}

/// Samples at most `vertex_budget` vertices of an object's bounded display
/// mesh and returns its world-space extreme points, one per support
/// direction, without duplicates.
fn silhouette_points(
    object: &RenderObject,
    vertex_budget: usize,
) -> Result<Vec<Vec3>, ViewportError> {
    let geometry = object.interaction_geometry();
    let count = geometry.vertex_count();
    if count == 0 {
        return Ok(Vec::new());
    }
    let stride = count.div_ceil(vertex_budget.max(1)).max(1);
    let directions = silhouette_directions();
    let mut best = vec![(f32::NEG_INFINITY, [0.0_f32; 3]); directions.len()];
    let transform = object.transform();
    for index in (0..count).step_by(stride) {
        let Some(local) = u32::try_from(index)
            .ok()
            .and_then(|index| geometry.position(index))
        else {
            continue;
        };
        let world = transform.transform_point(local)?;
        for (slot, direction) in best.iter_mut().zip(&directions) {
            let support = dot(world, *direction);
            if support > slot.0 {
                *slot = (support, world);
            }
        }
    }
    let mut points = Vec::with_capacity(best.len());
    for (support, point) in best {
        if support.is_finite() && !points.contains(&point) {
            points.push(point);
        }
    }
    Ok(points)
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
        assert!(top_labels.iter().all(|label| label.value_mm > 0));
        assert!(top_labels.iter().any(|label| label.value_mm == 50));
        assert!(top_labels.iter().any(|label| label.value_mm == 100));
        camera.set_view(0.0, -89.0).ok();
        let hidden = build_floor(&camera, DEFAULT_FLOOR_SIZE, true, true);
        assert!(hidden.is_ok_and(|value| !value.visible));
        assert!(build_floor_scale_labels(&camera).is_empty());
    }

    #[test]
    fn floor_labels_lie_flat_at_fixed_steps_and_read_left_to_right() {
        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(1000, 700).is_ok());
        let mut state = camera.state();
        state.distance = 300.0;
        assert!(camera.set_state(state).is_ok());
        assert!(camera.set_view(0.0, 25.0).is_ok());
        let labels = build_floor_scale_labels(&camera);
        assert!(!labels.is_empty());
        assert!(labels.len() <= MAX_FLOOR_LABELS_PER_AXIS * 2);
        for label in &labels {
            assert_eq!(label.position[1], 0.0, "labels lie on the build plane");
            assert_eq!(label.value_mm % 50, 0);
            assert_eq!(label.baseline, [1.0, 0.0, 0.0]);
            assert_eq!(label.down, [0.0, 0.0, 1.0]);
            assert!((0.0..=1.0).contains(&label.alpha));
        }
        // From behind, text turns around instead of reading mirrored.
        assert!(camera.set_view(180.0, 25.0).is_ok());
        let behind = build_floor_scale_labels(&camera);
        assert!(
            behind
                .iter()
                .all(|label| label.baseline == [-1.0, 0.0, 0.0])
        );
        // From the side, text follows the Z axis.
        assert!(camera.set_view(90.0, 25.0).is_ok());
        let side = build_floor_scale_labels(&camera);
        assert!(side.iter().all(|label| label.baseline[0] == 0.0));
    }

    #[test]
    fn distant_floor_labels_fade_instead_of_piling_up() {
        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(1000, 700).is_ok());
        let mut state = camera.state();
        state.distance = 400.0;
        assert!(camera.set_state(state).is_ok());
        assert!(camera.set_view(20.0, 12.0).is_ok());
        let labels = build_floor_scale_labels(&camera);
        let alpha_at = |value: u32| {
            labels
                .iter()
                .filter(|label| label.value_mm == value && label.position[0] == 24.0)
                .map(|label| label.alpha)
                .next()
                .unwrap_or(0.0)
        };
        let near = alpha_at(50);
        let far = alpha_at(1_500);
        assert!(near > 0.5, "near label is readable: {near}");
        assert!(far < 0.8 && far < near, "far label fades: {far} vs {near}");
        // Past the floor's fade there are no labels at all.
        assert!(alpha_at(2_000) < far);
    }

    #[test]
    fn floor_text_maps_glyphs_onto_the_plane_at_the_label_size() {
        let label = FloorScaleLabel {
            value_mm: 50,
            position: [50.0, 0.0, 24.0],
            extend: [0.0; 3],
            baseline: [1.0, 0.0, 0.0],
            down: [0.0, 0.0, 1.0],
            text_height_mm: 5.0,
            alpha: 0.5,
        };
        let quad = |x0: f32, y0: f32, x1: f32, y1: f32| PlanarText {
            size: [x1, y1],
            font_size: 10.0,
            vertices: [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
                .into_iter()
                .map(|position| PlanarTextVertex {
                    position,
                    uv: [0.5, 0.5],
                    alpha: 1.0,
                })
                .collect(),
            indices: vec![0, 1, 2, 0, 2, 3],
        };
        let text = quad(0.0, 0.0, 20.0, 10.0);
        let mesh = build_floor_text(&[label, label], |_| Some(&text), [0.2, 0.2, 0.2, 0.8]);
        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.indices[6..], [4, 5, 6, 4, 6, 7]);
        // 20 x 10 points at a 10 pt em, 5 mm high: 10 x 5 mm, centred.
        assert_eq!(mesh.vertices[0].position, [45.0, 0.0, 21.5]);
        assert_eq!(mesh.vertices[2].position, [55.0, 0.0, 26.5]);
        assert!((mesh.vertices[0].color[3] - 0.4).abs() < 1.0e-6);
        // A start-anchored label extends away from its anchor.
        let started = FloorScaleLabel {
            extend: [1.0, 0.0, 0.0],
            ..label
        };
        let mesh = build_floor_text(&[started], |_| Some(&text), [0.2; 4]);
        assert_eq!(mesh.vertices[0].position[0], 50.0);
        assert!(build_floor_text(&[label], |_| None, [0.2; 4]).is_empty());
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
                .any(|label| { label.value_mm == 50 && label.position == [24.0, 0.0, -50.0] })
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
