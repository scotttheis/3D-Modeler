//! Small shell-chrome glyphs and shared control metrics.
//!
//! The owner's artwork covers ribbon commands. Panel utilities that have no
//! supplied artwork (list selection helpers, the visibility toggle) are drawn
//! here as one consistent line-icon family, so the shell never repeats a
//! generic placeholder icon or depends on a font carrying a symbol glyph.

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2};

/// Font size for captions and secondary labels.
pub(crate) const FONT_CAPTION: f32 = 11.0;
/// Font size for compact control labels.
pub(crate) const FONT_SMALL: f32 = 12.5;
/// Font size for body text.
pub(crate) const FONT_BODY: f32 = 13.0;
/// Font size for titles.
pub(crate) const FONT_TITLE: f32 = 15.0;
/// Font size for prominent headings.
pub(crate) const FONT_HERO: f32 = 20.0;
/// Smallest spacing step in logical points.
pub(crate) const SPACE_1: f32 = 4.0;
/// Second spacing step in logical points.
pub(crate) const SPACE_2: f32 = 8.0;
/// Third spacing step in logical points.
pub(crate) const SPACE_3: f32 = 12.0;
/// Fourth spacing step in logical points.
pub(crate) const SPACE_4: f32 = 16.0;
/// Largest spacing step in logical points.
pub(crate) const SPACE_5: f32 = 24.0;

/// Corner radius for inline controls: buttons, fields, list rows.
pub(crate) const RADIUS_CONTROL: f32 = 6.0;
/// Corner radius for cards and floating surfaces.
pub(crate) const RADIUS_SURFACE: f32 = 10.0;
/// Height of a single-line field or compact button.
pub(crate) const CONTROL_HEIGHT: f32 = 26.0;
/// Height of a list row.
pub(crate) const ROW_HEIGHT: f32 = 28.0;

/// Axis colours shared by numeric fields and the viewport handles.
pub(crate) const AXIS_COLORS: [Color32; 3] = [
    Color32::from_rgb(0xE5, 0x5C, 0x5C),
    Color32::from_rgb(0x4C, 0xB8, 0x5C),
    Color32::from_rgb(0x5B, 0x7C, 0xF0),
];

/// Groups panel content under a strong section title and hairline on every platform.
pub(crate) fn section<R>(
    ui: &mut egui::Ui,
    palette: crate::theme::ThemePalette,
    title: &str,
    contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.push_id(title, |ui| {
        let (line, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().hline(
            line.x_range(),
            line.center().y,
            Stroke::new(1.0, palette.divider),
        );
        ui.add_space(SPACE_2);
        ui.label(
            egui::RichText::new(title)
                .size(FONT_SMALL)
                .strong()
                .color(palette.text),
        );
        ui.add_space(SPACE_2);
        contents(ui)
    })
    .inner
}

/// Lays out an inspector row with a fixed label and three equal axis fields.
/// Hides an overflowing distance suffix only when the selector shows that unit;
/// the original field is preserved until an edit, and parsing stays with the caller.
/// Returns which fields changed, leaving interpretation to the caller.
pub(crate) fn transform_field_grid(
    ui: &mut egui::Ui,
    palette: crate::theme::ThemePalette,
    label: &str,
    fields: &mut [String; 3],
    selected_unit: Option<&str>,
) -> [bool; 3] {
    let label_width = SPACE_5 + SPACE_4;
    let gap = SPACE_1 * 0.5;
    let field_width = (ui.available_width() - label_width - gap * 3.0) / 3.0;
    let field_margin = gap;
    let font = egui::FontId::proportional(FONT_SMALL);
    let mut changed = [false; 3];
    egui::Grid::new(label)
        .num_columns(4)
        .min_col_width(0.0)
        .spacing(egui::vec2(gap, SPACE_1))
        .min_row_height(CONTROL_HEIGHT)
        .show(ui, |ui| {
            ui.allocate_exact_size(Vec2::new(label_width, CONTROL_HEIGHT), egui::Sense::hover());
            for (axis, color) in ["X", "Y", "Z"].into_iter().zip(AXIS_COLORS) {
                ui.add_sized(
                    [field_width, CONTROL_HEIGHT],
                    egui::Label::new(egui::RichText::new(axis).size(FONT_CAPTION).color(color)),
                );
            }
            ui.end_row();
            ui.add_sized(
                [label_width, CONTROL_HEIGHT],
                egui::Label::new(
                    egui::RichText::new(label)
                        .size(FONT_CAPTION)
                        .color(palette.muted),
                ),
            );
            for (axis, field) in fields.iter_mut().enumerate() {
                let mut displayed = field.clone();
                let text_width = ui.fonts_mut(|fonts| {
                    fonts
                        .layout_no_wrap(displayed.clone(), font.clone(), AXIS_COLORS[axis])
                        .size()
                        .x
                });
                if text_width > field_width - field_margin * 2.0 - 1.0
                    && let Some(unit) = selected_unit
                    && let Some(value) = displayed.strip_suffix(&format!(" {unit}"))
                {
                    displayed = value.to_owned();
                }
                changed[axis] = ui
                    .add_sized(
                        [field_width, CONTROL_HEIGHT],
                        egui::TextEdit::singleline(&mut displayed)
                            .desired_width(field_width)
                            .margin(egui::vec2(field_margin, field_margin))
                            .font(font.clone())
                            .text_color(AXIS_COLORS[axis])
                            .align(egui::Align2::LEFT_CENTER),
                    )
                    .changed();
                if changed[axis] {
                    *field = displayed;
                }
            }
            ui.end_row();
        });
    changed
}

/// Sizes each inspector Apply action to the same field-grid width and height.
pub(crate) fn inspector_apply_button(
    ui: &mut egui::Ui,
    label: &str,
    tooltip: &str,
) -> egui::Response {
    ui.add_space(SPACE_2);
    ui.add_sized(
        [ui.available_width(), CONTROL_HEIGHT],
        egui::Button::new(label).corner_radius(RADIUS_CONTROL),
    )
    .on_hover_text(tooltip)
}

/// A line icon drawn on a 16-point grid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Glyph {
    /// Floor-aligned axes.
    Floor,
    /// Imported object axes.
    ObjectFront,
    /// Two shapes inside one outline.
    Group,
    /// Two separate shapes.
    Ungroup,
    /// A filled selection frame.
    SelectAll,
    /// An empty dashed selection frame.
    DeselectAll,
    /// A half-filled selection frame.
    InvertSelection,
    /// Object is shown.
    Visible,
    /// Object is hidden.
    Hidden,
    /// Mesh needs attention.
    Warning,
}

/// A two-position icon switch for floor or imported-object resize axes.
pub(crate) fn resize_basis_toggle(
    ui: &mut egui::Ui,
    object: bool,
    palette: crate::theme::ThemePalette,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(60., CONTROL_HEIGHT), egui::Sense::click());
    let tooltip = if object {
        "Resize relative to the object's imported front (first object for a selection). Click to use the front of the floor."
    } else {
        "Resize relative to the front of the floor. Click to use the object's imported front."
    };
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, ui.is_enabled(), tooltip));
    let painter = ui.painter();
    painter.rect(
        rect,
        13.,
        palette.surface,
        Stroke::new(1., palette.border),
        StrokeKind::Inside,
    );
    let progress = ui.ctx().animate_bool(response.id, object);
    let selected = Rect::from_min_size(
        rect.min + Vec2::new(2. + progress * 29., 2.),
        Vec2::new(27., 22.),
    );
    painter.rect_filled(
        selected,
        RADIUS_SURFACE,
        if response.hovered() {
            palette.accent_control_hover
        } else {
            palette.accent_control
        },
    );
    for (i, glyph) in [Glyph::Floor, Glyph::ObjectFront].into_iter().enumerate() {
        let center = rect.min + Vec2::new(15.5 + i as f32 * 29., 13.);
        paint_glyph(
            painter,
            Rect::from_center_size(center, Vec2::splat(17.)),
            glyph,
            if (i == 1) == object {
                palette.accent_text
            } else {
                palette.muted
            },
        );
    }
    response.on_hover_text(tooltip)
}

/// Paints `glyph` centred in `rect` with a stroke weight matched to its size.
pub(crate) fn paint_glyph(painter: &Painter, rect: Rect, glyph: Glyph, color: Color32) {
    let unit = rect.width().min(rect.height()) / 16.0;
    let origin = rect.center() - Vec2::splat(8.0 * unit);
    let at = |x: f32, y: f32| Pos2::new(origin.x + x * unit, origin.y + y * unit);
    let boxed = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(at(x0, y0), at(x1, y1));
    let stroke = Stroke::new((1.4 * unit).max(1.0), color);
    let radius = 1.5 * unit;
    match glyph {
        Glyph::Floor => {
            painter.add(egui::Shape::closed_line(
                vec![at(1., 11.), at(6., 4.), at(15., 6.), at(10., 14.)],
                stroke,
            ));
            painter.line_segment([at(4., 10.), at(12., 12.)], stroke);
            painter.line_segment([at(8., 5.), at(3., 12.)], stroke);
        }
        Glyph::ObjectFront => {
            painter.add(egui::Shape::closed_line(
                vec![
                    at(2., 5.),
                    at(8., 2.),
                    at(14., 5.),
                    at(14., 12.),
                    at(8., 15.),
                    at(2., 12.),
                ],
                stroke,
            ));
            painter.line_segment([at(2., 5.), at(8., 8.)], stroke);
            painter.line_segment([at(14., 5.), at(8., 8.)], stroke);
            painter.line_segment([at(8., 8.), at(8., 15.)], stroke);
        }
        Glyph::Group => {
            painter.rect_stroke(
                boxed(1.5, 1.5, 14.5, 14.5),
                2.0 * unit,
                stroke,
                StrokeKind::Middle,
            );
            painter.rect_filled(boxed(4.0, 4.0, 8.5, 8.5), 1.0 * unit, color);
            painter.rect_filled(
                boxed(7.5, 7.5, 12.0, 12.0),
                1.0 * unit,
                color.gamma_multiply(0.55),
            );
        }
        Glyph::Ungroup => {
            painter.rect_stroke(
                boxed(1.5, 1.5, 8.0, 8.0),
                radius,
                stroke,
                StrokeKind::Middle,
            );
            painter.rect_stroke(
                boxed(8.0, 8.0, 14.5, 14.5),
                radius,
                stroke,
                StrokeKind::Middle,
            );
        }
        Glyph::SelectAll => {
            painter.rect_stroke(
                boxed(1.5, 1.5, 14.5, 14.5),
                2.0 * unit,
                stroke,
                StrokeKind::Middle,
            );
            painter.rect_filled(boxed(4.5, 4.5, 11.5, 11.5), 1.0 * unit, color);
        }
        Glyph::DeselectAll => {
            for (start, end) in dashed_frame() {
                painter.line_segment([at(start.0, start.1), at(end.0, end.1)], stroke);
            }
        }
        Glyph::InvertSelection => {
            painter.rect_stroke(
                boxed(1.5, 1.5, 14.5, 14.5),
                2.0 * unit,
                stroke,
                StrokeKind::Middle,
            );
            painter.add(egui::Shape::convex_polygon(
                vec![at(4.0, 4.0), at(12.0, 4.0), at(4.0, 12.0)],
                color,
                Stroke::NONE,
            ));
        }
        Glyph::Visible | Glyph::Hidden => {
            let lid = |sign: f32| {
                (0..=12)
                    .map(|step| {
                        let t = step as f32 / 12.0;
                        let x = 1.5 + 13.0 * t;
                        let y = 8.0 - sign * 4.2 * (std::f32::consts::PI * t).sin();
                        at(x, y)
                    })
                    .collect::<Vec<_>>()
            };
            painter.add(egui::Shape::line(lid(1.0), stroke));
            painter.add(egui::Shape::line(lid(-1.0), stroke));
            if glyph == Glyph::Visible {
                painter.circle_filled(at(8.0, 8.0), 2.0 * unit, color);
            } else {
                painter.line_segment([at(3.0, 13.5), at(13.0, 2.5)], stroke);
            }
        }
        Glyph::Warning => {
            painter.add(egui::Shape::closed_line(
                vec![at(8.0, 2.0), at(14.5, 13.5), at(1.5, 13.5)],
                stroke,
            ));
            painter.line_segment([at(8.0, 6.5), at(8.0, 9.8)], stroke);
            painter.circle_filled(at(8.0, 11.7), 0.8 * unit, color);
        }
    }
}

type Segment = ((f32, f32), (f32, f32));

fn dashed_frame() -> [Segment; 8] {
    [
        ((2.0, 2.0), (6.0, 2.0)),
        ((10.0, 2.0), (14.0, 2.0)),
        ((14.0, 2.0), (14.0, 6.0)),
        ((14.0, 10.0), (14.0, 14.0)),
        ((14.0, 14.0), (10.0, 14.0)),
        ((6.0, 14.0), (2.0, 14.0)),
        ((2.0, 14.0), (2.0, 10.0)),
        ((2.0, 6.0), (2.0, 2.0)),
    ]
}

/// A square icon button with hover, focus, and accessible-name handling.
/// Returns the response so callers can attach a tooltip.
pub(crate) fn glyph_button(
    ui: &mut egui::Ui,
    glyph: Glyph,
    color: Color32,
    hover_fill: Color32,
    focus_stroke: Color32,
    accessible_label: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, accessible_label));
    if response.hovered() || response.has_focus() {
        ui.painter().rect_filled(rect, RADIUS_CONTROL, hover_fill);
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            RADIUS_CONTROL,
            Stroke::new(1.5, focus_stroke),
            StrokeKind::Inside,
        );
    }
    paint_glyph(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
        glyph,
        color,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}
