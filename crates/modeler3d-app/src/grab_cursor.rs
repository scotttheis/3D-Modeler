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

/// The jaw gap is the pointer hotspot in both states; only the jaws close.
fn paint(painter: &egui::Painter, pointer: Pos2, closed: bool) {
    let (tip, elbow) = if closed { (1.5, 6.0) } else { (6.0, 12.0) };
    let outline = Color32::from_rgb(25, 29, 35);
    let fill = Color32::from_rgb(242, 245, 249);
    for side in [-1., 1.] {
        let points = [(tip, 0.), (elbow, -5.), (elbow, 2.), (5., 11.), (3., 12.)]
            .map(|(x, y)| pointer + egui::vec2(side * x, y));
        painter.add(Shape::line(points.to_vec(), Stroke::new(5., outline)));
        painter.add(Shape::line(points.to_vec(), Stroke::new(2.5, fill)));
    }
    let wrist = egui::Rect::from_min_max(
        pointer + egui::vec2(-4., 11.),
        pointer + egui::vec2(4., 20.),
    );
    painter.rect(
        wrist,
        1.5,
        fill,
        Stroke::new(1.5, outline),
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [
            pointer + egui::vec2(-2., 15.),
            pointer + egui::vec2(2., 15.),
        ],
        Stroke::new(1.5, outline),
    );
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
