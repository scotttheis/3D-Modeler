//! Bounded recent-file preview decoding and mesh thumbnail rendering.

use std::{fs, io::Cursor, path::Path};

use eframe::egui::ColorImage;
use image::{ImageReader, RgbaImage, imageops};
use modeler3d_core::{Mesh, Scene, SceneObject, Transform, load_3mf_thumbnail};

use crate::{
    commands::CommandState,
    image_color::{MAX_IMAGE_BYTES, MAX_IMAGE_PIXELS},
};

const PREVIEW_SIZE: u32 = 64;
const PREVIEW_PADDING: f32 = 6.0;
const MAX_PREVIEW_FACES: usize = 8_192;
const MAX_PREVIEW_PARTS: usize = 256;

/// Loads a recent-file image or renders a small mesh preview without creating
/// GPU or egui texture resources. Call this from a background worker.
pub(crate) fn load_recent_preview(path: &Path) -> Option<ColorImage> {
    if !path.is_file() {
        return None;
    }
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    if is_recent_image_extension(&extension) {
        return load_image_file(path);
    }

    if extension == "3mf" {
        match load_3mf_thumbnail(path) {
            Ok(Some(bytes)) => return decode_image_thumbnail(&bytes),
            Ok(None) => {}
            Err(_) => return None,
        }
    }

    let mut state = CommandState::default();
    if extension == "3dp" {
        state.open_from_path(path).ok()?;
    } else if is_recent_model_extension(&extension) {
        // Use the same import resolver as the visible Add path. WRL remains a
        // recent entry in the reference, but its import path returns an error.
        state.import_from_path(path).ok()?;
    } else {
        return None;
    }
    render_scene_preview(state.scene())
}

fn is_recent_image_extension(extension: &str) -> bool {
    matches!(
        extension,
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "tif" | "tiff"
    )
}

fn is_recent_model_extension(extension: &str) -> bool {
    matches!(
        extension,
        "stl" | "obj" | "ply" | "3mf" | "gltf" | "glb" | "wrl"
    )
}

fn load_image_file(path: &Path) -> Option<ColorImage> {
    let bytes = fs::metadata(path).ok()?.len();
    if bytes > MAX_IMAGE_BYTES {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    decode_image_thumbnail(&bytes)
}

fn decode_image_thumbnail(bytes: &[u8]) -> Option<ColorImage> {
    if u64::try_from(bytes.len()).ok()? > MAX_IMAGE_BYTES {
        return None;
    }
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let (width, height) = reader.into_dimensions().ok()?;
    let pixels = u64::from(width).checked_mul(u64::from(height))?;
    if pixels == 0 || pixels > MAX_IMAGE_PIXELS {
        return None;
    }
    let image = image::load_from_memory(bytes)
        .ok()?
        .thumbnail(PREVIEW_SIZE, PREVIEW_SIZE)
        .to_rgba8();
    let mut canvas = RgbaImage::new(PREVIEW_SIZE, PREVIEW_SIZE);
    let left = i64::from((PREVIEW_SIZE - image.width()) / 2);
    let top = i64::from((PREVIEW_SIZE - image.height()) / 2);
    imageops::overlay(&mut canvas, &image, left, top);
    Some(ColorImage::from_rgba_unmultiplied(
        [PREVIEW_SIZE as usize, PREVIEW_SIZE as usize],
        canvas.as_raw(),
    ))
}

#[derive(Clone, Copy)]
struct MeshPart<'a> {
    mesh: &'a Mesh,
    transform: Transform,
    color: [f32; 4],
}

#[derive(Clone, Copy)]
struct WorldTriangle {
    vertices: [[f32; 3]; 3],
    color: [f32; 4],
}

#[derive(Clone, Copy)]
struct ProjectedVertex {
    point: [f32; 2],
    depth: f32,
}

#[derive(Clone, Copy)]
struct ProjectedTriangle {
    vertices: [ProjectedVertex; 3],
    color: [u8; 4],
}

fn render_scene_preview(scene: &Scene) -> Option<ColorImage> {
    // The combined top-level mesh already includes its group descendants.
    let parts = scene
        .objects()
        .iter()
        .filter(|object| object.is_visible())
        .take(MAX_PREVIEW_PARTS)
        .map(|object| MeshPart {
            mesh: object.mesh(),
            transform: *object.transform(),
            color: object.color(),
        })
        .collect::<Vec<_>>();
    render_mesh_parts(&parts)
}

/// Renders an Items thumbnail from the actual mesh, including hidden objects.
pub(crate) fn render_object_preview(object: &SceneObject) -> Option<ColorImage> {
    render_mesh_parts(&[MeshPart {
        mesh: object.mesh(),
        transform: *object.transform(),
        color: object.color(),
    }])
}

fn render_mesh_parts(parts: &[MeshPart<'_>]) -> Option<ColorImage> {
    let total_faces = parts.iter().fold(0_usize, |total, part| {
        total.saturating_add(part.mesh.face_count())
    });
    if total_faces == 0 {
        return None;
    }
    let stride = total_faces.div_ceil(MAX_PREVIEW_FACES).max(1);
    let mut offset = 0_usize;
    let mut triangles = Vec::with_capacity(total_faces.min(MAX_PREVIEW_FACES));
    for part in parts {
        let first = (stride - (offset % stride)) % stride;
        for face_index in (first..part.mesh.faces.len()).step_by(stride) {
            let face = *part.mesh.faces.get(face_index)?;
            let vertices = face.map(|index| {
                part.transform
                    .transform_point(part.mesh.vertices[index as usize])
            });
            if vertices.iter().flatten().any(|value| !value.is_finite()) {
                continue;
            }
            triangles.push(WorldTriangle {
                vertices,
                color: part.color,
            });
            if triangles.len() >= MAX_PREVIEW_FACES {
                break;
            }
        }
        offset = offset.saturating_add(part.mesh.face_count());
        if triangles.len() >= MAX_PREVIEW_FACES {
            break;
        }
    }
    if triangles.is_empty() {
        return None;
    }

    let mut projected = Vec::with_capacity(triangles.len());
    let mut minimum = [f32::INFINITY; 2];
    let mut maximum = [f32::NEG_INFINITY; 2];
    for triangle in triangles {
        let vertices = triangle.vertices.map(project_vertex);
        if vertices.iter().any(|vertex| {
            vertex.point.iter().any(|value| !value.is_finite()) || !vertex.depth.is_finite()
        }) {
            continue;
        }
        for vertex in vertices {
            for axis in 0..2 {
                minimum[axis] = minimum[axis].min(vertex.point[axis]);
                maximum[axis] = maximum[axis].max(vertex.point[axis]);
            }
        }
        projected.push(ProjectedTriangle {
            vertices,
            color: shaded_color(triangle.vertices, triangle.color),
        });
    }
    if projected.is_empty() {
        return None;
    }

    let available = PREVIEW_SIZE as f32 - PREVIEW_PADDING * 2.0;
    let width = (maximum[0] - minimum[0]).max(f32::EPSILON);
    let height = (maximum[1] - minimum[1]).max(f32::EPSILON);
    let scale = available / width.max(height);
    let center = [
        (minimum[0] + maximum[0]) * 0.5,
        (minimum[1] + maximum[1]) * 0.5,
    ];
    for triangle in &mut projected {
        for vertex in &mut triangle.vertices {
            vertex.point = [
                PREVIEW_SIZE as f32 * 0.5 + (vertex.point[0] - center[0]) * scale,
                PREVIEW_SIZE as f32 * 0.5 + (vertex.point[1] - center[1]) * scale,
            ];
        }
    }

    let pixel_count = (PREVIEW_SIZE * PREVIEW_SIZE) as usize;
    let mut rgba = vec![0_u8; pixel_count * 4];
    let mut depth_buffer = vec![f32::NEG_INFINITY; pixel_count];
    for triangle in &projected {
        rasterize_triangle(triangle, &mut rgba, &mut depth_buffer);
    }
    Some(ColorImage::from_rgba_unmultiplied(
        [PREVIEW_SIZE as usize, PREVIEW_SIZE as usize],
        &rgba,
    ))
}

fn project_vertex(point: [f32; 3]) -> ProjectedVertex {
    const SQRT_HALF: f32 = 0.707_106_77;
    const ISO_XZ: f32 = 0.408_248_28;
    const ISO_Y: f32 = 0.816_496_6;
    const ISO_DEPTH: f32 = 0.577_350_26;
    ProjectedVertex {
        point: [
            (point[0] - point[2]) * SQRT_HALF,
            (point[0] + point[2]) * ISO_XZ - point[1] * ISO_Y,
        ],
        depth: (point[0] + point[1] + point[2]) * ISO_DEPTH,
    }
}

fn shaded_color(vertices: [[f32; 3]; 3], color: [f32; 4]) -> [u8; 4] {
    let first = vertices[0];
    let second = vertices[1];
    let third = vertices[2];
    let a = [
        second[0] - first[0],
        second[1] - first[1],
        second[2] - first[2],
    ];
    let b = [
        third[0] - first[0],
        third[1] - first[1],
        third[2] - first[2],
    ];
    let normal = [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ];
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    let light = [0.32_f32, 0.84_f32, 0.44_f32];
    let intensity = if length > f32::EPSILON && length.is_finite() {
        let diffuse = (normal[0] * light[0] + normal[1] * light[1] + normal[2] * light[2]) / length;
        0.48 + 0.52 * diffuse.max(0.0)
    } else {
        0.72
    };
    [
        channel(color[0] * intensity),
        channel(color[1] * intensity),
        channel(color[2] * intensity),
        channel(color[3]),
    ]
}

fn channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn rasterize_triangle(triangle: &ProjectedTriangle, rgba: &mut [u8], depth_buffer: &mut [f32]) {
    let [first, second, third] = triangle.vertices;
    let area = edge(first.point, second.point, third.point);
    if !area.is_finite() || area.abs() <= f32::EPSILON {
        return;
    }
    let min_x = first.point[0]
        .min(second.point[0])
        .min(third.point[0])
        .floor()
        .max(0.0) as u32;
    let max_x = first.point[0]
        .max(second.point[0])
        .max(third.point[0])
        .ceil()
        .min(PREVIEW_SIZE as f32 - 1.0) as u32;
    let min_y = first.point[1]
        .min(second.point[1])
        .min(third.point[1])
        .floor()
        .max(0.0) as u32;
    let max_y = first.point[1]
        .max(second.point[1])
        .max(third.point[1])
        .ceil()
        .min(PREVIEW_SIZE as f32 - 1.0) as u32;
    if min_x > max_x || min_y > max_y {
        return;
    }

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let point = [x as f32 + 0.5, y as f32 + 0.5];
            let weights = [
                edge(second.point, third.point, point) / area,
                edge(third.point, first.point, point) / area,
                edge(first.point, second.point, point) / area,
            ];
            if weights.iter().any(|weight| *weight < -1.0e-5) {
                continue;
            }
            let depth =
                weights[0] * first.depth + weights[1] * second.depth + weights[2] * third.depth;
            let pixel_index = (y * PREVIEW_SIZE + x) as usize;
            if depth <= depth_buffer[pixel_index] {
                continue;
            }
            depth_buffer[pixel_index] = depth;
            let start = pixel_index * 4;
            blend_pixel(&mut rgba[start..start + 4], triangle.color);
        }
    }
}

fn edge(first: [f32; 2], second: [f32; 2], point: [f32; 2]) -> f32 {
    (point[0] - first[0]) * (second[1] - first[1]) - (point[1] - first[1]) * (second[0] - first[0])
}

fn blend_pixel(destination: &mut [u8], source: [u8; 4]) {
    let source_alpha = f32::from(source[3]) / 255.0;
    if source_alpha <= 0.0 {
        return;
    }
    if source[3] == u8::MAX {
        destination.copy_from_slice(&source);
        return;
    }
    let destination_alpha = f32::from(destination[3]) / 255.0;
    let output_alpha = source_alpha + destination_alpha * (1.0 - source_alpha);
    for channel_index in 0..3 {
        let source_channel = f32::from(source[channel_index]) / 255.0;
        let destination_channel = f32::from(destination[channel_index]) / 255.0;
        let output = (source_channel * source_alpha
            + destination_channel * destination_alpha * (1.0 - source_alpha))
            / output_alpha;
        destination[channel_index] = channel(output);
    }
    destination[3] = channel(output_alpha);
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, path::PathBuf};

    use super::*;
    use crate::commands::{CommandId, CommandOutcome};
    use modeler3d_core::PrimitiveKind;

    fn fixture(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../modeler3d-core/tests/fixtures")
            .join(relative)
    }

    fn temporary_directory(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        std::env::temp_dir().join(format!(
            "modeler3d-recent-preview-{label}-{}-{unique}",
            std::process::id()
        ))
    }

    fn assert_visible_thumbnail(image: &ColorImage) {
        assert_eq!(image.size, [PREVIEW_SIZE as usize, PREVIEW_SIZE as usize]);
        assert!(image.pixels.iter().any(|pixel| pixel.a() > 0));
    }

    #[test]
    fn project_and_all_importable_model_formats_produce_mesh_previews() {
        let directory = temporary_directory("mesh");
        assert!(fs::create_dir_all(&directory).is_ok());
        let visible_project = directory.join("visible.3dp");
        let mut state = CommandState::default();
        assert!(matches!(
            state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube)),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(state.save_to_path(&visible_project).is_ok());
        let hidden_project = fixture("projects/python-current.3dp");
        assert!(load_recent_preview(&hidden_project).is_none());

        let paths = vec![
            visible_project,
            fixture("formats/python_box.stl"),
            fixture("formats/python_box.obj"),
            fixture("formats/python_box.ply"),
            fixture("imports/python_multi_object_cm.3mf"),
            fixture("imports/python_multi_object.gltf"),
            fixture("imports/python_multi_object.glb"),
        ];
        for path in paths {
            let preview = load_recent_preview(&path);
            assert!(
                preview.is_some(),
                "{} should have a mesh preview",
                path.display()
            );
            assert_visible_thumbnail(&preview.expect("preview exists"));
        }
        assert!(fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn raster_recent_and_embedded_image_payloads_are_bounded_and_padded() {
        let directory = temporary_directory("raster");
        assert!(fs::create_dir_all(&directory).is_ok());
        let mut bytes = Cursor::new(Vec::new());
        let source = RgbaImage::from_pixel(16, 8, image::Rgba([240, 40, 20, 255]));
        assert!(
            image::DynamicImage::ImageRgba8(source)
                .write_to(&mut bytes, image::ImageFormat::Png)
                .is_ok()
        );
        let path = directory.join("thumbnail.png");
        assert!(fs::write(&path, bytes.get_ref()).is_ok());

        let preview = load_recent_preview(&path).expect("file image preview");
        assert_visible_thumbnail(&preview);
        assert!(preview.pixels.iter().any(|pixel| pixel.r() > pixel.g()));
        let embedded = decode_image_thumbnail(bytes.get_ref()).expect("embedded image preview");
        assert_visible_thumbnail(&embedded);
        let item = modeler3d_core::ThreeMfItem::new(
            modeler3d_core::make_box([2.0, 2.0, 2.0]).expect("thumbnail mesh"),
            Transform::identity(),
            String::from("Embedded thumbnail"),
            [0.2, 0.8, 0.3, 1.0],
        )
        .expect("3MF item");
        let package = modeler3d_core::export_3mf(&[item], None, Some(bytes.get_ref()))
            .expect("3MF package with embedded preview");
        let package_path = directory.join("embedded.3mf");
        assert!(fs::write(&package_path, package).is_ok());
        let package_preview = load_recent_preview(&package_path).expect("3MF preview");
        assert_visible_thumbnail(&package_preview);
        assert!(
            package_preview
                .pixels
                .iter()
                .any(|pixel| pixel.r() > pixel.g())
        );
        assert!(fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn missing_corrupt_and_unsupported_recents_keep_the_icon_fallback() {
        let directory = temporary_directory("fallback");
        assert!(fs::create_dir_all(&directory).is_ok());
        let missing = directory.join("missing.stl");
        let corrupt = directory.join("broken.3dp");
        let unsupported = directory.join("unknown.bin");
        assert!(fs::write(&corrupt, b"not a project").is_ok());
        assert!(fs::write(&unsupported, b"anything").is_ok());
        assert!(load_recent_preview(&missing).is_none());
        assert!(load_recent_preview(&corrupt).is_none());
        assert!(load_recent_preview(&unsupported).is_none());
        assert!(load_recent_preview(&fixture("wrl/python_reference.wrl")).is_none());
        assert!(fs::remove_dir_all(directory).is_ok());
    }
}

#[cfg(test)]
mod rb04_group_preview {
    use super::*;
    use modeler3d_core::make_box;
    #[test]
    fn recent_and_items_draw_the_combined_group_once_and_keep_hidden_items_visible_in_rows() {
        let mut scene = Scene::new();
        for (i, x) in [-12., 12.].into_iter().enumerate() {
            scene
                .add_object(SceneObject::new(make_box([12., 20., 30.]).expect("box")))
                .expect("add");
            scene
                .set_transform(
                    i,
                    Transform::identity().translated([x, 0., 0.]).expect("pose"),
                )
                .expect("set");
            scene
                .set_color(
                    i,
                    if i == 0 {
                        [1., 0., 0., 1.]
                    } else {
                        [0., 1., 0., 1.]
                    },
                )
                .expect("color");
        }
        scene.set_visible(0, false).expect("hide former child");
        scene.set_selection([0, 1]);
        scene.group_selected().expect("group");
        scene
            .set_transform(
                0,
                Transform::identity()
                    .translated([100., 20., 10.])
                    .expect("pose")
                    .rotated(23., [0., 1., 0.])
                    .expect("rotate"),
            )
            .expect("set");
        let revision = scene.revision();
        let history = scene.history_state_id();
        let recent = render_scene_preview(&scene).expect("recent");
        let item = render_object_preview(&scene.objects()[0]).expect("item");
        assert_eq!(recent.size, item.size);
        assert_eq!(recent.pixels, item.pixels);
        assert_eq!(revision, scene.revision());
        assert_eq!(history, scene.history_state_id());
        assert!(!scene.objects()[0].children()[0].is_visible());
        scene.set_visible(0, false).expect("hide root");
        assert!(render_scene_preview(&scene).is_none());
        assert!(render_object_preview(&scene.objects()[0]).is_some());
    }
}
