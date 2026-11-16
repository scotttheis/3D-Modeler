//! Short notices that appear over the viewport and fade away by themselves.
//!
//! First used when Merge groups objects that were not touching (OF-38); later
//! items reuse it for any brief, non-blocking message. A toast never takes
//! input and never needs to be dismissed.

use eframe::egui::{self, Align2, Color32, CornerRadius, Margin, RichText, Stroke, Vec2};

use crate::theme::ThemePalette;

/// How long a toast stays on screen, in seconds (about four, per D8).
pub(crate) const TOAST_SECONDS: f64 = 4.0;
/// The last part of that time, in seconds, over which it fades out.
const FADE_SECONDS: f64 = 0.4;
/// Gap between the top of the window and the toast: below the ribbon.
const TOP_OFFSET: f32 = 140.0;

/// One notice and the time it appeared.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Toast {
    message: String,
    shown_at: f64,
}

impl Toast {
    /// A toast showing `message`, appearing at egui time `now` (seconds).
    pub(crate) fn new(message: impl Into<String>, now: f64) -> Self {
        Self {
            message: message.into(),
            shown_at: now,
        }
    }

    /// The text shown.
    #[cfg(test)]
    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    /// Opacity at time `now`: 1 while showing, easing to 0 at the end, and
    /// `None` once the toast has expired.
    pub(crate) fn opacity(&self, now: f64) -> Option<f32> {
        let age = (now - self.shown_at).max(0.0);
        if age >= TOAST_SECONDS {
            return None;
        }
        let remaining = TOAST_SECONDS - age;
        Some((remaining / FADE_SECONDS).min(1.0) as f32)
    }

    /// Paints the toast centred near the top of the window. Returns `false`
    /// once it has expired, so the caller can drop it.
    pub(crate) fn show(&self, ctx: &egui::Context, palette: &ThemePalette) -> bool {
        let now = ctx.input(|input| input.time);
        let Some(opacity) = self.opacity(now) else {
            return false;
        };
        let fade = |color: Color32| color.gamma_multiply(opacity);
        egui::Area::new(egui::Id::unique("modeler3d-toast"))
            .order(egui::Order::Tooltip)
            .interactable(false)
            .anchor(Align2::CENTER_TOP, Vec2::new(0.0, TOP_OFFSET))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(fade(palette.surface_raised))
                    .stroke(Stroke::new(1.0, fade(palette.border)))
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(Margin::symmetric(16, 10))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 2],
                        blur: 10,
                        spread: 0,
                        color: fade(Color32::from_black_alpha(40)),
                    })
                    .show(ui, |ui| {
                        ui.label(RichText::new(&self.message).color(fade(palette.text)));
                    });
            });
        let remaining = TOAST_SECONDS - (now - self.shown_at);
        let next = if remaining <= FADE_SECONDS {
            1.0 / 30.0
        } else {
            remaining - FADE_SECONDS
        };
        ctx.request_repaint_after(std::time::Duration::from_secs_f64(next.max(0.0)));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_toast_shows_for_about_four_seconds_then_fades_and_expires() {
        let toast = Toast::new("Hello", 10.0);
        assert_eq!(toast.message(), "Hello");
        assert_eq!(toast.opacity(10.0), Some(1.0));
        assert_eq!(toast.opacity(13.0), Some(1.0));
        let fading = toast.opacity(13.8).unwrap_or_default();
        assert!(fading > 0.0 && fading < 1.0, "{fading}");
        assert_eq!(toast.opacity(14.0), None);
        assert_eq!(toast.opacity(20.0), None);
    }

    #[test]
    fn showing_a_toast_paints_it_until_it_expires() {
        let ctx = egui::Context::default();
        let palette = ThemePalette::light();
        let toast = Toast::new("Grouped", 0.0);
        let mut shown = Vec::new();
        for time in [0.0, 2.0, 3.9, 4.1] {
            let input = egui::RawInput {
                time: Some(time),
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                shown.push(toast.show(ui.ctx(), &palette));
            });
            output.textures_delta.clear();
        }
        assert_eq!(shown, vec![true, true, true, false]);
    }
}
