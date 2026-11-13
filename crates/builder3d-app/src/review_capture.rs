//! Deterministic review captures of the real shell.
//!
//! Visual work needs a current picture of the application in a known state.
//! Setting `BUILDER3D_REVIEW_CAPTURE` to a PNG path starts the ordinary native
//! window in a named state, waits for the viewport to settle, saves the
//! window's own framebuffer, and exits. No clicks or key presses are
//! simulated (an optional resting pointer position only reveals hover
//! states), the desktop is never photographed, and user settings are neither
//! read nor written, so a capture is repeatable and safe to run while the
//! machine is in use.
//!
//! | Variable | Values | Default |
//! |---|---|---|
//! | `BUILDER3D_REVIEW_CAPTURE` | output PNG path | harness off |
//! | `BUILDER3D_REVIEW_STATE` | `welcome`, `empty`, `move`, `rotate`, `resize`, `resize-drag`, `multi`, `merge`, `unselected`, `material`, `settings`, `save`, `export` | `move` |
//! | `BUILDER3D_REVIEW_THEME` | `light`, `dark` | `dark` |
//! | `BUILDER3D_REVIEW_SIZE` | `normal`, `big` (Settings > Size) | `normal` |
//! | `BUILDER3D_REVIEW_TAB` | a ribbon tab label such as `Object` | `Insert` |
//! | `BUILDER3D_REVIEW_VIEW` | comma list of view toggles to turn on: `wireframe`, `xray` | none |
//! | `BUILDER3D_REVIEW_POINTER` | `x,y` in window points: rests the pointer there to show a hover state (no button is pressed) | pointer outside |
//! | `BUILDER3D_REVIEW_INTERACTIVE` | `1`: set up the state with isolated settings and stay open for hands-on checks instead of capturing | off |
//! | `BUILDER3D_REVIEW_CAMERA` | `yaw,pitch,distance` in degrees and mm, applied after auto-framing; distance `0` keeps the framed distance | framed view |
//!
//! A capture shows what was painted. It is engineering evidence for layout and
//! styling; it does not exercise pointer behaviour and is not owner approval.

use std::{
    env,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use eframe::egui;

use crate::theme::{ThemeMode, UiSize};

/// Frames to wait before requesting the framebuffer, so fonts, artwork
/// textures, auto-framing, and the first viewport render have all settled.
const SETTLE_FRAMES: u32 = 45;
/// Frames after which a capture that never arrived is reported as a failure.
const TIMEOUT_FRAMES: u32 = 600;

/// The named application state a review capture starts in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReviewState {
    /// The welcome surface with no scene.
    Welcome,
    /// The scene surface with no objects.
    Empty,
    /// One selected cube with the Move handles.
    Move,
    /// One selected cube with the Rotate handles.
    Rotate,
    /// One selected cube with the Resize handles.
    Resize,
    /// The Resize state with the +X face handle held mid-drag. The drag is
    /// staged through the shell's own gesture code each frame; no pointer
    /// input is simulated and nothing is committed.
    ResizeDrag,
    /// Several primitives with one selected, for list and inspector density.
    Multi,
    /// A red cube (top of the Items list) and an overlapping blue cube,
    /// selected blue-first and merged.
    Merge,
    /// One cube with nothing selected.
    Unselected,
    /// One selected cube with the Material dialog open.
    Material,
    /// One selected cube with Settings open.
    Settings,
    /// One selected cube with the Save dialog open.
    Save,
    /// One selected cube with the Export dialog open.
    Export,
}

impl ReviewState {
    fn from_key(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "welcome" => Some(Self::Welcome),
            "empty" => Some(Self::Empty),
            "move" => Some(Self::Move),
            "rotate" => Some(Self::Rotate),
            "resize" => Some(Self::Resize),
            "resize-drag" => Some(Self::ResizeDrag),
            "multi" => Some(Self::Multi),
            "merge" => Some(Self::Merge),
            "unselected" => Some(Self::Unselected),
            "material" => Some(Self::Material),
            "settings" => Some(Self::Settings),
            "save" => Some(Self::Save),
            "export" => Some(Self::Export),
            _ => None,
        }
    }
}

/// What the shell should do after a review frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReviewProgress {
    /// Keep painting.
    Pending,
    /// The capture was written, or failed; the process should end.
    Finished,
}

/// One requested capture and its progress.
#[derive(Debug)]
pub(crate) struct ReviewCapture {
    output: PathBuf,
    state: ReviewState,
    theme: ThemeMode,
    size: UiSize,
    tab: Option<String>,
    camera: Option<[f32; 3]>,
    pointer: Option<[f32; 2]>,
    view: Vec<String>,
    interactive: bool,
    frames: u32,
    requested: bool,
    finished: Arc<AtomicBool>,
}

impl ReviewCapture {
    /// Reads the capture request from the environment. Returns `None` in
    /// ordinary use, and for an unrecognised state so a typo cannot silently
    /// capture the wrong screen.
    pub(crate) fn from_environment() -> Option<Self> {
        let output = PathBuf::from(env::var_os("BUILDER3D_REVIEW_CAPTURE")?);
        if output.as_os_str().is_empty() {
            return None;
        }
        let state = match env::var("BUILDER3D_REVIEW_STATE") {
            Ok(value) => match ReviewState::from_key(&value) {
                Some(state) => state,
                None => {
                    eprintln!("review capture: unknown BUILDER3D_REVIEW_STATE `{value}`");
                    return None;
                }
            },
            Err(_) => ReviewState::Move,
        };
        let theme = env::var("BUILDER3D_REVIEW_THEME")
            .map(|value| ThemeMode::from_key(&value))
            .unwrap_or_default();
        let size = env::var("BUILDER3D_REVIEW_SIZE")
            .ok()
            .and_then(|value| UiSize::from_key(&value))
            .unwrap_or_default();
        let tab = env::var("BUILDER3D_REVIEW_TAB")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let camera = match env::var("BUILDER3D_REVIEW_CAMERA") {
            Ok(value) if !value.trim().is_empty() => match parse_camera(&value) {
                Some(camera) => Some(camera),
                None => {
                    eprintln!(
                        "review capture: BUILDER3D_REVIEW_CAMERA must be `yaw,pitch,distance`"
                    );
                    return None;
                }
            },
            _ => None,
        };
        let pointer = match env::var("BUILDER3D_REVIEW_POINTER") {
            Ok(value) if !value.trim().is_empty() => match parse_numbers::<2>(&value) {
                Some(pointer) => Some(pointer),
                None => {
                    eprintln!("review capture: BUILDER3D_REVIEW_POINTER must be `x,y`");
                    return None;
                }
            },
            _ => None,
        };
        let view = env::var("BUILDER3D_REVIEW_VIEW")
            .map(|value| {
                value
                    .split(',')
                    .map(|part| part.trim().to_ascii_lowercase())
                    .filter(|part| !part.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        Some(Self {
            output,
            state,
            theme,
            size,
            tab,
            camera,
            pointer,
            view,
            interactive: env::var_os("BUILDER3D_REVIEW_INTERACTIVE")
                .is_some_and(|value| value == "1"),
            frames: 0,
            requested: false,
            finished: Arc::new(AtomicBool::new(false)),
        })
    }

    pub(crate) const fn state(&self) -> ReviewState {
        self.state
    }

    pub(crate) const fn theme(&self) -> ThemeMode {
        self.theme
    }

    pub(crate) const fn size(&self) -> UiSize {
        self.size
    }

    pub(crate) fn tab(&self) -> Option<&str> {
        self.tab.as_deref()
    }

    /// View toggles (`wireframe`, `xray`) to switch on.
    pub(crate) fn view_toggles(&self) -> &[String] {
        &self.view
    }

    /// Where the pointer rests, in window points, to reveal a hover state.
    pub(crate) const fn pointer(&self) -> Option<[f32; 2]> {
        self.pointer
    }

    /// The `[yaw, pitch, distance]` camera pose to apply after auto-framing.
    pub(crate) const fn camera(&self) -> Option<[f32; 3]> {
        self.camera
    }

    /// A settings path beside the capture, so the harness never reads or
    /// overwrites the user's real preferences.
    pub(crate) fn isolated_settings_path(&self) -> PathBuf {
        self.output.with_extension("review-settings.json")
    }

    /// Advances one painted frame: requests the framebuffer once the UI has
    /// settled, and reports completion after the backend has delivered it.
    pub(crate) fn advance(&mut self, ctx: &egui::Context) -> ReviewProgress {
        self.frames = self.frames.saturating_add(1);
        if self.interactive {
            return ReviewProgress::Pending;
        }
        if self.finished.load(Ordering::Acquire) {
            return ReviewProgress::Finished;
        }
        if !self.requested && self.frames >= SETTLE_FRAMES {
            let output = self.output.clone();
            let finished = Arc::clone(&self.finished);
            let description = format!("state={:?}, theme={}", self.state, self.theme.key());
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(
                egui::ScreenshotCallback::new(move |image| {
                    match save_png(&output, &image) {
                        Ok(()) => println!(
                            "review capture: wrote {} ({}x{} px, {description})",
                            output.display(),
                            image.size[0],
                            image.size[1],
                        ),
                        Err(error) => eprintln!(
                            "review capture: could not write {}: {error}",
                            output.display()
                        ),
                    }
                    finished.store(true, Ordering::Release);
                }),
            ));
            self.requested = true;
        }
        if self.frames >= TIMEOUT_FRAMES {
            eprintln!("review capture: the backend never delivered a framebuffer");
            return ReviewProgress::Finished;
        }
        ctx.request_repaint();
        ReviewProgress::Pending
    }
}

fn parse_camera(value: &str) -> Option<[f32; 3]> {
    parse_numbers::<3>(value)
}

fn parse_numbers<const N: usize>(value: &str) -> Option<[f32; N]> {
    let parts = value
        .split(',')
        .map(|part| part.trim().parse::<f32>().ok().filter(|v| v.is_finite()))
        .collect::<Option<Vec<_>>>()?;
    <[f32; N]>::try_from(parts).ok()
}

fn save_png(path: &Path, image: &egui::ColorImage) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let width = u32::try_from(image.size[0]).map_err(|error| error.to_string())?;
    let height = u32::try_from(image.size[1]).map_err(|error| error.to_string())?;
    image::save_buffer(
        path,
        image.as_raw(),
        width,
        height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_keys_are_case_insensitive_and_reject_unknown_names() {
        assert_eq!(ReviewState::from_key(" Move "), Some(ReviewState::Move));
        assert_eq!(ReviewState::from_key("WELCOME"), Some(ReviewState::Welcome));
        assert_eq!(ReviewState::from_key("mvoe"), None);
    }

    #[test]
    fn camera_pose_needs_three_finite_numbers() {
        assert_eq!(parse_camera("30, 25, 600"), Some([30.0, 25.0, 600.0]));
        assert_eq!(parse_camera("30,25"), None);
        assert_eq!(parse_camera("30,x,1"), None);
    }
}
