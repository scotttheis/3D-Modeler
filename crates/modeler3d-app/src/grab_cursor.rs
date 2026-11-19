//! Shared presentation adapter for every egui grab, including drag and drop.

use eframe::egui::{self, Color32, CursorIcon, Pos2, Shape, Stroke};

/// Runs after egui's built-in drag-and-drop plugin so its cursor is covered too.
#[derive(Default)]
pub(crate) struct GrabCursor;

impl egui::Plugin for GrabCursor {
    fn debug_name(&self) -> &'static str {
        "Robot claw cursor"
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
        paint(&painter, pointer, closed);
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
}
