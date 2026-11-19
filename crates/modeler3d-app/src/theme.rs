//! Original 3D Modeler theme tokens and platform UI-font setup.

use std::{fs, path::PathBuf, sync::Arc};

use eframe::egui::{
    Color32, Context, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Visuals,
};
use serde::{Deserialize, Serialize};

use crate::chrome::{
    FONT_BODY, FONT_CAPTION, FONT_HERO, FONT_SMALL, RADIUS_CONTROL, RADIUS_SURFACE, SPACE_2,
};
/// The two supported shell themes.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum ThemeMode {
    /// Light 3D Modeler theme.
    Light,
    /// Dark reference theme, which is the original application default.
    #[default]
    Dark,
}

/// The shell size chosen in Settings (owner request OF-20).
///
/// Big doubles every shell surface — text, icons, bars, panels and dialogs —
/// through egui's zoom factor. The 3D viewport keeps its physical resolution
/// because it is sized from `pixels_per_point`.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum UiSize {
    /// The original size.
    #[default]
    Normal,
    /// Twice the original size.
    Big,
}

impl UiSize {
    /// Returns the stable settings-file spelling.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Big => "big",
        }
    }

    /// Parses a persisted size; unknown text is `None`.
    #[must_use]
    pub fn from_key(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "normal" => Some(Self::Normal),
            "big" => Some(Self::Big),
            _ => None,
        }
    }

    /// The egui zoom factor for this size.
    #[must_use]
    pub const fn zoom_factor(self) -> f32 {
        match self {
            Self::Normal => 1.0,
            Self::Big => 2.0,
        }
    }
}

/// Matches the Windows 11 caption to the menu row when the app theme changes.
#[cfg(windows)]
pub(crate) fn apply_native_caption(ctx: &Context, frame: &eframe::Frame, mode: ThemeMode) {
    use raw_window_handle::HasWindowHandle;
    let key = eframe::egui::Id::unique("native-title-bar-theme");
    if ctx.data(|data| data.get_temp::<ThemeMode>(key)) == Some(mode) {
        return;
    }
    if let Ok(handle) = frame.window_handle() {
        let palette = ThemePalette::for_mode(mode);
        let [r, g, b, _] = palette.navigation.to_srgba_unmultiplied();
        let [tr, tg, tb, _] = palette.text.to_srgba_unmultiplied();
        if let Err(error) =
            modeler3d_platform::set_title_bar_colors(handle, [r, g, b], [tr, tg, tb])
        {
            crate::diagnostics::record_error("native title bar", error);
        }
        ctx.data_mut(|data| data.insert_temp(key, mode));
    }
}

/// Applies the shell size, touching egui only when it changes.
pub fn apply_size(ctx: &Context, size: UiSize) {
    let zoom = size.zoom_factor();
    if (ctx.zoom_factor() - zoom).abs() > f32::EPSILON {
        ctx.set_zoom_factor(zoom);
    }
}

impl ThemeMode {
    /// Returns the stable settings-file spelling used by the desktop adapter.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    /// Parses a persisted theme, falling back to the original dark default.
    #[must_use]
    pub fn from_key(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "dark" => Self::Dark,
            "light" => Self::Light,
            _ => Self::default(),
        }
    }
}

impl ThemeMode {
    /// Toggles between the supported themes.
    #[must_use]
    pub const fn toggled(self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::Light,
        }
    }
}

/// Palette values shared by ribbon, dialogs, object panel, and viewport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemePalette {
    /// Primary accent.
    pub accent: Color32,
    /// Accessible fill for controls that place text on the accent.
    pub accent_control: Color32,
    /// Hover fill for controls that place text on the accent.
    pub accent_control_hover: Color32,
    /// Dark accent for pressed states.
    pub accent_dark: Color32,
    /// Accent used by hovered controls.
    pub accent_hover: Color32,
    /// Accent outline used by checked and focused controls.
    pub accent_border: Color32,
    /// Text placed on a filled accent control.
    pub accent_text: Color32,
    /// Window background.
    pub background: Color32,
    /// Panel surface.
    pub panel: Color32,
    /// Base surface for buttons and cards.
    pub surface: Color32,
    /// Alternate surface for cards and text inputs.
    pub surface_alt: Color32,
    /// Raised navigation and selected surfaces.
    pub surface_raised: Color32,
    /// Primary text.
    pub text: Color32,
    /// Muted text.
    pub muted: Color32,
    /// Warning text, readable on this theme's panels (OF-39).
    pub warning_text: Color32,
    /// Error text, readable on this theme's panels (OF-39).
    pub error_text: Color32,
    /// Dividers.
    pub divider: Color32,
    /// Control and card border.
    pub border: Color32,
    /// Disabled control surface.
    pub disabled_bg: Color32,
    /// Disabled control text.
    pub disabled_text: Color32,
    /// Hover surface.
    pub hover: Color32,
    /// Pressed surface.
    pub pressed: Color32,
    /// Navigation surface.
    pub navigation: Color32,
    /// Hovered navigation surface.
    pub navigation_hover: Color32,
    /// Viewport surface.
    pub viewport: Color32,
    /// Build-plane line color.
    pub grid: Color32,
}

impl ThemePalette {
    /// Returns the original light palette from `L:\Builder3D\src\style.py`.
    #[must_use]
    pub fn light() -> Self {
        Self {
            accent: Color32::from_rgb(0x5B, 0x6F, 0xEA),
            // The source palette's white-on-accent label is 4.28:1. These
            // close indigo variants keep the control recognizable at 4.7:1.
            accent_control: Color32::from_rgb(0x56, 0x69, 0xDD),
            accent_control_hover: Color32::from_rgb(0x60, 0x6F, 0xCB),
            accent_dark: Color32::from_rgb(0x40, 0x52, 0xB5),
            accent_hover: Color32::from_rgb(0x6B, 0x7D, 0xED),
            accent_border: Color32::from_rgb(0x73, 0x84, 0xEF),
            accent_text: Color32::WHITE,
            background: Color32::from_rgb(0xF4, 0xF6, 0xFA),
            panel: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            surface_alt: Color32::from_rgb(0xFC, 0xFC, 0xFD),
            surface_raised: Color32::from_rgb(0xE8, 0xEA, 0xF5),
            text: Color32::from_rgb(0x1D, 0x24, 0x33),
            muted: Color32::from_rgb(0x66, 0x70, 0x85),
            warning_text: Color32::from_rgb(0x9A, 0x62, 0x00),
            error_text: Color32::from_rgb(0xB4, 0x23, 0x18),
            divider: Color32::from_rgb(0xDD, 0xE2, 0xEA),
            border: Color32::from_rgb(0xD5, 0xDA, 0xE4),
            disabled_bg: Color32::from_rgb(0xF4, 0xF5, 0xF7),
            disabled_text: Color32::from_rgb(0xA0, 0xA8, 0xB8),
            // A visible light-grey tile, as on 3D Builder's toolbar.
            hover: Color32::from_rgb(0xE4, 0xE7, 0xF0),
            pressed: Color32::from_rgb(0xE1, 0xE6, 0xFF),
            navigation: Color32::from_rgb(0xDC, 0xDF, 0xEE),
            navigation_hover: Color32::from_rgb(0xC9, 0xCF, 0xE6),
            viewport: Color32::from_rgb(0xF0, 0xF0, 0xF0),
            grid: Color32::from_rgba_unmultiplied(0xAD, 0xAD, 0xAD, 0x14),
        }
    }

    /// Returns the original dark palette from `L:\Builder3D\src\style.py`.
    #[must_use]
    pub fn dark() -> Self {
        Self {
            accent: Color32::from_rgb(0x58, 0xD9, 0xFF),
            accent_control: Color32::from_rgb(0x58, 0xD9, 0xFF),
            accent_control_hover: Color32::from_rgb(0x78, 0xE5, 0xFF),
            accent_dark: Color32::from_rgb(0xC1, 0xB4, 0xFF),
            accent_hover: Color32::from_rgb(0x78, 0xE5, 0xFF),
            accent_border: Color32::from_rgb(0x8D, 0xEA, 0xFF),
            accent_text: Color32::from_rgb(0x06, 0x14, 0x23),
            background: Color32::from_rgb(0x07, 0x13, 0x27),
            panel: Color32::from_rgb(0x0D, 0x1D, 0x3A),
            surface: Color32::from_rgb(0x10, 0x26, 0x44),
            surface_alt: Color32::from_rgb(0x0A, 0x18, 0x31),
            surface_raised: Color32::from_rgb(0x15, 0x31, 0x55),
            text: Color32::from_rgb(0xEA, 0xF7, 0xFF),
            muted: Color32::from_rgb(0x9D, 0xB5, 0xD6),
            warning_text: Color32::from_rgb(0xF5, 0xC1, 0x5C),
            error_text: Color32::from_rgb(0xFF, 0x9C, 0x92),
            divider: Color32::from_rgb(0x27, 0x49, 0x76),
            border: Color32::from_rgb(0x31, 0x59, 0x85),
            disabled_bg: Color32::from_rgb(0x0B, 0x19, 0x30),
            disabled_text: Color32::from_rgb(0x66, 0x81, 0xA6),
            hover: Color32::from_rgb(0x1B, 0x3A, 0x66),
            pressed: Color32::from_rgb(0x1A, 0x4B, 0x82),
            navigation: Color32::from_rgb(0x15, 0x31, 0x55),
            // Darker than the tab bar, as 3D Builder's tab hover is.
            navigation_hover: Color32::from_rgb(0x0B, 0x1E, 0x3A),
            // A neutral grey canvas, somewhat darker than the light theme's
            // (D4, OF-35); the viewport crate owns the shade so the floor
            // fades into exactly this colour.
            viewport: {
                let [r, g, b, _] = modeler3d_viewport::ViewportTheme::Dark.background_srgb8();
                Color32::from_rgb(r, g, b)
            },
            grid: Color32::from_rgba_unmultiplied(0x40, 0xB8, 0xDB, 0x1F),
        }
    }

    /// Returns the palette for a mode.
    #[must_use]
    pub fn for_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
        }
    }
}

/// Applies the shell palette to egui's baseline visuals.
pub fn apply(ctx: &Context, mode: ThemeMode) {
    let palette = ThemePalette::for_mode(mode);
    let mut visuals = match mode {
        ThemeMode::Light => Visuals::light(),
        ThemeMode::Dark => Visuals::dark(),
    };
    visuals.override_text_color = Some(palette.text);
    visuals.weak_text_color = Some(palette.muted);
    visuals.window_fill = palette.panel;
    visuals.window_stroke = eframe::egui::Stroke::new(1.0, palette.divider);
    visuals.window_corner_radius = eframe::egui::CornerRadius::same(RADIUS_SURFACE as u8);
    visuals.menu_corner_radius = eframe::egui::CornerRadius::same(RADIUS_SURFACE as u8);
    visuals.panel_fill = palette.background;
    visuals.faint_bg_color = palette.hover;
    visuals.extreme_bg_color = palette.viewport;
    visuals.text_edit_bg_color = Some(palette.surface_alt);
    visuals.code_bg_color = palette.surface_alt;
    visuals.selection.bg_fill = palette.pressed;
    visuals.selection.stroke.color = palette.accent_dark;
    visuals.widgets.noninteractive.bg_fill = palette.panel;
    visuals.widgets.noninteractive.fg_stroke.color = palette.text;
    visuals.widgets.noninteractive.bg_stroke.color = palette.divider;
    visuals.widgets.noninteractive.weak_bg_fill = palette.surface_alt;
    // Slider rails and checkbox wells use this fill; on the panel color
    // they vanished (white on white), leaving only floating handles.
    visuals.widgets.inactive.bg_fill = palette.surface_raised;
    visuals.slider_trailing_fill = true;
    visuals.widgets.inactive.weak_bg_fill = palette.surface;
    visuals.widgets.inactive.bg_stroke.color = palette.border;
    visuals.widgets.inactive.fg_stroke.color = palette.text;
    visuals.widgets.hovered.bg_fill = palette.hover;
    visuals.widgets.hovered.weak_bg_fill = palette.navigation_hover;
    visuals.widgets.hovered.bg_stroke.color = palette.accent_border;
    visuals.widgets.hovered.fg_stroke.color = palette.accent_dark;
    visuals.widgets.active.bg_fill = palette.pressed;
    visuals.widgets.active.weak_bg_fill = palette.pressed;
    visuals.widgets.active.bg_stroke.color = palette.accent;
    visuals.widgets.active.fg_stroke.color = palette.accent_dark;
    visuals.widgets.open.bg_fill = palette.surface_raised;
    // egui uses this for the active window's title bar, overriding its frame fill.
    visuals.widgets.open.weak_bg_fill = palette.surface_raised;
    visuals.widgets.open.fg_stroke.color = palette.text;
    // One control language: every state shares a radius, and resting controls
    // carry a hairline outline so fields and buttons read as controls instead
    // of loose text on the panel.
    let control_radius = eframe::egui::CornerRadius::same(RADIUS_CONTROL as u8);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = control_radius;
    }
    visuals.widgets.inactive.bg_stroke = eframe::egui::Stroke::new(1.0, palette.border);
    visuals.widgets.hovered.bg_stroke = eframe::egui::Stroke::new(1.0, palette.accent_border);
    visuals.widgets.active.bg_stroke = eframe::egui::Stroke::new(1.0, palette.accent);
    visuals.widgets.open.bg_stroke = eframe::egui::Stroke::new(1.0, palette.accent_border);
    visuals.selection.stroke.width = 1.0;
    visuals.popup_shadow = floating_shadow(mode);
    visuals.window_shadow = floating_shadow(mode);
    ctx.set_visuals(visuals);
}

/// Elevation for surfaces that float above the workspace: menus, dialogs,
/// and the transform strip. Dark surfaces need a denser shadow to separate.
#[must_use]
pub fn floating_shadow(mode: ThemeMode) -> eframe::egui::Shadow {
    eframe::egui::Shadow {
        offset: [0, 6],
        blur: 22,
        spread: 0,
        color: match mode {
            ThemeMode::Light => Color32::from_black_alpha(46),
            ThemeMode::Dark => Color32::from_black_alpha(120),
        },
    }
}

/// Configures the native platform UI font and reference control typography.
pub fn configure_fonts(ctx: &Context) {
    let mut definitions = FontDefinitions::default();
    if let Some(bytes) = read_font(platform_ui_font_candidates()) {
        let name = String::from("modeler3d-platform-ui");
        definitions
            .font_data
            .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
        if let Some(family) = definitions.families.get_mut(&FontFamily::Proportional) {
            family.insert(0, name);
        }
    }
    ctx.set_fonts(definitions);

    for theme in [eframe::egui::Theme::Light, eframe::egui::Theme::Dark] {
        let mut style = (*ctx.style_of(theme)).clone();
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(FONT_BODY));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(FONT_SMALL));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(FONT_CAPTION));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(FONT_HERO));
        style.spacing.item_spacing = eframe::egui::vec2(6.0, 5.0);
        style.spacing.button_padding = eframe::egui::vec2(SPACE_2, 6.0);
        ctx.set_style_of(theme, style);
    }
}

fn read_font(candidates: Vec<PathBuf>) -> Option<Vec<u8>> {
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .and_then(|path| fs::read(path).ok())
        .filter(|bytes| !bytes.is_empty())
}

#[cfg(target_os = "windows")]
fn platform_ui_font_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(windows_dir) = std::env::var_os("WINDIR") {
        paths.push(PathBuf::from(windows_dir).join("Fonts").join("segoeui.ttf"));
    }
    paths
}

#[cfg(target_os = "macos")]
fn platform_ui_font_candidates() -> Vec<PathBuf> {
    [
        "/System/Library/Fonts/SFNS.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

#[cfg(target_os = "linux")]
fn platform_ui_font_candidates() -> Vec<PathBuf> {
    [
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/opentype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn platform_ui_font_candidates() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_guide_light_palette_is_stable() {
        let palette = ThemePalette::light();
        assert_eq!(palette.accent, Color32::from_rgb(0x5B, 0x6F, 0xEA));
        assert_eq!(palette.accent_control, Color32::from_rgb(0x56, 0x69, 0xDD));
        assert_eq!(palette.background, Color32::from_rgb(0xF4, 0xF6, 0xFA));
        assert_eq!(palette.panel, Color32::WHITE);
        assert_eq!(ThemeMode::Light.toggled(), ThemeMode::Dark);
    }

    #[test]
    fn primary_text_and_accent_control_colors_meet_contrast_targets() {
        for palette in [ThemePalette::light(), ThemePalette::dark()] {
            assert!(contrast_ratio(palette.text, palette.panel) >= 4.5);
            assert!(contrast_ratio(palette.text, palette.background) >= 4.5);
            assert!(contrast_ratio(palette.muted, palette.panel) >= 4.5);
            assert!(contrast_ratio(palette.muted, palette.background) >= 4.5);
            assert!(contrast_ratio(palette.accent_text, palette.accent_control) >= 4.5);
            assert!(contrast_ratio(palette.accent_text, palette.accent_control_hover) >= 4.5);
            assert!(contrast_ratio(palette.accent_border, palette.panel) >= 3.0);
        }
    }

    fn contrast_ratio(foreground: Color32, background: Color32) -> f64 {
        let foreground = relative_luminance(foreground);
        let background = relative_luminance(background);
        let (lighter, darker) = if foreground >= background {
            (foreground, background)
        } else {
            (background, foreground)
        };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn relative_luminance(color: Color32) -> f64 {
        fn linear_channel(channel: u8) -> f64 {
            let channel = f64::from(channel) / 255.0;
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        }

        (0.2126 * linear_channel(color.r()))
            + (0.7152 * linear_channel(color.g()))
            + (0.0722 * linear_channel(color.b()))
    }
}
