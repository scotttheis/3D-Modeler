//! Shared presentation adapter for every egui grab, including drag and drop.
//!
//! Settings chooses the look: the robot claw (the original) or a traditional
//! hand. Both share one plugin, so every grab in the app changes together.

use eframe::egui::{self, Color32, CursorIcon, Pos2, Shape, Stroke};
use serde::{Deserialize, Serialize};

/// The grab cursor chosen in Settings.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GrabCursorStyle {
    /// The robot claw: two pincer arms that swing shut while holding.
    #[default]
    Claw,
    /// A traditional hand: open palm, closing into a fist while holding.
    Hand,
}

impl GrabCursorStyle {
    /// Both choices in the order Settings shows them.
    pub const ALL: [Self; 2] = [Self::Claw, Self::Hand];

    /// Returns the stable settings-file and MCP spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Claw => "claw",
            Self::Hand => "hand",
        }
    }

    /// Parses a persisted or MCP spelling; unknown text is `None`.
    #[must_use]
    pub fn from_key(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "claw" => Some(Self::Claw),
            "hand" => Some(Self::Hand),
            _ => None,
        }
    }

    /// The label a person reads in Settings.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Claw => "Claw",
            Self::Hand => "Hand",
        }
    }
}

fn style_id() -> egui::Id {
    egui::Id::unique("grab-cursor-style")
}

/// Tells the cursor plugin which look to paint from now on.
pub(crate) fn set_style(ctx: &egui::Context, style: GrabCursorStyle) {
    ctx.data_mut(|data| data.insert_temp(style_id(), style));
}

fn style(ctx: &egui::Context) -> GrabCursorStyle {
    ctx.data(|data| data.get_temp(style_id()))
        .unwrap_or_default()
}

/// Runs after egui's built-in drag-and-drop plugin so its cursor is covered too.
#[derive(Default)]
pub(crate) struct GrabCursor;

impl egui::Plugin for GrabCursor {
    fn debug_name(&self) -> &'static str {
        "Grab cursor"
    }

    fn on_end_pass(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx();
        let Some(pointer) =
            ctx.input(|input| input.focused.then(|| input.pointer.hover_pos()).flatten())
        else {
            return;
        };
        let closed = match ctx.output(|output| output.cursor_icon) {
            CursorIcon::Grab => false,
            CursorIcon::Grabbing => true,
            _ => return,
        };
        ctx.set_cursor_icon(CursorIcon::None);
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::unique("robot-claw-cursor"),
        ));
        match style(ctx) {
            GrabCursorStyle::Claw => paint(&painter, pointer, closed),
            GrabCursorStyle::Hand => paint_hand(&painter, pointer, closed),
        }
    }
}

/// Claw art, drawn on the 960-unit grid of `erros/claw.png` and scaled down.
const SCALE: f32 = 0.05;
/// Hub centre, which the arms pivot around when the claw closes.
const HUB: (f32, f32) = (490., 520.);
/// Arm centre line for the left arm, from the hub up to the jaw tip.
const ARM: [(f32, f32); 4] = [(405., 505.), (290., 390.), (290., 250.), (385., 115.)];
/// How far each arm swings inward when closed.
const CLOSE_RADIANS: f32 = 14. * std::f32::consts::PI / 180.;

/// The jaw gap between the tips is the pointer hotspot in both states; only
/// the arms swing shut. Shape follows the owner's reference icon: two flared
/// pincer arms on a round hub, a neck and a flared base.
fn paint(painter: &egui::Painter, pointer: Pos2, closed: bool) {
    let outline = Color32::from_rgb(25, 29, 35);
    let fill = Color32::from_rgb(242, 245, 249);
    let at = |x: f32, y: f32| pointer + egui::vec2((x - HUB.0) * SCALE, (y - 115.) * SCALE);
    let angle = if closed { CLOSE_RADIANS } else { 0. };
    // The left arm swings clockwise when closing; the right arm mirrors it.
    let arm = |mirror: bool| -> Vec<Pos2> {
        let (sin, cos) = angle.sin_cos();
        ARM.iter()
            .map(|&(x, y)| {
                let (dx, dy) = (x - HUB.0, y - HUB.1);
                let x = HUB.0 + dx * cos - dy * sin;
                let y = HUB.1 + dx * sin + dy * cos;
                at(if mirror { 2. * HUB.0 - x } else { x }, y)
            })
            .collect()
    };
    for (rounding, rect) in [
        (1.8, ((360., 720.), (620., 870.))),
        (0.8, ((430., 625.), (550., 705.))),
    ] {
        painter.rect(
            egui::Rect::from_min_max(at(rect.0.0, rect.0.1), at(rect.1.0, rect.1.1)),
            rounding,
            fill,
            Stroke::new(1.8, outline),
            egui::StrokeKind::Inside,
        );
    }
    // Dark pass first, then the light pass, so the arms read as outlined tubes.
    for (width, color) in [(150. * SCALE, outline), (70. * SCALE, fill)] {
        for mirror in [false, true] {
            let points = arm(mirror);
            painter.add(Shape::line(points.clone(), Stroke::new(width, color)));
            for point in points {
                painter.circle_filled(point, width / 2., color);
            }
        }
    }
    let hub = at(HUB.0, HUB.1);
    painter.circle_filled(hub, 125. * SCALE, outline);
    painter.circle_filled(hub, 70. * SCALE, fill);
}

/// Original hand contours in points, with the palm centre as the shared hotspot.
/// Each quadratic segment is (control x/y, end x/y). Rounded fingertips and
/// tapered wrists keep the silhouette readable at ordinary cursor sizes.
const OPEN_HAND: &[[f32; 4]] = &[
    [-6., 7., -8., 4.],
    [-10., 1., -11., -1.],
    [-12., -3., -10., -3.5],
    [-9., -4., -6., -0.5],
    [-5., 0., -5.5, -3.],
    [-6.5, -8., -7., -11.],
    [-7.5, -13.5, -5.5, -13.5],
    [-4., -13.5, -3.5, -11.],
    [-3., -8., -2.5, -5.],
    [-2., -4., -2., -6.],
    [-2.5, -12., -2.5, -15.],
    [-2.5, -17., -0.8, -17.],
    [1., -17., 1., -15.],
    [1., -10., 1., -6.],
    [1.5, -4.5, 2., -6.],
    [2.5, -10., 3., -13.],
    [3.3, -15., 5., -14.5],
    [6.5, -14., 6., -12.],
    [5.5, -8., 5., -5.],
    [5.2, -4., 5.8, -5.],
    [6.5, -7., 7., -9.],
    [7.6, -11., 9., -10.2],
    [10.3, -9.5, 9.5, -7.5],
    [8.5, -3., 8., 1.],
    [7.5, 5., 5., 7.5],
    [4.5, 8., 4.5, 10.],
    [0., 11., -4., 10.],
    [-4., 8., -5., 7.],
];
const CLOSED_HAND: &[[f32; 4]] = &[
    [-8., 4., -8., 0.],
    [-8.5, -3., -7., -4.5],
    [-6., -6., -4., -5.],
    [-3., -7.5, -0.5, -6.],
    [1., -8., 3., -6.],
    [5.5, -7., 6.5, -4.5],
    [8.5, -4.5, 8.5, -2.],
    [9., 3.5, 5., 7.],
    [4.5, 8., 4.5, 10.],
    [0., 11., -4., 10.],
    [-4., 8., -5., 7.],
];

/// Fill a fixed concave contour with horizontal trapezoids. Splitting at every
/// vertex height preserves finger gaps; egui's convex fill cannot do that.
/// The resulting geometry is cached once per pose, never rebuilt per frame.
fn hand_geometry(closed: bool) -> &'static (Vec<Pos2>, egui::Mesh) {
    type Geometry = (Vec<Pos2>, egui::Mesh);
    static OPEN: std::sync::OnceLock<Geometry> = std::sync::OnceLock::new();
    static CLOSED: std::sync::OnceLock<Geometry> = std::sync::OnceLock::new();
    let cache = if closed { &CLOSED } else { &OPEN };
    cache.get_or_init(|| {
        let mut points = vec![egui::pos2(-5., 7.)];
        for &[cx, cy, x, y] in if closed { CLOSED_HAND } else { OPEN_HAND } {
            let start = points[points.len() - 1];
            for step in 1..=6 {
                let t = step as f32 / 6.;
                let q = 1. - t;
                points.push(egui::pos2(
                    q * q * start.x + 2. * q * t * cx + t * t * x,
                    q * q * start.y + 2. * q * t * cy + t * t * y,
                ));
            }
        }
        let mut levels: Vec<_> = points.iter().map(|p| p.y).collect();
        levels.sort_by(f32::total_cmp);
        levels.dedup();
        let mut mesh = egui::Mesh::default();
        for band in levels.windows(2) {
            let mid = (band[0] + band[1]) * 0.5;
            let mut edges = Vec::new();
            for pair in points.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                if mid > a.y.min(b.y) && mid < a.y.max(b.y) {
                    let x = |y| a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y);
                    edges.push((x(mid), x(band[0]), x(band[1])));
                }
            }
            edges.sort_by(|a, b| a.0.total_cmp(&b.0));
            for pair in edges.as_chunks::<2>().0 {
                let base = mesh.vertices.len() as u32;
                for point in [
                    egui::pos2(pair[0].1, band[0]),
                    egui::pos2(pair[1].1, band[0]),
                    egui::pos2(pair[1].2, band[1]),
                    egui::pos2(pair[0].2, band[1]),
                ] {
                    mesh.colored_vertex(point, Color32::WHITE);
                }
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base, base + 2, base + 3);
            }
        }
        (points, mesh)
    })
}

/// Open palm and compact gripping fist, with a stable palm-centre hotspot.
fn paint_hand(painter: &egui::Painter, pointer: Pos2, closed: bool) {
    let outline = Color32::from_rgb(25, 29, 35);
    let fill = Color32::from_rgb(248, 250, 253);
    let at = |p: Pos2| pointer + p.to_vec2() * 1.05;
    let (contour, mesh) = hand_geometry(closed);
    let mut mesh = mesh.clone();
    for vertex in &mut mesh.vertices {
        vertex.pos = at(vertex.pos);
        vertex.color = fill;
    }
    // Light outer keyline remains legible over dark objects and the floor.
    let contour: Vec<_> = contour.iter().copied().map(at).collect();
    painter.add(Shape::closed_line(
        contour.clone(),
        Stroke::new(2.8, Color32::from_white_alpha(180)),
    ));
    painter.add(Shape::mesh(mesh));
    painter.add(Shape::closed_line(contour, Stroke::new(1.25, outline)));
    let crease = |points: &[(f32, f32)]| {
        painter.add(Shape::line(
            points.iter().map(|&(x, y)| at(egui::pos2(x, y))).collect(),
            Stroke::new(0.9, outline),
        ));
    };
    if closed {
        // Curled fingers and the thumb laid diagonally across their fronts.
        for x in [-3.5, 0., 3.5] {
            crease(&[(x, -4.), (x + 0.2, -1.5)]);
        }
        crease(&[
            (-7., 1.),
            (-5.5, -0.5),
            (-3., -0.8),
            (0.5, 0.2),
            (1.2, 1.2),
            (0.6, 2.4),
            (-1., 2.8),
            (-4., 2.),
        ]);
        crease(&[(-4., 2.), (-2.8, 4.)]);
    } else {
        crease(&[(-5.5, 0.), (-3.5, 1.5), (-3., 4.)]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn final_pass_replaces_grabs_and_restores_other_cursors() {
        let ctx = egui::Context::default();
        ctx.add_plugin(GrabCursor);
        for (icon, focused, present) in [
            (CursorIcon::Grab, true, true),
            (CursorIcon::Grabbing, true, true),
            (CursorIcon::Default, true, true),
            (CursorIcon::PointingHand, true, true),
            (CursorIcon::ResizeHorizontal, true, true),
            (CursorIcon::Grab, false, true),
            (CursorIcon::Grab, true, false),
        ] {
            let input = egui::RawInput {
                focused,
                events: vec![if present {
                    egui::Event::PointerMoved(egui::pos2(100., 100.))
                } else {
                    egui::Event::PointerGone
                }],
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| ui.ctx().set_cursor_icon(icon));
            let custom =
                focused && present && matches!(icon, CursorIcon::Grab | CursorIcon::Grabbing);
            assert_eq!(
                output.platform_output.cursor_icon,
                if custom { CursorIcon::None } else { icon }
            );
            if custom {
                assert!(!output.shapes.is_empty());
            }
            output.textures_delta.clear();
        }
    }

    #[test]
    fn built_in_drag_payload_gets_the_closed_claw() {
        let ctx = egui::Context::default();
        ctx.add_plugin(GrabCursor);
        let mut output = ctx.run_ui(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(egui::pos2(100., 100.))],
                ..Default::default()
            },
            |ui| egui::DragAndDrop::set_payload(ui.ctx(), "shortcut"),
        );
        assert_eq!(output.platform_output.cursor_icon, CursorIcon::None);
        assert!(!output.shapes.is_empty());
        output.textures_delta.clear();
    }

    #[test]
    fn style_round_trips_and_the_hand_is_painted_when_chosen() {
        for style in GrabCursorStyle::ALL {
            assert_eq!(GrabCursorStyle::from_key(style.key()), Some(style));
        }
        assert_eq!(
            GrabCursorStyle::from_key(" HAND "),
            Some(GrabCursorStyle::Hand)
        );
        assert_eq!(GrabCursorStyle::from_key("paw"), None);
        let ctx = egui::Context::default();
        ctx.add_plugin(GrabCursor);
        assert_eq!(style(&ctx), GrabCursorStyle::Claw);
        let mut counts = Vec::new();
        for chosen in GrabCursorStyle::ALL {
            set_style(&ctx, chosen);
            assert_eq!(style(&ctx), chosen);
            let mut output = ctx.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::PointerMoved(egui::pos2(100., 100.))],
                    ..Default::default()
                },
                |ui| ui.ctx().set_cursor_icon(CursorIcon::Grab),
            );
            assert_eq!(output.platform_output.cursor_icon, CursorIcon::None);
            counts.push(output.shapes.len());
            output.textures_delta.clear();
        }
        // The two looks are different drawings, not the same one twice.
        assert_ne!(counts[0], counts[1]);
    }
}
