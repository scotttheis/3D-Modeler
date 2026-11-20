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

/// Finger centre lines across the palm, left to right, with the open
/// fingertip height; points are relative to the hotspot in the palm.
const FINGERS: [(f32, f32); 4] = [(-5.4, -12.5), (-1.8, -15.), (1.8, -14.), (5.4, -11.)];
/// Height of the folded knuckles above the palm when the hand is closed.
const KNUCKLE_TOP: f32 = -6.;
/// Width of a finger's light fill; the dark outline adds `HAND_OUTLINE` per side.
const FINGER_WIDTH: f32 = 2.6;
const HAND_OUTLINE: f32 = 1.2;
/// Palm, as (left, top, right, bottom) around the hotspot.
const PALM: (f32, f32, f32, f32) = (-6.7, -3., 6.7, 8.);
/// The hand art is drawn on a small grid and enlarged to sit beside the claw.
const HAND_SCALE: f32 = 1.2;

/// The traditional hand: an open palm with spread fingers and the thumb out,
/// curling into a fist with the thumb across the front while holding. The
/// hotspot is the centre of the palm in both states.
fn paint_hand(painter: &egui::Painter, pointer: Pos2, closed: bool) {
    let outline = Color32::from_rgb(25, 29, 35);
    let fill = Color32::from_rgb(242, 245, 249);
    let at = |x: f32, y: f32| pointer + egui::vec2(x, y) * HAND_SCALE;
    let palm = egui::Rect::from_min_max(at(PALM.0, PALM.1), at(PALM.2, PALM.3));
    let capsule = |from: Pos2, to: Pos2| {
        for (width, color) in [
            (FINGER_WIDTH + 2. * HAND_OUTLINE, outline),
            (FINGER_WIDTH, fill),
        ] {
            let width = width * HAND_SCALE;
            painter.line_segment([from, to], Stroke::new(width, color));
            painter.circle_filled(from, width / 2., color);
            painter.circle_filled(to, width / 2., color);
        }
    };
    painter.rect_filled(
        palm.expand(HAND_OUTLINE * HAND_SCALE),
        4.5 * HAND_SCALE,
        outline,
    );
    if !closed {
        capsule(at(-4.5, 5.), at(-10., -0.5));
    }
    for (x, tip) in FINGERS {
        // Closed fingers fold over, leaving only the knuckles above the palm.
        let tip = if closed { KNUCKLE_TOP } else { tip };
        capsule(at(x, PALM.1 + 1.), at(x, tip));
    }
    // The palm fill covers the finger roots so the hand reads as one shape.
    painter.rect_filled(palm, 3.5 * HAND_SCALE, fill);
    if closed {
        // The thumb wraps across the front of the folded fingers.
        capsule(at(-6., 2.), at(1.5, 1.));
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
