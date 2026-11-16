//! Paint mode (OF-44), modelled on 3D Builder: the Paint bar replaces the
//! ribbon with Cancel, Done, the chosen colour and a Coverage slider. The
//! pointer becomes a spray can over the view, and each click on an object
//! paints it (full coverage) or the region around the clicked face (lower
//! coverage) as one undo entry. Dragging still turns the view.

use eframe::egui::{self, Align2, Color32, FontId, Sense, Stroke, Ui, Vec2};

use super::{Modeler3dApp, RIBBON_BUTTON_HEIGHT, ViewportRenderer, viewport_pixel_position};
use crate::theme::ThemePalette;

/// The colour Paint starts with when nothing is selected: 3D Builder's green.
pub(super) const DEFAULT_PAINT_COLOR: [f32; 4] = [0.29, 0.76, 0.38, 1.0];

/// Live state of the Paint bar.
#[derive(Clone, Copy, Debug)]
pub(super) struct PaintMode {
    pub(super) color: [f32; 4],
    /// Coverage slider, 0..=1. 1 paints the whole object.
    pub(super) coverage: f32,
    picker_open: bool,
    /// History position when Paint started; Cancel undoes back to it.
    entry_history: u64,
    /// Paint clicks that changed something, so Cancel undoes no further.
    strokes: usize,
}

impl PaintMode {
    /// Opens the colour drop-down (used by the review harness).
    pub(super) fn open_colour_picker(&mut self) {
        self.picker_open = true;
    }
}

impl Modeler3dApp {
    /// Paint button: opens the Paint bar, or closes it (keeping the paint)
    /// when it is already open.
    pub(super) fn toggle_paint_mode(&mut self) {
        if self.paint_mode.is_some() {
            self.finish_paint_mode(true);
            return;
        }
        if self.commands.scene().objects().is_empty() {
            self.commands.set_status("Add an object to paint");
            return;
        }
        self.measure_mode = None;
        self.active_brush = None;
        self.brush_settings_open = false;
        self.boolean_pick = None;
        self.viewport_drag = None;
        let color = self
            .commands
            .scene()
            .ordered_selection()
            .first()
            .and_then(|index| self.commands.scene().object(*index).ok())
            .map_or(self.paint_color, modeler3d_core::SceneObject::color);
        self.paint_mode = Some(PaintMode {
            color,
            coverage: self.paint_coverage,
            picker_open: false,
            entry_history: self.commands.scene().history_state_id(),
            strokes: 0,
        });
        self.commands
            .set_status("Paint: click an object to colour it. Lower Coverage paints one area.");
    }

    /// Leaves Paint. Done keeps every paint click; Cancel undoes them.
    pub(super) fn finish_paint_mode(&mut self, keep: bool) {
        let Some(mode) = self.paint_mode.take() else {
            return;
        };
        self.paint_color = mode.color;
        self.paint_coverage = mode.coverage;
        if keep {
            self.commands.set_status("Paint done");
            return;
        }
        let mut undone = 0;
        while undone < mode.strokes
            && self.commands.scene().history_state_id() != mode.entry_history
        {
            match self.commands.dispatch(crate::commands::CommandId::Undo) {
                Ok(_) => undone += 1,
                Err(error) => {
                    self.report_command_error(error);
                    break;
                }
            }
        }
        self.invalidate_presentation();
        self.commands.set_status(if undone == 0 {
            "Paint cancelled"
        } else {
            "Paint cancelled; the paint was removed"
        });
    }

    /// One click in the view while painting.
    pub(super) fn paint_click(&mut self, rect: egui::Rect, pointer: egui::Pos2) {
        let Some(mode) = self.paint_mode else {
            return;
        };
        let Some(scene) = self.preview.as_ref() else {
            return;
        };
        let size = self
            .viewport_renderer
            .as_ref()
            .map_or(self.camera.viewport_size(), ViewportRenderer::size);
        let screen = viewport_pixel_position(pointer, rect, size);
        let hit = match modeler3d_viewport::pick_scene(scene, &self.camera, screen[0], screen[1]) {
            Ok(Some(hit)) => hit,
            Ok(None) => {
                self.commands.set_status("Paint: click on an object");
                return;
            }
            Err(error) => {
                self.commands.set_status(error.to_string());
                return;
            }
        };
        let index = match self.commands.scene().object_index(&hit.object_id) {
            Ok(index) => index,
            Err(error) => {
                self.commands.set_status(error.to_string());
                return;
            }
        };
        let face = usize::try_from(hit.face_index).unwrap_or(usize::MAX);
        match self
            .commands
            .paint_at(index, face, mode.coverage, mode.color)
        {
            Ok(crate::commands::CommandOutcome::Applied(_)) => {
                if let Some(mode) = self.paint_mode.as_mut() {
                    mode.strokes += 1;
                }
                self.invalidate_presentation();
            }
            Ok(_) => {}
            Err(error) => self.report_command_error(error),
        }
    }

    /// The Paint bar, drawn where the ribbon commands normally are.
    pub(super) fn render_paint_bar(&mut self, ui: &mut Ui, palette: ThemePalette) {
        let Some(mut mode) = self.paint_mode else {
            return;
        };
        let mut finish = None;
        ui.horizontal(|ui| {
            ui.set_height(RIBBON_BUTTON_HEIGHT);
            ui.spacing_mut().item_spacing.x = 6.0;
            if paint_bar_button(
                ui,
                "Cancel",
                "Leave Paint and remove this paint",
                palette,
                |painter, rect, color| {
                    // Back arrow.
                    let mid = rect.center();
                    let stroke = Stroke::new(2.0, color);
                    painter.line_segment(
                        [mid + Vec2::new(-9.0, 0.0), mid + Vec2::new(9.0, 0.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [mid + Vec2::new(-9.0, 0.0), mid + Vec2::new(-2.0, -7.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [mid + Vec2::new(-9.0, 0.0), mid + Vec2::new(-2.0, 7.0)],
                        stroke,
                    );
                },
            ) {
                finish = Some(false);
            }
            if paint_bar_button(
                ui,
                "Done",
                "Leave Paint and keep the paint",
                palette,
                |painter, rect, color| {
                    // Tick.
                    let mid = rect.center();
                    let stroke = Stroke::new(2.0, color);
                    painter.line_segment(
                        [mid + Vec2::new(-8.0, 0.0), mid + Vec2::new(-2.0, 6.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [mid + Vec2::new(-2.0, 6.0), mid + Vec2::new(9.0, -7.0)],
                        stroke,
                    );
                },
            ) {
                finish = Some(true);
            }
            paint_bar_divider(ui, palette);
            ui.label(egui::RichText::new("Selected").color(palette.text));
            let (swatch, response) = ui.allocate_exact_size(Vec2::new(44.0, 26.0), Sense::click());
            let response = response
                .on_hover_text("Choose the paint colour")
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let fill = to_color32(mode.color);
            let block =
                egui::Rect::from_min_size(swatch.min + Vec2::new(0.0, 3.0), Vec2::splat(20.0));
            ui.painter().rect_filled(block, 3.0, fill);
            ui.painter().rect_stroke(
                block,
                3.0,
                Stroke::new(1.0, palette.divider),
                egui::StrokeKind::Inside,
            );
            let chevron = egui::pos2(block.right() + 12.0, block.center().y);
            let stroke = Stroke::new(1.5, palette.text);
            ui.painter().line_segment(
                [
                    chevron + Vec2::new(-5.0, -2.5),
                    chevron + Vec2::new(0.0, 2.5),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    chevron + Vec2::new(0.0, 2.5),
                    chevron + Vec2::new(5.0, -2.5),
                ],
                stroke,
            );
            if response.clicked() {
                mode.picker_open = !mode.picker_open;
            }
            if mode.picker_open {
                let area = egui::Area::new(egui::Id::unique("modeler3d-paint-colour"))
                    .order(egui::Order::Foreground)
                    .fixed_pos(swatch.left_bottom() + Vec2::new(0.0, 6.0))
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.set_max_width(260.0);
                            crate::dialogs::color_chooser(ui, &mut mode.color);
                        });
                    });
                if area.response.clicked_elsewhere() && !response.clicked() {
                    mode.picker_open = false;
                }
            }
            paint_bar_divider(ui, palette);
            ui.label(egui::RichText::new("Coverage").color(palette.text));
            let mut percent = (mode.coverage * 100.0).round();
            let slider = ui
                .add(
                    egui::Slider::new(&mut percent, 0.0..=100.0)
                        .step_by(1.0)
                        .fixed_decimals(0)
                        .suffix(" %"),
                )
                .on_hover_text(
                    "100 % paints the whole object. Lower paints only the area around \
                     where you click that faces the same way.",
                );
            if slider.changed() {
                mode.coverage = (percent / 100.0).clamp(0.0, 1.0);
            }
        });
        match finish {
            Some(keep) => {
                self.paint_mode = Some(mode);
                self.finish_paint_mode(keep);
            }
            None => self.paint_mode = Some(mode),
        }
    }
}

/// Draws the spray-can pointer with its nozzle at `pointer`, filled with the
/// paint colour so the user sees what a click will do.
pub(super) fn paint_spray_cursor(
    painter: &egui::Painter,
    pointer: egui::Pos2,
    color: [f32; 4],
    palette: ThemePalette,
) {
    let outline = Stroke::new(1.5, palette.text);
    let fill = to_color32(color);
    let at = |x: f32, y: f32| pointer + Vec2::new(x, y);
    // Spray dots fanning out from the nozzle toward the click point.
    for (x, y, r) in [
        (0.0, 0.0, 1.8),
        (-5.0, -3.0, 1.4),
        (-3.0, -7.0, 1.4),
        (-9.0, -1.0, 1.2),
        (-1.0, -11.0, 1.2),
        (-8.0, -8.0, 1.2),
    ] {
        painter.circle_filled(at(x, y), r + 0.8, palette.panel);
        painter.circle_filled(at(x, y), r, fill);
    }
    // Nozzle, then the can body in the paint colour.
    let nozzle = egui::Rect::from_min_max(at(4.0, 3.0), at(12.0, 10.0));
    painter.rect_filled(nozzle, 1.5, palette.panel);
    painter.rect_stroke(nozzle, 1.5, outline, egui::StrokeKind::Inside);
    let cap = egui::Rect::from_min_max(at(8.0, 9.0), at(22.0, 15.0));
    painter.rect_filled(cap, 2.0, palette.panel);
    painter.rect_stroke(cap, 2.0, outline, egui::StrokeKind::Inside);
    let body = egui::Rect::from_min_max(at(6.0, 14.0), at(24.0, 40.0));
    painter.rect_filled(body, 4.0, fill);
    painter.rect_stroke(body, 4.0, outline, egui::StrokeKind::Inside);
    let shine = egui::Rect::from_min_max(at(9.0, 18.0), at(12.0, 34.0));
    painter.rect_filled(shine, 1.5, Color32::from_white_alpha(70));
}

pub(super) fn paint_bar_button(
    ui: &mut Ui,
    label: &str,
    hint: &str,
    palette: ThemePalette,
    glyph: impl FnOnce(&egui::Painter, egui::Rect, Color32),
) -> bool {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(64.0, RIBBON_BUTTON_HEIGHT - 8.0), Sense::click());
    let response = response
        .on_hover_text(hint)
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 4.0, palette.accent_control.gamma_multiply(0.14));
    }
    let icon = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.top() + 20.0),
        Vec2::splat(24.0),
    );
    glyph(ui.painter(), icon, palette.text);
    ui.painter().text(
        egui::pos2(rect.center().x, rect.bottom() - 12.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(12.0),
        palette.text,
    );
    response.clicked()
}

pub(super) fn paint_bar_divider(ui: &mut Ui, palette: ThemePalette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, 34.0), Sense::hover());
    ui.painter().line_segment(
        [
            egui::pos2(rect.center().x, rect.top()),
            egui::pos2(rect.center().x, rect.bottom()),
        ],
        Stroke::new(1.0, palette.divider),
    );
}

fn to_color32(color: [f32; 4]) -> Color32 {
    let [r, g, b, _] = modeler3d_core::color_rgba8(color);
    Color32::from_rgb(r, g, b)
}
