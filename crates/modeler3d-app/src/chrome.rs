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

/// Shared floating dialog surface on every desktop platform.
pub(crate) fn dialog_frame(mode: crate::theme::ThemeMode) -> egui::Frame {
    let palette = crate::theme::ThemePalette::for_mode(mode);
    egui::Frame::new()
        .fill(palette.surface)
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(RADIUS_SURFACE)
        .shadow(crate::theme::floating_shadow(mode))
        .inner_margin(egui::Margin::same(SPACE_4 as i8))
}

/// Raised dialog title surface, using the active palette on every platform.
pub(crate) fn dialog_title_frame(mode: crate::theme::ThemeMode) -> egui::Frame {
    dialog_frame(mode).fill(crate::theme::ThemePalette::for_mode(mode).surface_raised)
}

/// Shared dialog action height and shape; the caller retains action semantics.
pub(crate) fn dialog_button(label: impl Into<egui::WidgetText>) -> egui::Button<'static> {
    egui::Button::new(label)
        .min_size(Vec2::new(0.0, CONTROL_HEIGHT))
        .corner_radius(RADIUS_CONTROL)
}

/// Primary dialog action with the palette's contrast-tested accent and text.
pub(crate) fn dialog_primary_button(
    label: impl Into<String>,
    palette: crate::theme::ThemePalette,
) -> impl egui::Widget {
    let label = label.into();
    move |ui: &mut egui::Ui| {
        let response = ui
            .scope(|ui| {
                // A Button::fill override hides all state fills. Scope the palette
                // instead, retaining egui's keyboard activation and accessibility.
                let enabled = ui.is_enabled();
                let visuals = ui.visuals_mut();
                visuals.override_text_color = Some(if enabled {
                    palette.accent_text
                } else {
                    palette.disabled_text
                });
                visuals.widgets.inactive.weak_bg_fill = if enabled {
                    palette.accent_control
                } else {
                    palette.disabled_bg
                };
                visuals.widgets.hovered.weak_bg_fill = palette.accent_control_hover;
                visuals.widgets.active.weak_bg_fill = palette.accent_dark;
                ui.add(dialog_button(egui::RichText::new(label).color(
                    if enabled {
                        palette.accent_text
                    } else {
                        palette.disabled_text
                    },
                )))
            })
            .inner;
        paint_focus_ring(ui, &response, palette);
        response
    }
}

/// The same two-point accent ring around custom controls on every platform.
pub(crate) fn paint_focus_ring(
    ui: &egui::Ui,
    response: &egui::Response,
    palette: crate::theme::ThemePalette,
) {
    if response.enabled() && response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(2.0),
            RADIUS_CONTROL,
            Stroke::new(2.0, palette.accent_dark),
            StrokeKind::Outside,
        );
    }
}

fn control_fill(response: &egui::Response, palette: crate::theme::ThemePalette) -> Color32 {
    if !response.enabled() {
        palette.disabled_bg
    } else if response.is_pointer_button_down_on() {
        palette.pressed
    } else if response.hovered() {
        palette.hover
    } else {
        palette.surface
    }
}

/// A single dialog footer row, with the first action placed at the right edge.
pub(crate) fn dialog_actions(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    ui.add_space(SPACE_4);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = SPACE_2;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), contents);
    });
}

/// Selectable error text with a palette-coloured warning glyph on every platform.
pub(crate) fn dialog_error(ui: &mut egui::Ui, palette: crate::theme::ThemePalette, text: &str) {
    ui.horizontal_top(|ui| {
        let (icon, _) = ui.allocate_exact_size(Vec2::splat(FONT_SMALL), egui::Sense::hover());
        paint_glyph(ui.painter(), icon, Glyph::Warning, palette.error_text);
        ui.add(
            egui::Label::new(
                egui::RichText::new(text)
                    .size(FONT_SMALL)
                    .color(palette.error_text),
            )
            .wrap()
            .selectable(true),
        );
    });
}

/// Viewport axis colours. Shell text uses the palette's contrast-tested variants.
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
            for (axis, color) in ["X", "Y", "Z"].into_iter().zip(palette.axis_text) {
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
                        .layout_no_wrap(displayed.clone(), font.clone(), palette.axis_text[axis])
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
                            .text_color(palette.axis_text[axis])
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
        control_fill(&response, palette),
        Stroke::new(
            1.0,
            if response.hovered() || response.is_pointer_button_down_on() {
                palette.accent_border
            } else {
                palette.border
            },
        ),
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
        if !response.enabled() {
            palette.disabled_bg
        } else if response.is_pointer_button_down_on() {
            palette.accent_dark
        } else if response.hovered() {
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
            if !response.enabled() {
                palette.disabled_text
            } else if (i == 1) == object {
                palette.accent_text
            } else {
                palette.muted
            },
        );
    }
    paint_focus_ring(ui, &response, palette);
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
    palette: crate::theme::ThemePalette,
    accessible_label: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::Button, response.enabled(), accessible_label)
    });
    if !response.enabled() || response.hovered() || response.is_pointer_button_down_on() {
        ui.painter()
            .rect_filled(rect, RADIUS_CONTROL, control_fill(&response, palette));
    }
    paint_glyph(
        ui.painter(),
        Rect::from_center_size(rect.center(), Vec2::splat(16.0)),
        glyph,
        if response.enabled() {
            color
        } else {
            palette.disabled_text
        },
    );
    paint_focus_ring(ui, &response, palette);
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{self, ThemeMode, ThemePalette};

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn gallery(
        ctx: &egui::Context,
        palette: ThemePalette,
        enabled: bool,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(640.0, 800.0))),
                events,
                ..Default::default()
            },
            |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    resize_basis_toggle(ui, false, palette);
                    glyph_button(ui, Glyph::Visible, palette.muted, palette, "Visibility");
                    section(ui, palette, "Transform", |ui| {
                        let mut fields = ["1".into(), "2".into(), "3".into()];
                        transform_field_grid(ui, palette, "Position", &mut fields, Some("mm"));
                        inspector_apply_button(ui, "Apply", "Apply position");
                    });
                    dialog_actions(ui, |ui| {
                        ui.add(dialog_primary_button("Save", palette));
                        ui.add(dialog_button("Cancel"));
                    });
                });
            },
        );
        output.textures_delta.clear();
        output
    }

    #[test]
    fn every_custom_control_family_is_accessible_and_reachable_with_tab() {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            theme::apply(&ctx, mode);
            let p = ThemePalette::for_mode(mode);
            let _ = gallery(&ctx, p, true, vec![]);
            let output = gallery(&ctx, p, true, vec![]);
            let nodes = output
                .platform_output
                .accesskit_update
                .expect("accessibility tree")
                .nodes;
            let mut expected = Vec::new();
            for label in [
                "Resize relative to",
                "Visibility",
                "Apply",
                "Save",
                "Cancel",
            ] {
                let (id, node) = nodes
                    .iter()
                    .find(|(_, node)| {
                        node.label().is_some_and(|name| name.starts_with(label))
                            && node.role() == egui::Role::Button
                    })
                    .unwrap_or_else(|| panic!("missing button: {label}"));
                assert!(
                    node.supports_action(egui::accesskit::Action::Focus),
                    "{label}"
                );
                assert!(!node.is_disabled(), "{label}");
                expected.push(*id);
            }
            let fields: Vec<_> = nodes
                .iter()
                .filter(|(_, node)| node.role() == egui::Role::TextInput)
                .collect();
            assert_eq!(
                fields.len(),
                3,
                "section/grid must retain three accessible text inputs"
            );
            for (id, field) in fields {
                assert!(field.supports_action(egui::accesskit::Action::Focus));
                expected.push(*id);
            }
            let mut visited = std::collections::HashSet::new();
            for _ in 0..expected.len() + 2 {
                let _ = gallery(&ctx, p, true, vec![key(egui::Key::Tab)]);
                // egui resolves Tab order at the end of the pass.
                let output = gallery(&ctx, p, true, vec![]);
                if let Some(focused) = ctx.memory(|memory| memory.focused()) {
                    visited.insert(egui::accesskit::NodeId(focused.value()));
                    assert!(
                        output.shapes.iter().any(|shape| {
                            matches!(&shape.shape, egui::Shape::Rect(rect)
                            if rect.stroke == Stroke::new(2.0, p.accent_dark))
                        }),
                        "{mode:?}: focused control {focused:?} needs a two-point accent ring"
                    );
                }
            }
            for id in expected {
                assert!(visited.contains(&id), "{mode:?}: Tab missed control {id:?}");
            }
        }
    }

    #[test]
    fn disabled_custom_controls_report_disabled_and_skip_tab() {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            theme::apply(&ctx, mode);
            let p = ThemePalette::for_mode(mode);
            let _ = gallery(&ctx, p, false, vec![]);
            let output = gallery(&ctx, p, false, vec![key(egui::Key::Tab)]);
            let nodes = output
                .platform_output
                .accesskit_update
                .expect("accessibility tree")
                .nodes;
            let controls: Vec<_> = nodes
                .iter()
                .filter(|(_, node)| {
                    matches!(node.role(), egui::Role::Button | egui::Role::TextInput)
                })
                .collect();
            assert_eq!(controls.len(), 8);
            assert!(controls.iter().all(|(_, node)| node.is_disabled()));
            assert_eq!(ctx.memory(|memory| memory.focused()), None);
        }
    }

    #[test]
    fn painted_buttons_show_hover_press_and_keyboard_activation() {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            let p = ThemePalette::for_mode(mode);
            for family in ["glyph", "resize", "primary"] {
                let ctx = egui::Context::default();
                theme::apply(&ctx, mode);
                let mut response = None;
                let mut render = |events| {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            events,
                            ..Default::default()
                        },
                        |ui| {
                            response = Some(match family {
                                "glyph" => {
                                    glyph_button(ui, Glyph::Visible, p.muted, p, "Visibility")
                                }
                                "resize" => resize_basis_toggle(ui, false, p),
                                _ => ui.add(dialog_primary_button("Save", p)),
                            });
                        },
                    );
                    output.textures_delta.clear();
                    output
                };
                let _ = render(vec![]);
                // Match the fixed gallery origin without relying on font metrics.
                let point = Pos2::new(20.0, 20.0);
                let _ = render(vec![egui::Event::PointerMoved(point)]);
                let hovered = render(vec![egui::Event::PointerMoved(point)]);
                let expected_hover = if family == "primary" {
                    p.accent_control_hover
                } else {
                    p.hover
                };
                assert!(
                    has_fill(&hovered, expected_hover),
                    "{mode:?} {family}: hover"
                );
                let pressed = render(vec![egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }]);
                let expected_press = if family == "primary" {
                    p.accent_dark
                } else {
                    p.pressed
                };
                assert!(
                    has_fill(&pressed, expected_press),
                    "{mode:?} {family}: pressed"
                );
                let _ = render(vec![egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }]);
                let _ = render(vec![key(egui::Key::Tab)]);
                let _ = render(vec![key(egui::Key::Enter)]);
                assert!(
                    response.as_ref().is_some_and(egui::Response::clicked),
                    "{family}: Enter activation"
                );
            }
        }
    }

    fn has_fill(output: &egui::FullOutput, color: Color32) -> bool {
        output
            .shapes
            .iter()
            .any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == color))
    }
}
