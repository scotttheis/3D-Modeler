use std::path::Path;

use crate::{
    ViewportError,
    camera::Camera,
    floor::{
        CONTACT_SHADOW_LAYERS, ContactCueCache, DEFAULT_FLOOR_SIZE, FLOOR_SURFACE_SUBDIVISIONS,
        FloorGeometry, FloorSurface, build_floor, expand_shadow_polygon,
        shadow_light_direction_world,
    },
    lighting::ViewportLighting,
    math::{Mat4, Vec3, normalize},
    scene::{MeshGeometry, RenderObject, RenderScene},
};

/// Bounded CPU raster dimensions for the no-accelerator fallback.
pub const MAX_CPU_RENDER_DIMENSION: u32 = 2_048;
/// Maximum triangles processed by one CPU fallback frame.
pub const MAX_CPU_RENDER_TRIANGLES: u64 = 4_000_000;

/// CPU fallback framebuffer and timing-neutral counters.
#[derive(Clone, Debug)]
pub struct CpuFrame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8 pixels, row-major top-left origin.
    pub rgba: Vec<u8>,
    /// Triangles visited by the bounded rasterizer.
    pub triangles: u64,
    /// Pixels that passed the depth test.
    pub shaded_pixels: u64,
}

/// Presentation options for the deterministic CPU fallback.
#[derive(Clone, Debug, PartialEq)]
pub struct CpuRenderOptions {
    /// RGBA8 clear color.
    pub clear_color: [u8; 4],
    /// Draw opaque black mesh edges over the depth-tested scene.
    pub wireframe: bool,
    /// World guide strokes, depth tested against the same scene as GPU guides.
    pub guide_lines: Vec<crate::GizmoLine>,
    /// Blend mesh colors translucently without changing authored geometry.
    pub xray: bool,
    /// Durable IDs drawn with the selection shade
    /// ([`crate::selection_shade`]).
    pub selected_object_ids: Vec<String>,
    /// View aligned key light, material highlights, and contact shadow strength.
    pub lighting: ViewportLighting,
    /// Whether the build-plane surface is drawn.
    pub floor_enabled: bool,
    /// Whether the metric floor grid is drawn.
    pub grid_visible: bool,
    /// RGBA floor surface color.
    pub floor_color: [f32; 4],
    /// RGBA major grid and orientation color.
    pub grid_major_color: [f32; 4],
    /// RGBA minor grid color.
    pub grid_minor_color: [f32; 4],
    /// RGBA contact shadow tint and its unscaled alpha.
    pub contact_shadow_color: [f32; 4],
}

impl Default for CpuRenderOptions {
    fn default() -> Self {
        Self {
            clear_color: [4, 7, 13, 255],
            wireframe: false,
            guide_lines: Vec::new(),
            xray: false,
            selected_object_ids: Vec::new(),
            lighting: ViewportLighting::default(),
            floor_enabled: false,
            grid_visible: false,
            floor_color: [0.025, 0.04, 0.075, 1.0],
            grid_major_color: [0.20, 0.26, 0.34, 0.55],
            grid_minor_color: [0.16, 0.20, 0.27, 0.30],
            contact_shadow_color: [0.05, 0.08, 0.12, 0.28],
        }
    }
}

/// Renders a scene using the deterministic CPU fallback.
pub fn render_cpu(
    scene: &RenderScene,
    camera: &Camera,
    width: u32,
    height: u32,
    interaction: bool,
) -> Result<CpuFrame, ViewportError> {
    render_cpu_with_options(
        scene,
        camera,
        width,
        height,
        interaction,
        &CpuRenderOptions::default(),
    )
}

/// Renders a deterministic CPU fallback with explicit display options.
pub fn render_cpu_with_options(
    scene: &RenderScene,
    camera: &Camera,
    width: u32,
    height: u32,
    interaction: bool,
    options: &CpuRenderOptions,
) -> Result<CpuFrame, ViewportError> {
    if width == 0
        || height == 0
        || width > MAX_CPU_RENDER_DIMENSION
        || height > MAX_CPU_RENDER_DIMENSION
    {
        return Err(ViewportError::InvalidInput(String::from(
            "CPU fallback dimensions must be between 1 and 2048",
        )));
    }
    let pixel_count = usize::try_from(width)
        .ok()
        .and_then(|value| {
            usize::try_from(height)
                .ok()
                .and_then(|height| value.checked_mul(height))
        })
        .ok_or_else(|| {
            ViewportError::CapacityExceeded(String::from("CPU framebuffer size overflow"))
        })?;
    let byte_count = pixel_count.checked_mul(4).ok_or_else(|| {
        ViewportError::CapacityExceeded(String::from("CPU pixel buffer size overflow"))
    })?;
    let scene_radius = scene
        .world_bounds()?
        .map(|bounds| {
            let extent = [
                bounds.maximum[0] - bounds.minimum[0],
                bounds.maximum[1] - bounds.minimum[1],
                bounds.maximum[2] - bounds.minimum[2],
            ];
            (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt() * 0.5
        })
        .unwrap_or(1.0);
    let view_matrix = camera.view_matrix()?;
    let view_projection = camera.view_projection(scene_radius)?;
    let light_direction = options.lighting.direction_in_view(view_matrix);
    let material_parameters = options.lighting.shader_parameters();
    let mut rgba = vec![0_u8; byte_count];
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel.copy_from_slice(&options.clear_color);
    }
    let mut depth = vec![f32::INFINITY; pixel_count];
    let mut triangles = 0_u64;
    let mut shaded_pixels = 0_u64;
    let mut floor = if options.floor_enabled {
        Some(build_floor(
            camera,
            DEFAULT_FLOOR_SIZE,
            options.grid_visible,
            true,
        )?)
    } else {
        None
    };
    if options.lighting.normalized().contact_shadow_percent > 0
        && let Some(floor) = floor.as_mut()
        && floor.visible
    {
        let mut cache = ContactCueCache::new();
        cache.settle(scene, 0)?;
        floor.contact_cues = cache.cues_for_frame(camera, scene, interaction, 0)?;
    }
    if let Some(floor) = floor.as_ref() {
        rasterize_floor_surface(
            floor,
            view_projection,
            width,
            height,
            options.floor_color,
            &mut rgba,
            &mut depth,
            &mut shaded_pixels,
        );
        rasterize_contact_cues(
            floor,
            camera,
            view_projection,
            width,
            height,
            options.contact_shadow_color,
            options.lighting,
            &mut rgba,
            &mut depth,
            &mut shaded_pixels,
        );
    }
    // X-ray (OF-47): selected objects are solid and drawn first, so the
    // pale see-through ones blend over them.
    let is_selected = |object: &crate::scene::RenderObject| {
        options
            .selected_object_ids
            .iter()
            .any(|id| id == object.object_id())
    };
    let mut draw_order = scene.objects().iter().collect::<Vec<_>>();
    if options.xray {
        draw_order.sort_by_key(|object| !is_selected(object));
    }
    let mut outlines = Vec::new();
    for object in draw_order {
        let geometry = if interaction {
            object.interaction_geometry()
        } else {
            object.full_geometry()
        };
        let object_transform = object.transform();
        let transform = view_projection.multiply(object_transform);
        let model_view = view_matrix.multiply(object_transform);
        let normal_transform = model_view.inverse()?.transpose();
        let selected = options
            .selected_object_ids
            .iter()
            .any(|id| id == object.object_id());
        let color = if options.xray {
            crate::xray_shade(object.color(), selected)
        } else if selected {
            crate::selection_shade(object.color())
        } else {
            object.color()
        };
        let see_through = options.xray && !selected;
        if options.xray
            && selected
            && let Some(edges) = crate::outline::OutlineEdges::build(geometry)
            && let Ok(local_eye) = object_transform
                .inverse()
                .and_then(|inverse| inverse.transform_point(camera.position()))
        {
            for (start, end) in edges.visible_segments(local_eye) {
                outlines.push((
                    object_transform.transform_point(start)?,
                    object_transform.transform_point(end)?,
                ));
            }
        }
        rasterize_geometry(
            geometry,
            transform,
            model_view,
            normal_transform,
            light_direction,
            color,
            object,
            selected,
            see_through,
            material_parameters,
            width,
            height,
            &mut rgba,
            &mut depth,
            &mut triangles,
            &mut shaded_pixels,
        )?;
        if options.wireframe {
            for (first, second) in object
                .full_geometry()
                .wireframe_segments(object.transform())?
            {
                let Some(first) = project(view_projection, first, width, height) else {
                    continue;
                };
                let Some(second) = project(view_projection, second, width, height) else {
                    continue;
                };
                rasterize_line(
                    first,
                    second,
                    [0.0, 0.0, 0.0, 1.0],
                    width,
                    height,
                    &mut rgba,
                    &depth,
                    &mut shaded_pixels,
                );
            }
        }
    }
    // The X-ray selection outline, on top of everything it is not behind.
    for (first, second) in outlines {
        let lift = |point: [f32; 3]| {
            let eye = camera.position();
            [
                point[0] + (eye[0] - point[0]) * 0.002,
                point[1] + (eye[1] - point[1]) * 0.002,
                point[2] + (eye[2] - point[2]) * 0.002,
            ]
        };
        let Some(first) = project(view_projection, lift(first), width, height) else {
            continue;
        };
        let Some(second) = project(view_projection, lift(second), width, height) else {
            continue;
        };
        rasterize_line(
            first,
            second,
            crate::outline::OUTLINE_COLOR,
            width,
            height,
            &mut rgba,
            &depth,
            &mut shaded_pixels,
        );
    }
    if let Some(floor) = floor.as_ref()
        && options.grid_visible
    {
        rasterize_floor_lines(
            floor,
            view_projection,
            width,
            height,
            options.grid_major_color,
            options.grid_minor_color,
            &mut rgba,
            &depth,
            &mut shaded_pixels,
        );
    }
    for line in &options.guide_lines {
        if let (Some(start), Some(end)) = (
            project(view_projection, line.start, width, height),
            project(view_projection, line.end, width, height),
        ) {
            rasterize_line(
                start,
                end,
                line.color,
                width,
                height,
                &mut rgba,
                &depth,
                &mut shaded_pixels,
            );
        }
    }
    Ok(CpuFrame {
        width,
        height,
        rgba,
        triangles,
        shaded_pixels,
    })
}

#[allow(clippy::too_many_arguments)]
fn rasterize_floor_surface(
    floor: &FloorGeometry,
    view_projection: Mat4,
    width: u32,
    height: u32,
    color: [f32; 4],
    rgba: &mut [u8],
    depth: &mut [f32],
    shaded_pixels: &mut u64,
) {
    let Some(surface) = floor.surface else {
        return;
    };
    let [minimum_x, y, minimum_z] = surface.minimum;
    let [maximum_x, _, maximum_z] = surface.maximum;
    let subdivisions = FLOOR_SURFACE_SUBDIVISIONS as f32;
    for z_index in 0..FLOOR_SURFACE_SUBDIVISIONS {
        let z0 = minimum_z + (maximum_z - minimum_z) * z_index as f32 / subdivisions;
        let z1 = minimum_z + (maximum_z - minimum_z) * (z_index + 1) as f32 / subdivisions;
        for x_index in 0..FLOOR_SURFACE_SUBDIVISIONS {
            let x0 = minimum_x + (maximum_x - minimum_x) * x_index as f32 / subdivisions;
            let x1 = minimum_x + (maximum_x - minimum_x) * (x_index + 1) as f32 / subdivisions;
            let points = [[x0, y, z0], [x1, y, z0], [x1, y, z1], [x0, y, z1]];
            let Some(projected) = points
                .map(|point| project(view_projection, point, width, height))
                .into_iter()
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            rasterize_floor_triangle(
                [projected[0], projected[1], projected[2]],
                [points[0], points[1], points[2]],
                color,
                surface,
                width,
                height,
                rgba,
                depth,
                shaded_pixels,
            );
            rasterize_floor_triangle(
                [projected[0], projected[2], projected[3]],
                [points[0], points[2], points[3]],
                color,
                surface,
                width,
                height,
                rgba,
                depth,
                shaded_pixels,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_contact_cues(
    floor: &FloorGeometry,
    camera: &Camera,
    view_projection: Mat4,
    width: u32,
    height: u32,
    color: [f32; 4],
    lighting: ViewportLighting,
    rgba: &mut [u8],
    depth: &mut [f32],
    shaded_pixels: &mut u64,
) {
    let strength = f32::from(lighting.normalized().contact_shadow_percent) / 100.0;
    if strength <= 0.0 {
        return;
    }
    let light_world = shadow_light_direction_world(camera, lighting);
    for cue in &floor.contact_cues {
        let polygon = cue.shadow_polygon(light_world);
        if polygon.len() < 3 {
            continue;
        }
        for (layer_scale, layer_alpha) in CONTACT_SHADOW_LAYERS {
            let expanded = expand_shadow_polygon(&polygon, layer_scale);
            let Some(projected) = expanded
                .iter()
                .map(|point| project(view_projection, *point, width, height))
                .collect::<Option<Vec<_>>>()
            else {
                continue;
            };
            let mut layer_color = color;
            layer_color[3] = (color[3] * strength * layer_alpha).clamp(0.0, 1.0);
            for index in 1..projected.len() - 1 {
                rasterize_triangle(
                    [projected[0], projected[index], projected[index + 1]],
                    layer_color,
                    true,
                    width,
                    height,
                    rgba,
                    depth,
                    shaded_pixels,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_floor_lines(
    floor: &FloorGeometry,
    view_projection: Mat4,
    width: u32,
    height: u32,
    major_color: [f32; 4],
    minor_color: [f32; 4],
    rgba: &mut [u8],
    depth: &[f32],
    shaded_pixels: &mut u64,
) {
    for line in floor.lines.iter().chain(&floor.front_cue) {
        let Some(first) = project(view_projection, line.start, width, height) else {
            continue;
        };
        let Some(second) = project(view_projection, line.end, width, height) else {
            continue;
        };
        let is_major = line.width_pixels > 1.0;
        let base_color = if is_major { major_color } else { minor_color };
        let authored_alpha = if is_major { 0.55 } else { 0.30 };
        let alpha_scale = line.color[3] / authored_alpha;
        let mut start_color = base_color;
        start_color[3] *= line.start_alpha * alpha_scale;
        let mut end_color = base_color;
        end_color[3] *= line.end_alpha * alpha_scale;
        rasterize_line_with_colors(
            first,
            second,
            start_color,
            end_color,
            width,
            height,
            rgba,
            depth,
            shaded_pixels,
        );
    }
}

/// Writes a bounded CPU frame as an RGB PPM capture.
pub fn write_ppm(frame: &CpuFrame, path: &Path) -> Result<(), ViewportError> {
    let width = usize::try_from(frame.width)
        .map_err(|_| ViewportError::InvalidInput(String::from("PPM width is too large")))?;
    let height = usize::try_from(frame.height)
        .map_err(|_| ViewportError::InvalidInput(String::from("PPM height is too large")))?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(format!("P6\n{} {}\n255\n", frame.width, frame.height).as_bytes());
    bytes.reserve(width.saturating_mul(height).saturating_mul(3));
    for pixel in frame.rgba.as_chunks::<4>().0 {
        bytes.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, bytes).map_err(|error| ViewportError::io(path, error))
}

#[allow(clippy::too_many_arguments)]
fn rasterize_geometry(
    geometry: &MeshGeometry,
    transform: Mat4,
    model_view: Mat4,
    normal_transform: Mat4,
    light_direction: Vec3,
    color: [f32; 4],
    object: &RenderObject,
    selected: bool,
    xray: bool,
    material_parameters: [f32; 4],
    width: u32,
    height: u32,
    rgba: &mut [u8],
    depth: &mut [f32],
    triangles: &mut u64,
    shaded_pixels: &mut u64,
) -> Result<(), ViewportError> {
    for face_index in 0..geometry.face_count() {
        if *triangles >= MAX_CPU_RENDER_TRIANGLES {
            return Err(ViewportError::CapacityExceeded(String::from(
                "CPU fallback triangle budget reached",
            )));
        }
        let Some([first_index, second_index, third_index]) = geometry.triangle(face_index) else {
            return Err(ViewportError::InvalidInput(String::from(
                "CPU geometry triangle is truncated",
            )));
        };
        let Some(first_position) = geometry.position(first_index) else {
            return Err(ViewportError::InvalidInput(String::from(
                "CPU vertex is unavailable",
            )));
        };
        let Some(second_position) = geometry.position(second_index) else {
            return Err(ViewportError::InvalidInput(String::from(
                "CPU vertex is unavailable",
            )));
        };
        let Some(third_position) = geometry.position(third_index) else {
            return Err(ViewportError::InvalidInput(String::from(
                "CPU vertex is unavailable",
            )));
        };
        let Some(first) = project(transform, first_position, width, height) else {
            continue;
        };
        let Some(second) = project(transform, second_position, width, height) else {
            continue;
        };
        let Some(third) = project(transform, third_position, width, height) else {
            continue;
        };
        let authored_face_color = object.face_color_for_geometry(geometry, face_index);
        let mut face_color = authored_face_color
            .map(|authored| authored.map(|channel| f32::from(channel) / 255.0))
            .unwrap_or(color);
        if authored_face_color.is_some() {
            face_color = if xray {
                crate::xray_shade(face_color, false)
            } else if selected {
                crate::selection_shade(face_color)
            } else {
                face_color
            };
        }
        let first_normal = geometry.normal(first_index).ok_or_else(|| {
            ViewportError::InvalidInput(String::from("CPU vertex normal is unavailable"))
        })?;
        let second_normal = geometry.normal(second_index).ok_or_else(|| {
            ViewportError::InvalidInput(String::from("CPU vertex normal is unavailable"))
        })?;
        let third_normal = geometry.normal(third_index).ok_or_else(|| {
            ViewportError::InvalidInput(String::from("CPU vertex normal is unavailable"))
        })?;
        let view_positions = [
            model_view.transform_point(first_position)?,
            model_view.transform_point(second_position)?,
            model_view.transform_point(third_position)?,
        ];
        let view_normals = [
            transform_normal(normal_transform, first_normal)?,
            transform_normal(normal_transform, second_normal)?,
            transform_normal(normal_transform, third_normal)?,
        ];
        *triangles = triangles.saturating_add(1);
        rasterize_triangle_internal(
            [first, second, third],
            [face_color; 3],
            xray,
            width,
            height,
            rgba,
            depth,
            shaded_pixels,
            None,
            Some(TriangleLighting {
                normals: view_normals,
                view_positions,
                light_direction,
                material_parameters,
                surface: object
                    .texture()
                    .filter(|_| {
                        authored_face_color.is_none_or(|authored| {
                            authored == modeler3d_core::color_rgba8(object.color())
                        })
                    })
                    .map(|(image, tile_mm)| SurfaceTexture {
                        image,
                        tile_mm,
                        local_positions: [first_position, second_position, third_position],
                        local_normals: [first_normal, second_normal, third_normal],
                        authored: object.color(),
                        selected,
                        xray,
                    }),
            }),
        );
    }
    Ok(())
}

fn project(matrix: Mat4, point: Vec3, width: u32, height: u32) -> Option<[f32; 3]> {
    let transformed = matrix.transform4([point[0], point[1], point[2], 1.0]);
    if transformed[3].abs() <= f32::EPSILON {
        return None;
    }
    let ndc = [
        transformed[0] / transformed[3],
        transformed[1] / transformed[3],
        transformed[2] / transformed[3],
    ];
    if ndc.iter().any(|value| !value.is_finite()) {
        return None;
    }
    Some([
        (ndc[0] + 1.0) * 0.5 * width as f32,
        (1.0 - ndc[1]) * 0.5 * height as f32,
        ndc[2],
    ])
}

fn transform_normal(matrix: Mat4, normal: Vec3) -> Result<Vec3, ViewportError> {
    let transformed = matrix.transform4([normal[0], normal[1], normal[2], 0.0]);
    normalize(
        [transformed[0], transformed[1], transformed[2]],
        "CPU view normal",
    )
}

fn interpolate_vector(vertices: [Vec3; 3], weights: [f32; 3]) -> Vec3 {
    [
        vertices[0][0] * weights[0] + vertices[1][0] * weights[1] + vertices[2][0] * weights[2],
        vertices[0][1] * weights[0] + vertices[1][1] * weights[1] + vertices[2][1] * weights[2],
        vertices[0][2] * weights[0] + vertices[1][2] * weights[1] + vertices[2][2] * weights[2],
    ]
}

#[allow(clippy::too_many_arguments)]
fn rasterize_triangle(
    vertices: [[f32; 3]; 3],
    color: [f32; 4],
    xray: bool,
    width: u32,
    height: u32,
    rgba: &mut [u8],
    depth: &mut [f32],
    shaded_pixels: &mut u64,
) {
    rasterize_triangle_with_colors(
        vertices,
        [color; 3],
        xray,
        width,
        height,
        rgba,
        depth,
        shaded_pixels,
    );
}

#[allow(clippy::too_many_arguments)]
fn rasterize_triangle_with_colors(
    vertices: [[f32; 3]; 3],
    colors: [[f32; 4]; 3],
    xray: bool,
    width: u32,
    height: u32,
    rgba: &mut [u8],
    depth: &mut [f32],
    shaded_pixels: &mut u64,
) {
    rasterize_triangle_internal(
        vertices,
        colors,
        xray,
        width,
        height,
        rgba,
        depth,
        shaded_pixels,
        None,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
fn rasterize_floor_triangle(
    vertices: [[f32; 3]; 3],
    world_positions: [Vec3; 3],
    base_color: [f32; 4],
    surface: FloorSurface,
    width: u32,
    height: u32,
    rgba: &mut [u8],
    depth: &mut [f32],
    shaded_pixels: &mut u64,
) {
    rasterize_triangle_internal(
        vertices,
        [base_color; 3],
        false,
        width,
        height,
        rgba,
        depth,
        shaded_pixels,
        Some((world_positions, surface)),
        None,
    );
}

#[derive(Clone, Copy)]
struct TriangleLighting<'a> {
    normals: [Vec3; 3],
    view_positions: [Vec3; 3],
    light_direction: Vec3,
    material_parameters: [f32; 4],
    surface: Option<SurfaceTexture<'a>>,
}

/// A textured triangle (OF-46): the same box projection and tint rule as
/// the GPU shader, sampled per pixel.
#[derive(Clone, Copy)]
struct SurfaceTexture<'a> {
    image: &'a modeler3d_core::TextureImage,
    tile_mm: f32,
    local_positions: [Vec3; 3],
    local_normals: [Vec3; 3],
    authored: [f32; 4],
    selected: bool,
    xray: bool,
}

impl SurfaceTexture<'_> {
    fn color_at(&self, weights: [f32; 3]) -> [f32; 4] {
        let position = interpolate_vector(self.local_positions, weights);
        let normal = interpolate_vector(self.local_normals, weights);
        let texel = self.image.sample_box(position, normal, self.tile_mm);
        let mut base = self.authored;
        for (channel, value) in base.iter_mut().zip(texel).take(3) {
            *channel += (crate::renderer::srgb_to_linear(value) - *channel) * texel[3];
        }
        if self.xray {
            crate::xray_shade(base, self.selected)
        } else if self.selected {
            crate::selection_shade(base)
        } else {
            base
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_triangle_internal(
    vertices: [[f32; 3]; 3],
    colors: [[f32; 4]; 3],
    xray: bool,
    width: u32,
    height: u32,
    rgba: &mut [u8],
    depth: &mut [f32],
    shaded_pixels: &mut u64,
    floor: Option<([Vec3; 3], FloorSurface)>,
    mesh_lighting: Option<TriangleLighting>,
) {
    let minimum_x = vertices
        .iter()
        .map(|vertex| vertex[0])
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as u32;
    let maximum_x = vertices
        .iter()
        .map(|vertex| vertex[0])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(width.saturating_sub(1) as f32) as u32;
    let minimum_y = vertices
        .iter()
        .map(|vertex| vertex[1])
        .fold(f32::INFINITY, f32::min)
        .floor()
        .max(0.0) as u32;
    let maximum_y = vertices
        .iter()
        .map(|vertex| vertex[1])
        .fold(f32::NEG_INFINITY, f32::max)
        .ceil()
        .min(height.saturating_sub(1) as f32) as u32;
    if minimum_x > maximum_x || minimum_y > maximum_y {
        return;
    }
    let area = edge(
        [vertices[0][0], vertices[0][1]],
        [vertices[1][0], vertices[1][1]],
        [vertices[2][0], vertices[2][1]],
    );
    if area.abs() <= f32::EPSILON {
        return;
    }
    for y in minimum_y..=maximum_y {
        for x in minimum_x..=maximum_x {
            let point = [x as f32 + 0.5, y as f32 + 0.5];
            let w0 = edge(
                [vertices[1][0], vertices[1][1]],
                [vertices[2][0], vertices[2][1]],
                point,
            ) / area;
            let w1 = edge(
                [vertices[2][0], vertices[2][1]],
                [vertices[0][0], vertices[0][1]],
                point,
            ) / area;
            let w2 = edge(
                [vertices[0][0], vertices[0][1]],
                [vertices[1][0], vertices[1][1]],
                point,
            ) / area;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let z = w0 * vertices[0][2] + w1 * vertices[1][2] + w2 * vertices[2][2];
            let index = y as usize * width as usize + x as usize;
            if z >= depth[index] {
                continue;
            }
            if !xray {
                depth[index] = z;
            }
            let mut color = [0.0; 4];
            for channel in 0..4 {
                color[channel] =
                    w0 * colors[0][channel] + w1 * colors[1][channel] + w2 * colors[2][channel];
            }
            if let Some((world_positions, surface)) = floor {
                let world = [
                    w0 * world_positions[0][0]
                        + w1 * world_positions[1][0]
                        + w2 * world_positions[2][0],
                    w0 * world_positions[0][1]
                        + w1 * world_positions[1][1]
                        + w2 * world_positions[2][1],
                    w0 * world_positions[0][2]
                        + w1 * world_positions[1][2]
                        + w2 * world_positions[2][2],
                ];
                let cell_x = (world[0] / surface.checker_cell_size).floor() as i64;
                let cell_z = (world[2] / surface.checker_cell_size).floor() as i64;
                let shade = if (cell_x + cell_z) & 1 == 0 {
                    -0.021
                } else {
                    0.021
                };
                for channel in &mut color[..3] {
                    *channel = (*channel + shade).clamp(0.0, 1.0);
                }
                color[3] *= surface.fade_alpha_at(world);
            }
            if let Some(lighting) = mesh_lighting {
                if let Some(surface) = lighting.surface {
                    color = surface.color_at([w0, w1, w2]);
                }
                let normal = interpolate_vector(lighting.normals, [w0, w1, w2]);
                let view_position = interpolate_vector(lighting.view_positions, [w0, w1, w2]);
                color = ViewportLighting::shade_sample(
                    color,
                    normal,
                    view_position,
                    lighting.light_direction,
                    lighting.material_parameters,
                );
            }
            let offset = index * 4;
            blend_pixel(&mut rgba[offset..offset + 4], color);
            *shaded_pixels = shaded_pixels.saturating_add(1);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_line(
    first: [f32; 3],
    second: [f32; 3],
    color: [f32; 4],
    width: u32,
    height: u32,
    rgba: &mut [u8],
    depth: &[f32],
    shaded_pixels: &mut u64,
) {
    rasterize_line_with_colors(
        first,
        second,
        color,
        color,
        width,
        height,
        rgba,
        depth,
        shaded_pixels,
    );
}

#[allow(clippy::too_many_arguments)]
fn rasterize_line_with_colors(
    first: [f32; 3],
    second: [f32; 3],
    start_color: [f32; 4],
    end_color: [f32; 4],
    width: u32,
    height: u32,
    rgba: &mut [u8],
    depth: &[f32],
    shaded_pixels: &mut u64,
) {
    let distance = ((second[0] - first[0]).powi(2) + (second[1] - first[1]).powi(2)).sqrt();
    let steps = distance.ceil().max(1.0) as usize;
    for step in 0..=steps {
        let factor = step as f32 / steps as f32;
        let point = [
            first[0] + (second[0] - first[0]) * factor,
            first[1] + (second[1] - first[1]) * factor,
            first[2] + (second[2] - first[2]) * factor,
        ];
        let x = point[0].round();
        let y = point[1].round();
        if !(0.0..width as f32).contains(&x) || !(0.0..height as f32).contains(&y) {
            continue;
        }
        let index = y as usize * width as usize + x as usize;
        if point[2] > depth[index] {
            continue;
        }
        let mut color = [0.0; 4];
        for channel in 0..4 {
            color[channel] =
                start_color[channel] + (end_color[channel] - start_color[channel]) * factor;
        }
        let offset = index * 4;
        blend_pixel(&mut rgba[offset..offset + 4], color);
        *shaded_pixels = shaded_pixels.saturating_add(1);
    }
}

fn blend_pixel(destination: &mut [u8], color: [f32; 4]) {
    let alpha = color[3].clamp(0.0, 1.0);
    for (channel, value) in color[..3].iter().enumerate() {
        let existing = f32::from(destination[channel]) / 255.0;
        destination[channel] = channel_value(existing * (1.0 - alpha) + value * alpha);
    }
    destination[3] = 255;
}

fn channel_value(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn edge(first: [f32; 2], second: [f32; 2], point: [f32; 2]) -> f32 {
    (point[0] - first[0]) * (second[1] - first[1]) - (point[1] - first[1]) * (second[0] - first[0])
}

#[cfg(test)]
mod tests {
    use modeler3d_core::{Scene, SceneObject, make_box};

    use super::*;
    use crate::scene::RenderScene;

    #[test]
    fn cpu_fallback_draws_a_material_texture_like_the_gpu() {
        let stripes = image::RgbaImage::from_fn(16, 16, |x, _| {
            if x < 8 {
                image::Rgba([230, 20, 20, 255])
            } else {
                image::Rgba([20, 20, 230, 255])
            }
        });
        let mut png = Vec::new();
        assert!(
            image::DynamicImage::ImageRgba8(stripes)
                .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
                .is_ok()
        );
        let texture = modeler3d_core::register_texture(
            modeler3d_core::TextureImage::from_source_bytes(&png).expect("texture"),
        );
        let mut core_scene = Scene::new();
        let mesh = make_box([20.0, 20.0, 20.0]).expect("box");
        assert!(core_scene.add_object(SceneObject::new(mesh)).is_ok());
        assert!(core_scene.apply_texture(&[0], &texture, 10.0).is_ok());
        let render_scene = RenderScene::from_core(&core_scene).expect("render scene");
        assert!(render_scene.objects()[0].texture().is_some());
        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(96, 96).is_ok());
        let bounds = render_scene.world_bounds().ok().flatten().expect("bounds");
        assert!(camera.frame_bounds(&[bounds]).is_ok());
        let frame = render_cpu(&render_scene, &camera, 96, 96, false).expect("frame");
        let (mut reddish, mut bluish) = (0, 0);
        for pixel in frame.rgba.as_chunks::<4>().0 {
            let (r, g, b) = (
                i32::from(pixel[0]),
                i32::from(pixel[1]),
                i32::from(pixel[2]),
            );
            if r > b + 60 && r > g + 60 {
                reddish += 1;
            }
            if b > r + 60 && b > g + 60 {
                bluish += 1;
            }
        }
        // Both stripes show on the default green box instead of plain green.
        assert!(reddish > 50 && bluish > 50, "red {reddish}, blue {bluish}");
    }

    #[test]
    fn guide_strokes_cross_front_surfaces_and_hide_behind_meshes() {
        let mut scene = Scene::new();
        scene
            .add_object(SceneObject::new(make_box([2.; 3]).expect("box")))
            .expect("object");
        let scene = RenderScene::from_core(&scene).expect("render scene");
        let mut camera = Camera::new();
        camera.set_viewport_size(128, 128).expect("viewport");
        let bounds = scene.world_bounds().expect("bounds").expect("box bounds");
        camera.frame_bounds(&[bounds]).expect("frame");
        let direction =
            crate::normalize(crate::subtract(camera.center(), camera.position()), "view")
                .expect("view");
        let right =
            crate::normalize(crate::cross(direction, [0., 1., 0.]), "right").expect("right");
        for (offset, visible) in [(-2.0, true), (2.0, false)] {
            let center = crate::add(camera.center(), crate::scale(direction, offset));
            let frame = render_cpu_with_options(
                &scene,
                &camera,
                128,
                128,
                false,
                &CpuRenderOptions {
                    guide_lines: vec![crate::GizmoLine {
                        start: crate::add(center, crate::scale(right, -0.25)),
                        end: crate::add(center, crate::scale(right, 0.25)),
                        color: [1., 0., 0., 1.],
                        width_pixels: 1.0,
                    }],
                    ..CpuRenderOptions::default()
                },
            )
            .expect("render");
            let red_pixels = frame
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|pixel| pixel[..3] == [255, 0, 0])
                .count();
            assert_eq!(
                red_pixels > 4,
                visible,
                "offset {offset}, red pixels {red_pixels}"
            );
        }
    }

    #[test]
    fn cpu_fallback_rasterizes_a_real_triangle_scene() {
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
        let mut camera = Camera::new();
        assert!(camera.set_viewport_size(64, 64).is_ok());
        let bounds = render_scene.world_bounds().ok().flatten();
        let Some(bounds) = bounds else {
            return;
        };
        assert!(camera.frame_bounds(&[bounds]).is_ok());
        let frame = render_cpu(&render_scene, &camera, 64, 64, false);
        assert!(frame.is_ok());
        assert!(frame.is_ok_and(|value| value.triangles > 0 && value.shaded_pixels > 0));

        let frame = render_cpu_with_options(
            &render_scene,
            &camera,
            64,
            64,
            false,
            &CpuRenderOptions {
                floor_enabled: true,
                grid_visible: true,
                ..CpuRenderOptions::default()
            },
        );
        assert!(frame.is_ok_and(|value| value.triangles > 0 && value.shaded_pixels > 0));
    }
}
