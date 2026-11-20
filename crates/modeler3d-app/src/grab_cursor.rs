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

/// Hand art traced from the owner's two reference drawings (an open hand and a
/// fist, 2026-11-20), in points with each pose's centroid as the hotspot.
/// Each pose is the outer silhouette followed by the white interior; the band
/// between them is the dark outline, and the fist's finger creases are notches
/// in its interior.
const OPEN_HAND: &[&[(f32, f32)]] = &[
    &[
        (-5.25, -13.34),
        (-5.70, -13.12),
        (-6.47, -12.12),
        (-6.58, -11.68),
        (-6.47, -10.79),
        (-2.82, -1.94),
        (-2.93, -1.83),
        (-6.47, -7.36),
        (-7.36, -8.69),
        (-7.91, -9.24),
        (-8.91, -9.57),
        (-9.90, -9.35),
        (-10.57, -8.80),
        (-10.79, -8.36),
        (-10.90, -7.25),
        (-10.57, -6.37),
        (-6.58, -0.06),
        (-6.14, 0.38),
        (-5.92, 1.05),
        (-4.48, 3.26),
        (-4.48, 3.70),
        (-4.70, 3.92),
        (-5.25, 3.70),
        (-8.24, 0.49),
        (-9.46, -0.39),
        (-10.23, -0.61),
        (-11.34, -0.50),
        (-12.34, 0.27),
        (-12.67, 1.27),
        (-12.45, 2.26),
        (-6.80, 8.13),
        (-5.92, 9.23),
        (-4.59, 10.45),
        (-3.59, 11.11),
        (-1.60, 12.00),
        (0.06, 12.33),
        (2.05, 12.33),
        (4.26, 11.78),
        (6.58, 10.56),
        (8.35, 8.90),
        (9.02, 8.02),
        (9.68, 6.69),
        (10.12, 5.14),
        (10.23, 4.37),
        (10.12, 3.92),
        (10.23, 3.04),
        (10.12, 2.93),
        (10.23, 2.82),
        (9.79, 0.49),
        (9.46, -0.39),
        (9.35, -3.49),
        (9.13, -4.71),
        (9.24, -5.81),
        (9.02, -7.69),
        (9.13, -7.80),
        (8.91, -9.35),
        (8.24, -10.24),
        (7.58, -10.57),
        (6.58, -10.57),
        (5.92, -10.24),
        (5.37, -9.57),
        (5.26, -9.24),
        (5.26, -6.70),
        (5.37, -6.48),
        (5.26, -6.37),
        (5.48, -5.15),
        (5.37, -5.04),
        (5.48, -2.60),
        (5.26, -2.49),
        (4.92, -3.27),
        (4.92, -3.93),
        (4.70, -4.37),
        (4.70, -4.93),
        (3.15, -12.23),
        (2.93, -12.67),
        (1.94, -13.45),
        (0.83, -13.56),
        (0.28, -13.34),
        (-0.39, -12.67),
        (-0.72, -11.57),
        (0.94, -3.93),
        (0.83, -3.38),
        (-2.93, -12.56),
        (-3.93, -13.34),
        (-4.59, -13.45),
    ],
    &[
        (-4.81, -12.67),
        (-4.15, -12.56),
        (-3.59, -12.12),
        (0.28, -2.60),
        (0.94, -1.94),
        (1.72, -2.16),
        (1.83, -3.49),
        (1.27, -5.48),
        (0.50, -9.91),
        (0.06, -11.46),
        (0.28, -12.23),
        (0.72, -12.67),
        (1.49, -12.78),
        (1.83, -12.67),
        (2.38, -12.12),
        (4.48, -2.16),
        (4.81, -1.83),
        (5.59, -1.72),
        (6.25, -2.49),
        (6.14, -4.93),
        (6.03, -5.04),
        (6.14, -5.15),
        (6.03, -5.26),
        (6.14, -5.48),
        (6.03, -9.13),
        (6.69, -9.80),
        (7.47, -9.80),
        (8.02, -9.35),
        (8.24, -8.91),
        (8.58, -2.94),
        (8.69, -2.83),
        (8.58, -2.38),
        (8.69, -2.27),
        (8.58, -2.16),
        (8.69, -0.50),
        (9.24, 1.49),
        (9.46, 2.93),
        (9.46, 4.37),
        (9.24, 5.58),
        (8.69, 7.02),
        (7.80, 8.35),
        (6.14, 9.90),
        (4.15, 11.00),
        (2.05, 11.56),
        (0.06, 11.56),
        (-1.49, 11.23),
        (-3.37, 10.34),
        (-4.26, 9.68),
        (-11.78, 1.82),
        (-11.89, 1.16),
        (-11.67, 0.71),
        (-10.90, 0.16),
        (-10.23, 0.16),
        (-9.35, 0.60),
        (-5.70, 4.37),
        (-5.03, 4.70),
        (-4.48, 4.70),
        (-3.71, 3.92),
        (-3.71, 3.15),
        (-4.04, 2.48),
        (-4.26, 2.37),
        (-4.48, 1.71),
        (-6.80, -2.05),
        (-8.02, -3.71),
        (-8.13, -4.15),
        (-10.12, -7.25),
        (-10.12, -7.92),
        (-9.79, -8.47),
        (-9.13, -8.80),
        (-8.35, -8.58),
        (-7.47, -7.47),
        (-3.48, -1.17),
        (-2.71, -0.50),
        (-2.05, -0.72),
        (-1.82, -1.50),
        (-5.81, -11.23),
        (-5.59, -12.12),
        (-5.14, -12.56),
    ],
];
const CLOSED_HAND: &[&[(f32, f32)]] = &[
    &[
        (3.21, -10.31),
        (2.81, -10.44),
        (1.22, -10.31),
        (0.69, -10.04),
        (-0.24, -8.85),
        (-1.17, -9.51),
        (-2.64, -9.64),
        (-3.57, -9.24),
        (-4.50, -7.92),
        (-4.90, -7.92),
        (-5.29, -8.18),
        (-6.49, -8.18),
        (-7.15, -7.92),
        (-7.82, -7.38),
        (-8.35, -6.32),
        (-8.22, -4.73),
        (-9.41, -4.19),
        (-9.95, -3.80),
        (-10.48, -2.73),
        (-10.34, -1.14),
        (-8.48, 1.39),
        (-8.75, 3.12),
        (-8.75, 4.84),
        (-8.48, 5.77),
        (-7.82, 6.97),
        (-6.76, 8.03),
        (-5.83, 8.57),
        (-5.43, 8.57),
        (-5.03, 8.83),
        (0.95, 9.63),
        (3.21, 9.50),
        (5.21, 8.57),
        (7.47, 6.84),
        (8.93, 5.38),
        (9.72, 4.05),
        (10.39, 1.52),
        (10.39, 0.32),
        (9.72, -2.07),
        (9.19, -3.00),
        (4.94, -8.58),
    ],
    &[
        (2.95, -9.38),
        (8.26, -2.60),
        (9.06, -1.14),
        (9.46, 1.39),
        (9.33, 2.32),
        (8.66, 4.05),
        (8.00, 4.98),
        (6.53, 6.31),
        (4.01, 8.17),
        (2.95, 8.57),
        (1.09, 8.70),
        (-4.76, 7.90),
        (-5.96, 7.37),
        (-6.89, 6.57),
        (-7.55, 5.51),
        (-7.82, 4.71),
        (-7.69, 2.72),
        (-7.55, 2.58),
        (-6.22, 4.18),
        (-5.83, 4.18),
        (-5.56, 3.65),
        (-9.55, -1.67),
        (-9.41, -2.87),
        (-8.88, -3.40),
        (-7.95, -3.66),
        (-7.29, -3.40),
        (-5.69, -1.40),
        (-5.43, -1.27),
        (-5.03, -1.40),
        (-4.90, -2.07),
        (-7.42, -5.39),
        (-7.29, -6.45),
        (-6.36, -7.25),
        (-5.56, -7.25),
        (-5.03, -6.99),
        (-3.03, -4.33),
        (-2.37, -3.66),
        (-1.84, -3.80),
        (-1.84, -4.46),
        (-3.57, -6.85),
        (-3.43, -7.92),
        (-2.77, -8.58),
        (-2.37, -8.71),
        (-1.44, -8.58),
        (0.29, -6.45),
        (1.09, -6.05),
        (1.35, -6.32),
        (1.35, -6.72),
        (0.69, -7.78),
        (0.69, -8.31),
        (0.95, -8.85),
        (1.75, -9.51),
    ],
];

/// Fill a fixed concave contour with horizontal trapezoids. Splitting at every
/// vertex height preserves finger gaps; egui's convex fill cannot do that.
fn fill_contour(points: &[Pos2]) -> egui::Mesh {
    let mut closed = points.to_vec();
    closed.extend(points.first().copied());
    let mut levels: Vec<_> = points.iter().map(|p| p.y).collect();
    levels.sort_by(f32::total_cmp);
    levels.dedup();
    let mut mesh = egui::Mesh::default();
    for band in levels.windows(2) {
        let mid = (band[0] + band[1]) * 0.5;
        let mut edges = Vec::new();
        for pair in closed.windows(2) {
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
    mesh
}

/// Contours and fills for one pose, built once and never rebuilt per frame.
fn hand_geometry(closed: bool) -> &'static [(Vec<Pos2>, egui::Mesh)] {
    type Geometry = Vec<(Vec<Pos2>, egui::Mesh)>;
    static OPEN: std::sync::OnceLock<Geometry> = std::sync::OnceLock::new();
    static CLOSED: std::sync::OnceLock<Geometry> = std::sync::OnceLock::new();
    let (cache, art) = if closed {
        (&CLOSED, CLOSED_HAND)
    } else {
        (&OPEN, OPEN_HAND)
    };
    cache.get_or_init(|| {
        art.iter()
            .map(|contour| {
                let points: Vec<_> = contour.iter().map(|&(x, y)| egui::pos2(x, y)).collect();
                let mesh = fill_contour(&points);
                (points, mesh)
            })
            .collect()
    })
}

/// Open palm and gripping fist, with a stable centre hotspot.
fn paint_hand(painter: &egui::Painter, pointer: Pos2, closed: bool) {
    let outline = Color32::from_rgb(25, 29, 35);
    let fill = Color32::from_rgb(248, 250, 253);
    let at = |p: Pos2| pointer + p.to_vec2() * 1.05;
    for (index, (contour, mesh)) in hand_geometry(closed).iter().enumerate() {
        let outer = index == 0;
        let contour: Vec<_> = contour.iter().copied().map(at).collect();
        if outer {
            // Light outer keyline remains legible over dark objects and the floor.
            painter.add(Shape::closed_line(
                contour.clone(),
                Stroke::new(2.6, Color32::from_white_alpha(170)),
            ));
        }
        let mut mesh = mesh.clone();
        for vertex in &mut mesh.vertices {
            vertex.pos = at(vertex.pos);
            vertex.color = if outer { outline } else { fill };
        }
        painter.add(Shape::mesh(mesh));
        // A thin stroke on every edge smooths the untextured fill.
        let width = if outer { 0.8 } else { 0.5 };
        painter.add(Shape::closed_line(contour, Stroke::new(width, outline)));
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
