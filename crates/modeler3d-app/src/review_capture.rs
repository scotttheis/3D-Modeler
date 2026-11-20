//! Deterministic review captures of the real shell.
//!
//! Visual work needs a current picture of the application in a known state.
//! Setting `MODELER3D_REVIEW_CAPTURE` to a PNG path starts the ordinary native
//! window in a named state, waits for the viewport to settle, saves the
//! window's own framebuffer, and exits. No clicks or key presses are
//! simulated (an optional resting pointer position only reveals hover
//! states), the desktop is never photographed, and user settings are neither
//! read nor written, so a capture is repeatable and safe to run while the
//! machine is in use.
//!
//! | Variable | Values | Default |
//! |---|---|---|
//! | `MODELER3D_REVIEW_CAPTURE` | output PNG path | harness off |
//! | `MODELER3D_REVIEW_STATE` | `welcome`, `empty`, `move`, `rotate`, `rotate-tilted`, `rotate-drag`, `resize`, `resize-drag`, `resize-floor`, `resize-object`, `resize-settle-before`, `resize-settle`, `grab-ready`, `grab-held`, `multi`, `merge`, `unselected`, `material`, `settings`, `save`, `export`, `menu`, `colors`, `mirror`, `merge-apart`, `repair`, `paste`, `align`, `paint`, `paint-picker`, `xray`, `texture`, `texture-dialog`, `grab`, `crease`, `crease-sharp`, `smooth-brush`, `simplify`, `simplify-area`, `subdivide`, `hollow`, `hollow-dialog`, `hollow-error`, `measure`, `measure-start`, `centre`, `settings-note`, `no-shortcuts` | `move` |
//! | `MODELER3D_REVIEW_TEXTURES` | `;`-separated picture paths for the `texture` states (one per object, reused in turn) | a built-in brick picture |
//! | `MODELER3D_REVIEW_THEME` | `light`, `dark` | `dark` |
//! | `MODELER3D_REVIEW_SIZE` | `normal`, `big` (Settings > Size) | `normal` |
//! | `MODELER3D_REVIEW_GRAB_CURSOR` | `claw`, `hand` (Settings > Grab cursor) | `claw` |
//! | `MODELER3D_REVIEW_TAB` | a ribbon tab label such as `Object` | `Insert` |
//! | `MODELER3D_REVIEW_VIEW` | comma list of view toggles to turn on: `wireframe`, `xray` | none |
//! | `MODELER3D_REVIEW_POINTER` | `x,y` in window points: rests the pointer there to show a hover state (no button is pressed) | pointer outside |
//! | `MODELER3D_REVIEW_INTERACTIVE` | `1`: set up the state with isolated settings and stay open for hands-on checks instead of capturing | off |
//! | `MODELER3D_REVIEW_CAMERA` | `yaw,pitch,distance` in degrees and mm, applied after auto-framing; distance `0` keeps the framed distance | framed view |
//!
//! A capture shows what was painted. It is engineering evidence for layout and
//! styling; it does not exercise pointer behaviour and is not owner approval.
//! `brush-settings` shows Grab selected with its controls; `brush-dismissed`
//! toggles it off after deselecting the mesh, leaving the controls closed.

use std::{
    env,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use eframe::egui;

use crate::grab_cursor::GrabCursorStyle;
use crate::theme::{ThemeMode, UiSize};

/// Frames to wait before requesting the framebuffer, so fonts, artwork
/// textures, auto-framing, and the first viewport render have all settled.
const SETTLE_FRAMES: u32 = 45;
/// Frames after which a capture that never arrived is reported as a failure.
const TIMEOUT_FRAMES: u32 = 600;

/// The named application state a review capture starts in.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReviewState {
    /// Grab selected with its settings window open.
    BrushSettings,
    /// Edit tool controls after selection loss and switching tools.
    EditSimplify,
    EditSmooth,
    EditBlob,
    EditOff,
    /// Grab toggled off after the mesh selection is lost.
    BrushDismissed,
    /// The welcome surface with no scene.
    Welcome,
    /// The scene surface with no objects.
    Empty,
    /// One selected cube with the Move handles.
    Move,
    /// One selected cube held partway through a +X Move drag, so the
    /// position boxes show the live, uncommitted value (OF-70).
    MoveDrag,
    /// One selected cube with the Rotate handles.
    Rotate,
    /// A cube already rotated around several axes.
    RotateTilted,
    /// A tilted cube held partway through a local-axis rotation.
    RotateDrag,
    /// One selected cube with the Resize handles.
    Resize,
    /// Tilted object resized along fixed floor axes.
    ResizeFloor,
    /// Nonuniformly resized, tilted cube before/after Settle.
    ResizeSettleBefore,
    ResizeSettle,
    /// Tilted object resized along its imported axes.
    ResizeObject,
    /// The Resize state with the +X face handle held mid-drag. The drag is
    /// staged through the shell's own gesture code each frame; no pointer
    /// input is simulated and nothing is committed.
    ResizeDrag,
    /// Open claw over a Resize handle / closed claw holding it.
    GrabReady,
    GrabHeld,
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
    /// One selected cube with the application menu open.
    Menu,
    /// Red, green, blue and yellow cubes in a row with the red and blue
    /// ones selected, to compare selected and unselected colours.
    Colors,
    /// One selected cube with the Mirror dialog open.
    Mirror,
    /// Two cubes that do not touch, merged: they are grouped and the
    /// "not touching" notice shows.
    MergeApart,
    /// One selected cube with Repair run: the Repair dialog shows its result.
    Repair,
    /// One cube copied and pasted twice.
    Paste,
    /// Two selected cubes with the Align dialog open.
    Align,
    /// The Paint bar open (OF-44) over a cube with its top painted red and a
    /// sphere painted blue, nothing selected, Coverage at 0 %.
    Paint,
    /// The `Paint` state with the colour drop-down open.
    PaintPicker,
    /// X-ray on (OF-47) with a selected green cube and an unselected sphere
    /// overlapping it, as in the owner's 3D Builder reference.
    Xray,
    /// A cube, a sphere and a cylinder with material textures (OF-46),
    /// nothing selected; pictures from `MODELER3D_REVIEW_TEXTURES`.
    Texture,
    /// One textured, selected cube with the Material dialog open on its
    /// Texture section.
    TextureDialog,
    /// One selected sphere pulled out by a Grab stroke (OF-48): the point on
    /// its upper right is pinched and pulled up and out, with the desktop's
    /// Grab settings.
    Grab,
    /// One selected sphere with a Crease stroke across its top (OF-49), with
    /// the desktop's Crease settings.
    Crease,
    /// The same Crease stroke at full Sharpness (OF-59).
    CreaseSharp,
    /// One selected stock cylinder with a strong Smooth Brush stroke over its
    /// top rim (OF-50), with the desktop's Smooth Brush settings.
    SmoothBrush,
    /// One selected stock sphere simplified by 50 % (OF-51) with the
    /// Simplify window open showing the result.
    Simplify,
    /// One selected stock sphere with a 50 % Simplify brushed across its
    /// top (OF-60), the Simplify window open on "Area I brush over". Best
    /// with `MODELER3D_REVIEW_VIEW=wireframe`.
    SimplifyArea,
    /// Two cubes with 48 faces each (OF-52): left, the old doubling run
    /// twice; right, selected, the new even split run once; the Subdivide
    /// window open. Best with `MODELER3D_REVIEW_VIEW=wireframe`.
    Subdivide,
    /// The stock torus hollowed with a 1.5 mm wall (OF-54), nothing
    /// selected and X-ray on so the inner shell shows through.
    Hollow,
    /// One selected stock sphere with a fresh Hollow window (OF-61: the
    /// 2 mm default).
    HollowDialog,
    /// One selected stock sphere after a 1 mm Hollow that cannot be done:
    /// the message with its Copy buttons in the window and the status bar
    /// (OF-61).
    HollowError,
    /// The Measure bar open (OF-55) over a cube and a cylinder, nothing
    /// selected, with a finished ruler from the cube's front bottom-left
    /// corner to the cylinder's top rim.
    Measure,
    /// The `Measure` state with only the first end placed: with
    /// `MODELER3D_REVIEW_POINTER` over a model the live ruler follows it.
    MeasureStart,
    /// The `multi` scene after a double-click on the sphere's Items row
    /// (OF-63): the sphere is the only selection and the camera frames it.
    Centre,
    /// One cube after a first start that imported the Python settings
    /// (OF-64): the real import path runs on a stand-in registry value, so
    /// the settings note shows under the ribbon.
    SettingsNote,
    /// The empty scene with an emptied shortcut bar, so the dim
    /// "Drag shortcuts here" hint shows.
    NoShortcuts,
    /// Startup update offer, with no network or installer worker.
    Update,
}

impl ReviewState {
    fn from_key(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "welcome" => Some(Self::Welcome),
            "empty" => Some(Self::Empty),
            "move" => Some(Self::Move),
            "move-drag" => Some(Self::MoveDrag),
            "rotate" => Some(Self::Rotate),
            "rotate-tilted" => Some(Self::RotateTilted),
            "rotate-drag" => Some(Self::RotateDrag),
            "resize" => Some(Self::Resize),
            "resize-floor" => Some(Self::ResizeFloor),
            "resize-settle-before" => Some(Self::ResizeSettleBefore),
            "resize-settle" => Some(Self::ResizeSettle),
            "resize-object" => Some(Self::ResizeObject),
            "resize-drag" => Some(Self::ResizeDrag),
            "grab-ready" => Some(Self::GrabReady),
            "grab-held" => Some(Self::GrabHeld),
            "multi" => Some(Self::Multi),
            "merge" => Some(Self::Merge),
            "unselected" => Some(Self::Unselected),
            "material" => Some(Self::Material),
            "settings" => Some(Self::Settings),
            "save" => Some(Self::Save),
            "export" => Some(Self::Export),
            "menu" => Some(Self::Menu),
            "colors" => Some(Self::Colors),
            "mirror" => Some(Self::Mirror),
            "merge-apart" => Some(Self::MergeApart),
            "repair" => Some(Self::Repair),
            "paste" => Some(Self::Paste),
            "align" => Some(Self::Align),
            "paint" => Some(Self::Paint),
            "paint-picker" => Some(Self::PaintPicker),
            "xray" => Some(Self::Xray),
            "texture" => Some(Self::Texture),
            "texture-dialog" => Some(Self::TextureDialog),
            "grab" => Some(Self::Grab),
            "edit-simplify" => Some(Self::EditSimplify),
            "edit-smooth" => Some(Self::EditSmooth),
            "edit-blob" => Some(Self::EditBlob),
            "edit-off" => Some(Self::EditOff),
            "brush-settings" => Some(Self::BrushSettings),
            "brush-dismissed" => Some(Self::BrushDismissed),
            "crease" => Some(Self::Crease),
            "crease-sharp" => Some(Self::CreaseSharp),
            "smooth-brush" => Some(Self::SmoothBrush),
            "simplify" => Some(Self::Simplify),
            "simplify-area" => Some(Self::SimplifyArea),
            "subdivide" => Some(Self::Subdivide),
            "hollow" => Some(Self::Hollow),
            "hollow-dialog" => Some(Self::HollowDialog),
            "hollow-error" => Some(Self::HollowError),
            "measure" => Some(Self::Measure),
            "measure-start" => Some(Self::MeasureStart),
            "centre" => Some(Self::Centre),
            "settings-note" => Some(Self::SettingsNote),
            "no-shortcuts" => Some(Self::NoShortcuts),
            "update" => Some(Self::Update),
            _ => None,
        }
    }
}

/// Pictures for the `texture` review states: `MODELER3D_REVIEW_TEXTURES`
/// (`;`-separated paths), or one generated brick picture written to the
/// temporary folder so the state works without outside files.
pub(crate) fn review_texture_paths() -> Vec<PathBuf> {
    if let Ok(value) = env::var("MODELER3D_REVIEW_TEXTURES") {
        let paths = value
            .split(';')
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        if !paths.is_empty() {
            return paths;
        }
    }
    let path = env::temp_dir().join(format!(
        "modeler3d-review-bricks-{}.png",
        std::process::id()
    ));
    let bricks = image::RgbaImage::from_fn(128, 128, |x, y| {
        let row = y / 32;
        let offset = if row % 2 == 0 { 0 } else { 32 };
        let mortar = y % 32 < 3 || (x + offset) % 64 < 3;
        if mortar {
            image::Rgba([214, 208, 196, 255])
        } else {
            let shade = ((x * 7 + y * 13) % 23) as u8;
            image::Rgba([160 + shade, 64 + shade / 2, 48, 255])
        }
    });
    if bricks.save(&path).is_err() {
        return Vec::new();
    }
    vec![path]
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
    grab_cursor: GrabCursorStyle,
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
        let output = PathBuf::from(env::var_os("MODELER3D_REVIEW_CAPTURE")?);
        if output.as_os_str().is_empty() {
            return None;
        }
        let state = match env::var("MODELER3D_REVIEW_STATE") {
            Ok(value) => match ReviewState::from_key(&value) {
                Some(state) => state,
                None => {
                    eprintln!("review capture: unknown MODELER3D_REVIEW_STATE `{value}`");
                    return None;
                }
            },
            Err(_) => ReviewState::Move,
        };
        let theme = env::var("MODELER3D_REVIEW_THEME")
            .map(|value| ThemeMode::from_key(&value))
            .unwrap_or_default();
        let size = env::var("MODELER3D_REVIEW_SIZE")
            .ok()
            .and_then(|value| UiSize::from_key(&value))
            .unwrap_or_default();
        let grab_cursor = env::var("MODELER3D_REVIEW_GRAB_CURSOR")
            .ok()
            .and_then(|value| GrabCursorStyle::from_key(&value))
            .unwrap_or_default();
        let tab = env::var("MODELER3D_REVIEW_TAB")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());
        let camera = match env::var("MODELER3D_REVIEW_CAMERA") {
            Ok(value) if !value.trim().is_empty() => match parse_camera(&value) {
                Some(camera) => Some(camera),
                None => {
                    eprintln!(
                        "review capture: MODELER3D_REVIEW_CAMERA must be `yaw,pitch,distance`"
                    );
                    return None;
                }
            },
            _ => None,
        };
        let pointer = match env::var("MODELER3D_REVIEW_POINTER") {
            Ok(value) if !value.trim().is_empty() => match parse_numbers::<2>(&value) {
                Some(pointer) => Some(pointer),
                None => {
                    eprintln!("review capture: MODELER3D_REVIEW_POINTER must be `x,y`");
                    return None;
                }
            },
            _ => None,
        };
        let view = env::var("MODELER3D_REVIEW_VIEW")
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
            grab_cursor,
            tab,
            camera,
            pointer,
            view,
            interactive: env::var_os("MODELER3D_REVIEW_INTERACTIVE")
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

    pub(crate) const fn grab_cursor(&self) -> GrabCursorStyle {
        self.grab_cursor
    }

    pub(crate) fn tab(&self) -> Option<&str> {
        self.tab.as_deref()
    }

    /// View toggles (`wireframe`, `xray`) to switch on.
    pub(crate) fn view_toggles(&self) -> &[String] {
        &self.view
    }

    /// Positions the resting pointer on a handle in a review-only state.
    pub(crate) fn set_pointer(&mut self, pointer: [f32; 2]) {
        self.pointer = Some(pointer);
    }

    /// Where the resting pointer is placed, in window points.
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

pub(crate) fn save_png(path: &Path, image: &egui::ColorImage) -> Result<(), String> {
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
