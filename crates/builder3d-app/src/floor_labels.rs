//! Floor distance labels: egui text laid flat on the build plane.
//!
//! The viewport crate decides where each label lies and how it fades; this
//! adapter supplies the glyphs. Each label string is laid out once by egui,
//! its glyph mesh is mapped onto the floor plane, and the renderer draws it
//! in the 3D pass with egui's own font atlas, so labels foreshorten with
//! perspective and objects standing on them hide them. When the viewport is
//! on the CPU fallback, the same world-space mesh is projected and painted
//! through egui instead (without object occlusion).

use std::collections::BTreeMap;

use builder3d_viewport::{
    Camera, FloorTextMesh, Mat4, PlanarText, PlanarTextVertex, ViewportTheme,
    build_floor_scale_labels, build_floor_text,
};
use eframe::egui::{self, Color32, FontId};

/// Em size the label glyphs are rasterised at, in points. Large enough that
/// labels close to the camera stay crisp when magnified on the floor.
const LABEL_FONT_POINTS: f32 = 28.0;
/// Thin grey of 3D Builder's floor numbers, sRGB with straight alpha.
const LABEL_COLOR: [f32; 4] = [0.36, 0.36, 0.38, 0.62];

/// Builds this frame's floor-label mesh with UVs normalised to an atlas of
/// `atlas_size` texels. The text darkens with the floor in the dark theme.
pub(crate) fn floor_label_mesh(
    painter: &egui::Painter,
    camera: &Camera,
    atlas_size: [f32; 2],
    theme: ViewportTheme,
) -> FloorTextMesh {
    let labels = build_floor_scale_labels(camera);
    if labels.is_empty() || atlas_size[0] <= 0.0 || atlas_size[1] <= 0.0 {
        return FloorTextMesh::default();
    }
    let mut texts = BTreeMap::new();
    for label in &labels {
        texts.entry(label.value_mm).or_insert_with(|| {
            let galley = painter.layout_no_wrap(
                format!("{} mm", label.value_mm),
                FontId::proportional(LABEL_FONT_POINTS),
                Color32::WHITE,
            );
            planar_text(&galley, LABEL_FONT_POINTS, atlas_size)
        });
    }
    let tone = theme.tone();
    let color = [
        LABEL_COLOR[0] * tone,
        LABEL_COLOR[1] * tone,
        LABEL_COLOR[2] * tone,
        LABEL_COLOR[3],
    ];
    build_floor_text(&labels, |value| texts.get(&value), color)
}

/// Converts an egui galley's glyph meshes into the viewport's toolkit-neutral
/// planar text.
fn planar_text(galley: &egui::Galley, font_size: f32, atlas_size: [f32; 2]) -> PlanarText {
    let mut text = PlanarText {
        size: [galley.size().x, galley.size().y],
        font_size,
        ..PlanarText::default()
    };
    for placed in &galley.rows {
        let mesh = &placed.row.visuals.mesh;
        let Ok(base) = u32::try_from(text.vertices.len()) else {
            break;
        };
        text.vertices
            .extend(mesh.vertices.iter().map(|vertex| PlanarTextVertex {
                position: [placed.pos.x + vertex.pos.x, placed.pos.y + vertex.pos.y],
                uv: [vertex.uv.x / atlas_size[0], vertex.uv.y / atlas_size[1]],
                alpha: 1.0,
            }));
        text.indices
            .extend(mesh.indices.iter().map(|index| base.saturating_add(*index)));
    }
    text
}

/// Paints the floor-label mesh through egui for the CPU viewport fallback.
pub(crate) fn paint_floor_label_mesh(
    painter: &egui::Painter,
    rect: egui::Rect,
    matrix: Mat4,
    mesh: &FloorTextMesh,
) {
    let mut shape = egui::Mesh::with_texture(egui::TextureId::default());
    for vertex in &mesh.vertices {
        let [x, y, z] = vertex.position;
        let clip = matrix.transform4([x, y, z, 1.0]);
        if clip[3] <= 1.0e-4 || clip.iter().any(|value| !value.is_finite()) {
            // A label crossing the near plane would smear; skip the frame's
            // labels rather than draw a streak.
            return;
        }
        let [r, g, b, a] = vertex
            .color
            .map(|channel| (channel.clamp(0.0, 1.0) * 255.0) as u8);
        shape.vertices.push(egui::epaint::Vertex {
            pos: egui::pos2(
                rect.left() + (clip[0] / clip[3] + 1.0) * rect.width() * 0.5,
                rect.top() + (1.0 - clip[1] / clip[3]) * rect.height() * 0.5,
            ),
            uv: egui::pos2(vertex.uv[0], vertex.uv[1]),
            color: Color32::from_rgba_unmultiplied(r, g, b, a),
        });
    }
    shape.indices.clone_from(&mesh.indices);
    painter.add(egui::Shape::mesh(shape));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_glyphs_become_planar_text_with_normalised_uvs() {
        let ctx = egui::Context::default();
        let mut produced = None;
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let galley = ui.painter().layout_no_wrap(
                String::from("150 mm"),
                FontId::proportional(LABEL_FONT_POINTS),
                Color32::WHITE,
            );
            produced = Some(planar_text(&galley, LABEL_FONT_POINTS, [1024.0, 512.0]));
        });
        output.textures_delta.clear();
        let text = produced.unwrap_or_default();
        assert!(!text.vertices.is_empty());
        assert_eq!(text.indices.len() % 3, 0);
        assert!(text.size[0] > text.size[1]);
        for vertex in &text.vertices {
            assert!((0.0..=1.0).contains(&vertex.uv[0]));
            assert!((0.0..=1.0).contains(&vertex.uv[1]));
            assert!(vertex.position[0] >= -1.0 && vertex.position[0] <= text.size[0] + 1.0);
        }
    }
}
