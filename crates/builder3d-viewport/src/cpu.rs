use std::path::Path;

use crate::{
    ViewportError,
    camera::Camera,
    floor::{DEFAULT_FLOOR_SIZE, FloorGeometry, build_floor},
    math::{Mat4, Vec3},
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
    /// Blend mesh colors translucently without changing authored geometry.
    pub xray: bool,
    /// Durable IDs receiving the selection tint.
    pub selected_object_ids: Vec<String>,
    /// RGBA tint mixed into selected authored colors.
    pub selected_tint: [f32; 4],
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
}

impl Default for CpuRenderOptions {
    fn default() -> Self {
        Self {
            clear_color: [4, 7, 13, 255],
            wireframe: false,
            xray: false,
            selected_object_ids: Vec::new(),
            selected_tint: [0.0, 0.55, 1.0, 1.0],
            floor_enabled: false,
            grid_visible: false,
            floor_color: [0.025, 0.04, 0.075, 1.0],
            grid_major_color: [0.20, 0.26, 0.34, 0.55],
            grid_minor_color: [0.16, 0.20, 0.27, 0.30],
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
    let view_projection = camera.view_projection(scene_radius)?;
    let mut rgba = vec![0_u8; byte_count];
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel.copy_from_slice(&options.clear_color);
    }
    let mut depth = vec![f32::INFINITY; pixel_count];
    let mut triangles = 0_u64;
    let mut shaded_pixels = 0_u64;
    let floor = if options.floor_enabled {
        Some(build_floor(
            camera,
            DEFAULT_FLOOR_SIZE,
            options.grid_visible,
            true,
        )?)
    } else {
        None
    };
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
    }
    for object in scene.objects() {
        let geometry = if interaction {
            object.interaction_geometry()
        } else {
            object.full_geometry()
        };
        let transform = view_projection.multiply(object.transform());
        let selected = options
            .selected_object_ids
            .iter()
            .any(|id| id == object.object_id());
        let mut color = object.color();
        if selected {
            for (channel, tint) in color[..3].iter_mut().zip(options.selected_tint) {
                *channel = *channel * 0.62 + tint * 0.38;
            }
        }
        if options.xray {
            color[3] = (color[3] * 0.30).clamp(0.05, 0.45);
        }
        rasterize_geometry(
            geometry,
            transform,
            color,
            object,
            selected,
            options.selected_tint,
            options.xray,
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
    let points = [
        [minimum_x, y, minimum_z],
        [maximum_x, y, minimum_z],
        [maximum_x, y, maximum_z],
        [minimum_x, y, maximum_z],
    ];
    let Some(projected) = points
        .map(|point| project(view_projection, point, width, height))
        .into_iter()
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    rasterize_triangle(
        [projected[0], projected[1], projected[2]],
        color,
        false,
        width,
        height,
        rgba,
        depth,
        shaded_pixels,
    );
    rasterize_triangle(
        [projected[0], projected[2], projected[3]],
        color,
        false,
        width,
        height,
        rgba,
        depth,
        shaded_pixels,
    );
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
        let color = if line.width_pixels > 1.0 {
            major_color
        } else {
            minor_color
        };
        rasterize_line(
            first,
            second,
            color,
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
    color: [f32; 4],
    object: &RenderObject,
    selected: bool,
    selected_tint: [f32; 4],
    xray: bool,
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
        let Some([first, second, third]) = geometry.triangle(face_index) else {
            return Err(ViewportError::InvalidInput(String::from(
                "CPU geometry triangle is truncated",
            )));
        };
        let Some(first) = geometry.position(first) else {
            return Err(ViewportError::InvalidInput(String::from(
                "CPU vertex is unavailable",
            )));
        };
        let Some(second) = geometry.position(second) else {
            return Err(ViewportError::InvalidInput(String::from(
                "CPU vertex is unavailable",
            )));
        };
        let Some(third) = geometry.position(third) else {
            return Err(ViewportError::InvalidInput(String::from(
                "CPU vertex is unavailable",
            )));
        };
        let Some(first) = project(transform, first, width, height) else {
            continue;
        };
        let Some(second) = project(transform, second, width, height) else {
            continue;
        };
        let Some(third) = project(transform, third, width, height) else {
            continue;
        };
        let authored_face_color = object.face_color_for_geometry(geometry, face_index);
        let mut face_color = authored_face_color
            .map(|authored| authored.map(|channel| f32::from(channel) / 255.0))
            .unwrap_or(color);
        if authored_face_color.is_some() {
            if selected {
                for (channel, tint) in face_color[..3].iter_mut().zip(selected_tint) {
                    *channel = *channel * 0.62 + tint * 0.38;
                }
            }
            if xray {
                face_color[3] = (face_color[3] * 0.30).clamp(0.05, 0.45);
            }
        }
        *triangles = triangles.saturating_add(1);
        rasterize_triangle(
            [first, second, third],
            face_color,
            xray,
            width,
            height,
            rgba,
            depth,
            shaded_pixels,
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
    use builder3d_core::{Scene, SceneObject, make_box};

    use super::*;
    use crate::scene::RenderScene;

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
