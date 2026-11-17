//! Native egui shell for 3D Modeler.
//!
//! This module is a presentation adapter. Model mutations are routed through
//! [`crate::commands::CommandState`]; the shell owns only layout, focus,
//! preferences, and renderer-neutral interaction cues.

use crate::chrome::{CONTROL_HEIGHT, Glyph, RADIUS_CONTROL, ROW_HEIGHT};

mod app_tools;
mod measure_ruler;
mod paint_bar;
use app_tools::{RecentOpen, recent_is_project};
use measure_ruler::MeasureMode;
use paint_bar::PaintMode;
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use eframe::egui::{
    self, Align, Align2, Color32, FontId, Key, Layout, RichText, Sense, Stroke, Ui, Vec2,
};
use modeler3d_core::{
    BlobOptions, BrushDirection, BrushFalloff, BrushSymmetry, CreasePolishOptions,
    ElasticGrabOptions, Mesh, SmoothBrushOptions, Transform, blob, build_parametric_torus,
    crease_polish, elastic_grab, smooth_brush,
};
use modeler3d_viewport::{
    Aabb, Camera, GizmoAxis, GizmoLine, GizmoMode, InteractionState, MOVE_GIZMO_SCALE, Mat4,
    ROTATE_RING_SEGMENTS, RenderScene, RendererConfig, SnapViewContext, ViewportRenderOptions,
    ViewportRenderer, ViewportTheme, build_gizmo_with_bounds, dot, hit_test_with_bounds,
    pick_scene, subtract,
};

use crate::artwork::{
    OriginalAsset, ThemeArtwork, load_original_image, original_asset_for_command,
    original_asset_root,
};
use crate::commands::{
    BrushStroke, CommandDescriptor, CommandError, CommandId, CommandOutcome, CommandState,
    DialogKind, IconKey, RibbonItem, RibbonTab, file_menu_spec, ribbon_spec,
};
use crate::desktop_jobs::{
    DesktopJobEvent, DesktopJobRequest, DesktopJobResult, MeshJobOperation, run_desktop_job,
};
use crate::dialogs::{DialogAction, DialogState};
use crate::numeric_controls::{
    format_distance, parse_angle_degrees, parse_distance_mm, parse_scale_factor,
};
use crate::operations::{OperationFailure, OperationHandle, OperationPhase};
use crate::preferences::DisplayUnit;
use crate::print_handoff::{PreparedPrint, discard_prepared_print, launch_prepared_print};
use crate::theme::{self, ThemeMode, ThemePalette, UiSize};
use crate::transform_gesture::{ResizeFrame, ResizeHandle, TransformGesture};
use crate::transforms::{Alignment, Axis, world_bounds};
use crate::ui_state::{
    RecentEntry, RecentEntryKind, WorkspaceSurface, command_key, default_favorites, move_favorite,
    normalize_favorites, normalized_tab_order, place_favorite_at, recent_entry, tab_commands,
};
use modeler3d_platform::{
    CapabilityProfile, SlicerInfo, current_runtime_paths, discover_capability_profile,
    read_legacy_settings, slicer_info_from_path,
};

const RIBBON_BUTTON_HEIGHT: f32 = 68.0;
const FAVORITE_ICON_SIZE: f32 = 22.0;
const FAVORITE_BUTTON_WIDTH: f32 = 36.0;
const FAVORITE_BUTTON_HEIGHT: f32 = 34.0;
const FAVORITE_OVERFLOW_WIDTH: f32 = 40.0;
/// Shown on the shortcut bar when it has no shortcuts (owner idea, 2026-11-16).
const EMPTY_FAVORITES_HINT: &str = "Drag shortcuts here";
/// Width of the bar that shows where a dragged shortcut will land.
const FAVORITE_DROP_MARKER_WIDTH: f32 = 3.0;

/// What a shortcut drag carries: the command key of a shortcut being
/// reordered on the bar, or of a ribbon command being dragged up to it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ShortcutDrag(String);
/// Room the empty-bar hint needs before it is drawn.
const EMPTY_FAVORITES_HINT_WIDTH: f32 = 130.0;
const TOP_ROW_SPACING: f32 = 1.0;
const SLICER_BUTTON_WIDTH: f32 = 84.0;
const TRANSFORM_AXIS_LABEL_WIDTH: f32 = 14.0;
const TRANSFORM_FIELD_WIDTH: f32 = 92.0;
const THEME_TOGGLE_WIDTH: f32 = 52.0;
/// Room for the top-right cluster: Slicer, theme toggle, "Save", "Save As",
/// Undo and Redo, with spacing. The labelled buttons size to their text, so
/// this is a measured upper bound.
const TOP_RIGHT_CLUSTER_WIDTH: f32 = SLICER_BUTTON_WIDTH
    + THEME_TOGGLE_WIDTH
    + 64.0
    + 82.0
    + 2.0 * 36.0
    + 6.0 * TOP_ROW_SPACING
    + 12.0;
const MOVE_ARROW_HEAD_LENGTH_PIXELS: f32 = 12.0;
const MOVE_ARROW_SHAFT_LENGTH_PIXELS: f32 = 22.0;
const MOVE_ARROW_HEAD_HALF_WIDTH_PIXELS: f32 = 5.0;
const MOVE_ARROW_LENGTH_PIXELS: f32 =
    (MOVE_ARROW_SHAFT_LENGTH_PIXELS + MOVE_ARROW_HEAD_LENGTH_PIXELS) * MOVE_GIZMO_SCALE;
const MOVE_ARROW_PICK_RADIUS_PIXELS: f32 =
    MOVE_ARROW_HEAD_HALF_WIDTH_PIXELS * MOVE_GIZMO_SCALE + 5.0;

#[derive(Clone, Debug)]
enum ViewportDrag {
    Orbit,
    Pan,
    Transform(Box<TransformGesture>),
    Brush(Box<BrushGesture>),
    /// Shift + drag from empty floor: a selection rectangle in screen points.
    BoxSelect {
        start: egui::Pos2,
        current: egui::Pos2,
    },
}

/// Below this size (points, either side) a Shift box gesture is a click.
const BOX_SELECT_MIN_SIZE: f32 = 4.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActiveBrush {
    Grab,
    Crease,
    SmoothBrush,
    Blob,
    /// The Subdivide window's "Area I brush over" (OF-52).
    SubdivideArea,
    /// The Simplify window's "Area I brush over" (OF-60).
    SimplifyArea,
}

impl ActiveBrush {
    const fn from_command(command: CommandId) -> Option<Self> {
        match command {
            CommandId::Grab => Some(Self::Grab),
            CommandId::Crease => Some(Self::Crease),
            CommandId::SmoothBrush => Some(Self::SmoothBrush),
            CommandId::Blob => Some(Self::Blob),
            _ => None,
        }
    }

    const fn command(self) -> CommandId {
        match self {
            Self::Grab => CommandId::Grab,
            Self::Crease => CommandId::Crease,
            Self::SmoothBrush => CommandId::SmoothBrush,
            Self::Blob => CommandId::Blob,
            Self::SubdivideArea => CommandId::Subdivide,
            Self::SimplifyArea => CommandId::Simplify,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Grab => "Grab",
            Self::Crease => "Crease",
            Self::SmoothBrush => "Smooth Brush",
            Self::Blob => "Blob",
            Self::SubdivideArea => "Subdivide",
            Self::SimplifyArea => "Simplify",
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct BrushSettings {
    grab_radius_mm: f32,
    grab_strength: f32,
    grab_falloff: BrushFalloff,
    grab_symmetry: BrushSymmetry,
    crease_radius_mm: f32,
    crease_strength: f32,
    crease_direction: BrushDirection,
    crease_falloff: BrushFalloff,
    crease_sharpness: f32,
    smooth_radius_mm: f32,
    smooth_strength: f32,
    smooth_shrinkage: f32,
    smooth_falloff: BrushFalloff,
    blob_radius_mm: f32,
    blob_strength: f32,
    blob_direction: BrushDirection,
    blob_falloff: BrushFalloff,
    subdivide_radius_mm: f32,
    simplify_radius_mm: f32,
    simplify_percent: u32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        let grab = ElasticGrabOptions::default();
        let crease = CreasePolishOptions::default();
        let smooth = SmoothBrushOptions::default();
        let blob = BlobOptions::default();
        Self {
            grab_radius_mm: grab.radius_mm,
            // Blender's Grab default: the grabbed point stays under the pointer (OF-48).
            grab_strength: 1.0,
            grab_falloff: grab.falloff,
            grab_symmetry: grab.symmetry,
            // OF-49: a wider default so the crease moves more of the surface around it.
            crease_radius_mm: 8.0,
            crease_strength: crease.strength,
            crease_direction: crease.direction,
            crease_falloff: crease.falloff,
            crease_sharpness: crease.sharpness,
            smooth_radius_mm: smooth.radius_mm,
            smooth_strength: smooth.strength,
            // OF-50 / D9: Blender's sculpt Smooth keeps the whole movement toward
            // the neighbour average; sharp rims are protected in the core instead.
            smooth_shrinkage: 1.0,
            smooth_falloff: smooth.falloff,
            blob_radius_mm: blob.radius_mm,
            blob_strength: blob.strength,
            blob_direction: blob.direction,
            blob_falloff: blob.falloff,
            subdivide_radius_mm: 5.0,
            simplify_radius_mm: 5.0,
            simplify_percent: 50,
        }
    }
}

#[derive(Clone, Debug)]
struct BrushGesture {
    tool: ActiveBrush,
    object_id: String,
    source_mesh: Mesh,
    source_transform: Transform,
    centers: Vec<[f32; 3]>,
    start_center: [f32; 3],
    offset: [f32; 3],
    /// World point the stroke started on; Grab drags it across a
    /// camera-facing plane through this point (OF-48).
    start_world: [f32; 3],
    /// Unit normal of the Grab drag plane (toward the camera).
    plane_normal: [f32; 3],
    revision: u64,
}

/// Where a pointer ray crosses the camera-facing Grab plane, like Blender's
/// Grab brush: the grabbed point follows the pointer across the screen even
/// off the object, instead of sliding along the surface it was picked on.
fn grab_plane_target(
    ray: modeler3d_viewport::Ray,
    plane_point: [f32; 3],
    plane_normal: [f32; 3],
) -> Option<[f32; 3]> {
    let denominator = dot(ray.direction, plane_normal);
    if denominator.abs() < 1.0e-6 {
        return None;
    }
    let distance = dot(subtract(plane_point, ray.origin), plane_normal) / denominator;
    (distance.is_finite() && distance > 0.0).then(|| ray.at(distance))
}

#[derive(Clone, Copy, Debug)]
struct BooleanPick {
    operation: modeler3d_core::BooleanOperation,
    source_index: usize,
    source_is_tool: bool,
    clear_selection: bool,
}

#[derive(Clone, Copy, Debug)]
struct ViewportInput {
    pointer_position: Option<egui::Pos2>,
    pointer_delta: Vec2,
    primary_pressed: bool,
    primary_down: bool,
    primary_released: bool,
    secondary_pressed: bool,
    secondary_down: bool,
    secondary_released: bool,
    middle_pressed: bool,
    middle_down: bool,
    middle_released: bool,
    scroll_y: f32,
    /// Native touchpad pinch factor; one means no pinch this frame.
    pinch_zoom_factor: f32,
    escape_pressed: bool,
    focused: bool,
    modifiers: egui::Modifiers,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PendingReplacement {
    NewScene,
    OpenProject(PathBuf),
    Exit,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PendingOverwrite {
    SaveProject(PathBuf),
    Export(PathBuf),
}

struct RecentPreviewRequest {
    generation: u64,
    path: PathBuf,
}

struct RecentPreviewResult {
    generation: u64,
    path: PathBuf,
    image: Option<egui::ColorImage>,
}

struct RunningDesktopJob {
    handle: OperationHandle,
    receiver: Receiver<DesktopJobEvent>,
    pending_terminal: Option<Result<DesktopJobResult, String>>,
    title: String,
    document_generation: u64,
    source_revision: u64,
    target_object_id: Option<String>,
    dialog_generation: u64,
    resume_dialog: Option<DialogState>,
    cancel_on_drop: bool,
    /// Launch the slicer as soon as staging succeeds (the top-bar Slicer
    /// button) instead of handing the staged files to the print dialog.
    launch_when_staged: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConfirmationChoice {
    Save,
    Discard,
    Cancel,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum TransformPanelMode {
    #[default]
    Move,
    Rotate,
    Resize,
}

impl TransformPanelMode {
    const fn label(self) -> &'static str {
        match self {
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Resize => "Resize",
        }
    }
}

/// The native 3D Modeler application shell state.
pub struct Modeler3dApp {
    updater: crate::updates::Updater,
    /// Portable application command state.
    pub(crate) commands: CommandState,
    active_tab: RibbonTab,
    surface: WorkspaceSurface,
    menu_open: bool,
    /// Open while the first-use slicer chooser under the Slicer button shows.
    slicer_chooser_open: bool,
    /// Slicers found when the chooser opened.
    slicer_choices: Vec<SlicerInfo>,
    ribbon_pinned: bool,
    ribbon_expanded: bool,
    ribbon_rect: Option<egui::Rect>,
    transform_strip_rect: Option<egui::Rect>,
    dialog: Option<DialogState>,
    theme_mode: ThemeMode,
    /// Shell size from Settings; Big doubles every shell surface (OF-20).
    ui_size: UiSize,
    close_requested: bool,
    confirm_action: Option<PendingReplacement>,
    pending_overwrite: Option<PendingOverwrite>,
    pending_after_save: Option<PendingReplacement>,
    customize_ribbon: bool,
    selection_anchor: Option<String>,
    sticky_selection: bool,
    placement_command: Option<CommandId>,
    /// Measure ruler (OF-55); `None` when the Measure bar is closed.
    measure_mode: Option<MeasureMode>,
    placement_axis: Axis,
    placement_alignment: Alignment,
    settings_path: PathBuf,
    settings_warning: Option<String>,
    invalid_settings_file: bool,
    transform_signature: String,
    last_numeric_axis: Option<usize>,
    numeric_edited_axes: [bool; 3],
    position_fields: [String; 3],
    size_fields: [String; 3],
    resize_size_preview: Option<(modeler3d_core::Transform, DisplayUnit, u64)>,
    rotation_fields: [String; 3],
    scale_fields: [String; 3],
    arrange_gap_field: String,
    transform_error: Option<String>,
    transform_panel_mode: TransformPanelMode,
    proportional_resize: bool,
    object_resize_basis: bool,
    gizmo_frame: Option<(String, ResizeFrame)>,
    name_field: String,
    preview_revision: u64,
    preview: Option<RenderScene>,
    preview_mesh_override: Option<(String, Mesh)>,
    preview_error: Option<String>,
    wgpu_render_state: Option<eframe::egui_wgpu::RenderState>,
    capability_profile: Option<CapabilityProfile>,
    viewport_renderer: Option<ViewportRenderer>,
    viewport_texture_id: Option<egui::TextureId>,
    viewport_error: Option<String>,
    last_cpu_fallback: Option<bool>,
    last_clear_color_error: Option<String>,
    pub(crate) camera: Camera,
    camera_auto_frame_pending: bool,
    viewport_drag: Option<ViewportDrag>,
    active_brush: Option<ActiveBrush>,
    brush_settings_open: bool,
    /// Native image picker used by Color; replaceable so tests never open
    /// a real OS dialog.
    brush_settings: BrushSettings,
    boolean_pick: Option<BooleanPick>,
    /// Paint mode (OF-44); `None` when the Paint bar is closed.
    paint_mode: Option<PaintMode>,
    /// Last paint colour and coverage, kept between Paint sessions.
    paint_color: [f32; 4],
    paint_coverage: f32,
    viewport_gizmo_consumed: bool,
    artwork_root: PathBuf,
    artwork_cache: HashMap<PathBuf, egui::TextureHandle>,
    artwork_cache_theme: ThemeMode,
    missing_artwork: BTreeSet<PathBuf>,
    recent_preview_request_sender: Sender<RecentPreviewRequest>,
    recent_preview_request_receiver: Option<Receiver<RecentPreviewRequest>>,
    recent_preview_result_sender: Option<Sender<RecentPreviewResult>>,
    recent_preview_result_receiver: Receiver<RecentPreviewResult>,
    recent_preview_worker_started: bool,
    recent_preview_generation: u64,
    recent_preview_generation_signal: Arc<AtomicU64>,
    recent_preview_paths: Vec<PathBuf>,
    recent_preview_pending: HashSet<PathBuf>,
    recent_preview_textures: HashMap<PathBuf, egui::TextureHandle>,
    item_textures: HashMap<String, egui::TextureHandle>,
    item_revision: u64,
    item_health: HashMap<String, bool>,
    desktop_job: Option<RunningDesktopJob>,
    mcp_bridge: Option<crate::mcp_bridge::McpDesktopIntegration>,
    dialog_generation: u64,
    review: Option<crate::review_capture::ReviewCapture>,
    /// A brief notice fading out over the viewport (OF-38).
    toast: Option<crate::toast::Toast>,
}

impl Default for Modeler3dApp {
    fn default() -> Self {
        let (recent_preview_request_sender, recent_preview_request_receiver) = mpsc::channel();
        let (recent_preview_result_sender, recent_preview_result_receiver) = mpsc::channel();
        Self {
            updater: crate::updates::Updater::default(),
            commands: CommandState::default(),
            active_tab: RibbonTab::Insert,
            surface: WorkspaceSurface::Welcome,
            menu_open: false,
            slicer_chooser_open: false,
            slicer_choices: Vec::new(),
            ribbon_pinned: false,
            ribbon_expanded: true,
            ribbon_rect: None,
            transform_strip_rect: None,
            dialog: None,
            theme_mode: ThemeMode::Dark,
            ui_size: UiSize::Normal,
            close_requested: false,
            confirm_action: None,
            pending_overwrite: None,
            pending_after_save: None,
            customize_ribbon: false,
            selection_anchor: None,
            sticky_selection: false,
            placement_command: None,
            measure_mode: None,
            placement_axis: Axis::X,
            placement_alignment: Alignment::Center,
            settings_path: current_runtime_paths().config_dir.join("settings.json"),
            settings_warning: None,
            invalid_settings_file: false,
            transform_signature: String::new(),
            last_numeric_axis: None,
            numeric_edited_axes: [false; 3],
            position_fields: [String::new(), String::new(), String::new()],
            size_fields: [String::new(), String::new(), String::new()],
            resize_size_preview: None,
            rotation_fields: [
                String::from("0 deg"),
                String::from("0 deg"),
                String::from("0 deg"),
            ],
            scale_fields: [
                String::from("1.0x"),
                String::from("1.0x"),
                String::from("1.0x"),
            ],
            arrange_gap_field: String::from("0 mm"),
            transform_error: None,
            transform_panel_mode: TransformPanelMode::Move,
            proportional_resize: false,
            object_resize_basis: false,
            gizmo_frame: None,
            name_field: String::new(),
            preview_revision: u64::MAX,
            preview: None,
            preview_mesh_override: None,
            preview_error: None,
            wgpu_render_state: None,
            capability_profile: None,
            viewport_renderer: None,
            viewport_texture_id: None,
            viewport_error: None,
            last_cpu_fallback: None,
            last_clear_color_error: None,
            camera: Camera::new(),
            camera_auto_frame_pending: true,
            viewport_drag: None,
            active_brush: None,
            brush_settings_open: false,
            brush_settings: BrushSettings::default(),
            boolean_pick: None,
            paint_mode: None,
            paint_color: paint_bar::DEFAULT_PAINT_COLOR,
            paint_coverage: 1.0,
            viewport_gizmo_consumed: false,
            artwork_root: original_asset_root(),
            artwork_cache: HashMap::new(),
            artwork_cache_theme: ThemeMode::Dark,
            missing_artwork: BTreeSet::new(),
            recent_preview_request_sender,
            recent_preview_request_receiver: Some(recent_preview_request_receiver),
            recent_preview_result_sender: Some(recent_preview_result_sender),
            recent_preview_result_receiver,
            recent_preview_worker_started: false,
            recent_preview_generation: 0,
            recent_preview_generation_signal: Arc::new(AtomicU64::new(0)),
            recent_preview_paths: Vec::new(),
            recent_preview_pending: HashSet::new(),
            recent_preview_textures: HashMap::new(),
            item_textures: HashMap::new(),
            item_revision: u64::MAX,
            item_health: HashMap::new(),
            desktop_job: None,
            mcp_bridge: None,
            dialog_generation: 0,
            review: None,
            toast: None,
        }
    }
}

impl Modeler3dApp {
    /// Creates the shell with eframe's creation context and bounded settings
    /// recovery. A missing settings file is a normal first-run state.
    #[must_use]
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        theme::configure_fonts(&creation_context.egui_ctx);
        let mut app = Self::default();
        if let Some(review) = crate::review_capture::ReviewCapture::from_environment() {
            // A review capture starts from defaults in an isolated settings
            // file and without the MCP gateway, so it is repeatable.
            app.initialize_viewport(creation_context);
            app.enter_review_state(review);
            return app;
        }
        app.load_preferences();
        app.updater.start(&creation_context.egui_ctx);
        app.initialize_viewport(creation_context);
        match crate::mcp_bridge::McpDesktopIntegration::start() {
            Ok(bridge) => app.mcp_bridge = Some(bridge),
            Err(error) => {
                crate::diagnostics::record_warning("optional MCP bridge", &error);
                app.commands.set_status(format!("MCP unavailable: {error}"));
            }
        }
        app
    }

    /// Puts the shell into the named state of a review capture. See
    /// [`crate::review_capture`].
    fn enter_review_state(&mut self, review: crate::review_capture::ReviewCapture) {
        use crate::review_capture::ReviewState;
        if review.state() == ReviewState::Update {
            self.updater = crate::updates::Updater::review();
        }

        self.settings_path = review.isolated_settings_path();
        self.theme_mode = review.theme();
        self.ui_size = review.size();
        // A real start seeds the shortcut bar when it is empty; do the same so
        // captures show it.
        let favorites = if review.state() == ReviewState::NoShortcuts {
            Vec::new()
        } else {
            default_favorites()
        };
        if review.state() == ReviewState::NoShortcuts
            || self.commands.preferences().ribbon_favorites().is_empty()
        {
            self.commands
                .preferences_mut()
                .set_ribbon_favorites(favorites);
        }
        if let Some(tab) = review.tab().and_then(|label| {
            RibbonTab::all()
                .into_iter()
                .find(|tab| tab.label() == label)
        }) {
            self.active_tab = tab;
        }
        let primitives: &[modeler3d_core::PrimitiveKind] = match review.state() {
            ReviewState::Welcome
            | ReviewState::Empty
            | ReviewState::NoShortcuts
            | ReviewState::Update => &[],
            ReviewState::Move
            | ReviewState::Rotate
            | ReviewState::Resize
            | ReviewState::ResizeDrag
            | ReviewState::Unselected
            | ReviewState::Material
            | ReviewState::Settings
            | ReviewState::Save
            | ReviewState::Export
            | ReviewState::Menu
            | ReviewState::SettingsNote => &[modeler3d_core::PrimitiveKind::Cube],
            ReviewState::Merge => &[
                modeler3d_core::PrimitiveKind::Cube,
                modeler3d_core::PrimitiveKind::Cube,
            ],
            ReviewState::Multi | ReviewState::Centre => &[
                modeler3d_core::PrimitiveKind::Cube,
                modeler3d_core::PrimitiveKind::Cylinder,
                modeler3d_core::PrimitiveKind::Sphere,
                modeler3d_core::PrimitiveKind::Cone,
            ],
            ReviewState::Colors => &[modeler3d_core::PrimitiveKind::Cube; 4],
            ReviewState::Mirror => &[modeler3d_core::PrimitiveKind::Cube],
            ReviewState::MergeApart => &[modeler3d_core::PrimitiveKind::Cube; 2],
            ReviewState::Repair | ReviewState::Paste => &[modeler3d_core::PrimitiveKind::Cube],
            ReviewState::Align => &[modeler3d_core::PrimitiveKind::Cube; 2],
            ReviewState::Paint | ReviewState::PaintPicker | ReviewState::Xray => &[
                modeler3d_core::PrimitiveKind::Cube,
                modeler3d_core::PrimitiveKind::Sphere,
            ],
            ReviewState::Texture => &[
                modeler3d_core::PrimitiveKind::Cube,
                modeler3d_core::PrimitiveKind::Sphere,
                modeler3d_core::PrimitiveKind::Cylinder,
            ],
            ReviewState::TextureDialog => &[modeler3d_core::PrimitiveKind::Cube],
            ReviewState::Grab
            | ReviewState::Crease
            | ReviewState::CreaseSharp
            | ReviewState::EditSimplify
            | ReviewState::EditSmooth
            | ReviewState::EditBlob
            | ReviewState::EditOff
            | ReviewState::BrushSettings
            | ReviewState::BrushDismissed => &[modeler3d_core::PrimitiveKind::Sphere],
            ReviewState::SmoothBrush => &[modeler3d_core::PrimitiveKind::Cylinder],
            ReviewState::Simplify | ReviewState::SimplifyArea => {
                &[modeler3d_core::PrimitiveKind::Sphere]
            }
            ReviewState::Subdivide => &[modeler3d_core::PrimitiveKind::Cube; 2],
            ReviewState::Hollow => &[modeler3d_core::PrimitiveKind::Torus],
            ReviewState::HollowDialog | ReviewState::HollowError => {
                &[modeler3d_core::PrimitiveKind::Sphere]
            }
            ReviewState::Measure | ReviewState::MeasureStart => &[
                modeler3d_core::PrimitiveKind::Cube,
                modeler3d_core::PrimitiveKind::Cylinder,
            ],
        };
        for (index, kind) in primitives.iter().copied().enumerate() {
            self.dispatch(CommandId::AddPrimitive(kind));
            let placement = self.commands.scene().objects().last().map(|object| {
                let resting = object.position();
                (
                    object.object_id().to_owned(),
                    [index as f32 * 45.0, resting[1], resting[2]],
                )
            });
            if let Some((id, position)) = placement {
                let _ = self.commands.set_object_position_by_id(&id, position);
            }
        }
        if review.state() == ReviewState::Unselected {
            self.commands.set_selection_indices(std::iter::empty());
        }
        match review.state() {
            ReviewState::Material => self.dispatch(CommandId::Material),
            ReviewState::Settings => self.open_dialog(DialogKind::Settings),
            ReviewState::Save => self.open_dialog(DialogKind::SaveProject),
            ReviewState::Export => self.open_dialog(DialogKind::Export),
            ReviewState::Menu => self.menu_open = true,
            ReviewState::Mirror => self.dispatch(CommandId::Mirror),
            ReviewState::Paste => {
                self.dispatch(CommandId::Copy);
                self.dispatch(CommandId::Paste);
                self.dispatch(CommandId::Paste);
            }
            ReviewState::Repair => {
                self.dispatch(CommandId::Repair);
                self.apply_dialog_action(crate::dialogs::DialogAction::Repair);
            }
            ReviewState::Align => {
                self.commands.set_selection_indices([0, 1]);
                self.dispatch(CommandId::Align);
            }
            ReviewState::SettingsNote => {
                // The registry read is replaced by one supported value; the
                // rest is the real first-start import into the isolated file.
                let value = match review.theme() {
                    ThemeMode::Light => "light",
                    ThemeMode::Dark => "dark",
                };
                let current = std::collections::BTreeMap::from([(
                    "theme_mode".to_owned(),
                    vec![value.to_owned()],
                )]);
                let legacy = modeler3d_platform::LegacySettings {
                    current_present: current.keys().cloned().collect(),
                    current,
                    ..modeler3d_platform::LegacySettings::default()
                };
                // A first start has no Rust settings file; the harness has
                // already written its own isolated one by now.
                let _ = std::fs::remove_file(&self.settings_path);
                self.load_preferences_from_legacy(&legacy);
                self.theme_mode = review.theme();
                self.ui_size = review.size();
            }
            _ => {}
        }
        if review.state() == ReviewState::Colors {
            let colors = [
                [0.86, 0.16, 0.16, 1.0],
                [0.18, 0.66, 0.24, 1.0],
                [0.16, 0.36, 0.90, 1.0],
                [0.95, 0.80, 0.15, 1.0],
            ];
            for (index, color) in colors.into_iter().enumerate() {
                self.commands.set_selection_indices([index]);
                let _ = self.commands.set_selected_color(color);
            }
            self.commands.set_selection_indices([0, 2]);
        }
        if matches!(
            review.state(),
            ReviewState::Measure | ReviewState::MeasureStart
        ) {
            self.stage_review_ruler(review.state() == ReviewState::Measure);
        }
        if matches!(
            review.state(),
            ReviewState::Paint | ReviewState::PaintPicker
        ) {
            let top_face = self.commands.scene().objects().first().and_then(|cube| {
                cube.mesh()
                    .faces
                    .iter()
                    .position(|face| cube.mesh().face_normal(*face)[1] > 0.9)
            });
            if let Some(face) = top_face {
                let _ = self
                    .commands
                    .paint_at(0, face, 0.0, [0.86, 0.16, 0.16, 1.0]);
            }
            let _ = self.commands.paint_at(1, 0, 1.0, [0.16, 0.36, 0.90, 1.0]);
            self.commands.set_selection_indices(std::iter::empty());
            self.dispatch(CommandId::Paint);
            if let Some(mode) = self.paint_mode.as_mut() {
                mode.coverage = 0.0;
                if review.state() == ReviewState::PaintPicker {
                    mode.open_colour_picker();
                }
            }
        }
        if matches!(
            review.state(),
            ReviewState::Texture | ReviewState::TextureDialog
        ) {
            let pictures = crate::review_capture::review_texture_paths();
            let count = self.commands.scene().objects().len();
            for index in 0..count {
                self.commands.set_selection_indices([index]);
                let picture = pictures.get(index % pictures.len().max(1)).cloned();
                let _ = self
                    .commands
                    .apply_texture_to_selection(picture.as_deref(), 20.0);
            }
            if review.state() == ReviewState::Texture {
                self.commands.set_selection_indices(std::iter::empty());
            } else {
                self.commands.set_selection_indices([0]);
                self.dispatch(CommandId::Material);
            }
        }
        if review.state() == ReviewState::Xray {
            if let Some(id) = self
                .commands
                .scene()
                .objects()
                .get(1)
                .map(|object| object.object_id().to_owned())
            {
                let _ = self
                    .commands
                    .set_object_position_by_id(&id, [-14.0, 20.0, 6.0]);
            }
            self.commands.set_selection_indices([0]);
            let _ = self.commands.set_selected_color([0.29, 0.76, 0.38, 1.0]);
            self.commands.set_selection_indices([1]);
            let _ = self.commands.set_selected_color([0.16, 0.36, 0.90, 1.0]);
            self.commands.set_selection_indices([0]);
            if !self.commands.display_state().xray {
                self.dispatch(CommandId::ToggleXray);
            }
        }
        if review.state() == ReviewState::Grab {
            self.commands.set_selection_indices([0]);
            let center = self.commands.scene().objects().first().and_then(|sphere| {
                sphere
                    .mesh()
                    .vertices
                    .iter()
                    .copied()
                    .max_by(|first, second| {
                        (first[0] + first[1]).total_cmp(&(second[0] + second[1]))
                    })
            });
            if let Some(center) = center {
                let settings = self.brush_settings;
                let stroke = BrushStroke::Grab {
                    center_mm: center,
                    offset_mm: [9.0, 9.0, 0.0],
                    options: ElasticGrabOptions {
                        radius_mm: settings.grab_radius_mm,
                        strength: settings.grab_strength,
                        falloff: settings.grab_falloff,
                        symmetry: settings.grab_symmetry,
                        limits: Default::default(),
                    },
                };
                let _ = self.commands.apply_brush_stroke(stroke);
            }
        }
        if matches!(
            review.state(),
            ReviewState::Crease | ReviewState::CreaseSharp
        ) {
            if review.state() == ReviewState::CreaseSharp {
                // OF-59: full Sharpness, with the Crease settings showing the slider.
                self.brush_settings.crease_sharpness = 1.0;
                self.active_brush = Some(ActiveBrush::Crease);
                self.brush_settings_open = true;
            }
            self.commands.set_selection_indices([0]);
            let radius = self
                .commands
                .scene()
                .objects()
                .first()
                .map_or(0.0, |sphere| {
                    sphere
                        .mesh()
                        .vertices
                        .iter()
                        .map(|vertex| vertex[1])
                        .fold(0.0_f32, f32::max)
                });
            // A curved stroke over the top, from front left to back right.
            let centers: Vec<[f32; 3]> = (0..=16)
                .map(|step| {
                    let x = -0.6 * radius + 1.2 * radius * step as f32 / 16.0;
                    let z = 0.3 * x;
                    let y = (radius * radius - x * x - z * z).max(0.0).sqrt();
                    [x, y, z]
                })
                .collect();
            let settings = self.brush_settings;
            let stroke = BrushStroke::Crease {
                centers_mm: centers,
                options: CreasePolishOptions {
                    radius_mm: settings.crease_radius_mm,
                    strength: settings.crease_strength,
                    direction: settings.crease_direction,
                    falloff: settings.crease_falloff,
                    sharpness: settings.crease_sharpness,
                    limits: Default::default(),
                },
            };
            let _ = self.commands.apply_brush_stroke(stroke);
        }
        if matches!(
            review.state(),
            ReviewState::HollowDialog | ReviewState::HollowError
        ) {
            self.commands.set_selection_indices([0]);
            self.dispatch(CommandId::Hollow);
            if review.state() == ReviewState::HollowError {
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.set_hollow_wall(1.0);
                }
                self.apply_dialog_action(crate::dialogs::DialogAction::Hollow(
                    modeler3d_core::HollowOptions {
                        wall_thickness_mm: 1.0,
                        ..Default::default()
                    },
                ));
            }
        }
        if review.state() == ReviewState::Hollow {
            self.commands.set_selection_indices([0]);
            // A 40 mm torus with an 8 mm tube; adding a torus opens Edit Torus.
            self.apply_dialog_action(crate::dialogs::DialogAction::Torus(
                modeler3d_core::TorusParameters {
                    radius_mm: 20.0,
                    tube_mm: 8.0,
                    sides: 64,
                    steps: 48,
                    ..Default::default()
                },
            ));
            self.dialog = None;
            match self.commands.hollow_selected(1.5) {
                Ok(message) => self.commands.set_status(message),
                Err(error) => self.commands.set_status(error.to_string()),
            }
            self.commands.set_selection_indices(std::iter::empty());
            if !self.commands.display_state().xray {
                self.dispatch(CommandId::ToggleXray);
            }
        }
        if review.state() == ReviewState::Subdivide {
            let limits = modeler3d_core::MeshOperationLimits::default();
            for index in 0..2 {
                let source = self
                    .commands
                    .scene()
                    .objects()
                    .get(index)
                    .map(|object| (object.object_id().to_owned(), object.mesh().clone()));
                let Some((id, mesh)) = source else {
                    continue;
                };
                let refined = if index == 0 {
                    modeler3d_core::subdivide_mesh(&mesh)
                        .and_then(|once| modeler3d_core::subdivide_mesh(&once.mesh))
                        .map(|twice| twice.mesh)
                } else {
                    modeler3d_core::subdivide_mesh_even(&mesh, None, limits).map(|even| even.mesh)
                };
                if let Ok(refined) = refined {
                    let generation = self.commands.document_generation();
                    let revision = self.commands.scene().revision();
                    let _ = self
                        .commands
                        .replace_mesh_if_current(generation, revision, &id, refined);
                }
            }
            self.commands.set_selection_indices([1]);
            self.dispatch(CommandId::Subdivide);
        }
        if review.state() == ReviewState::Simplify {
            self.commands.set_selection_indices([0]);
            let source = self
                .commands
                .scene()
                .objects()
                .first()
                .map(|object| (object.object_id().to_owned(), object.mesh().clone()));
            if let Some((id, mesh)) = source
                && let Ok(result) = modeler3d_core::simplify_mesh_by_percent(
                    &mesh,
                    50,
                    modeler3d_core::MeshOperationLimits::default(),
                )
            {
                let generation = self.commands.document_generation();
                let revision = self.commands.scene().revision();
                let message = format!(
                    "Simplify: {} \u{2192} {} faces",
                    crate::desktop_jobs::group_thousands(result.report.before_faces),
                    crate::desktop_jobs::group_thousands(result.report.after_faces)
                );
                let _ =
                    self.commands
                        .replace_mesh_if_current(generation, revision, &id, result.mesh);
                self.dispatch(CommandId::Simplify);
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.set_message(message);
                }
            }
        }
        if review.state() == ReviewState::SimplifyArea {
            self.commands.set_selection_indices([0]);
            let radius = self
                .commands
                .scene()
                .objects()
                .first()
                .map_or(0.0, |sphere| {
                    sphere
                        .mesh()
                        .vertices
                        .iter()
                        .map(|vertex| vertex[1])
                        .fold(0.0_f32, f32::max)
                });
            // A stroke over the top, from front left to back right.
            let centers: Vec<[f32; 3]> = (0..=16)
                .map(|step| {
                    let x = -0.6 * radius + 1.2 * radius * step as f32 / 16.0;
                    let z = 0.3 * x;
                    let y = (radius * radius - x * x - z * z).max(0.0).sqrt();
                    [x, y, z]
                })
                .collect();
            self.dispatch(CommandId::Simplify);
            if let Some(dialog) = self.dialog.as_mut() {
                dialog.set_simplify_area(6.0, 50);
            }
            self.apply_dialog_action(crate::dialogs::DialogAction::SimplifyArea(Some((6.0, 50))));
            let stroke = BrushStroke::SimplifyArea {
                centers_mm: centers,
                radius_mm: 6.0,
                percent: 50,
            };
            if let Ok(outcome) = self.commands.apply_brush_stroke(stroke)
                && let Some(dialog) = self.dialog.as_mut()
            {
                let message = match outcome {
                    crate::commands::CommandOutcome::Applied(message)
                    | crate::commands::CommandOutcome::Noop(message) => message,
                    _ => String::new(),
                };
                dialog.set_message(message);
            }
        }
        if review.state() == ReviewState::SmoothBrush {
            self.commands.set_selection_indices([0]);
            let top = self.commands.scene().objects().first().map(|cylinder| {
                let vertices = &cylinder.mesh().vertices;
                let top = vertices
                    .iter()
                    .map(|vertex| vertex[1])
                    .fold(f32::MIN, f32::max);
                let radius = vertices
                    .iter()
                    .map(|vertex| (vertex[0] * vertex[0] + vertex[2] * vertex[2]).sqrt())
                    .fold(0.0_f32, f32::max);
                (top, radius)
            });
            if let Some((top, radius)) = top {
                // Three passes around the top rim.
                let centers: Vec<[f32; 3]> = (0..48)
                    .map(|step| {
                        let angle = step as f32 * std::f32::consts::TAU / 16.0;
                        [radius * angle.cos(), top, radius * angle.sin()]
                    })
                    .collect();
                let settings = self.brush_settings;
                let stroke = BrushStroke::SmoothBrush {
                    centers_mm: centers,
                    options: SmoothBrushOptions {
                        radius_mm: settings.smooth_radius_mm,
                        strength: settings.smooth_strength,
                        shrinkage: settings.smooth_shrinkage,
                        falloff: settings.smooth_falloff,
                        limits: Default::default(),
                    },
                };
                let _ = self.commands.apply_brush_stroke(stroke);
            }
        }
        if review.state() == ReviewState::MergeApart {
            if let Some(id) = self
                .commands
                .scene()
                .objects()
                .get(1)
                .map(|object| object.object_id().to_owned())
            {
                let _ = self
                    .commands
                    .set_object_position_by_id(&id, [90.0, 20.0, 0.0]);
            }
            self.commands.set_selection_indices([0, 1]);
            self.dispatch(CommandId::Merge);
        }
        if review.state() == ReviewState::Merge {
            let colors = [[0.86, 0.16, 0.16, 1.0], [0.16, 0.36, 0.90, 1.0]];
            for (index, color) in colors.into_iter().enumerate() {
                self.commands.set_selection_indices([index]);
                let _ = self.commands.set_selected_color(color);
            }
            if let Some(id) = self
                .commands
                .scene()
                .objects()
                .get(1)
                .map(|object| object.object_id().to_owned())
            {
                let _ = self
                    .commands
                    .set_object_position_by_id(&id, [22.0, 20.0, 12.0]);
            }
            self.commands.set_selection_indices(std::iter::empty());
            self.commands.set_selection_indices([1, 0]);
            self.dispatch(CommandId::Merge);
        }
        self.transform_panel_mode = match review.state() {
            ReviewState::Rotate => TransformPanelMode::Rotate,
            ReviewState::Resize | ReviewState::ResizeDrag => TransformPanelMode::Resize,
            _ => TransformPanelMode::Move,
        };
        if review.state() != ReviewState::Welcome {
            self.surface = WorkspaceSurface::Scene;
        }
        for toggle in review.view_toggles() {
            match toggle.as_str() {
                "wireframe" => self.dispatch(CommandId::ToggleWireframe),
                "xray" => self.dispatch(CommandId::ToggleXray),
                other => eprintln!("review capture: unknown view toggle `{other}`"),
            }
        }
        self.camera_auto_frame_pending = true;
        self.invalidate_presentation();
        if matches!(
            review.state(),
            ReviewState::BrushSettings | ReviewState::BrushDismissed
        ) {
            self.commands.set_selection_indices([0]);
            self.active_tab = RibbonTab::Edit;
            self.dispatch(CommandId::Grab);
            if review.state() == ReviewState::BrushDismissed {
                self.commands.deselect_all();
                self.dispatch(CommandId::Grab);
            }
        }
        if matches!(
            review.state(),
            ReviewState::EditSimplify
                | ReviewState::EditSmooth
                | ReviewState::EditBlob
                | ReviewState::EditOff
        ) {
            self.active_tab = RibbonTab::Edit;
            self.commands.preferences_mut().set_ribbon_favorites(vec![
                "Edit:Simplify".into(),
                "Edit:Smooth".into(),
                "Edit:Blob".into(),
            ]);
            self.commands.set_selection_indices([0]);
            self.dispatch(CommandId::Simplify);
            self.commands.deselect_all();
            if review.state() != ReviewState::EditSimplify {
                self.dispatch(CommandId::Smooth);
            }
            if matches!(review.state(), ReviewState::EditBlob | ReviewState::EditOff) {
                self.dispatch(CommandId::Blob);
            }
            if review.state() == ReviewState::EditOff {
                self.dispatch(CommandId::Blob);
            }
        }
        self.review = Some(review);
    }

    fn poll_mcp_bridge(&mut self, context: &egui::Context) {
        if let Some(mut bridge) = self.mcp_bridge.take() {
            bridge.poll(self, context);
            self.mcp_bridge = Some(bridge);
        }
    }

    fn initialize_viewport(&mut self, creation_context: &eframe::CreationContext<'_>) {
        let Some(render_state) = creation_context.wgpu_render_state.clone() else {
            self.set_viewport_error(
                "viewport initialization",
                String::from("eframe did not provide a native wgpu render state"),
            );
            return;
        };
        let profile = match discover_capability_profile_blocking() {
            Ok(profile) => profile,
            Err(error) => {
                self.set_viewport_error("viewport capability discovery", error);
                return;
            }
        };
        let config = RendererConfig {
            width: 1,
            height: 1,
            adapter_index: None,
            capture_path: None,
        };
        match ViewportRenderer::new_on_shared_device(
            &profile,
            &config,
            &render_state.instance,
            &render_state.adapter,
            &render_state.device,
            &render_state.queue,
        ) {
            Ok(renderer) => {
                let adapter = renderer.adapter_info();
                crate::diagnostics::record_info(
                    "viewport initialized",
                    format!(
                        "adapter={} / {} / {}; driver={} ({})",
                        adapter.name,
                        adapter.backend,
                        adapter.device_type,
                        adapter.driver,
                        adapter.driver_info
                    ),
                );
                let texture_id = render_state.renderer.write().register_native_texture(
                    &render_state.device,
                    renderer.color_view(),
                    eframe::wgpu::FilterMode::Linear,
                );
                self.capability_profile = Some(profile);
                self.viewport_renderer = Some(renderer);
                self.viewport_texture_id = Some(texture_id);
                self.wgpu_render_state = Some(render_state);
            }
            Err(error) => self.set_viewport_error("viewport renderer creation", error.to_string()),
        }
    }

    fn set_viewport_error(&mut self, context: &str, error: String) {
        if self.viewport_error.as_deref() != Some(error.as_str()) {
            crate::diagnostics::record_error(context, &error);
        }
        self.viewport_error = Some(error);
    }

    fn load_preferences(&mut self) {
        let legacy = read_legacy_settings();
        self.load_preferences_from_legacy(&legacy);
    }

    fn load_preferences_from_legacy(&mut self, legacy: &modeler3d_platform::LegacySettings) {
        match crate::preferences::Preferences::load_with_legacy(&self.settings_path, legacy) {
            Ok(report) => {
                *self.commands.preferences_mut() = report.preferences;
                self.theme_mode = self.commands.preferences().theme_mode();
                self.ui_size = self.commands.preferences().ui_size();
                self.ribbon_pinned = self.commands.preferences().ribbon_pinned();
                // Only a first start gets the default shortcuts. A saved bar,
                // even one the person emptied, is theirs and is kept.
                if !report.ribbon_favorites_saved {
                    self.commands
                        .preferences_mut()
                        .set_ribbon_favorites(default_favorites());
                }
                let enabled = self.commands.preferences().object_snapping_enabled();
                self.commands.set_snap_enabled(enabled);
                if !report.warnings.is_empty() {
                    self.settings_warning = Some(report.warnings.join(" "));
                    self.commands.set_status("Some settings need review");
                }
                if report.migrated_legacy {
                    self.persist_preferences();
                    self.commands
                        .set_status("Python preferences migrated to Rust settings");
                }
            }
            Err(error) => {
                self.invalid_settings_file = self.settings_path.is_file();
                self.settings_warning = Some(format!(
                    "Settings could not be read; the original file was left untouched. Defaults are active: {error}"
                ));
                self.commands.set_status("Settings need repair");
                self.commands
                    .preferences_mut()
                    .set_ribbon_favorites(default_favorites());
                self.commands.set_snap_enabled(true);
                self.ribbon_pinned = false;
            }
        }
    }

    fn persist_preferences(&mut self) {
        if self.invalid_settings_file && self.settings_path.is_file() {
            match preserve_invalid_settings(&self.settings_path) {
                Ok(backup) => {
                    self.settings_warning = Some(format!(
                        "The unreadable settings file was preserved at {}; valid settings are saved separately.",
                        backup.display()
                    ));
                }
                Err(error) => {
                    self.commands.set_status(format!(
                        "Could not preserve unreadable settings; no settings were overwritten: {error}"
                    ));
                    return;
                }
            }
        }
        if let Err(error) = self.commands.save_preferences_to_path(&self.settings_path) {
            self.commands
                .set_status(format!("Could not save settings: {error}"));
        } else {
            self.invalid_settings_file = false;
        }
    }

    fn window_title(&self) -> String {
        let project_name = self
            .commands
            .project_path()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled");
        let marker = if self.commands.is_dirty() { " *" } else { "" };
        format!("3D Modeler - {project_name}{marker}")
    }

    fn invalidate_presentation(&mut self) {
        self.transform_signature.clear();
        self.preview_revision = u64::MAX;
    }

    fn synchronize_surface(&mut self) {
        if !self.commands.scene().objects().is_empty() {
            self.surface = WorkspaceSurface::Scene;
        }
        self.invalidate_presentation();
    }

    /// Updates the visible workspace after an MCP request changes the active
    /// document. Project replacements always enter the scene workspace, even
    /// when the opened project is empty.
    pub(crate) fn synchronize_mcp_presentation(
        &mut self,
        previous_project_id: &str,
        previous_revision: u64,
        project_replaced: bool,
    ) {
        if project_replaced || self.commands.scene().project_id() != previous_project_id {
            self.finish_new_scene_presentation();
            self.persist_preferences();
        } else if self.commands.scene().revision() != previous_revision {
            self.synchronize_surface();
        }
    }

    /// Dispatches one command and routes typed dialog/exit effects to the
    /// shell. This is the only command entry point used by widgets.
    pub(crate) fn dispatch(&mut self, command: CommandId) {
        // Dismissing a selected modifier does not require a mesh selection.
        if self.command_selected(command) && self.dismiss_modifier(command) {
            return;
        }
        if command == CommandId::Paint {
            self.toggle_paint_mode();
            return;
        }
        if command == CommandId::Measure {
            self.toggle_measure_mode();
            return;
        }
        // Other commands close the Measure bar; Undo, Redo and the view
        // commands work while measuring.
        if self.measure_mode.is_some() && !measure_keeps_running_for(command) {
            self.finish_measure_mode();
        }
        // Any other command closes the Paint bar and keeps the paint;
        // Undo and Redo work inside Paint.
        if self.paint_mode.is_some() && !matches!(command, CommandId::Undo | CommandId::Redo) {
            self.finish_paint_mode(true);
        }
        if is_edit_tool(command) {
            // Tool intent is independent of the eventual mesh target.
            self.apply_dialog_action(DialogAction::Close);
            self.active_brush = None;
            self.brush_settings_open = false;
            self.viewport_drag = None;
            self.preview_mesh_override = None;
            self.boolean_pick = None;
            self.placement_command = None;
            if let Some(kind) = modifier_dialog(command)
                && (!matches!(kind, DialogKind::EditAction(_))
                    || self.commands.selected_count() == 0)
            {
                self.open_dialog(kind);
                return;
            }
        }
        let mesh_operation = match command {
            CommandId::Smooth => Some(MeshJobOperation::Smooth),
            _ => None,
        };
        if let Some(operation) = mesh_operation {
            self.start_mesh_job(operation, false);
            self.synchronize_surface();
            return;
        }
        if let Some(brush) = ActiveBrush::from_command(command) {
            self.toggle_brush(brush);
            return;
        }
        if let Some(operation) = boolean_operation_for_command(command) {
            self.dispatch_boolean(operation);
            return;
        }
        if matches!(
            command,
            CommandId::Mirror | CommandId::Align | CommandId::Arrange
        ) {
            self.placement_command = Some(command);
            self.placement_axis = Axis::X;
            self.placement_alignment = Alignment::Center;
            self.arrange_gap_field = String::from("0 mm");
            self.transform_error = None;
            return;
        }
        if command == CommandId::Material && self.commands.selected_count() == 0 {
            self.commands.set_status("Select an object to paint");
            return;
        }
        if command == CommandId::NewScene && self.commands.is_dirty() {
            self.confirm_action = Some(PendingReplacement::NewScene);
            return;
        }
        match self.commands.dispatch(command) {
            Ok(CommandOutcome::OpenDialog(kind)) => self.open_dialog(kind),
            Ok(CommandOutcome::ExitRequested) => self.request_exit(),
            Ok(CommandOutcome::Applied(message)) | Ok(CommandOutcome::Noop(message)) => {
                self.commands.set_status(message);
            }
            Err(error) => self.report_command_error(error),
        }
        if command == CommandId::Modeler3dMenu {
            self.surface = WorkspaceSurface::Welcome;
        }
        if command == CommandId::NewScene {
            self.finish_new_scene_presentation();
        }
        match command {
            CommandId::ResetView => {
                self.camera.reset_view();
                self.camera_auto_frame_pending = true;
            }
            CommandId::CenterView | CommandId::FrameSelection => {
                self.camera_auto_frame_pending = true;
            }
            CommandId::ViewTop => {
                let _ = self.camera.set_view(0.0, 89.0);
            }
            CommandId::ViewSide => {
                let _ = self.camera.set_view(90.0, 35.0);
            }
            _ => {}
        }
        if command == CommandId::ToggleSnap {
            let enabled = self.commands.snap_enabled();
            self.commands
                .preferences_mut()
                .set_object_snapping_enabled(enabled);
        }
        self.synchronize_surface();
        self.persist_preferences();
    }

    fn toggle_brush(&mut self, brush: ActiveBrush) {
        self.viewport_drag = None;
        self.preview_mesh_override = None;
        self.boolean_pick = None;
        self.measure_mode = None;
        if self.active_brush == Some(brush) {
            self.active_brush = None;
            self.brush_settings_open = false;
            self.commands
                .set_status(format!("{} deactivated", brush.label()));
        } else {
            self.active_brush = Some(brush);
            self.brush_settings_open = true;
            self.commands.set_status(format!(
                "{} active — hold Shift and drag on a mesh",
                brush.label()
            ));
        }
        self.invalidate_presentation();
    }

    fn dismiss_modifier(&mut self, command: CommandId) -> bool {
        if modifier_dialog(command).is_some() {
            self.apply_dialog_action(DialogAction::Close);
            self.boolean_pick = None;
        } else if self.placement_command == Some(command) {
            self.placement_command = None;
            self.transform_error = None;
        } else if let Some(brush) = ActiveBrush::from_command(command) {
            self.toggle_brush(brush);
        } else {
            return false;
        }
        true
    }

    fn dispatch_boolean(&mut self, operation: modeler3d_core::BooleanOperation) {
        let selected = self.commands.scene().ordered_selection();
        match selected.len() {
            0 => self.commands.set_status(format!(
                "{} requires a selected object",
                boolean_label(operation)
            )),
            1 => {
                let source_index = selected[0];
                if operation == modeler3d_core::BooleanOperation::Difference {
                    let candidates = self
                        .commands
                        .scene()
                        .objects()
                        .iter()
                        .enumerate()
                        .filter(|(index, object)| *index != source_index && object.is_visible())
                        .map(|(index, _)| index)
                        .collect::<Vec<_>>();
                    if candidates.len() == 1 {
                        self.apply_boolean_pair(candidates[0], source_index, operation, true);
                    } else {
                        self.start_boolean_pick(operation, source_index, true, true);
                    }
                } else {
                    self.start_boolean_pick(operation, source_index, false, false);
                }
            }
            2 => self.apply_boolean_pair(selected[0], selected[1], operation, false),
            _ => self.commands.set_status(format!(
                "{} requires exactly two selected objects",
                boolean_label(operation)
            )),
        }
    }

    fn start_boolean_pick(
        &mut self,
        operation: modeler3d_core::BooleanOperation,
        source_index: usize,
        source_is_tool: bool,
        clear_selection: bool,
    ) {
        self.active_brush = None;
        self.viewport_drag = None;
        self.preview_mesh_override = None;
        self.boolean_pick = Some(BooleanPick {
            operation,
            source_index,
            source_is_tool,
            clear_selection,
        });
        self.commands.set_status(format!(
            "{}: click the {} object; Escape cancels",
            boolean_label(operation),
            if source_is_tool { "target" } else { "tool" }
        ));
    }

    fn apply_boolean_pair(
        &mut self,
        target_index: usize,
        tool_index: usize,
        operation: modeler3d_core::BooleanOperation,
        clear_selection: bool,
    ) {
        match self
            .commands
            .apply_boolean_pair(target_index, tool_index, operation, clear_selection)
        {
            Ok(CommandOutcome::Applied(message)) | Ok(CommandOutcome::Noop(message)) => {
                self.commands.set_status(message);
                self.boolean_pick = None;
                self.invalidate_presentation();
                self.persist_preferences();
            }
            Ok(CommandOutcome::OpenDialog(_) | CommandOutcome::ExitRequested) => {
                self.commands
                    .set_status("Boolean route returned an unexpected shell action");
                self.boolean_pick = None;
            }
            Err(error) => self.report_command_error(error),
        }
    }

    fn request_exit(&mut self) {
        if self.commands.is_dirty() {
            self.dialog = None;
            self.dialog_generation = self.dialog_generation.wrapping_add(1);
            self.pending_after_save = None;
            self.confirm_action = Some(PendingReplacement::Exit);
        } else {
            self.close_requested = true;
        }
    }

    fn handle_native_close_request(&mut self, ctx: &egui::Context) {
        if self.close_requested {
            return;
        }
        if self.commands.is_dirty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.request_exit();
        } else {
            self.close_requested = true;
        }
    }

    fn finish_confirmation(&mut self, choice: ConfirmationChoice) {
        let Some(action) = self.confirm_action.take() else {
            return;
        };
        match choice {
            ConfirmationChoice::Cancel => self.commands.set_status("Cancelled"),
            ConfirmationChoice::Save => self.save_before_replacement(action),
            ConfirmationChoice::Discard => self.complete_replacement(action),
        }
    }

    fn save_before_replacement(&mut self, action: PendingReplacement) {
        if let Some(path) = self.commands.project_path().map(Path::to_path_buf) {
            if let Err(error) = self.commands.save_document_to_path(&path) {
                self.report_command_error(error);
                self.confirm_action = Some(action);
                return;
            }
            self.complete_replacement(action);
        } else {
            self.pending_after_save = Some(action);
            self.open_dialog(DialogKind::SaveProject);
        }
    }

    fn complete_replacement(&mut self, action: PendingReplacement) {
        match action {
            PendingReplacement::NewScene => match self.commands.dispatch(CommandId::NewScene) {
                Ok(_) => {
                    self.finish_new_scene_presentation();
                    self.commands.set_status("New scene");
                    self.persist_preferences();
                }
                Err(error) => self.report_command_error(error),
            },
            PendingReplacement::OpenProject(path) => match self.open_project(&path) {
                Ok(_) => {}
                Err(error) => self.report_command_error(error),
            },
            PendingReplacement::Exit => self.close_requested = true,
        }
    }

    fn finish_overwrite_confirmation(&mut self, overwrite: bool) {
        let Some(pending) = self.pending_overwrite.take() else {
            return;
        };
        if !overwrite {
            self.commands.set_status("Cancelled");
            return;
        }
        let action = match pending {
            PendingOverwrite::SaveProject(path) => DialogAction::Save(path, false),
            PendingOverwrite::Export(path) => DialogAction::Export(path, false),
        };
        self.apply_dialog_action_with_overwrite(action, true);
    }

    fn finish_new_scene_presentation(&mut self) {
        self.reset_document_caches();
        self.surface = WorkspaceSurface::Scene;
        self.selection_anchor = None;
        self.camera.reset_view();
        self.camera_auto_frame_pending = true;
        self.viewport_drag = None;
        self.active_brush = None;
        self.boolean_pick = None;
        self.preview_mesh_override = None;
        self.invalidate_presentation();
    }

    fn open_project(&mut self, path: &Path) -> Result<String, CommandError> {
        self.commands.open_from_path(path)?;
        self.reset_document_caches();
        self.surface = WorkspaceSurface::Scene;
        self.selection_anchor = None;
        self.camera.reset_view();
        self.camera_auto_frame_pending = true;
        self.viewport_drag = None;
        self.active_brush = None;
        self.boolean_pick = None;
        self.preview_mesh_override = None;
        self.invalidate_presentation();
        self.persist_preferences();
        Ok(format!("Opened {}", path.display()))
    }

    fn reset_document_caches(&mut self) {
        self.item_revision = u64::MAX;
        self.item_textures.clear();
        self.item_health.clear();
        self.gizmo_frame = None;
        self.measure_mode = None;
    }

    fn open_import_with_picker(&mut self, picker: impl FnOnce(&Path) -> Option<PathBuf>) {
        let directory = self
            .commands
            .preferences()
            .last_import_directory()
            .filter(|path| path.is_dir())
            .map(Path::to_path_buf)
            .unwrap_or_else(modeler3d_platform::documents_directory);
        if let Some(path) = picker(&directory) {
            self.dialog_generation = self.dialog_generation.wrapping_add(1);
            self.preview_mesh_override = None;
            let unit = self.commands.preferences().default_import_unit();
            // Keep a useful retry/error dialog if parsing fails.
            let mut dialog = DialogState::new(DialogKind::ImportModel, self.theme_mode);
            dialog.set_import_unit(unit);
            dialog.set_import_path(&path);
            self.dialog = Some(dialog);
            self.apply_dialog_action(DialogAction::Import(path, unit));
        }
    }

    fn open_dialog(&mut self, kind: DialogKind) {
        if kind == DialogKind::ImportModel {
            self.open_import_with_picker(native_import_picker);
            return;
        }
        self.dialog_generation = self.dialog_generation.wrapping_add(1);
        self.preview_mesh_override = None;
        let mut dialog = DialogState::new(kind, self.theme_mode);
        dialog.set_import_unit(self.commands.preferences().default_import_unit());
        dialog.set_display_unit(self.commands.preferences().display_unit());
        dialog.set_viewport_lighting(self.commands.preferences().viewport_lighting());
        dialog.set_preferred_slicer(
            self.commands
                .preferences()
                .preferred_slicer_path()
                .map(Path::to_path_buf),
            self.commands
                .preferences()
                .preferred_slicer_label()
                .map(str::to_owned),
        );
        dialog.set_recent_file_count(self.commands.preferences().recent_files().paths().len());
        dialog.sync_ui_size(self.ui_size);
        if matches!(kind, DialogKind::SaveProject | DialogKind::Export) {
            dialog.set_suggested_path(self.commands.project_path());
        }
        if kind == DialogKind::Torus {
            dialog.load_torus_from_state(&self.commands);
        }
        if kind == DialogKind::Material {
            dialog.seed_material_color(&self.commands);
        }
        self.dialog = Some(dialog);
    }

    fn start_mesh_job(&mut self, operation: MeshJobOperation, preserve_dialog: bool) {
        let Some(index) = self.commands.scene().selection().iter().next().copied() else {
            let message = format!("{} requires a selected object", operation.label());
            self.commands.set_status(message.clone());
            if preserve_dialog && let Some(dialog) = self.dialog.as_mut() {
                dialog.set_error(message);
            }
            return;
        };
        let Ok(object) = self.commands.scene().object(index) else {
            self.commands
                .set_status("Selected object is no longer available");
            return;
        };
        let target_object_id = object.object_id().to_owned();
        let mesh = object.mesh().clone();
        let request = DesktopJobRequest::Mesh { operation, mesh };
        self.start_desktop_job(
            operation.label(),
            request,
            Some(target_object_id),
            preserve_dialog,
        );
    }

    fn start_print_prep_job(&mut self, bed_size_mm: f32) {
        let snapshot = self.commands.scene().snapshot();
        self.start_desktop_job(
            "Print preparation",
            DesktopJobRequest::PrintPrep {
                snapshot,
                bed_size_mm,
            },
            None,
            true,
        );
    }

    fn start_print_staging_job(&mut self, slicer: SlicerInfo) {
        let snapshot = self.commands.scene().snapshot();
        let document_generation = self.commands.document_generation();
        self.start_desktop_job(
            "Prepare slicer handoff",
            DesktopJobRequest::PreparePrint {
                snapshot,
                slicer,
                temp_root: current_runtime_paths().temp_dir,
                document_generation,
            },
            None,
            true,
        );
    }

    /// The top-bar Slicer button. With a saved slicer it exports the visible
    /// objects straight to it, with no dialog; the first time, it opens the
    /// chooser. The saved slicer is changed only in Settings.
    fn slicer_button_clicked(&mut self) {
        let preferences = self.commands.preferences();
        let Some(path) = preferences.preferred_slicer_path().map(Path::to_path_buf) else {
            self.slicer_choices = crate::print_handoff::discover_print_slicers();
            self.slicer_chooser_open = true;
            return;
        };
        match slicer_info_from_path(&path, preferences.preferred_slicer_label()) {
            Ok(slicer) => self.send_to_slicer(slicer),
            Err(error) => self.commands.set_status(format!(
                "Saved slicer is unavailable ({error}); change it in Settings"
            )),
        }
    }

    /// Remembers the slicer picked in the first-use chooser and sends to it.
    fn choose_slicer(&mut self, slicer: SlicerInfo) {
        self.slicer_chooser_open = false;
        self.slicer_choices.clear();
        self.apply_preferred_slicer(Some(slicer.path.clone()), Some(slicer.label.clone()));
        self.send_to_slicer(slicer);
    }

    fn send_to_slicer(&mut self, slicer: SlicerInfo) {
        if self.desktop_job.is_some() {
            self.commands
                .set_status("Cannot send to the slicer: another operation is still running");
            return;
        }
        let snapshot = self.commands.scene().snapshot();
        let document_generation = self.commands.document_generation();
        self.start_desktop_job(
            "Send to slicer",
            DesktopJobRequest::PreparePrint {
                snapshot,
                slicer,
                temp_root: current_runtime_paths().temp_dir,
                document_generation,
            },
            None,
            false,
        );
        if let Some(job) = self.desktop_job.as_mut() {
            job.launch_when_staged = true;
        }
    }

    fn start_desktop_job(
        &mut self,
        title: &str,
        request: DesktopJobRequest,
        target_object_id: Option<String>,
        preserve_dialog: bool,
    ) {
        if self.desktop_job.is_some() {
            let message = format!("Cannot start {title}: another operation is still running");
            self.commands.set_status(message.clone());
            if preserve_dialog && let Some(dialog) = self.dialog.as_mut() {
                dialog.set_error(message);
            }
            return;
        }
        let handle = match self.commands.operations().start(title, "desktop", None) {
            Ok(handle) => handle,
            Err(error) => {
                let message = error.to_string();
                self.commands.set_status(message.clone());
                if preserve_dialog && let Some(dialog) = self.dialog.as_mut() {
                    dialog.set_error(message);
                }
                return;
            }
        };
        let operation_id = handle.operation_id.clone();
        let _ = self.commands.operations().update(
            &operation_id,
            Some(OperationPhase::Running),
            Some(0),
            Some("Starting isolated work"),
        );
        let (sender, receiver) = mpsc::sync_channel(32);
        let worker_handle = handle.clone();
        let spawn = thread::Builder::new()
            .name(format!(
                "modeler3d-{}",
                title.to_ascii_lowercase().replace(' ', "-")
            ))
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_desktop_job(request, &worker_handle, |percent, message| {
                        let _ = sender.try_send(DesktopJobEvent::Progress {
                            percent,
                            message: message.to_owned(),
                        });
                    })
                }))
                .unwrap_or_else(|_| Err(String::from("operation worker stopped unexpectedly")));
                if let Err(disconnected) = sender.send(DesktopJobEvent::Finished(result)) {
                    discard_unreceived_job_event(disconnected.0);
                }
            });
        if let Err(error) = spawn {
            let message = format!("Could not start {title}: {error}");
            let _ = self.commands.operations().fail(
                &operation_id,
                OperationFailure::new("worker_start_failed", message.clone()).retryable(true),
            );
            self.commands.set_status(message.clone());
            if preserve_dialog && let Some(dialog) = self.dialog.as_mut() {
                dialog.set_error(message);
            }
            return;
        }
        let document_generation = self.commands.document_generation();
        let source_revision = self.commands.scene().revision();
        let dialog_generation = self.dialog_generation;
        let resume_dialog = preserve_dialog.then(|| self.dialog.take()).flatten();
        self.commands.set_status(format!("{title} running"));
        self.desktop_job = Some(RunningDesktopJob {
            handle,
            receiver,
            pending_terminal: None,
            title: title.to_owned(),
            document_generation,
            source_revision,
            target_object_id,
            dialog_generation,
            resume_dialog,
            cancel_on_drop: true,
            launch_when_staged: false,
        });
    }

    fn poll_desktop_job(&mut self, ctx: &egui::Context) {
        let mut progress = Vec::new();
        let mut terminal = None;
        let mut finish_pending_terminal = false;
        if let Some(job) = self.desktop_job.as_mut() {
            if let Some(result) = job.pending_terminal.take() {
                terminal = Some(result);
                finish_pending_terminal = true;
            } else {
                loop {
                    match job.receiver.try_recv() {
                        Ok(DesktopJobEvent::Progress { percent, message }) => {
                            progress.push((percent, message));
                        }
                        Ok(DesktopJobEvent::Finished(result)) => {
                            terminal = Some(result);
                            break;
                        }
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            terminal = Some(Err(String::from(
                                "operation worker stopped before returning a result",
                            )));
                            break;
                        }
                    }
                }
            }
        }
        if let Some(job) = self.desktop_job.as_ref() {
            for (percent, message) in progress {
                let _ = self.commands.operations().update(
                    &job.handle.operation_id,
                    Some(OperationPhase::Running),
                    Some(percent),
                    Some(&message),
                );
                if !job.handle.is_cancelled() {
                    self.commands
                        .set_status(format!("{}: {message}", job.title));
                }
            }
        }
        if !finish_pending_terminal
            && terminal.as_ref().is_some_and(Result::is_ok)
            && let Some(Ok(result)) = terminal.take()
        {
            if let Some(job) = self.desktop_job.as_mut()
                && !job.handle.is_cancelled()
            {
                let _ = self.commands.operations().update(
                    &job.handle.operation_id,
                    Some(OperationPhase::Committing),
                    Some(95),
                    Some("Result ready; applying on the UI thread"),
                );
                self.commands
                    .set_status(format!("{}: applying completed result", job.title));
                job.pending_terminal = Some(Ok(result));
            } else {
                terminal = Some(Ok(result));
            }
        }
        if let Some(result) = terminal
            && let Some(job) = self.desktop_job.take()
        {
            self.finish_desktop_job(job, result);
        }
        if self.desktop_job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
    }

    fn finish_desktop_job(
        &mut self,
        mut job: RunningDesktopJob,
        result: Result<DesktopJobResult, String>,
    ) {
        job.cancel_on_drop = false;
        if job.handle.is_cancelled() {
            if let Ok(result) = result {
                discard_uncommitted_job_result(result);
            }
            let message = format!("{} cancelled; no scene changes were applied", job.title);
            let _ = self
                .commands
                .operations()
                .cancelled(&job.handle.operation_id, message.clone());
            self.commands.set_status(message.clone());
            self.restore_job_dialog(&mut job, |dialog| dialog.set_message(message));
            return;
        }
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                self.fail_desktop_job(&mut job, error);
                return;
            }
        };
        match result {
            DesktopJobResult::Mesh {
                operation,
                mesh,
                message,
            } => {
                let Some(target_object_id) = job.target_object_id.as_deref() else {
                    self.fail_desktop_job(
                        &mut job,
                        String::from("mesh operation lost its target identity"),
                    );
                    return;
                };
                if !self.job_source_is_current(&job)
                    || self
                        .commands
                        .scene()
                        .object_index(target_object_id)
                        .is_err()
                {
                    self.fail_desktop_job(
                        &mut job,
                        String::from(
                            "scene changed while the operation was running; result was discarded",
                        ),
                    );
                    return;
                }
                let _ = self.commands.operations().update(
                    &job.handle.operation_id,
                    Some(OperationPhase::Committing),
                    Some(95),
                    Some("Applying completed mesh result"),
                );
                match self.commands.replace_mesh_if_current(
                    job.document_generation,
                    job.source_revision,
                    target_object_id,
                    mesh,
                ) {
                    Ok(()) => {
                        let status = message.clone();
                        let _ = self.commands.operations().complete(
                            &job.handle.operation_id,
                            Some(serde_json::json!({ "changed": true })),
                        );
                        self.commands.set_status(status);
                        if matches!(
                            operation,
                            MeshJobOperation::Repair
                                | MeshJobOperation::Simplify(_)
                                | MeshJobOperation::Subdivide
                        ) {
                            self.restore_job_dialog(&mut job, |dialog| {
                                dialog.set_message(message);
                            });
                        }
                        self.synchronize_surface();
                        self.persist_preferences();
                    }
                    Err(error) => self.fail_desktop_job(&mut job, error.to_string()),
                }
            }
            DesktopJobResult::Unchanged { message } => {
                let _ = self.commands.operations().complete(
                    &job.handle.operation_id,
                    Some(serde_json::json!({ "changed": false })),
                );
                self.commands.set_status(message.clone());
                self.restore_job_dialog(&mut job, |dialog| dialog.set_message(message));
            }
            DesktopJobResult::PrintPrep {
                bed_size_mm,
                report,
            } => {
                if !self.job_source_is_current(&job) {
                    self.fail_desktop_job(
                        &mut job,
                        String::from("scene changed while print preparation was running; report was discarded"),
                    );
                    return;
                }
                let summary = report.summary();
                let result_message = format!(
                    "Print prep ({bed_size_mm:.1} mm bed): {} error(s), {} warning(s)",
                    report.errors.len(),
                    report.warnings.len()
                );
                let _ = self.commands.operations().update(
                    &job.handle.operation_id,
                    Some(OperationPhase::Committing),
                    Some(95),
                    Some("Displaying print-readiness report"),
                );
                self.commands
                    .operations()
                    .complete(
                        &job.handle.operation_id,
                        Some(serde_json::json!({
                            "errors": report.errors.len(),
                            "warnings": report.warnings.len(),
                        })),
                    )
                    .ok();
                self.commands.set_status(result_message.clone());
                let has_errors = !report.errors.is_empty();
                let has_warnings = !report.warnings.is_empty();
                self.restore_job_dialog(&mut job, |dialog| {
                    dialog.set_report(
                        format!("Print prep ({bed_size_mm:.1} mm bed): {summary}"),
                        has_errors,
                        has_warnings,
                    );
                });
            }
            DesktopJobResult::PreparedPrint(prepared_guard) => {
                let Some(prepared) = prepared_guard.into_prepared() else {
                    self.fail_desktop_job(
                        &mut job,
                        String::from("staged print result was already consumed"),
                    );
                    return;
                };
                if !self.job_source_is_current(&job) {
                    let _ = discard_prepared_print(&prepared);
                    self.fail_desktop_job(
                        &mut job,
                        String::from(
                            "scene changed while print files were staged; files were discarded",
                        ),
                    );
                    return;
                }
                if job.launch_when_staged {
                    match launch_prepared_print(&prepared) {
                        Ok(launch) => {
                            let message = format!(
                                "Sent {} object(s) to {}",
                                prepared.objects_exported, launch.slicer.label
                            );
                            self.commands.set_status(message);
                            let _ = self.commands.operations().complete(
                                &job.handle.operation_id,
                                Some(serde_json::json!({ "staged": true, "launched": true })),
                            );
                        }
                        Err(error) => {
                            let _ = discard_prepared_print(&prepared);
                            self.fail_desktop_job(&mut job, error.to_string());
                        }
                    }
                    return;
                }
                let message = format!(
                    "Prepared {} object(s) as {} for {}",
                    prepared.objects_exported, prepared.format, prepared.slicer.label
                );
                let _ = self.commands.operations().update(
                    &job.handle.operation_id,
                    Some(OperationPhase::Committing),
                    Some(95),
                    Some("Attaching validated staged files to the print dialog"),
                );
                if self.can_restore_job_dialog(&job) {
                    self.restore_job_dialog(&mut job, |dialog| {
                        dialog.set_prepared_print(prepared);
                        dialog.set_message(message.clone());
                    });
                    self.commands.set_status(message);
                } else {
                    let _ = discard_prepared_print(&prepared);
                    let message =
                        String::from("Print dialog closed or changed; staged files were discarded");
                    self.commands.set_status(message.clone());
                    let _ = self.commands.operations().complete(
                        &job.handle.operation_id,
                        Some(serde_json::json!({ "staged": false })),
                    );
                    return;
                }
                let _ = self.commands.operations().complete(
                    &job.handle.operation_id,
                    Some(serde_json::json!({ "staged": true })),
                );
            }
        }
    }

    fn fail_desktop_job(&mut self, job: &mut RunningDesktopJob, error: String) {
        let message = format!("{} failed: {error}", job.title);
        let _ = self.commands.operations().fail(
            &job.handle.operation_id,
            OperationFailure::new("desktop_job_failed", error.clone()).retryable(true),
        );
        self.commands.set_status(message.clone());
        self.restore_job_dialog(job, |dialog| dialog.set_error(message));
    }

    fn job_source_is_current(&self, job: &RunningDesktopJob) -> bool {
        self.commands.document_generation() == job.document_generation
            && self.commands.scene().revision() == job.source_revision
    }

    fn can_restore_job_dialog(&self, job: &RunningDesktopJob) -> bool {
        self.dialog.is_none()
            && self.dialog_generation == job.dialog_generation
            && self.commands.document_generation() == job.document_generation
            && job.resume_dialog.is_some()
    }

    fn restore_job_dialog<F>(&mut self, job: &mut RunningDesktopJob, update: F)
    where
        F: FnOnce(&mut DialogState),
    {
        if self.can_restore_job_dialog(job)
            && let Some(mut dialog) = job.resume_dialog.take()
        {
            update(&mut dialog);
            self.dialog = Some(dialog);
        }
    }

    fn prepared_print_is_current(&self, prepared: &PreparedPrint) -> bool {
        let current_ids = self
            .commands
            .scene()
            .objects()
            .iter()
            .filter(|object| object.is_visible())
            .map(|object| object.object_id().to_owned())
            .collect::<Vec<_>>();
        prepared
            .source_document_generation
            .is_none_or(|generation| generation == self.commands.document_generation())
            && prepared.source_revision == self.commands.scene().revision()
            && prepared.source_object_ids == current_ids
    }

    fn report_command_error(&mut self, error: CommandError) {
        self.commands.set_status(error.to_string());
    }

    fn refresh_preview(&mut self) {
        let revision = self.commands.scene().revision();
        if revision == self.preview_revision && self.preview_mesh_override.is_none() {
            return;
        }
        self.preview_revision = revision;
        match RenderScene::from_core(self.commands.scene()) {
            Ok(scene) => {
                self.preview = match self.preview_mesh_override.as_ref() {
                    Some((object_id, mesh)) => scene.with_mesh_preview(object_id, mesh).ok(),
                    None => Some(scene),
                };
                self.preview_error = None;
            }
            Err(error) => {
                self.preview = None;
                let error = error.to_string();
                if self.preview_error.as_deref() != Some(error.as_str()) {
                    crate::diagnostics::record_error("scene preview construction", &error);
                }
                self.preview_error = Some(error);
            }
        }
    }

    fn render_top(&mut self, ui: &mut Ui) {
        let palette = ThemePalette::for_mode(self.theme_mode);
        if self.artwork_cache_theme != self.theme_mode {
            self.artwork_cache.clear();
            self.artwork_cache_theme = self.theme_mode;
        }
        let response = egui::Panel::top("modeler3d-ribbon")
            .frame(
                egui::Frame::new()
                    .fill(palette.panel)
                    .stroke(Stroke::new(1.0, palette.divider))
                    .inner_margin(egui::Margin::same(3)),
            )
            .show(ui, |ui| {
                self.render_menu_and_utility(ui, palette);
                if self.paint_mode.is_some() {
                    ui.add_space(2.0);
                    self.render_paint_bar(ui, palette);
                } else if self.measure_mode.is_some() {
                    ui.add_space(2.0);
                    self.render_measure_bar(ui, palette);
                } else if self.ribbon_expanded {
                    ui.add_space(2.0);
                    self.render_active_tab(ui, palette);
                }
            });
        self.ribbon_rect = Some(response.response.rect);
        self.render_settings_note(ui, palette);
    }

    /// A settings message (OF-64) sits in its own strip under the ribbon, so
    /// it pushes the workspace down instead of covering anything, and stays
    /// until Dismiss is pressed (MCP: `set_app_settings`
    /// `dismiss_settings_notice`).
    fn render_settings_note(&mut self, ui: &mut Ui, palette: ThemePalette) {
        let Some(warning) = self.settings_warning.clone() else {
            return;
        };
        let mut dismissed = false;
        egui::Panel::top("modeler3d-settings-note")
            .frame(
                egui::Frame::new()
                    .fill(palette.surface_alt)
                    .stroke(Stroke::new(1.0, palette.divider))
                    .inner_margin(egui::Margin::symmetric(12, 4)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        dismissed = ui
                            .button("Dismiss")
                            .on_hover_text("Close this note")
                            .clicked();
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(warning).color(palette.warning_text),
                                )
                                .wrap(),
                            );
                        });
                    });
                });
            });
        if dismissed {
            self.dismiss_settings_note();
        }
    }

    /// Closes the settings note; returns whether one was showing.
    pub(crate) fn dismiss_settings_note(&mut self) -> bool {
        self.settings_warning.take().is_some()
    }

    fn toggle_theme(&mut self) {
        let mode = match self.theme_mode {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        };
        self.apply_theme_mode(mode);
    }

    fn render_menu_and_utility(&mut self, ui: &mut Ui, palette: ThemePalette) {
        let mut menu_response = None;
        let mut slicer_response = None;
        let mut active_tab_rect = None;
        let bar = egui::Frame::new()
            .fill(palette.navigation)
            .inner_margin(egui::Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = TOP_ROW_SPACING;
                    let (menu_rect, response) =
                        ui.allocate_exact_size(Vec2::new(36.0, 34.0), Sense::click());
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Button, true, "Application menu")
                    });
                    if response.hovered() || self.menu_open {
                        ui.painter().rect_filled(
                            menu_rect,
                            6.0,
                            if self.menu_open {
                                palette.pressed
                            } else {
                                palette.navigation_hover
                            },
                        );
                    }
                    let painter = ui.painter_at(menu_rect);
                    for line_y in [-6.0, 0.0, 6.0] {
                        painter.line_segment(
                            [
                                egui::pos2(
                                    menu_rect.center().x - 8.0,
                                    menu_rect.center().y + line_y,
                                ),
                                egui::pos2(
                                    menu_rect.center().x + 8.0,
                                    menu_rect.center().y + line_y,
                                ),
                            ],
                            Stroke::new(2.0, palette.text),
                        );
                    }
                    let response =
                        response.on_hover_text("Open project, save, settings, or close 3D Modeler");
                    if response.clicked() {
                        self.menu_open = !self.menu_open;
                    }
                    menu_response = Some(response);

                    active_tab_rect = self.render_tabs(ui, palette);
                    self.render_favorites(ui, palette);

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if self.utility_button(
                            ui,
                            CommandId::Redo,
                            "Redo the last undone change (Ctrl+Y)",
                            palette,
                        ) {
                            self.dispatch(CommandId::Redo);
                        }
                        if self.utility_button(
                            ui,
                            CommandId::Undo,
                            "Undo the last model change (Ctrl+Z)",
                            palette,
                        ) {
                            self.dispatch(CommandId::Undo);
                        }
                        if labelled_save_button(
                            ui,
                            "Save As",
                            true,
                            "Save the project under a new name (Ctrl+Shift+S)",
                            palette,
                        ) {
                            self.dispatch(CommandId::SaveProjectAs);
                        }
                        if labelled_save_button(
                            ui,
                            "Save",
                            false,
                            "Save the project (Ctrl+S)",
                            palette,
                        ) {
                            self.dispatch(CommandId::SaveProject);
                        }
                        if theme_toggle(ui, self.theme_mode, palette) {
                            self.toggle_theme();
                        }
                        let tooltip = self
                            .commands
                            .preferences()
                            .preferred_slicer_label()
                            .map_or_else(
                                || String::from("Choose the slicer to send your models to"),
                                |label| format!("Send the visible models to {label}"),
                            );
                        let response = slicer_button(ui, &tooltip, palette);
                        if response.clicked() {
                            self.slicer_button_clicked();
                        }
                        slicer_response = Some(response);
                    });
                });
            });

        if let Some(tab_rect) = active_tab_rect.filter(|_| self.ribbon_expanded) {
            attach_tab_to_ribbon(ui, tab_rect, bar.response.rect.bottom(), palette);
        }

        if let Some(response) = slicer_response {
            let mut chosen = None;
            let mut browse = false;
            let choices = &self.slicer_choices;
            egui::Popup::from_response(&response)
                .open_bool(&mut self.slicer_chooser_open)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .align(egui::RectAlign::BOTTOM_END)
                .frame(
                    egui::Frame::new()
                        .fill(palette.panel)
                        .stroke(Stroke::new(1.0, palette.divider))
                        .corner_radius(crate::chrome::RADIUS_SURFACE)
                        .shadow(theme::floating_shadow(self.theme_mode))
                        .inner_margin(egui::Margin::same(10)),
                )
                .width(260.0)
                .show(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    ui.label(RichText::new("Choose your slicer").strong());
                    ui.label(
                        RichText::new(
                            "The Slicer button will send to it every time. \
                             Change it later in Settings.",
                        )
                        .small()
                        .color(palette.muted),
                    );
                    ui.add_space(4.0);
                    if choices.is_empty() {
                        ui.label(RichText::new("No installed slicers were found.").weak());
                    }
                    for slicer in choices {
                        if menu_row(ui, &slicer.label, palette)
                            .on_hover_text(slicer.path.display().to_string())
                            .clicked()
                        {
                            chosen = Some(slicer.clone());
                        }
                    }
                    ui.separator();
                    if menu_row(ui, "Browse for a slicer...", palette).clicked() {
                        browse = true;
                    }
                });
            if browse {
                self.slicer_chooser_open = false;
                if let Some(path) = native_slicer_picker(None) {
                    match slicer_info_from_path(&path, None) {
                        Ok(slicer) => chosen = Some(slicer),
                        Err(error) => self
                            .commands
                            .set_status(format!("Could not use selected slicer: {error}")),
                    }
                }
            }
            if let Some(slicer) = chosen {
                self.choose_slicer(slicer);
            }
        }

        let mut menu_command = None;
        let mut customize_ribbon = false;
        if let Some(response) = menu_response {
            egui::Popup::from_response(&response)
                .open_bool(&mut self.menu_open)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClick)
                .frame(
                    egui::Frame::new()
                        .fill(palette.panel)
                        .stroke(Stroke::new(1.0, palette.divider))
                        .corner_radius(crate::chrome::RADIUS_SURFACE)
                        .shadow(theme::floating_shadow(self.theme_mode))
                        .inner_margin(egui::Margin::same(6)),
                )
                .width(228.0)
                .show(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    for item in file_menu_spec() {
                        match item {
                            RibbonItem::Separator => {
                                ui.separator();
                            }
                            RibbonItem::Command(command) => {
                                let response = menu_row(ui, command.label, palette)
                                    .on_hover_text(command_help(command.id));
                                if response.clicked() {
                                    menu_command = Some(command.id);
                                    ui.close();
                                }
                            }
                        }
                    }
                    ui.separator();
                    if menu_row(ui, "Customize ribbon", palette).clicked() {
                        customize_ribbon = true;
                        ui.close();
                    }
                });
        }
        if customize_ribbon {
            self.customize_ribbon = true;
        }
        if let Some(command) = menu_command {
            if command == CommandId::Modeler3dMenu {
                self.show_start_screen();
            } else if command == CommandId::Exit {
                self.request_exit();
            } else {
                self.dispatch(command);
            }
        }
    }
    fn utility_button(
        &mut self,
        ui: &mut Ui,
        command: CommandId,
        help: &'static str,
        palette: ThemePalette,
    ) -> bool {
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(36.0), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::Button, true, utility_accessible_label(command))
        });
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let painter = ui.painter_at(rect);
        if response.hovered() || response.has_focus() {
            painter.rect_filled(rect, 6.0, palette.hover);
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect,
                6.0,
                Stroke::new(1.5, palette.accent_border),
                egui::StrokeKind::Inside,
            );
        }
        let icon_rect = egui::Rect::from_center_size(rect.center(), Vec2::splat(20.0));
        match command {
            CommandId::Undo | CommandId::Redo => {
                self.paint_command_icon(ui, icon_rect, command, IconKey::Edit, palette);
            }
            CommandId::SaveProject => paint_save_symbol(&painter, icon_rect, palette.muted, false),
            CommandId::SaveProjectAs => {
                paint_save_symbol(&painter, icon_rect, palette.muted, true);
            }
            _ => {}
        }
        response.on_hover_text(help).clicked()
    }

    fn render_favorites(&mut self, ui: &mut Ui, palette: ThemePalette) {
        let available = all_command_keys();
        let current = normalize_favorites(
            self.commands
                .preferences()
                .ribbon_favorites()
                .iter()
                .cloned(),
            available,
        );
        if current != self.commands.preferences().ribbon_favorites() {
            self.commands
                .preferences_mut()
                .set_ribbon_favorites(current.clone());
        }

        let fixed_row_width = TOP_RIGHT_CLUSTER_WIDTH;
        let available_width = (ui.available_width() - fixed_row_width).max(0.0);
        let horizontal_margins = 8.0;
        let spacing = TOP_ROW_SPACING;
        let all_width = horizontal_margins
            + current.len() as f32 * FAVORITE_BUTTON_WIDTH
            + current.len().saturating_sub(1) as f32 * spacing;
        let visible_count = if available_width >= all_width {
            current.len()
        } else if available_width >= horizontal_margins + FAVORITE_OVERFLOW_WIDTH {
            let button_budget =
                (available_width - horizontal_margins - FAVORITE_OVERFLOW_WIDTH - spacing).max(0.0);
            (((button_budget + spacing) / (FAVORITE_BUTTON_WIDTH + spacing)).floor() as usize)
                .min(current.len())
        } else {
            0
        };
        let has_overflow = visible_count < current.len()
            && available_width >= horizontal_margins + FAVORITE_OVERFLOW_WIDTH;
        let has_favorites_strip = visible_count > 0 || has_overflow;
        let mut clicked = None;
        let mut unpin = None;
        let mut button_rects = Vec::with_capacity(visible_count);
        let zone_left = ui.cursor().left();
        let row = ui.horizontal(|ui| {
            if has_favorites_strip {
                ui.add_space(4.0);
            }
            for key in current.iter().take(visible_count) {
                if let Some((tab, command)) = find_command_key(key) {
                    let enabled = self.command_selected(command.id)
                        || command_enabled(command.id, &self.commands);
                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(FAVORITE_BUTTON_WIDTH, FAVORITE_BUTTON_HEIGHT),
                        Sense::click_and_drag(),
                    );
                    button_rects.push(rect);
                    response.widget_info(|| {
                        egui::WidgetInfo::labeled(egui::Role::Button, enabled, command.label)
                    });
                    let response = response.on_hover_cursor(if enabled {
                        egui::CursorIcon::PointingHand
                    } else {
                        egui::CursorIcon::Default
                    });
                    if response.hovered() || response.has_focus() || response.context_menu_opened()
                    {
                        ui.painter().rect_filled(rect, 6.0, palette.hover);
                    }
                    if response.has_focus() {
                        ui.painter().rect_stroke(
                            rect,
                            6.0,
                            Stroke::new(1.5, palette.accent_border),
                            egui::StrokeKind::Inside,
                        );
                    }
                    self.paint_command_icon(
                        ui,
                        egui::Rect::from_center_size(
                            rect.center(),
                            Vec2::splat(FAVORITE_ICON_SIZE),
                        ),
                        command.id,
                        command.icon,
                        if enabled {
                            palette
                        } else {
                            ThemePalette {
                                accent: palette.disabled_text,
                                ..palette
                            }
                        },
                    );
                    response.dnd_set_drag_payload(ShortcutDrag(key.clone()));
                    let mut remove_this = false;
                    response.context_menu(|ui| {
                        if ui.button("Remove from shortcut bar").clicked() {
                            remove_this = true;
                            ui.close();
                        }
                    });
                    if remove_this {
                        unpin = Some(key.clone());
                    }
                    if response.on_hover_text(command.label).clicked() && enabled {
                        clicked = Some((tab, command.id));
                    }
                }
            }
            if has_overflow {
                let overflow = egui::ComboBox::from_id_salt("favorite-overflow")
                    .width(FAVORITE_OVERFLOW_WIDTH)
                    .selected_text("More")
                    .show_ui(ui, |ui| {
                        for key in current.iter().skip(visible_count) {
                            if let Some((tab, command)) = find_command_key(key)
                                && ui
                                    .selectable_label(false, command.label)
                                    .on_hover_text(command_help(command.id))
                                    .clicked()
                            {
                                clicked = Some((tab, command.id));
                            }
                        }
                    });
                overflow.response.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::Button, true, "More favorite commands")
                });
            }
            if has_favorites_strip {
                ui.add_space(4.0);
            }
            // An empty bar says where shortcuts go, in small dim text.
            if current.is_empty() && available_width >= EMPTY_FAVORITES_HINT_WIDTH {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(EMPTY_FAVORITES_HINT)
                        .small()
                        .color(palette.muted.gamma_multiply(0.7)),
                );
            }
        });
        // The whole strip left of the right-hand cluster takes drops, so a
        // shortcut can land past the last one or on an empty bar.
        let drop_zone = egui::Rect::from_x_y_ranges(
            zone_left..=zone_left + available_width.max(row.response.rect.width()),
            row.response.rect.y_range(),
        );
        let dropped = self.favorites_drop(
            ui,
            drop_zone,
            &current,
            &button_rects,
            has_overflow,
            spacing,
            palette,
        );
        if let Some(next) = dropped {
            self.commands.preferences_mut().set_ribbon_favorites(next);
            self.persist_preferences();
        } else if let Some(key) = unpin {
            let mut next = current.clone();
            next.retain(|favorite| favorite != &key);
            self.commands.preferences_mut().set_ribbon_favorites(next);
            self.persist_preferences();
        }
        if let Some((tab, command)) = clicked {
            if tab != self.active_tab {
                self.active_tab = tab;
                self.artwork_cache.clear();
            }
            self.ribbon_expanded = true;
            self.dispatch(command);
        }
    }

    /// While a shortcut or ribbon command is dragged, shows a bar in the gap
    /// on the shortcut bar where it will land and its icon under the pointer.
    /// Returns the new bar when it is dropped there and the bar changes.
    #[allow(clippy::too_many_arguments)]
    fn favorites_drop(
        &mut self,
        ui: &Ui,
        zone: egui::Rect,
        current: &[String],
        button_rects: &[egui::Rect],
        has_overflow: bool,
        spacing: f32,
        palette: ThemePalette,
    ) -> Option<Vec<String>> {
        let ctx = ui.ctx().clone();
        let payload = egui::DragAndDrop::payload::<ShortcutDrag>(&ctx)?;
        let pointer = ctx.input(|input| input.pointer.latest_pos())?;
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        self.paint_shortcut_drag_ghost(&ctx, &payload.0, pointer, palette);
        if !zone.contains(pointer) {
            return None;
        }
        let visible = button_rects.len();
        let mut index = button_rects
            .iter()
            .filter(|rect| rect.center().x < pointer.x)
            .count();
        // On a full bar the gap before "More" would hide a newly placed
        // shortcut, so it lands before the last visible one instead.
        let shown_already = current.iter().take(visible).any(|key| key == &payload.0);
        if has_overflow && index == visible && !shown_already {
            index = visible.saturating_sub(1);
        }
        let next = place_favorite_at(current, &payload.0, index)?;
        let marker_x = match index
            .checked_sub(1)
            .and_then(|before| button_rects.get(before))
        {
            Some(before) => before.right() + spacing / 2.0,
            None => button_rects
                .first()
                .map_or(zone.left() + 4.0 + FAVORITE_DROP_MARKER_WIDTH, |first| {
                    first.left() - spacing / 2.0
                }),
        };
        let marker = egui::Rect::from_center_size(
            egui::pos2(marker_x, zone.center().y),
            Vec2::new(FAVORITE_DROP_MARKER_WIDTH, FAVORITE_BUTTON_HEIGHT),
        );
        ui.painter()
            .rect_filled(marker, FAVORITE_DROP_MARKER_WIDTH / 2.0, palette.accent);
        if ctx.input(|input| input.pointer.any_released()) {
            egui::DragAndDrop::clear_payload(&ctx);
            return Some(next);
        }
        None
    }

    /// Paints the dragged command's icon beside the pointer.
    fn paint_shortcut_drag_ghost(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        pointer: egui::Pos2,
        palette: ThemePalette,
    ) {
        let Some((_, command)) = find_command_key(key) else {
            return;
        };
        egui::Area::new(egui::Id::unique("shortcut-drag-ghost"))
            .order(egui::Order::Tooltip)
            .fixed_pos(pointer + Vec2::new(12.0, 12.0))
            .interactable(false)
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(
                    Vec2::new(FAVORITE_BUTTON_WIDTH, FAVORITE_BUTTON_HEIGHT),
                    Sense::hover(),
                );
                ui.painter().rect_filled(rect, 6.0, palette.surface_raised);
                ui.painter().rect_stroke(
                    rect,
                    6.0,
                    Stroke::new(1.0, palette.accent_border),
                    egui::StrokeKind::Inside,
                );
                self.paint_command_icon(
                    ui,
                    egui::Rect::from_center_size(rect.center(), Vec2::splat(FAVORITE_ICON_SIZE)),
                    command.id,
                    command.icon,
                    palette,
                );
            });
    }

    fn set_favorite(&mut self, key: &str, pinned: bool) {
        let mut favorites = self.commands.preferences().ribbon_favorites().to_vec();
        if pinned {
            if !favorites.iter().any(|favorite| favorite == key) {
                favorites.push(key.to_owned());
            }
        } else {
            favorites.retain(|favorite| favorite != key);
        }
        self.commands
            .preferences_mut()
            .set_ribbon_favorites(favorites);
        self.persist_preferences();
    }

    /// Paints the ribbon tabs and returns the active tab's tile.
    fn render_tabs(&mut self, ui: &mut Ui, palette: ThemePalette) -> Option<egui::Rect> {
        let mut selected_tab = None;
        let mut active_rect = None;
        // Tabs share the top row with the menu button instead of a nested
        // row, so both centre on the same line (OF-19).
        {
            for tab in RibbonTab::all() {
                let selected = tab == self.active_tab;
                let (rect, response) =
                    ui.allocate_exact_size(Vec2::new(62.0, 34.0), Sense::click());
                if selected {
                    active_rect = Some(rect);
                }
                response.widget_info(|| {
                    egui::WidgetInfo::selected(egui::Role::Tab, true, selected, tab.label())
                });
                let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                // 3D Builder: the active tab is a raised tile; hovering any
                // other tab shows a darker tile on the tab bar.
                let fill = if selected {
                    Some(palette.panel)
                } else if response.is_pointer_button_down_on() {
                    Some(palette.pressed)
                } else if response.hovered() || response.has_focus() {
                    Some(palette.navigation_hover)
                } else {
                    None
                };
                if let Some(fill) = fill {
                    ui.painter().rect_filled(rect, RADIUS_CONTROL, fill);
                }
                if response.has_focus() {
                    ui.painter().rect_stroke(
                        rect,
                        RADIUS_CONTROL,
                        Stroke::new(1.5, palette.accent_border),
                        egui::StrokeKind::Inside,
                    );
                }
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    tab.label(),
                    FontId::proportional(17.0),
                    if selected {
                        palette.accent_dark
                    } else {
                        palette.text
                    },
                );
                let response = response.on_hover_text(format!("{} commands", tab.label()));
                if response.clicked() {
                    selected_tab = Some(tab);
                }
            }
        }
        if let Some(tab) = selected_tab {
            if tab == self.active_tab {
                if !self.ribbon_pinned {
                    self.ribbon_expanded = !self.ribbon_expanded;
                }
            } else {
                self.show_ribbon_tab(tab);
            }
        }
        active_rect
    }

    fn render_active_tab(&mut self, ui: &mut Ui, palette: ThemePalette) {
        let original_items = ribbon_spec()
            .into_iter()
            .find_map(|(tab, items)| (tab == self.active_tab).then_some(items))
            .unwrap_or_default();
        let mut separator_positions = Vec::new();
        let mut command_position = 0;
        for item in &original_items {
            match item {
                RibbonItem::Command(_) => command_position += 1,
                RibbonItem::Separator => separator_positions.push(command_position),
            }
        }
        let commands = self.ordered_tab_commands(self.active_tab);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            let command_width = (ui.available_width() - 38.0).max(0.0);
            ui.allocate_ui_with_layout(
                Vec2::new(command_width, RIBBON_BUTTON_HEIGHT),
                Layout::left_to_right(Align::Min),
                |ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("ribbon-commands")
                        .auto_shrink([false, false])
                        .max_height(RIBBON_BUTTON_HEIGHT)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (index, command) in commands.iter().copied().enumerate() {
                                    if separator_positions.contains(&index) {
                                        ribbon_separator(ui, palette);
                                    }
                                    self.render_command_button(ui, command, palette);
                                }
                                if separator_positions.contains(&commands.len()) {
                                    ribbon_separator(ui, palette);
                                }
                            });
                        });
                },
            );
            self.render_pin_control(ui, palette);
        });
    }

    fn render_command_button(
        &mut self,
        ui: &mut Ui,
        command: CommandDescriptor,
        palette: ThemePalette,
    ) {
        let selected = self.command_selected(command.id);
        let enabled = selected || command_enabled(command.id, &self.commands);
        let text_color = if !enabled {
            palette.disabled_text
        } else if selected {
            palette.accent_dark
        } else {
            palette.text
        };
        // Dragging a ribbon command up to the shortcut bar pins it there.
        let sense = Sense::click_and_drag();
        let label_width = ui
            .painter()
            .layout_no_wrap(
                command.label.to_owned(),
                FontId::proportional(12.7),
                text_color,
            )
            .size()
            .x;
        let button_width = (label_width + 14.0).clamp(64.0, 100.0);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(button_width, RIBBON_BUTTON_HEIGHT), sense);
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, command.label));
        let response = response.on_hover_cursor(if enabled {
            egui::CursorIcon::PointingHand
        } else {
            egui::CursorIcon::Default
        });
        let painter = ui.painter_at(rect);
        if selected || response.hovered() || response.has_focus() {
            let fill = if selected {
                palette.pressed
            } else {
                palette.hover
            };
            painter.rect_filled(rect, 8.0, fill);
        }
        if selected || response.has_focus() {
            painter.rect_stroke(
                rect,
                8.0,
                Stroke::new(1.0, palette.accent_border),
                egui::StrokeKind::Inside,
            );
        }
        self.paint_command_icon(
            ui,
            egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.top() + 21.0),
                Vec2::splat(42.0),
            ),
            command.id,
            command.icon,
            palette,
        );
        painter.text(
            egui::pos2(rect.center().x, rect.top() + 55.0),
            Align2::CENTER_CENTER,
            command.label,
            FontId::proportional(12.7),
            text_color,
        );
        let favorite_key = command_key(self.active_tab, command);
        response.dnd_set_drag_payload(ShortcutDrag(favorite_key.clone()));
        let is_pinned = self
            .commands
            .preferences()
            .ribbon_favorites()
            .iter()
            .any(|key| key == &favorite_key);
        let mut pin_action = None;
        response.context_menu(|ui| {
            ui.label(RichText::new(command.label).strong());
            ui.label(command_summary(command.id));
            ui.separator();
            if ui
                .button(if is_pinned {
                    "Remove from shortcut bar"
                } else {
                    "Pin to shortcut bar"
                })
                .clicked()
            {
                pin_action = Some(!is_pinned);
                ui.close();
            }
        });
        if let Some(pin) = pin_action {
            self.set_favorite(&favorite_key, pin);
        }
        let response = response.on_hover_text(if enabled {
            command_help(command.id)
        } else {
            command_disabled_help(command.id, &self.commands)
        });
        if response.clicked() && enabled {
            self.dispatch(command.id);
        }
    }

    fn render_pin_control(&mut self, ui: &mut Ui, palette: ThemePalette) {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(30.0, RIBBON_BUTTON_HEIGHT), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::Role::Button,
                true,
                self.ribbon_pinned,
                if self.ribbon_pinned {
                    "Unpin command ribbon"
                } else {
                    "Pin command ribbon"
                },
            )
        });
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let painter = ui.painter_at(rect);
        if self.ribbon_pinned {
            painter.rect_filled(rect, 6.0, palette.pressed);
            painter.rect_stroke(
                rect,
                6.0,
                Stroke::new(1.0, palette.accent_border),
                egui::StrokeKind::Inside,
            );
        } else if response.hovered() || response.has_focus() {
            painter.rect_filled(rect, 6.0, palette.hover);
        }
        if response.has_focus() {
            painter.rect_stroke(
                rect,
                6.0,
                Stroke::new(1.5, palette.accent_border),
                egui::StrokeKind::Inside,
            );
        }
        self.paint_original_icon(
            ui,
            egui::Rect::from_center_size(
                egui::pos2(rect.center().x, rect.top() + 24.0),
                Vec2::splat(20.0),
            ),
            OriginalAsset::Action("Pin.png"),
            palette,
        );
        painter.text(
            egui::pos2(rect.center().x, rect.bottom() - 8.0),
            Align2::CENTER_CENTER,
            "Pin",
            FontId::proportional(10.5),
            if self.ribbon_pinned {
                palette.accent_dark
            } else {
                palette.muted
            },
        );
        if response
            .on_hover_text(if self.ribbon_pinned {
                "Keep the command strip open"
            } else {
                "Pin the command strip open"
            })
            .clicked()
        {
            self.apply_ribbon_pinned(!self.ribbon_pinned);
        }
    }

    fn paint_command_icon(
        &mut self,
        ui: &Ui,
        rect: egui::Rect,
        command: CommandId,
        fallback: IconKey,
        palette: ThemePalette,
    ) {
        if let Some(asset) = original_asset_for_command(command) {
            self.paint_original_icon(ui, rect, asset, palette);
        } else {
            ThemeArtwork::new(self.theme_mode).paint_icon(ui.painter(), rect, fallback, palette);
        }
    }

    fn paint_original_icon(
        &mut self,
        ui: &Ui,
        rect: egui::Rect,
        asset: OriginalAsset,
        _palette: ThemePalette,
    ) {
        let relative = asset.relative_path(self.theme_mode);
        let path = self.artwork_root.join(&relative);
        match self.original_texture_id(ui.ctx(), asset) {
            Ok(texture_id) => {
                ui.painter().image(
                    texture_id,
                    rect,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            Err(error) => {
                if self.missing_artwork.insert(path.clone()) {
                    self.commands.set_status(format!(
                        "Original artwork could not be loaded: {} ({error})",
                        relative.display()
                    ));
                }
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    "!",
                    FontId::proportional(18.0),
                    Color32::from_rgb(0xC6, 0x42, 0x42),
                );
            }
        }
    }

    fn original_texture_id(
        &mut self,
        ctx: &egui::Context,
        asset: OriginalAsset,
    ) -> Result<egui::TextureId, crate::artwork::ArtworkError> {
        let relative = asset.relative_path(self.theme_mode);
        let path = self.artwork_root.join(&relative);
        if let Some(texture) = self.artwork_cache.get(&path) {
            return Ok(texture.id());
        }
        let image = load_original_image(&path)?;
        let texture = ctx.load_texture(
            format!("modeler3d-{}-{}", self.theme_mode.key(), relative.display()),
            image,
            egui::TextureOptions::LINEAR,
        );
        let id = texture.id();
        self.artwork_cache.insert(path, texture);
        Ok(id)
    }

    fn command_selected(&self, command: CommandId) -> bool {
        if modifier_dialog(command).is_some_and(|kind| {
            self.dialog
                .as_ref()
                .is_some_and(|dialog| dialog.kind() == kind)
        }) || self.placement_command == Some(command)
            || self
                .boolean_pick
                .as_ref()
                .is_some_and(|pick| boolean_operation_for_command(command) == Some(pick.operation))
        {
            return true;
        }
        let display = self.commands.display_state();
        match command {
            CommandId::ToggleGrid => display.grid_visible,
            CommandId::ToggleWireframe => display.wireframe,
            CommandId::ToggleXray => display.xray,
            CommandId::ToggleSnap => display.snap_enabled,
            command => self
                .active_brush
                .is_some_and(|brush| brush.command() == command),
        }
    }

    fn render_welcome(&mut self, ui: &mut Ui) {
        let palette = ThemePalette::for_mode(self.theme_mode);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(palette.background))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(46.0);
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() * 0.22).min(260.0));
                            let (logo_rect, _) =
                                ui.allocate_exact_size(Vec2::splat(72.0), Sense::hover());
                            self.paint_original_icon(
                                ui,
                                logo_rect,
                                OriginalAsset::Brandmark,
                                palette,
                            );
                            ui.add_space(18.0);
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("Imagine, Design, Play")
                                        .small()
                                        .strong()
                                        .color(palette.accent),
                                );
                                ui.add_space(2.0);
                                ui.heading(RichText::new("3D Modeler").size(30.0).strong());
                                ui.label(
                                    RichText::new(
                                        "Start a scene, open a model, or return to a recent project.",
                                    )
                                    .color(palette.muted),
                                );
                            });
                        });
                        ui.add_space(30.0);
                        ui.horizontal(|ui| {
                            ui.add_space((ui.available_width() * 0.22).min(260.0));
                            if self.welcome_action_card(
                                ui,
                                palette,
                                OriginalAsset::Action("New Scene.png"),
                                "New Scene",
                                "Start with an empty workspace",
                            ) {
                                self.dispatch(CommandId::NewScene);
                            }
                            ui.add_space(18.0);
                            if self.welcome_action_card(
                                ui,
                                palette,
                                OriginalAsset::Action("Folder and Open.png"),
                                "Open Project",
                                "Load a saved 3D Modeler project",
                            ) {
                                self.dispatch(CommandId::OpenProject);
                            }
                        });
                        ui.add_space(34.0);

                        let entries = self.recent_entries();
                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            ui.label(
                                RichText::new("RECENT")
                                    .size(12.0)
                                    .strong()
                                    .color(palette.muted),
                            );
                        });
                        ui.add_space(10.0);
                        if entries.is_empty() {
                            egui::Frame::new()
                                .fill(palette.surface_alt)
                                .stroke(Stroke::new(1.0, palette.divider))
                                .corner_radius(10.0)
                                .outer_margin(egui::Margin::symmetric(20, 0))
                                .inner_margin(egui::Margin::same(18))
                                .show(ui, |ui| {
                                    ui.set_min_width((ui.available_width() - 4.0).max(220.0));
                                    ui.label(
                                        RichText::new("Recent projects and models appear here.")
                                            .color(palette.muted),
                                    );
                                });
                        } else {
                            let (columns, column_width) = recent_grid_layout(
                                ui.available_width(),
                                ui.spacing().item_spacing.x,
                            );
                            for row in entries.chunks(columns) {
                                ui.horizontal(|ui| {
                                    for entry in row {
                                        let _ = ui.allocate_ui_with_layout(
                                            // Include the wrapped source path in the scroll range.
                                            Vec2::new(column_width, 120.0),
                                            Layout::top_down(Align::Min),
                                            |ui| self.render_recent_entry(ui, entry, palette),
                                        );
                                    }
                                });
                                ui.add_space(10.0);
                            }
                        }
                    });
            });
    }

    fn welcome_action_card(
        &mut self,
        ui: &mut Ui,
        palette: ThemePalette,
        asset: OriginalAsset,
        title: &'static str,
        subtitle: &'static str,
    ) -> bool {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(280.0, 96.0), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, title));
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        let painter = ui.painter_at(rect);
        painter.rect_filled(
            rect,
            12.0,
            if response.hovered() || response.has_focus() {
                palette.hover
            } else {
                palette.panel
            },
        );
        painter.rect_stroke(
            rect,
            12.0,
            Stroke::new(
                1.0,
                if response.hovered() || response.has_focus() {
                    palette.accent_border
                } else {
                    palette.border
                },
            ),
            egui::StrokeKind::Inside,
        );
        self.paint_original_icon(
            ui,
            egui::Rect::from_center_size(
                egui::pos2(rect.left() + 54.0, rect.center().y),
                Vec2::splat(46.0),
            ),
            asset,
            palette,
        );
        painter.text(
            egui::pos2(rect.left() + 94.0, rect.center().y - 10.0),
            Align2::LEFT_CENTER,
            title,
            FontId::proportional(15.0),
            palette.text,
        );
        painter.text(
            egui::pos2(rect.left() + 94.0, rect.center().y + 13.0),
            Align2::LEFT_CENTER,
            subtitle,
            FontId::proportional(11.5),
            palette.muted,
        );
        response.on_hover_text(subtitle).clicked()
    }

    fn recent_entries(&self) -> Vec<RecentEntry> {
        self.commands
            .preferences()
            .recent_files()
            .paths()
            .iter()
            .map(|path| recent_entry(path))
            .collect()
    }

    fn update_recent_previews(&mut self, ctx: &egui::Context) {
        let paths = self.commands.preferences().recent_files().paths().to_vec();
        if paths != self.recent_preview_paths {
            self.recent_preview_generation = self.recent_preview_generation.wrapping_add(1);
            self.recent_preview_generation_signal
                .store(self.recent_preview_generation, Ordering::Relaxed);
            self.recent_preview_paths.clone_from(&paths);
            self.recent_preview_pending.clear();
            self.recent_preview_textures.clear();
            if !paths.is_empty() {
                self.start_recent_preview_worker();
            }
            for path in paths {
                let request = RecentPreviewRequest {
                    generation: self.recent_preview_generation,
                    path: path.clone(),
                };
                if self.recent_preview_request_sender.send(request).is_ok() {
                    self.recent_preview_pending.insert(path);
                }
            }
        }

        while let Ok(result) = self.recent_preview_result_receiver.try_recv() {
            if result.generation != self.recent_preview_generation
                || !self.recent_preview_pending.remove(&result.path)
            {
                continue;
            }
            if let Some(image) = result.image {
                let name = format!(
                    "recent-preview-{}-{}",
                    result.generation,
                    result.path.display()
                );
                let texture = ctx.load_texture(name, image, egui::TextureOptions::LINEAR);
                self.recent_preview_textures.insert(result.path, texture);
            }
        }
        if !self.recent_preview_pending.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn invalidate_recent_previews_after_clear(&mut self) {
        self.recent_preview_generation = self.recent_preview_generation.wrapping_add(1);
        self.recent_preview_generation_signal
            .store(self.recent_preview_generation, Ordering::Relaxed);
        self.recent_preview_paths.clear();
        self.recent_preview_pending.clear();
        self.recent_preview_textures.clear();
    }

    fn start_recent_preview_worker(&mut self) {
        if self.recent_preview_worker_started {
            return;
        }
        self.recent_preview_worker_started = true;
        let Some(requests) = self.recent_preview_request_receiver.take() else {
            return;
        };
        let Some(results) = self.recent_preview_result_sender.take() else {
            return;
        };
        let current_generation = Arc::clone(&self.recent_preview_generation_signal);
        let _ = std::thread::Builder::new()
            .name(String::from("modeler3d-recent-previews"))
            .spawn(move || {
                while let Ok(request) = requests.recv() {
                    if request.generation != current_generation.load(Ordering::Relaxed) {
                        continue;
                    }
                    let image = crate::recent_preview::load_recent_preview(&request.path);
                    if request.generation != current_generation.load(Ordering::Relaxed) {
                        continue;
                    }
                    if results
                        .send(RecentPreviewResult {
                            generation: request.generation,
                            path: request.path,
                            image,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });
    }

    fn render_recent_entry(&mut self, ui: &mut Ui, entry: &RecentEntry, palette: ThemePalette) {
        let kind_label = match entry.kind {
            RecentEntryKind::Supported => "Ready",
            RecentEntryKind::Missing => "Missing",
            RecentEntryKind::Unsupported => "Unsupported format",
        };
        egui::Frame::new()
            .fill(palette.panel)
            .stroke(Stroke::new(1.0, palette.border))
            .corner_radius(10.0)
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                ui.set_min_size(Vec2::new(256.0, 62.0));
                ui.horizontal(|ui| {
                    let is_project = entry
                        .path
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("3dp"));
                    let (icon_rect, _) = ui.allocate_exact_size(Vec2::splat(42.0), Sense::hover());
                    let preview_rect =
                        egui::Rect::from_center_size(icon_rect.center(), Vec2::splat(40.0));
                    if let Some(texture) = self.recent_preview_textures.get(&entry.path) {
                        ui.painter().image(
                            texture.id(),
                            preview_rect,
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    } else {
                        self.paint_original_icon(
                            ui,
                            egui::Rect::from_center_size(icon_rect.center(), Vec2::splat(34.0)),
                            if is_project {
                                OriginalAsset::Action("Folder and Open.png")
                            } else {
                                OriginalAsset::Insert("Cube")
                            },
                            palette,
                        );
                    }
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        let color = if entry.kind == RecentEntryKind::Supported {
                            palette.text
                        } else {
                            palette.muted
                        };
                        ui.label(RichText::new(&entry.label).color(color).strong());
                        ui.label(RichText::new(kind_label).small().color(palette.muted));
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if entry.kind == RecentEntryKind::Supported {
                            if ui.small_button("Open").clicked() {
                                self.open_recent(&entry.path);
                            }
                        } else if ui.small_button("Remove").clicked() {
                            self.remove_recent_file(&entry.path);
                            if entry.kind == RecentEntryKind::Missing {
                                self.commands.set_status(format!(
                                    "File not found; removed recent entry: {}",
                                    entry.path.display()
                                ));
                            } else {
                                self.commands.set_status("Removed recent entry");
                            }
                        }
                    });
                });
                ui.label(
                    RichText::new(entry.path.display().to_string())
                        .small()
                        .color(palette.muted),
                );
            });
    }

    fn open_recent(&mut self, path: &Path) {
        match self.try_open_recent(path, true) {
            Ok(RecentOpen::Missing) => self.commands.set_status(format!(
                "File not found; removed recent entry: {}",
                path.display()
            )),
            Ok(_) => {}
            Err(error) => self.report_command_error(error),
        }
    }

    /// Opens one recent entry the way the Welcome list does: a missing file
    /// is dropped from the list, a `.3dp` opens as the project (with unsaved
    /// changes it asks first when `ask_if_dirty`, otherwise it is refused by
    /// the caller), and a model is imported into the current scene.
    fn try_open_recent(
        &mut self,
        path: &Path,
        ask_if_dirty: bool,
    ) -> Result<RecentOpen, CommandError> {
        if !path.exists() {
            self.remove_recent_file(path);
            return Ok(RecentOpen::Missing);
        }
        let is_project = recent_is_project(path);
        if is_project && self.commands.is_dirty() && ask_if_dirty {
            self.confirm_action = Some(PendingReplacement::OpenProject(path.to_path_buf()));
            return Ok(RecentOpen::AskedToSave);
        }
        if is_project {
            self.open_project(path)?;
        } else {
            self.commands.import_from_path(path)?;
        }
        self.surface = WorkspaceSurface::Scene;
        self.invalidate_presentation();
        self.persist_preferences();
        Ok(if is_project {
            RecentOpen::Project
        } else {
            RecentOpen::Model
        })
    }

    fn render_sidebar_action(
        &mut self,
        ui: &mut Ui,
        label: &str,
        command: CommandId,
        palette: ThemePalette,
        has_objects: bool,
    ) -> bool {
        let enabled = has_objects && command_enabled(command, &self.commands);
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_HEIGHT), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, label));
        if enabled && (response.hovered() || response.has_focus()) {
            ui.painter()
                .rect_filled(rect, RADIUS_CONTROL, palette.hover);
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect,
                RADIUS_CONTROL,
                Stroke::new(1.5, palette.accent_border),
                egui::StrokeKind::Inside,
            );
        }
        let color = if enabled {
            palette.text
        } else {
            palette.disabled_text
        };
        let icon_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 8.0, rect.center().y - 8.0),
            Vec2::splat(16.0),
        );
        let glyph = match command {
            CommandId::Group => Some(Glyph::Group),
            CommandId::Ungroup => Some(Glyph::Ungroup),
            CommandId::SelectAll => Some(Glyph::SelectAll),
            CommandId::DeselectAll => Some(Glyph::DeselectAll),
            CommandId::InvertSelection => Some(Glyph::InvertSelection),
            _ => None,
        };
        if let Some(glyph) = glyph {
            let glyph_color = if enabled {
                palette.muted
            } else {
                palette.disabled_text
            };
            crate::chrome::paint_glyph(ui.painter(), icon_rect, glyph, glyph_color);
        } else {
            self.paint_command_icon(
                ui,
                icon_rect,
                command,
                IconKey::Edit,
                if enabled {
                    palette
                } else {
                    ThemePalette {
                        accent: palette.disabled_text,
                        ..palette
                    }
                },
            );
        }
        ui.painter().text(
            egui::pos2(rect.left() + 32.0, rect.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(13.0),
            color,
        );
        response
            .on_hover_text(if enabled {
                command_help(command)
            } else {
                command_disabled_help(command, &self.commands)
            })
            .clicked()
            && enabled
    }

    fn render_objects(&mut self, ui: &mut Ui) {
        let palette = ThemePalette::for_mode(self.theme_mode);
        let rows = self
            .commands
            .scene()
            .objects()
            .iter()
            .enumerate()
            .map(|(index, object)| {
                (
                    index,
                    object.name().to_owned(),
                    object.is_visible(),
                    self.commands.scene().selection().contains(&index),
                    object.children().len(),
                )
            })
            .collect::<Vec<_>>();
        if self.item_revision != self.commands.scene().revision() {
            self.item_textures.clear();
            self.item_health.clear();
            for object in self.commands.scene().objects() {
                if object.mesh().faces.len() <= 1_000_000 {
                    self.item_health
                        .insert(object.object_id().to_owned(), item_is_healthy(object));
                }
            }
            self.item_revision = self.commands.scene().revision();
        }
        let modifiers = ui.ctx().input(|input| input.modifiers);
        let mut selection_click = None;
        let mut centre_click = None;
        let mut repair_click = None;
        let mut visibility_toggle = None;
        let mut command_click = None;
        let mut context_command = None;
        egui::Panel::right("modeler3d-items")
            .resizable(true)
            .default_size(236.0)
            .size_range(210.0..=250.0)
            .frame(
                egui::Frame::new()
                    .fill(palette.panel)
                    .stroke(Stroke::new(1.0, palette.divider))
                    .inner_margin(egui::Margin::same(10)),
            )
            .show(ui, |ui| {
                egui::Frame::new()
                    .fill(palette.surface_alt)
                    .stroke(Stroke::new(1.0, palette.border))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::same(8))
                    .show(ui, |ui| {
                        for (label, command) in [
                            ("Group", CommandId::Group),
                            ("Ungroup", CommandId::Ungroup),
                            ("Select all", CommandId::SelectAll),
                            ("Deselect all", CommandId::DeselectAll),
                            ("Invert selection", CommandId::InvertSelection),
                        ] {
                            if self.render_sidebar_action(
                                ui,
                                label,
                                command,
                                palette,
                                !rows.is_empty(),
                            ) {
                                command_click = Some(command);
                            }
                            ui.add_space(2.0);
                        }
                        ui.separator();
                        ui.checkbox(&mut self.sticky_selection, "Sticky selection")
                            .on_hover_text("Keep the current selection when clicking empty space.");
                    });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Items")
                            .size(15.0)
                            .strong()
                            .color(palette.text),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} of {} selected",
                                self.commands.selected_count(),
                                rows.len()
                            ))
                            .small()
                            .color(palette.muted),
                        );
                    });
                });
                ui.add_space(6.0);
                let inspector_reserved = if rows.is_empty() { 0.0 } else { 210.0 };
                let list_max_height =
                    (ui.available_height() - inspector_reserved).clamp(80.0, 150.0);
                egui::ScrollArea::vertical()
                    .id_salt("object-list")
                    .max_height(list_max_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (index, name, visible, selected, child_count) in &rows {
                            ui.horizontal(|ui| {
                                let visibility_label = if *visible { "Shown" } else { "Hidden" };
                                let (icon_slot, _) =
                                    ui.allocate_exact_size(Vec2::splat(28.0), Sense::hover());
                                self.paint_object_icon(ui, icon_slot, *index, palette);
                                let object_id = self.commands.scene().objects()[*index].object_id();
                                if self.item_health.get(object_id) == Some(&false)
                                    && crate::chrome::glyph_button(
                                        ui,
                                        Glyph::Warning,
                                        Color32::from_rgb(0xE0, 0x9A, 0x2B),
                                        palette.hover,
                                        palette.accent_border,
                                        &format!("Repair {name}"),
                                    )
                                    .on_hover_text(
                                        "Mesh needs repair; select and repair this object",
                                    )
                                    .clicked()
                                {
                                    repair_click = Some(*index);
                                }
                                if crate::chrome::glyph_button(
                                    ui,
                                    if *visible {
                                        Glyph::Visible
                                    } else {
                                        Glyph::Hidden
                                    },
                                    if *visible {
                                        palette.muted
                                    } else {
                                        palette.disabled_text
                                    },
                                    palette.hover,
                                    palette.accent_border,
                                    &format!("{visibility_label}: {name}"),
                                )
                                .on_hover_text(if *visible {
                                    format!("Hide {name}")
                                } else {
                                    format!("Show {name}")
                                })
                                .clicked()
                                {
                                    visibility_toggle = Some((*index, !*visible));
                                }
                                let trailing = if *child_count > 0 { 34.0 } else { 0.0 };
                                let (row_rect, row_response) = ui.allocate_exact_size(
                                    Vec2::new((ui.available_width() - trailing).max(40.0), 24.0),
                                    Sense::click(),
                                );
                                row_response.widget_info(|| {
                                    egui::WidgetInfo::selected(
                                        egui::Role::Button,
                                        true,
                                        *selected,
                                        name.as_str(),
                                    )
                                });
                                if *selected {
                                    ui.painter().rect_filled(
                                        row_rect,
                                        RADIUS_CONTROL,
                                        palette.pressed,
                                    );
                                } else if row_response.hovered() || row_response.has_focus() {
                                    ui.painter().rect_filled(
                                        row_rect,
                                        RADIUS_CONTROL,
                                        palette.hover,
                                    );
                                }
                                if row_response.has_focus() {
                                    ui.painter().rect_stroke(
                                        row_rect,
                                        RADIUS_CONTROL,
                                        Stroke::new(1.5, palette.accent_border),
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                let name_color = if *selected {
                                    palette.accent_dark
                                } else if *visible {
                                    palette.text
                                } else {
                                    palette.disabled_text
                                };
                                let name_galley = ui.painter().layout(
                                    name.clone(),
                                    FontId::proportional(13.0),
                                    name_color,
                                    f32::INFINITY,
                                );
                                ui.painter_at(row_rect.shrink2(Vec2::new(8.0, 0.0))).galley(
                                    egui::pos2(
                                        row_rect.left() + 8.0,
                                        row_rect.center().y - name_galley.size().y * 0.5,
                                    ),
                                    name_galley,
                                    name_color,
                                );
                                let response = row_response.on_hover_text(if *child_count > 0 {
                                    "Click to select; Ctrl/Command adds; Shift selects a range;                                      double-click centres on the whole group"
                                } else {
                                    "Click to select; Ctrl/Command adds; Shift selects a range;                                      double-click centres on it"
                                });
                                if response.clicked() {
                                    selection_click = Some(*index);
                                }
                                if response.double_clicked() {
                                    centre_click = Some(*index);
                                }
                                response.context_menu(|ui| {
                                    ui.label(RichText::new(name).strong());
                                    ui.separator();
                                    for (action_label, action) in [
                                        ("Copy", CommandId::Copy),
                                        ("Cut", CommandId::Cut),
                                        ("Paste", CommandId::Paste),
                                        ("Duplicate", CommandId::Duplicate),
                                        ("Delete", CommandId::Delete),
                                    ] {
                                        if ui
                                            .add_enabled(
                                                command_enabled(action, &self.commands),
                                                egui::Button::new(action_label),
                                            )
                                            .clicked()
                                        {
                                            context_command = Some(action);
                                            ui.close();
                                        }
                                    }
                                });
                                if *child_count > 0 {
                                    ui.label(
                                        RichText::new(format!("({child_count})"))
                                            .small()
                                            .color(palette.muted),
                                    );
                                }
                            });
                        }
                    });
                if rows.is_empty() {
                    ui.add_space(10.0);
                    ui.label(RichText::new("No objects yet").color(palette.muted));
                    ui.label(RichText::new("Use Insert or Import Model to begin.").weak());
                } else {
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .id_salt("item-inspector")
                        .max_height(ui.available_height())
                        .show(ui, |ui| {
                            self.render_transform_inspector(ui, palette);
                            self.render_arrange_control(ui, palette);
                        });
                }
            });
        if let Some(command) = command_click {
            self.dispatch(command);
        }
        if let Some(index) = selection_click {
            self.select_item_row(index, modifiers);
            self.commands.set_status(format!(
                "Selected {}",
                self.commands.scene().objects()[index].name()
            ));
            self.invalidate_presentation();
        }
        if let Some(index) = centre_click {
            // Both clicks of a double-click also arrive as single clicks
            // (select, then toggle off); centring ends with this row as the
            // only selection, exactly like a double-click in the 3D view.
            self.centre_on_object(index);
        }
        if let Some(command) = context_command {
            self.dispatch(command);
        }
        if let Some(index) = repair_click {
            self.commands.select_index(index);
            self.dispatch(CommandId::Repair);
        }
        if let Some((index, visible)) = visibility_toggle {
            match self.commands.set_visibility(index, visible) {
                Ok(CommandOutcome::Applied(message)) | Ok(CommandOutcome::Noop(message)) => {
                    self.commands.set_status(message);
                    self.invalidate_presentation();
                    self.persist_preferences();
                }
                Err(error) => self.report_command_error(error),
                Ok(CommandOutcome::OpenDialog(_) | CommandOutcome::ExitRequested) => {}
            }
        }
    }

    fn select_item_row(&mut self, index: usize, modifiers: egui::Modifiers) {
        let anchor = if modifiers.shift {
            self.selection_anchor
                .as_deref()
                .and_then(|id| self.commands.scene().object_index(id).ok())
        } else {
            None
        };
        self.commands
            .select_index_with_modifiers(index, !modifiers.shift, anchor);
        self.selection_anchor = self
            .commands
            .scene()
            .objects()
            .get(index)
            .map(|o| o.object_id().to_owned());
    }

    fn render_transform_inspector(&mut self, ui: &mut Ui, palette: ThemePalette) {
        let selected = self
            .commands
            .scene()
            .selection()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        if selected.is_empty() {
            ui.label(RichText::new("Select an object to edit its transform.").weak());
            return;
        }
        let index = selected[0];
        let unit = self.commands.preferences().display_unit();
        let signature = format!(
            "{}:{selected:?}:{}:{}",
            self.commands.scene().revision(),
            unit.label(),
            self.transform_panel_mode.label()
        );
        if signature != self.transform_signature {
            self.transform_signature = signature;
            self.last_numeric_axis = None;
            self.numeric_edited_axes = [false; 3];
            let object = &self.commands.scene().objects()[index];
            self.name_field = object.name().to_owned();
            let bounds = world_bounds(self.commands.scene(), &selected).ok();
            let extents = bounds
                .map(|bounds| {
                    [
                        bounds.maximum[0] - bounds.minimum[0],
                        bounds.maximum[1] - bounds.minimum[1],
                        bounds.maximum[2] - bounds.minimum[2],
                    ]
                })
                .unwrap_or([0.0; 3]);
            self.position_fields = if selected.len() > 1 {
                bounds.map_or([0.; 3], |b| b.center())
            } else {
                object.position()
            }
            .into_iter()
            .map(|value| format_distance(value, unit))
            .collect::<Vec<_>>()
            .try_into()
            .unwrap_or_else(|_| [String::new(), String::new(), String::new()]);
            self.size_fields = extents
                .into_iter()
                .map(|value| format_distance(value, unit))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap_or_else(|_| [String::new(), String::new(), String::new()]);
            self.rotation_fields = crate::rotation::degrees(object.transform())
                .unwrap_or([0.0; 3])
                .map(crate::numeric_controls::format_angle_degrees);
            self.scale_fields = [
                String::from("1.0x"),
                String::from("1.0x"),
                String::from("1.0x"),
            ];
        }
        if selected.len() == 1 {
            ui.horizontal(|ui| {
                let rename_width = 64.0;
                ui.add_sized(
                    [
                        (ui.available_width() - rename_width - ui.spacing().item_spacing.x)
                            .max(80.0),
                        CONTROL_HEIGHT,
                    ],
                    egui::TextEdit::singleline(&mut self.name_field)
                        .align(egui::Align2::LEFT_CENTER),
                );
                if ui
                    .add_sized([rename_width, CONTROL_HEIGHT], egui::Button::new("Rename"))
                    .clicked()
                {
                    match self.commands.rename_selected(self.name_field.clone()) {
                        Ok(_) => self.invalidate_presentation(),
                        Err(error) => self.transform_error = Some(error.to_string()),
                    }
                }
            });
        }
        if ui.input(|input| input.key_pressed(Key::Escape)) {
            self.transform_signature.clear();
        }
        ui.collapsing("Transform", |ui| {
            ui.label(
                RichText::new(format!(
                    "{} values (millimetres are authoritative)",
                    self.transform_panel_mode.label()
                ))
                .small()
                .color(palette.muted),
            );
            match self.transform_panel_mode {
                TransformPanelMode::Move => {
                    for (axis_index, (axis, field)) in ["X", "Y", "Z"]
                        .into_iter()
                        .zip(&mut self.position_fields)
                        .enumerate()
                    {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [14.0, CONTROL_HEIGHT],
                                egui::Label::new(
                                    RichText::new(axis)
                                        .strong()
                                        .color(crate::chrome::AXIS_COLORS[axis_index]),
                                ),
                            );
                            if ui
                                .add_sized(
                                    [132.0, CONTROL_HEIGHT],
                                    egui::TextEdit::singleline(field)
                                        .align(egui::Align2::LEFT_CENTER),
                                )
                                .changed()
                            {
                                self.last_numeric_axis = Some(axis_index);
                                self.numeric_edited_axes[axis_index] = true;
                            }
                        });
                    }
                    if ui
                        .button("Apply move")
                        .on_hover_text("Apply the current position and preserve the size")
                        .clicked()
                    {
                        self.apply_transform_fields(index, unit);
                    }
                }
                TransformPanelMode::Rotate => {
                    for (axis_index, (axis, field)) in ["X", "Y", "Z"]
                        .into_iter()
                        .zip(&mut self.rotation_fields)
                        .enumerate()
                    {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [14.0, CONTROL_HEIGHT],
                                egui::Label::new(
                                    RichText::new(axis)
                                        .strong()
                                        .color(crate::chrome::AXIS_COLORS[axis_index]),
                                ),
                            );
                            if ui
                                .add_sized(
                                    [132.0, CONTROL_HEIGHT],
                                    egui::TextEdit::singleline(field)
                                        .align(egui::Align2::LEFT_CENTER),
                                )
                                .changed()
                            {
                                self.last_numeric_axis = Some(axis_index);
                                self.numeric_edited_axes[axis_index] = true;
                            }
                        });
                    }
                    if ui
                        .button("Apply rotation")
                        .on_hover_text("Apply one rotation history command")
                        .clicked()
                    {
                        self.apply_rotation_fields();
                    }
                }
                TransformPanelMode::Resize => {
                    ui.checkbox(&mut self.proportional_resize, "Keep proportions");
                    ui.horizontal(|ui| {
                        ui.label("Resize basis");
                        ui.selectable_value(&mut self.object_resize_basis, false, "Grid");
                        ui.selectable_value(&mut self.object_resize_basis, true, "Object");
                    });
                    for (axis_index, (axis, field)) in ["X", "Y", "Z"]
                        .into_iter()
                        .zip(&mut self.size_fields)
                        .enumerate()
                    {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [14.0, CONTROL_HEIGHT],
                                egui::Label::new(
                                    RichText::new(axis)
                                        .strong()
                                        .color(crate::chrome::AXIS_COLORS[axis_index]),
                                ),
                            );
                            if ui
                                .add_sized(
                                    [132.0, CONTROL_HEIGHT],
                                    egui::TextEdit::singleline(field)
                                        .align(egui::Align2::LEFT_CENTER),
                                )
                                .changed()
                            {
                                self.last_numeric_axis = Some(axis_index);
                                self.numeric_edited_axes[axis_index] = true;
                            }
                        });
                    }
                    if ui
                        .button("Apply size")
                        .on_hover_text("Apply the current size and preserve the position")
                        .clicked()
                    {
                        self.apply_transform_fields(index, unit);
                    }
                    ui.label("Scale factors");
                    for (axis, field) in ["X", "Y", "Z"].into_iter().zip(&mut self.scale_fields) {
                        ui.horizontal(|ui| {
                            ui.label(axis);
                            ui.add_sized([132.0, 22.0], egui::TextEdit::singleline(field));
                        });
                    }
                    if ui
                        .button("Apply scale")
                        .on_hover_text("Apply one scale history command")
                        .clicked()
                    {
                        self.apply_scale_fields();
                    }
                }
            }
            ui.horizontal(|ui| {
                let mut display_unit = unit;
                egui::ComboBox::from_id_salt("display-unit")
                    .selected_text(display_unit.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut display_unit, DisplayUnit::Millimetre, "mm");
                        ui.selectable_value(&mut display_unit, DisplayUnit::Inch, "in");
                    });
                if display_unit != unit {
                    self.apply_display_unit(display_unit);
                }
            });
            if let Some(error) = &self.transform_error {
                ui.label(RichText::new(error).color(Color32::from_rgb(0xB0, 0x35, 0x35)));
            }
        });
    }

    fn paint_object_icon(
        &mut self,
        ui: &Ui,
        slot: egui::Rect,
        index: usize,
        _palette: ThemePalette,
    ) {
        let object = &self.commands.scene().objects()[index];
        let id = object.object_id().to_owned();
        if !self.item_textures.contains_key(&id)
            && let Some(image) = crate::recent_preview::render_object_preview(object)
        {
            let texture =
                ui.ctx()
                    .load_texture(format!("item-{id}"), image, egui::TextureOptions::LINEAR);
            self.item_textures.insert(id.clone(), texture);
        }
        if let Some(texture) = self.item_textures.get(&id) {
            ui.painter().image(
                texture.id(),
                slot,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
    }

    fn render_transform_strip(&mut self, ctx: &egui::Context) {
        if self.surface != WorkspaceSurface::Scene {
            self.transform_strip_rect = None;
            return;
        }
        let palette = ThemePalette::for_mode(self.theme_mode);
        let response = egui::Area::new(egui::Id::unique("modeler3d-transform-strip"))
            .enabled(!self.interaction_blocked())
            .order(egui::Order::Foreground)
            .anchor(Align2::CENTER_BOTTOM, Vec2::new(0.0, -12.0))
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(palette.panel)
                    .stroke(Stroke::new(1.0, palette.border))
                    .corner_radius(crate::chrome::RADIUS_SURFACE)
                    .shadow(theme::floating_shadow(self.theme_mode))
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .show(ui, |ui| {
                        // Each tab sits straight above its box, the same
                        // width, with a blank over the axis letter (OF-65).
                        ui.horizontal(|ui| {
                            for mode in [
                                TransformPanelMode::Move,
                                TransformPanelMode::Rotate,
                                TransformPanelMode::Resize,
                            ] {
                                ui.allocate_exact_size(
                                    Vec2::new(TRANSFORM_AXIS_LABEL_WIDTH, 28.0),
                                    egui::Sense::hover(),
                                );
                                let selected = mode == self.transform_panel_mode;
                                let button = egui::Button::new(RichText::new(mode.label()).color(
                                    if selected {
                                        palette.accent_dark
                                    } else {
                                        palette.text
                                    },
                                ))
                                .stroke(Stroke::NONE)
                                .corner_radius(RADIUS_CONTROL)
                                .fill(if selected {
                                    palette.pressed
                                } else {
                                    Color32::TRANSPARENT
                                });
                                if ui
                                    .add_sized([TRANSFORM_FIELD_WIDTH, 28.0], button)
                                    .clicked()
                                {
                                    self.transform_panel_mode = mode;
                                }
                            }
                        });
                        if self.commands.selected_count() > 0 {
                            let mut apply = false;
                            ui.horizontal(|ui| {
                                let fields = match self.transform_panel_mode {
                                    TransformPanelMode::Move => &mut self.position_fields,
                                    TransformPanelMode::Rotate => &mut self.rotation_fields,
                                    TransformPanelMode::Resize => &mut self.size_fields,
                                };
                                for (axis_index, (axis, field)) in
                                    ["X", "Y", "Z"].into_iter().zip(fields).enumerate()
                                {
                                    ui.add_sized(
                                        [TRANSFORM_AXIS_LABEL_WIDTH, CONTROL_HEIGHT],
                                        egui::Label::new(
                                            RichText::new(axis)
                                                .strong()
                                                .color(crate::chrome::AXIS_COLORS[axis_index]),
                                        ),
                                    );
                                    let response = ui.add_sized(
                                        [TRANSFORM_FIELD_WIDTH, CONTROL_HEIGHT],
                                        egui::TextEdit::singleline(field)
                                            .align(egui::Align2::LEFT_CENTER),
                                    );
                                    if response.changed() {
                                        self.last_numeric_axis = Some(axis_index);
                                        self.numeric_edited_axes[axis_index] = true;
                                    }
                                    if (response.has_focus() || response.lost_focus())
                                        && ui.input(|input| input.key_pressed(Key::Enter))
                                    {
                                        apply = true;
                                    }
                                    if response.has_focus()
                                        && ui.input(|input| input.key_pressed(Key::Escape))
                                    {
                                        self.transform_signature.clear();
                                    }
                                }
                                apply |= ui
                                    .add_sized([60.0, CONTROL_HEIGHT], egui::Button::new("Apply"))
                                    .clicked();
                            });
                            if apply {
                                if self.transform_panel_mode == TransformPanelMode::Rotate {
                                    self.apply_rotation_fields();
                                } else if let Some(index) =
                                    self.commands.scene().selection().iter().next().copied()
                                {
                                    self.apply_transform_fields(
                                        index,
                                        self.commands.preferences().display_unit(),
                                    );
                                }
                            }
                        }
                    });
            });
        self.transform_strip_rect = Some(response.response.rect);
    }

    fn render_arrange_control(&mut self, ui: &mut Ui, palette: ThemePalette) {
        if self.commands.selected_count() < 2 {
            return;
        }
        ui.collapsing("Arrange selection", |ui| {
            ui.label(
                RichText::new("Place selected objects along X with a bounded gap.")
                    .small()
                    .color(palette.muted),
            );
            ui.horizontal(|ui| {
                ui.label("Gap");
                ui.add_sized(
                    [100.0, 22.0],
                    egui::TextEdit::singleline(&mut self.arrange_gap_field),
                );
                if ui.button("Arrange").clicked() {
                    match parse_distance_mm(
                        &self.arrange_gap_field,
                        self.commands.preferences().display_unit(),
                    ) {
                        Ok(gap) => match self.commands.arrange_selected_with_gap(gap) {
                            Ok(CommandOutcome::Applied(message))
                            | Ok(CommandOutcome::Noop(message)) => {
                                self.commands.set_status(message);
                                self.invalidate_presentation();
                            }
                            Err(error) => self.transform_error = Some(error.to_string()),
                            Ok(CommandOutcome::OpenDialog(_) | CommandOutcome::ExitRequested) => {}
                        },
                        Err(error) => self.transform_error = Some(error.to_owned()),
                    }
                }
            });
        });
    }

    fn apply_rotation_fields(&mut self) {
        if !self.numeric_edited_axes.contains(&true) {
            return;
        }
        let Some(index) = self.commands.scene().selection().iter().next().copied() else {
            return;
        };
        let mut values =
            match crate::rotation::degrees(self.commands.scene().objects()[index].transform()) {
                Ok(values) => values,
                Err(error) => {
                    self.transform_error = Some(error.to_string());
                    return;
                }
            };
        for (axis, value) in values.iter_mut().enumerate() {
            if self.numeric_edited_axes[axis] {
                match parse_angle_degrees(&self.rotation_fields[axis]) {
                    Ok(parsed) => *value = parsed,
                    Err(error) => {
                        self.transform_error = Some(format!("Rotation: {error}"));
                        return;
                    }
                }
            }
        }
        match self.commands.set_selected_rotation(values) {
            Ok(CommandOutcome::Applied(message)) | Ok(CommandOutcome::Noop(message)) => {
                self.transform_error = None;
                self.commands.set_status(message);
                self.transform_signature.clear();
                self.numeric_edited_axes = [false; 3];
                self.invalidate_presentation();
                self.persist_preferences();
            }
            Err(error) => self.transform_error = Some(error.to_string()),
            Ok(CommandOutcome::OpenDialog(_) | CommandOutcome::ExitRequested) => {}
        }
    }

    fn apply_scale_fields(&mut self) {
        let values = self
            .scale_fields
            .iter()
            .map(|field| parse_scale_factor(field).map_err(str::to_owned))
            .collect::<Result<Vec<_>, _>>();
        let Ok(values) = values else {
            self.transform_error = Some(String::from("Scale: enter positive finite factors"));
            return;
        };
        let Ok(values) = values.try_into() else {
            self.transform_error = Some(String::from("Scale: enter all three values"));
            return;
        };
        match self.commands.scale_selected(values) {
            Ok(CommandOutcome::Applied(message)) | Ok(CommandOutcome::Noop(message)) => {
                self.transform_error = None;
                self.commands.set_status(message);
                self.invalidate_presentation();
                self.persist_preferences();
            }
            Err(error) => self.transform_error = Some(error.to_string()),
            Ok(CommandOutcome::OpenDialog(_) | CommandOutcome::ExitRequested) => {}
        }
    }

    fn apply_transform_fields(&mut self, index: usize, unit: DisplayUnit) {
        if !self.numeric_edited_axes.contains(&true) {
            return;
        }
        let indices = self
            .commands
            .scene()
            .selection()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        let bounds = match world_bounds(self.commands.scene(), &indices) {
            Ok(bounds) => bounds,
            Err(error) => {
                self.transform_error = Some(error.to_string());
                return;
            }
        };
        // Display strings are rounded. Only edited axes may replace exact model values.
        let parse = |fields: &[String; 3], mut values: [f32; 3]| -> Result<[f32; 3], String> {
            for (axis, value) in values.iter_mut().enumerate() {
                if self.numeric_edited_axes[axis] {
                    *value = parse_distance_mm(&fields[axis], unit).map_err(str::to_owned)?;
                }
            }
            Ok(values)
        };
        let outcome = if self.transform_panel_mode == TransformPanelMode::Move {
            let current = if indices.len() == 1 {
                self.commands.scene().objects()[index].position()
            } else {
                bounds.center()
            };
            match parse(&self.position_fields, current) {
                Ok(position) => self.commands.set_selected_position(position),
                Err(error) => {
                    self.transform_error = Some(format!("Position: {error}"));
                    return;
                }
            }
        } else {
            let mut size = match parse(&self.size_fields, bounds.extents()) {
                Ok(size) => size,
                Err(error) => {
                    self.transform_error = Some(format!("Size: {error}"));
                    return;
                }
            };
            if self.proportional_resize
                && let Ok(bounds) = world_bounds(self.commands.scene(), &indices)
            {
                let current = bounds.extents();
                let edited = self
                    .last_numeric_axis
                    .or_else(|| (0..3).find(|i| (size[*i] - current[*i]).abs() > 1e-5));
                if let Some(axis) = edited
                    && current[axis] > f32::EPSILON
                {
                    size = current.map(|value| value * size[axis] / current[axis]);
                }
            }
            self.commands.set_selected_size(size)
        };
        match outcome {
            Ok(_) => {
                self.transform_error = None;
                self.transform_signature.clear();
                self.numeric_edited_axes = [false; 3];
                self.commands.set_status(format!(
                    "Updated {}",
                    self.commands.scene().objects()[index].name()
                ));
                self.invalidate_presentation();
                self.persist_preferences();
            }
            Err(error) => self.transform_error = Some(error.to_string()),
        }
    }

    fn render_operation_progress(&mut self, ctx: &egui::Context) {
        let Ok(Some(status)) = self.commands.operations().active_status() else {
            return;
        };
        // Only an active operation needs progress and cancellation controls.
        egui::Window::new(&status.title)
            .id(egui::Id::unique("modeler3d-operation-progress"))
            .anchor(Align2::RIGHT_BOTTOM, Vec2::new(-12.0, -12.0))
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(status.phase.as_str());
                if let Some(progress) = status.progress {
                    ui.add(
                        egui::ProgressBar::new(f32::from(progress) / 100.0).desired_width(160.0),
                    );
                }
                if matches!(
                    status.phase,
                    OperationPhase::Queued | OperationPhase::Running
                ) {
                    if ui.button("Cancel").clicked() {
                        let _ = self.commands.operations().cancel(&status.operation_id);
                        self.commands
                            .set_status(format!("{} cancellation requested", status.title));
                    }
                } else if status.phase == OperationPhase::Cancelling {
                    ui.add_enabled(false, egui::Button::new("Cancelling..."));
                } else if status.phase == OperationPhase::Committing {
                    ui.add_enabled(false, egui::Button::new("Applying..."));
                }
            });
    }

    fn render_viewport(&mut self, ui: &mut Ui) {
        self.refresh_preview();
        let palette = ThemePalette::for_mode(self.theme_mode);
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(palette.viewport))
            .show(ui, |ui| {
                let rect = ui.available_rect_before_wrap();
                let response = ui.allocate_rect(rect, Sense::click_and_drag());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 0.0, palette.viewport);
                let pixels_per_point = ui.ctx().pixels_per_point();
                self.resize_viewport_target(rect.size(), pixels_per_point);
                if let Some(renderer) = &self.viewport_renderer {
                    let (width, height) = renderer.size();
                    let _ = self.camera.set_viewport_size(width, height);
                }
                self.frame_camera_if_pending();
                let input = viewport_input(ui.ctx());
                let interactions_allowed = viewport_pointer_interactions_allowed(
                    self.interaction_blocked(),
                    response.contains_pointer(),
                    self.viewport_drag.is_some(),
                );
                self.update_gizmo_frame();
                self.viewport_gizmo_consumed = false;
                self.route_viewport_input(rect, pixels_per_point, input, interactions_allowed);
                if response.double_clicked()
                    && interactions_allowed
                    && let Some(pointer) = input.pointer_position
                    && rect.contains(pointer)
                {
                    self.frame_double_clicked(rect, pointer);
                }
                // Pointer release can commit a model command after the frame's
                // initial preview refresh. Rebuild before rendering with the
                // committed scene revision so floor cues use matching bounds.
                self.refresh_preview();
                self.update_gizmo_frame();
                let staged_review_drag = self.stage_review_resize_drag(rect);
                self.sync_resize_size_fields();
                let theme_color = palette
                    .viewport
                    .to_srgba_unmultiplied()
                    .map(|channel| f32::from(channel) / 255.0);
                let gizmo_mode = self.gizmo_mode();
                let display = self.commands.display_state();
                let selected_object_ids = self
                    .commands
                    .scene()
                    .selection()
                    .iter()
                    .filter_map(|index| self.commands.scene().objects().get(*index))
                    .map(|object| object.object_id().to_owned())
                    .collect::<Vec<_>>();
                let text_atlas = self.wgpu_render_state.as_ref().and_then(|state| {
                    state
                        .renderer
                        .read()
                        .texture(&egui::TextureId::default())
                        .and_then(|texture| texture.texture.clone())
                });
                let atlas_size = text_atlas.as_ref().map_or_else(
                    || {
                        ui.ctx()
                            .fonts(|fonts| fonts.font_image_size())
                            .map(|size| size as f32)
                    },
                    |texture| [texture.width() as f32, texture.height() as f32],
                );
                let viewport_theme = match self.theme_mode {
                    ThemeMode::Light => ViewportTheme::Light,
                    ThemeMode::Dark => ViewportTheme::Dark,
                };
                let floor_text = crate::floor_labels::floor_label_mesh(
                    &painter,
                    &self.camera,
                    atlas_size,
                    viewport_theme,
                );
                let render_options = ViewportRenderOptions {
                    grid_visible: display.grid_visible,
                    wireframe: display.wireframe,
                    xray: display.xray,
                    selected_object_ids,
                    lighting: self.commands.preferences().viewport_lighting(),
                    theme: viewport_theme,
                    floor_text,
                };
                let derived_preview = match (&self.preview, &self.viewport_drag) {
                    (Some(scene), Some(ViewportDrag::Transform(drag))) => {
                        Mat4::from_rows(drag.delta.rows())
                            .and_then(|delta| scene.with_world_preview(&drag.snapshot.ids(), delta))
                            .ok()
                    }
                    _ => None,
                };
                let mesh_preview = if derived_preview.is_none() {
                    self.preview.as_ref().and_then(|scene| {
                        self.preview_mesh_override
                            .as_ref()
                            .and_then(|(object_id, mesh)| {
                                scene.with_mesh_preview(object_id, mesh).ok()
                            })
                    })
                } else {
                    None
                };
                if let (Some(renderer), Some(scene), Some(texture_id)) = (
                    self.viewport_renderer.as_mut(),
                    derived_preview
                        .as_ref()
                        .or(mesh_preview.as_ref())
                        .or(self.preview.as_ref()),
                    self.viewport_texture_id,
                ) {
                    let mut clear_color_error = None;
                    match renderer.set_clear_color(theme_color) {
                        Ok(()) => self.last_clear_color_error = None,
                        Err(error) => {
                            let error = error.to_string();
                            if self.last_clear_color_error.as_deref() != Some(error.as_str()) {
                                crate::diagnostics::record_error("viewport clear color", &error);
                            }
                            self.last_clear_color_error = Some(error.clone());
                            clear_color_error = Some(error);
                        }
                    }
                    renderer.set_text_atlas(text_atlas.as_ref());
                    let mut cpu_fallback = false;
                    match renderer.render_with_options(
                        scene,
                        &self.camera,
                        if self.viewport_drag.is_some() {
                            InteractionState::Active
                        } else {
                            InteractionState::Resting
                        },
                        self.commands.scene().revision(),
                        None,
                        gizmo_mode,
                        None,
                        &render_options,
                    ) {
                        Ok(report) => {
                            self.viewport_error = clear_color_error;
                            cpu_fallback = report.cpu_fallback;
                            if report.cpu_fallback && self.last_cpu_fallback != Some(true) {
                                crate::diagnostics::record_warning(
                                    "viewport CPU fallback",
                                    format!(
                                        "adapter={} / {} / {}; frame={:.0} ms; triangles={}; shaded pixels={}",
                                        report.adapter.name,
                                        report.adapter.backend,
                                        report.adapter.device_type,
                                        report.frame_time_ms,
                                        report.cpu_triangles,
                                        report.cpu_shaded_pixels
                                    ),
                                );
                            }
                            self.last_cpu_fallback = Some(report.cpu_fallback);
                            crate::diagnostics::record_viewport_frame(&report);
                        }
                        Err(error) => {
                            let error = error.to_string();
                            if self.viewport_error.as_deref() != Some(error.as_str()) {
                                crate::diagnostics::record_error("viewport rendering", &error);
                            }
                            self.viewport_error = Some(error);
                        }
                    }
                    painter.image(
                        texture_id,
                        rect,
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    if cpu_fallback && let Ok(matrix) = self.camera.view_projection(1.0) {
                        crate::floor_labels::paint_floor_label_mesh(
                            &painter,
                            rect,
                            matrix,
                            &render_options.floor_text,
                        );
                    }
                    if cpu_fallback {
                        painter.text(
                            rect.left_top() + egui::vec2(12.0, 10.0),
                            Align2::LEFT_TOP,
                            "CPU viewport fallback",
                            FontId::proportional(11.0),
                            palette.muted,
                        );
                    }
                }
                let hover = ui.ctx().input(|input| input.pointer.hover_pos());
                let hovering = response.hovered() && !self.interaction_blocked();
                if let Some(mode) = self.paint_mode {
                    // Paint hides the move handles and shows the spray can.
                    if hovering
                        && !matches!(self.viewport_drag, Some(ViewportDrag::Orbit | ViewportDrag::Pan))
                        && let Some(pointer) = hover.filter(|pointer| rect.contains(*pointer))
                    {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::None);
                        paint_bar::paint_spray_cursor(&painter, pointer, mode.color, palette);
                    }
                } else if self.measure_mode.is_some() {
                    // Measure hides the move handles and shows the ruler.
                    let pointer = (hovering
                        && !matches!(
                            self.viewport_drag,
                            Some(ViewportDrag::Orbit | ViewportDrag::Pan)
                        ))
                    .then_some(hover)
                    .flatten()
                    .filter(|pointer| rect.contains(*pointer));
                    let free = ui.ctx().input(|input| input.modifiers.shift);
                    self.update_measure_hover(rect, pointer, free);
                    if pointer.is_some() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }
                    self.paint_measure_overlay(&painter, rect, palette);
                } else {
                    if gizmo_mode == GizmoMode::Resize {
                        self.paint_resize_handles(&painter, rect, palette);
                    } else {
                        self.paint_transform_handles(&painter, rect);
                    }
                    if let Some(icon) =
                        self.transform_handle_cursor(rect, hover, pixels_per_point, hovering)
                    {
                        ui.ctx().set_cursor_icon(icon);
                    }
                }
                self.paint_box_selection(&painter, palette);
                if staged_review_drag {
                    self.viewport_drag = None;
                }
                if let Some(error) = self.preview_error.as_ref().or(self.viewport_error.as_ref()) {
                    painter.text(
                        rect.left_top() + egui::vec2(12.0, 10.0),
                        Align2::LEFT_TOP,
                        format!("Viewport unavailable: {error}"),
                        FontId::proportional(12.0),
                        palette.accent_dark,
                    );
                }
                if response.clicked_by(egui::PointerButton::Primary)
                    && !self.viewport_gizmo_consumed
                    && let Some(pointer) = input.pointer_position
                    && rect.contains(pointer)
                {
                    if self.paint_mode.is_some() {
                        self.paint_click(rect, pointer);
                    } else if self.measure_mode.is_some() {
                        self.measure_click(rect, pointer, input.modifiers.shift);
                    } else {
                        self.pick_viewport_object(rect, pointer, input.modifiers);
                    }
                }
            });
    }

    fn resize_viewport_target(&mut self, size_points: Vec2, pixels_per_point: f32) {
        let requested_size = physical_viewport_size(size_points, pixels_per_point);
        let Some(renderer) = self.viewport_renderer.as_mut() else {
            return;
        };
        if renderer.size() == requested_size {
            return;
        }
        match renderer.resize(requested_size.0, requested_size.1) {
            Ok(()) => {
                self.viewport_error = None;
                self.update_native_viewport_texture();
            }
            Err(error) => self.viewport_error = Some(error.to_string()),
        }
    }

    fn update_native_viewport_texture(&mut self) {
        let (Some(render_state), Some(renderer), Some(texture_id)) = (
            self.wgpu_render_state.as_ref(),
            self.viewport_renderer.as_ref(),
            self.viewport_texture_id,
        ) else {
            return;
        };
        render_state
            .renderer
            .write()
            .update_egui_texture_from_wgpu_texture(
                &render_state.device,
                renderer.color_view(),
                eframe::wgpu::FilterMode::Linear,
                texture_id,
            );
    }

    fn frame_camera_if_pending(&mut self) {
        if !self.camera_auto_frame_pending {
            return;
        }
        let Some(scene) = self.preview.as_ref() else {
            return;
        };
        let bounds = scene
            .objects()
            .iter()
            .filter_map(|object| object.world_bounds().ok())
            .collect::<Vec<_>>();
        if bounds.is_empty() {
            return;
        }
        match self.camera.frame_bounds(&bounds) {
            Ok(()) => self.camera_auto_frame_pending = false,
            Err(error) => self.viewport_error = Some(error.to_string()),
        }
        if let Some([yaw, pitch, distance]) = self.review.as_ref().and_then(|r| r.camera()) {
            let mut state = self.camera.state();
            state.yaw_degrees = yaw;
            state.pitch_degrees = pitch.clamp(-89.0, 89.0);
            if distance > 0.0 {
                state.distance = distance;
            }
            if let Err(error) = self.camera.set_state(state) {
                eprintln!("review capture: camera pose rejected: {error}");
            }
        }
        if self
            .review
            .as_ref()
            .is_some_and(|review| review.state() == crate::review_capture::ReviewState::Centre)
        {
            // Index 2 is the sphere; centring needs the real viewport size.
            self.centre_on_object(2);
        }
    }

    /// Frames the exact frontmost object under a double-click without
    /// toggling selection or changing model history. An empty double-click is
    /// deliberately a no-op, matching the reference viewport.
    fn frame_double_clicked(&mut self, rect: egui::Rect, pointer: egui::Pos2) {
        self.viewport_drag = None;
        let Some(scene) = self.preview.as_ref() else {
            return;
        };
        let size = self
            .viewport_renderer
            .as_ref()
            .map_or(self.camera.viewport_size(), ViewportRenderer::size);
        let screen = viewport_pixel_position(pointer, rect, size);
        let Ok(Some(hit)) = pick_scene(scene, &self.camera, screen[0], screen[1]) else {
            self.viewport_gizmo_consumed = true;
            return;
        };
        self.viewport_gizmo_consumed = true;
        if let Ok(index) = self.commands.scene().object_index(&hit.object_id) {
            self.centre_on_object(index);
        }
    }

    /// Centres the camera on one top-level object, as a double-click on it
    /// does in the 3D view or in the Items list (OF-63): the object becomes
    /// the only selection and the camera frames it, keeping its angle. A
    /// group is one top-level object, so its whole outline is framed. A
    /// hidden object is selected but not framed (framing ignores hidden
    /// objects everywhere). No model history is created.
    pub(crate) fn centre_on_object(&mut self, index: usize) -> bool {
        let Some(name) = self
            .commands
            .scene()
            .objects()
            .get(index)
            .map(|object| object.name().to_owned())
        else {
            return false;
        };
        self.commands.select_index(index);
        self.selection_anchor = self
            .commands
            .scene()
            .objects()
            .get(index)
            .map(|object| object.object_id().to_owned());
        self.surface = WorkspaceSurface::Scene;
        let framed = self.frame_objects(&[index]);
        self.commands.set_status(if framed {
            format!("Centred on {name}")
        } else {
            format!("{name} is hidden; show it to centre on it")
        });
        self.invalidate_presentation();
        framed
    }

    /// The one desktop framing rule, shared by the 3D-view and Items
    /// double-click and by MCP `frame_view` in the live app: the camera keeps
    /// its angle and fits the visible objects among `indices` to the current
    /// viewport. Hidden objects are skipped, as in the Python reference.
    /// Returns whether the camera moved.
    pub(crate) fn frame_objects(&mut self, indices: &[usize]) -> bool {
        let scene = self.commands.scene();
        let mut bounds = Vec::new();
        for &index in indices {
            if !scene
                .objects()
                .get(index)
                .is_some_and(|object| object.is_visible())
            {
                continue;
            }
            if let Ok(world) = world_bounds(scene, &[index])
                && let Ok(aabb) = Aabb::new(world.minimum, world.maximum)
            {
                bounds.push(aabb);
            }
        }
        if bounds.is_empty() || self.camera.frame_bounds(&bounds).is_err() {
            return false;
        }
        self.camera_auto_frame_pending = false;
        true
    }

    fn gizmo_mode(&self) -> GizmoMode {
        match self.transform_panel_mode {
            TransformPanelMode::Move => GizmoMode::Move,
            TransformPanelMode::Rotate => GizmoMode::Rotate,
            TransformPanelMode::Resize => GizmoMode::Resize,
        }
    }

    fn update_gizmo_frame(&mut self) {
        let scene = self.commands.scene();
        let indices = scene.selection().iter().copied().collect::<Vec<_>>();
        let key = format!(
            "{}:{indices:?}:{}:{:?}",
            scene.revision(),
            self.object_resize_basis,
            self.transform_panel_mode
        );
        if self
            .gizmo_frame
            .as_ref()
            .is_some_and(|(old, _)| old == &key)
        {
            return;
        }
        self.gizmo_frame = ResizeFrame::from_scene(
            scene,
            &indices,
            self.object_resize_basis && self.transform_panel_mode == TransformPanelMode::Resize,
        )
        .ok()
        .map(|frame| (key, frame));
    }

    fn selected_gizmo_state(&self) -> Option<[f32; 3]> {
        let center = self.gizmo_frame.as_ref()?.1.center;
        if let Some(ViewportDrag::Transform(drag)) = &self.viewport_drag {
            return Mat4::from_rows(drag.delta.rows())
                .ok()?
                .transform_point(center)
                .ok();
        }
        Some(center)
    }

    #[cfg(test)]
    fn project_handle(&self, point: [f32; 3], rect: egui::Rect) -> Option<(egui::Pos2, f32)> {
        let matrix = self
            .camera
            .view_projection(self.current_scene_radius().unwrap_or(1.))
            .ok()?;
        project_to_viewport(matrix, rect, point)
    }

    fn selected_gizmo_bounds(&self) -> Option<Aabb> {
        let (_, frame) = self.gizmo_frame.as_ref()?;
        let center = self.selected_gizmo_state()?;
        let half_extents = frame.extents.map(|extent| extent.abs() * 0.5);
        Aabb::new(
            [
                center[0] - half_extents[0],
                center[1] - half_extents[1],
                center[2] - half_extents[2],
            ],
            [
                center[0] + half_extents[0],
                center[1] + half_extents[1],
                center[2] + half_extents[2],
            ],
        )
        .ok()
    }

    fn paint_transform_handles(&self, painter: &egui::Painter, rect: egui::Rect) {
        let Some(center) = self.selected_gizmo_state() else {
            return;
        };
        let Ok(matrix) = self
            .camera
            .view_projection(self.current_scene_radius().unwrap_or(1.))
        else {
            return;
        };
        let mode = self.gizmo_mode();
        let Ok(geometry) =
            build_gizmo_with_bounds(&self.camera, center, mode, self.selected_gizmo_bounds())
        else {
            return;
        };
        // The interaction controls remain visible over solid meshes on both
        // GPU and CPU paths; hit testing consumes the same world geometry.
        let color = |rgba: [f32; 4]| {
            Color32::from(egui::Rgba::from_rgba_unmultiplied(
                rgba[0], rgba[1], rgba[2], rgba[3],
            ))
        };
        let pixels_per_point = painter.ctx().pixels_per_point().max(1.0e-6);
        let active_axis = self.active_gizmo_axis_index();
        let hovered_axis = active_axis
            .is_none()
            .then(|| {
                painter
                    .ctx()
                    .input(|input| input.pointer.hover_pos())
                    .filter(|position| rect.contains(*position))
                    .and_then(|position| {
                        self.transform_axis_at_pointer(rect, position, pixels_per_point, mode)
                    })
            })
            .flatten();
        if mode == GizmoMode::Move {
            for (axis_index, line) in geometry.lines.iter().enumerate() {
                let Some(anchor) = project_gizmo_point(matrix, rect, line.start) else {
                    continue;
                };
                let Some([axis_start, axis_end]) =
                    project_gizmo_line(matrix, rect, line.start, line.end)
                else {
                    continue;
                };
                let axis_direction = axis_end - axis_start;
                let direction_length = axis_direction.length();
                if !direction_length.is_finite() || direction_length <= 1.0e-3 {
                    continue;
                }
                let direction = axis_direction / direction_length;
                let pixel_scale = MOVE_GIZMO_SCALE / pixels_per_point;
                let shaft_end = anchor + direction * (MOVE_ARROW_SHAFT_LENGTH_PIXELS * pixel_scale);
                let axis_color = color(line.color);
                let emphasized =
                    Some(axis_index) == hovered_axis || Some(axis_index) == active_axis;
                paint_gizmo_shaft(
                    painter,
                    anchor,
                    shaft_end,
                    axis_color,
                    line.width_pixels,
                    emphasized,
                );
                paint_gizmo_arrowhead(
                    painter,
                    shaft_end,
                    direction,
                    axis_color,
                    MOVE_GIZMO_SCALE,
                    emphasized,
                );
            }
        } else {
            for (line_index, line) in geometry.lines.iter().enumerate() {
                let axis_index = line_index / ROTATE_RING_SEGMENTS as usize;
                if let Some([start, end]) = project_gizmo_line(matrix, rect, line.start, line.end) {
                    paint_gizmo_shaft(
                        painter,
                        start,
                        end,
                        color(line.color),
                        line.width_pixels,
                        Some(axis_index) == hovered_axis || Some(axis_index) == active_axis,
                    );
                }
            }
            if mode == GizmoMode::Rotate {
                paint_curved_rotate_arrows(
                    painter,
                    matrix,
                    rect,
                    &geometry.lines,
                    active_axis.or(hovered_axis),
                );
            }
            for triangle in &geometry.triangles {
                let points = clip_gizmo_triangle(
                    matrix,
                    rect,
                    [triangle.first, triangle.second, triangle.third],
                );
                if points.len() >= 3 {
                    painter.add(egui::Shape::convex_polygon(
                        points,
                        color(triangle.color),
                        Stroke::NONE,
                    ));
                }
            }
        }
    }

    // Sizes follow the same uncommitted delta as the mesh. Once it disappears
    // (release, Escape, focus loss or a blocked gesture), restore scene sizes.
    // Leave idle text alone so a person's numeric edits survive repainting.
    fn sync_resize_size_fields(&mut self) {
        let delta = match &self.viewport_drag {
            Some(ViewportDrag::Transform(drag)) if drag.active_resize_handle().is_some() => {
                Some(drag.delta)
            }
            _ => None,
        };
        if delta.is_none() && self.resize_size_preview.is_none() {
            return;
        }
        let unit = self.commands.preferences().display_unit();
        let preview = delta.map(|delta| (delta, unit, self.commands.scene().revision()));
        if preview.is_some() && preview == self.resize_size_preview {
            return;
        }
        let indices = self
            .commands
            .scene()
            .selection()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        if indices.is_empty() {
            self.resize_size_preview = None;
            return;
        }
        match crate::transforms::world_bounds_with_delta(
            self.commands.scene(),
            &indices,
            delta.unwrap_or_else(modeler3d_core::Transform::identity),
        ) {
            Ok(bounds) => {
                self.size_fields = bounds.extents().map(|value| format_distance(value, unit));
                self.numeric_edited_axes = [false; 3];
                self.last_numeric_axis = None;
            }
            Err(error) => self.viewport_error = Some(error.to_string()),
        }
        self.resize_size_preview = preview;
    }

    fn paint_resize_handles(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        _palette: ThemePalette,
    ) {
        let delta = match &self.viewport_drag {
            Some(ViewportDrag::Transform(drag)) => Mat4::from_rows(drag.delta.rows()).ok(),
            _ => None,
        };
        let Some(overlay) = self.resize_overlay(rect, delta) else {
            return;
        };
        let dragging = self.active_resize_handle();
        let hovered = dragging
            .is_none()
            .then(|| {
                painter
                    .ctx()
                    .input(|input| input.pointer.hover_pos())
                    .filter(|position| rect.contains(*position))
                    .and_then(|position| overlay.handle_at(position))
            })
            .flatten();
        overlay.paint(painter, dragging.or(hovered), dragging);
    }

    /// For the `resize-drag` review capture: holds the +X face handle
    /// 90 points outward for this frame, through the ordinary gesture code.
    /// Returns whether a drag was staged; the caller drops it after painting
    /// so it is never committed.
    fn stage_review_resize_drag(&mut self, rect: egui::Rect) -> bool {
        use crate::review_capture::ReviewState;
        if self.viewport_drag.is_some()
            || self
                .review
                .as_ref()
                .is_none_or(|review| review.state() != ReviewState::ResizeDrag)
        {
            return false;
        }
        let Some(face) = self.resize_overlay(rect, None).and_then(|overlay| {
            overlay
                .faces
                .into_iter()
                .find(|face| face.handle == ResizeHandle::Face { axis: 0, side: 1.0 })
        }) else {
            return false;
        };
        let Some(mut drag) = self.begin_transform_drag(rect, face.center, 1.0) else {
            return false;
        };
        let size = self
            .viewport_renderer
            .as_ref()
            .map_or(self.camera.viewport_size(), ViewportRenderer::size);
        let screen = viewport_pixel_position(face.center + face.direction * 90.0, rect, size);
        if let (ViewportDrag::Transform(gesture), Some(radius)) =
            (&mut drag, self.current_scene_radius())
            && let Ok(ray) = self.camera.screen_ray(screen[0], screen[1], radius)
        {
            let _ = gesture.update_with_snap(ray, true);
        }
        self.viewport_drag = Some(drag);
        true
    }

    /// Lays out the Resize overlay for the current gizmo frame, moved by an
    /// in-progress drag `delta`.
    fn resize_overlay(
        &self,
        rect: egui::Rect,
        delta: Option<Mat4>,
    ) -> Option<crate::resize_overlay::ResizeOverlay> {
        let frame = self.gizmo_frame.as_ref()?.1;
        let matrix = self
            .camera
            .view_projection(self.current_scene_radius().unwrap_or(1.))
            .ok()?;
        let view_direction = modeler3d_viewport::normalize(
            subtract(self.camera.center(), self.camera.position()),
            "camera view direction",
        )
        .ok()?;
        crate::resize_overlay::ResizeOverlay::layout(
            &frame,
            delta,
            |point| project_gizmo_point(matrix, rect, point),
            view_direction,
        )
    }

    fn begin_brush_gesture(
        &mut self,
        rect: egui::Rect,
        pointer: egui::Pos2,
        size: (u32, u32),
    ) -> Option<ViewportDrag> {
        let tool = self.active_brush?;
        let scene = self.preview.as_ref()?;
        let screen = viewport_pixel_position(pointer, rect, size);
        let hit = match pick_scene(scene, &self.camera, screen[0], screen[1]) {
            Ok(Some(hit)) => hit,
            Ok(None) => {
                self.commands
                    .set_status("Brush stroke requires a mesh pick");
                return None;
            }
            Err(error) => {
                self.commands.set_status(error.to_string());
                return None;
            }
        };
        let index = match self.commands.scene().object_index(&hit.object_id) {
            Ok(index) => index,
            Err(error) => {
                self.commands.set_status(error.to_string());
                return None;
            }
        };
        let object = match self.commands.scene().object(index) {
            Ok(object) => object,
            Err(error) => {
                self.commands.set_status(error.to_string());
                return None;
            }
        };
        let inverse = match object.transform().inverse() {
            Ok(inverse) => inverse,
            Err(error) => {
                self.commands.set_status(error.to_string());
                return None;
            }
        };
        let center = inverse.transform_point(hit.position);
        let plane_normal = modeler3d_viewport::normalize(
            subtract(self.camera.position(), self.camera.center()),
            "grab plane normal",
        )
        .unwrap_or([0.0, 0.0, 1.0]);
        let object_id = object.object_id().to_owned();
        let source_mesh = object.mesh().clone();
        self.commands.select_index(index);
        self.surface = WorkspaceSurface::Scene;
        Some(ViewportDrag::Brush(Box::new(BrushGesture {
            tool,
            object_id,
            source_mesh,
            source_transform: inverse,
            centers: vec![center],
            start_center: center,
            offset: [0.0; 3],
            start_world: hit.position,
            plane_normal,
            revision: self.commands.scene().revision(),
        })))
    }

    fn update_brush_gesture(&mut self, rect: egui::Rect, pointer: egui::Pos2, size: (u32, u32)) {
        let scene_radius = self.current_scene_radius().unwrap_or(1.0);
        let Some(ViewportDrag::Brush(gesture)) = self.viewport_drag.as_mut() else {
            return;
        };
        let Some(scene) = self.preview.as_ref() else {
            return;
        };
        let screen = viewport_pixel_position(pointer, rect, size);
        if gesture.tool == ActiveBrush::Grab {
            let Some(target) = self
                .camera
                .screen_ray(screen[0], screen[1], scene_radius)
                .ok()
                .and_then(|ray| grab_plane_target(ray, gesture.start_world, gesture.plane_normal))
            else {
                return;
            };
            let local_target = gesture.source_transform.transform_point(target);
            gesture.offset = subtract(local_target, gesture.start_center);
            self.update_brush_preview();
            return;
        }
        let hit = match pick_scene(scene, &self.camera, screen[0], screen[1]) {
            Ok(Some(hit)) if hit.object_id == gesture.object_id => hit,
            Ok(Some(_)) | Ok(None) => return,
            Err(error) => {
                self.commands.set_status(error.to_string());
                return;
            }
        };
        let local = gesture.source_transform.transform_point(hit.position);
        match gesture.tool {
            // Grab follows the camera-facing plane above and never gets here.
            ActiveBrush::Grab => {}
            ActiveBrush::Crease
            | ActiveBrush::SmoothBrush
            | ActiveBrush::Blob
            | ActiveBrush::SubdivideArea
            | ActiveBrush::SimplifyArea => {
                let last = gesture
                    .centers
                    .last()
                    .copied()
                    .unwrap_or(gesture.start_center);
                let delta = subtract(local, last);
                if dot(delta, delta).sqrt() >= 0.25 && gesture.centers.len() < 256 {
                    gesture.centers.push(local);
                }
            }
        }
        self.update_brush_preview();
    }

    fn brush_stroke(&self, gesture: &BrushGesture) -> BrushStroke {
        let settings = self.brush_settings;
        match gesture.tool {
            ActiveBrush::Grab => BrushStroke::Grab {
                center_mm: gesture.start_center,
                offset_mm: gesture.offset,
                options: ElasticGrabOptions {
                    radius_mm: settings.grab_radius_mm,
                    strength: settings.grab_strength,
                    falloff: settings.grab_falloff,
                    symmetry: settings.grab_symmetry,
                    limits: Default::default(),
                },
            },
            ActiveBrush::Crease => BrushStroke::Crease {
                centers_mm: gesture.centers.clone(),
                options: CreasePolishOptions {
                    radius_mm: settings.crease_radius_mm,
                    strength: settings.crease_strength,
                    direction: settings.crease_direction,
                    falloff: settings.crease_falloff,
                    sharpness: settings.crease_sharpness,
                    limits: Default::default(),
                },
            },
            ActiveBrush::SmoothBrush => BrushStroke::SmoothBrush {
                centers_mm: gesture.centers.clone(),
                options: SmoothBrushOptions {
                    radius_mm: settings.smooth_radius_mm,
                    strength: settings.smooth_strength,
                    shrinkage: settings.smooth_shrinkage,
                    falloff: settings.smooth_falloff,
                    limits: Default::default(),
                },
            },
            ActiveBrush::Blob => BrushStroke::Blob {
                centers_mm: gesture.centers.clone(),
                options: BlobOptions {
                    radius_mm: settings.blob_radius_mm,
                    strength: settings.blob_strength,
                    direction: settings.blob_direction,
                    falloff: settings.blob_falloff,
                    limits: Default::default(),
                },
            },
            ActiveBrush::SubdivideArea => BrushStroke::SubdivideArea {
                centers_mm: gesture.centers.clone(),
                radius_mm: settings.subdivide_radius_mm,
            },
            ActiveBrush::SimplifyArea => BrushStroke::SimplifyArea {
                centers_mm: gesture.centers.clone(),
                radius_mm: settings.simplify_radius_mm,
                percent: settings.simplify_percent,
            },
        }
    }

    fn brush_preview_mesh(
        &self,
        gesture: &BrushGesture,
    ) -> Result<Mesh, modeler3d_core::CoreError> {
        let stroke = self.brush_stroke(gesture);
        match stroke {
            BrushStroke::Grab {
                center_mm,
                offset_mm,
                options,
            } => elastic_grab(&gesture.source_mesh, center_mm, offset_mm, options)
                .map(|result| result.mesh),
            BrushStroke::Crease {
                centers_mm,
                options,
            } => {
                crease_polish(&gesture.source_mesh, &centers_mm, options).map(|result| result.mesh)
            }
            BrushStroke::SmoothBrush {
                centers_mm,
                options,
            } => smooth_brush(&gesture.source_mesh, &centers_mm, options).map(|result| result.mesh),
            BrushStroke::Blob {
                centers_mm,
                options,
            } => blob(&gesture.source_mesh, &centers_mm, options).map(|result| result.mesh),
            BrushStroke::SubdivideArea {
                centers_mm,
                radius_mm,
            } => {
                let faces =
                    modeler3d_core::faces_near_path(&gesture.source_mesh, &centers_mm, radius_mm);
                if faces.is_empty() {
                    return Ok(gesture.source_mesh.clone());
                }
                modeler3d_core::subdivide_mesh_even(
                    &gesture.source_mesh,
                    Some(&faces),
                    modeler3d_core::MeshOperationLimits::default(),
                )
                .map(|result| result.mesh)
            }
            BrushStroke::SimplifyArea {
                centers_mm,
                radius_mm,
                percent,
            } => {
                let faces =
                    modeler3d_core::faces_near_path(&gesture.source_mesh, &centers_mm, radius_mm);
                if faces.is_empty() {
                    return Ok(gesture.source_mesh.clone());
                }
                modeler3d_core::simplify_mesh_by_percent_in_area(
                    &gesture.source_mesh,
                    percent,
                    Some(&faces),
                    modeler3d_core::MeshOperationLimits::default(),
                )
                .map(|result| result.mesh)
            }
        }
    }

    fn update_brush_preview(&mut self) {
        let Some(ViewportDrag::Brush(gesture)) = self.viewport_drag.as_ref() else {
            return;
        };
        match self.brush_preview_mesh(gesture) {
            Ok(mesh) => {
                self.preview_mesh_override = Some((gesture.object_id.clone(), mesh));
                self.preview_revision = u64::MAX;
                self.commands
                    .set_status(format!("{} preview", gesture.tool.label()));
            }
            Err(error) => self.commands.set_status(format!("Brush preview: {error}")),
        }
    }

    fn commit_brush_gesture(&mut self, gesture: BrushGesture) {
        let stroke = self.brush_stroke(&gesture);
        self.preview_mesh_override = None;
        match self.commands.apply_brush_stroke(stroke) {
            Ok(CommandOutcome::Applied(message)) | Ok(CommandOutcome::Noop(message)) => {
                self.commands.set_status(message);
                self.invalidate_presentation();
                self.persist_preferences();
            }
            Ok(CommandOutcome::OpenDialog(_) | CommandOutcome::ExitRequested) => {
                self.commands
                    .set_status("Brush returned an unexpected shell action");
            }
            Err(error) => self.report_command_error(error),
        }
    }

    fn route_viewport_input(
        &mut self,
        rect: egui::Rect,
        pixels_per_point: f32,
        input: ViewportInput,
        interactions_allowed: bool,
    ) {
        let stale_transform = matches!(&self.viewport_drag, Some(ViewportDrag::Transform(drag)) if drag.snapshot.revision != self.commands.scene().revision() || &drag.snapshot.selection != self.commands.scene().selection());
        let stale_brush = matches!(&self.viewport_drag, Some(ViewportDrag::Brush(gesture)) if gesture.revision != self.commands.scene().revision());
        if self.measure_mode.is_some() && input.escape_pressed && self.viewport_drag.is_none() {
            self.measure_escape();
            self.viewport_gizmo_consumed = true;
            return;
        }
        if self.paint_mode.is_some() && input.escape_pressed && self.viewport_drag.is_none() {
            self.finish_paint_mode(true);
            self.viewport_gizmo_consumed = true;
            return;
        }
        if self.boolean_pick.is_some() && input.escape_pressed {
            self.boolean_pick = None;
            self.commands.set_status("Boolean pick cancelled");
            self.viewport_gizmo_consumed = true;
            return;
        }
        if self.viewport_drag.is_some()
            && (!input.focused || input.escape_pressed || stale_transform || stale_brush)
        {
            let brush_cancelled = matches!(self.viewport_drag, Some(ViewportDrag::Brush(_)));
            self.viewport_drag = None;
            if brush_cancelled {
                self.preview_mesh_override = None;
                self.invalidate_presentation();
            }
            self.commands.set_status("Viewport gesture cancelled");
            return;
        }
        if self.boolean_pick.is_some()
            && input.primary_pressed
            && input
                .pointer_position
                .is_some_and(|position| rect.contains(position))
        {
            if let Some(position) = input.pointer_position {
                self.handle_boolean_pick(rect, position);
            }
            self.viewport_gizmo_consumed = true;
            return;
        }
        if !interactions_allowed || !input.focused {
            if self.viewport_drag.take().is_some() {
                self.preview_mesh_override = None;
                self.commands.set_status("Viewport gesture cancelled");
            }
            return;
        }
        let size = self
            .viewport_renderer
            .as_ref()
            .map_or(self.camera.viewport_size(), ViewportRenderer::size);
        if self.viewport_drag.is_none()
            && (input.scroll_y.abs() > f32::EPSILON
                || (input.pinch_zoom_factor - 1.0).abs() > f32::EPSILON)
            && input
                .pointer_position
                .is_some_and(|position| rect.contains(position))
        {
            if input.scroll_y.abs() > f32::EPSILON {
                let _ = self.camera.zoom(input.scroll_y);
            }
            if input.pinch_zoom_factor.is_finite() {
                let _ = self.camera.zoom_factor(1.0 - input.pinch_zoom_factor);
            }
        }
        if self.viewport_drag.is_none()
            && input.primary_pressed
            && let Some(position) = input
                .pointer_position
                .filter(|position| rect.contains(*position))
        {
            if self.active_brush.is_some() && input.modifiers.shift {
                self.preview_mesh_override = None;
                self.viewport_drag = self.begin_brush_gesture(rect, position, size);
                self.viewport_gizmo_consumed = true;
            } else if self.paint_mode.is_some() || self.measure_mode.is_some() {
                // While painting or measuring a drag turns the view; a click
                // paints or places a ruler end.
                self.viewport_drag = Some(ViewportDrag::Orbit);
                self.viewport_gizmo_consumed = false;
            } else {
                let transform = self.begin_transform_drag(rect, position, pixels_per_point);
                self.viewport_drag = if transform.is_some() {
                    transform
                } else if input.modifiers.shift && !self.object_under_pointer(rect, position) {
                    Some(ViewportDrag::BoxSelect {
                        start: position,
                        current: position,
                    })
                } else {
                    Some(ViewportDrag::Orbit)
                };
                self.viewport_gizmo_consumed =
                    matches!(self.viewport_drag, Some(ViewportDrag::Transform(_)));
            }
        } else if self.viewport_drag.is_none()
            && (input.middle_pressed || input.secondary_pressed)
            && input
                .pointer_position
                .is_some_and(|position| rect.contains(position))
        {
            self.viewport_drag = Some(ViewportDrag::Pan);
        }
        let current_scene_radius = self.current_scene_radius();
        if let Some(position) = input.pointer_position {
            match self.viewport_drag.as_mut() {
                Some(ViewportDrag::Orbit) if input.primary_down || input.primary_released => {
                    let delta = input.pointer_delta * pixels_per_point;
                    let _ = self.camera.orbit(delta.x, delta.y);
                }
                Some(ViewportDrag::BoxSelect { current, .. })
                    if input.primary_down || input.primary_released =>
                {
                    *current = position;
                }
                Some(ViewportDrag::Pan)
                    if input.middle_down
                        || input.secondary_down
                        || input.middle_released
                        || input.secondary_released =>
                {
                    let delta = input.pointer_delta * pixels_per_point;
                    let _ = self.camera.pan(-delta.x, delta.y);
                }
                Some(ViewportDrag::Transform(drag))
                    if (input.primary_down || input.primary_released)
                        && input.pointer_delta != Vec2::ZERO =>
                {
                    let screen = viewport_pixel_position(position, rect, size);
                    if let Some(radius) = current_scene_radius
                        && let Ok(ray) = self.camera.screen_ray(screen[0], screen[1], radius)
                    {
                        match drag.update_with_snap(ray, input.modifiers.shift) {
                            Ok(Some(result)) if result.snapped => {
                                self.commands.set_status("Snap preview");
                            }
                            Ok(_) => {}
                            Err(error) => self.viewport_error = Some(error.to_string()),
                        }
                    }
                }
                _ => {}
            }
        }
        if matches!(self.viewport_drag, Some(ViewportDrag::Brush(_)))
            && (input.primary_down || input.primary_released)
            && input.pointer_delta != Vec2::ZERO
            && let Some(position) = input.pointer_position
        {
            self.update_brush_gesture(rect, position, size);
        }
        if input.primary_released {
            match self.viewport_drag.take() {
                Some(ViewportDrag::Transform(drag)) => {
                    match self
                        .commands
                        .commit_transform_snapshot(&drag.snapshot, drag.delta)
                    {
                        Ok(outcome) => {
                            self.commands.set_status(match outcome {
                                CommandOutcome::Applied(message)
                                | CommandOutcome::Noop(message) => message,
                                CommandOutcome::OpenDialog(_) | CommandOutcome::ExitRequested => {
                                    String::from("Move")
                                }
                            });
                            self.invalidate_presentation();
                            self.persist_preferences();
                        }
                        Err(error) => self.report_command_error(error),
                    }
                }
                Some(ViewportDrag::Brush(gesture)) => self.commit_brush_gesture(*gesture),
                Some(ViewportDrag::BoxSelect { start, current }) => {
                    let marquee = egui::Rect::from_two_pos(start, current);
                    if marquee.width() >= BOX_SELECT_MIN_SIZE
                        || marquee.height() >= BOX_SELECT_MIN_SIZE
                    {
                        self.apply_box_selection(rect, marquee);
                        self.viewport_gizmo_consumed = true;
                    }
                }
                Some(ViewportDrag::Orbit | ViewportDrag::Pan) | None => {}
            }
        }
        if input.middle_released || input.secondary_released {
            self.viewport_drag = None;
        }
    }

    fn begin_transform_drag(
        &self,
        rect: egui::Rect,
        pointer: egui::Pos2,
        pixels_per_point: f32,
    ) -> Option<ViewportDrag> {
        let center = self.selected_gizmo_state()?;
        let scene_radius = self.current_scene_radius()?;
        let size = self
            .viewport_renderer
            .as_ref()
            .map_or(self.camera.viewport_size(), ViewportRenderer::size);
        let screen = viewport_pixel_position(pointer, rect, size);
        let ray = self
            .camera
            .screen_ray(screen[0], screen[1], scene_radius)
            .ok()?;
        let snapshot = self.commands.capture_transform_selection().ok()?;
        let mode = self.gizmo_mode();
        let drag = if mode == GizmoMode::Resize {
            let frame = self.gizmo_frame.as_ref()?.1;
            let handle = self.resize_handle_at_pointer(rect, pointer, pixels_per_point)?;
            TransformGesture::resize(snapshot, frame, handle, ray)?
        } else if mode == GizmoMode::Move {
            let axis = self.move_axis_at_pointer(rect, pointer, pixels_per_point)?;
            let mut gesture = TransformGesture::move_axis(snapshot, center, axis.vector(), ray)?;
            if let Ok(snap_snapshot) = self.commands.capture_viewport_snap_snapshot() {
                let midpoint = [
                    (snap_snapshot.moving_bounds.minimum[0]
                        + snap_snapshot.moving_bounds.maximum[0])
                        * 0.5,
                    (snap_snapshot.moving_bounds.minimum[1]
                        + snap_snapshot.moving_bounds.maximum[1])
                        * 0.5,
                    (snap_snapshot.moving_bounds.minimum[2]
                        + snap_snapshot.moving_bounds.maximum[2])
                        * 0.5,
                ];
                gesture.configure_move_snapping(
                    snap_snapshot,
                    self.snap_view_context(midpoint),
                    self.commands.snap_enabled(),
                );
            }
            gesture
        } else {
            let hit = hit_test_with_bounds(
                &self.camera,
                center,
                mode,
                screen[0],
                screen[1],
                scene_radius,
                self.selected_gizmo_bounds(),
            )
            .ok()??;
            TransformGesture::rotate(snapshot, center, hit.axis.vector(), ray)?
        };
        Some(ViewportDrag::Transform(Box::new(drag)))
    }

    fn transform_axis_at_pointer(
        &self,
        rect: egui::Rect,
        pointer: egui::Pos2,
        pixels_per_point: f32,
        mode: GizmoMode,
    ) -> Option<usize> {
        match mode {
            GizmoMode::Move => self
                .move_axis_at_pointer(rect, pointer, pixels_per_point)
                .map(gizmo_axis_index),
            GizmoMode::Rotate => {
                let center = self.selected_gizmo_state()?;
                let scene_radius = self.current_scene_radius()?;
                let size = self
                    .viewport_renderer
                    .as_ref()
                    .map_or(self.camera.viewport_size(), ViewportRenderer::size);
                let screen = viewport_pixel_position(pointer, rect, size);
                let hit = hit_test_with_bounds(
                    &self.camera,
                    center,
                    GizmoMode::Rotate,
                    screen[0],
                    screen[1],
                    scene_radius,
                    self.selected_gizmo_bounds(),
                )
                .ok()??;
                Some(gizmo_axis_index(hit.axis))
            }
            GizmoMode::Resize => None,
        }
    }

    fn move_axis_at_pointer(
        &self,
        rect: egui::Rect,
        pointer: egui::Pos2,
        pixels_per_point: f32,
    ) -> Option<GizmoAxis> {
        let center = self.selected_gizmo_state()?;
        let scene_radius = self.current_scene_radius()?;
        let matrix = self.camera.view_projection(scene_radius).ok()?;
        let geometry = build_gizmo_with_bounds(
            &self.camera,
            center,
            GizmoMode::Move,
            self.selected_gizmo_bounds(),
        )
        .ok()?;
        let pixels_per_point = pixels_per_point.max(1.0e-6);
        let hit_radius = MOVE_ARROW_PICK_RADIUS_PIXELS / pixels_per_point;
        let axes = [GizmoAxis::X, GizmoAxis::Y, GizmoAxis::Z];
        let mut nearest: Option<(f32, f32, GizmoAxis)> = None;
        for (axis, line) in axes.into_iter().zip(&geometry.lines) {
            let Some(anchor) = project_gizmo_point(matrix, rect, line.start) else {
                continue;
            };
            let Some([axis_start, axis_end]) =
                project_gizmo_line(matrix, rect, line.start, line.end)
            else {
                continue;
            };
            let projected_axis = axis_end - axis_start;
            let projected_length = projected_axis.length();
            if !projected_length.is_finite() || projected_length <= 1.0e-3 {
                continue;
            }
            let tip = anchor
                + projected_axis / projected_length * (MOVE_ARROW_LENGTH_PIXELS / pixels_per_point);
            let distance = distance_to_screen_segment(pointer, anchor, tip);
            if distance > hit_radius {
                continue;
            }
            let depth =
                project_to_viewport(matrix, rect, line.start).map_or(1.0, |(_, depth)| depth);
            if nearest.is_none_or(|(best_distance, best_depth, _)| {
                distance
                    .total_cmp(&best_distance)
                    .then(depth.total_cmp(&best_depth))
                    .is_lt()
            }) {
                nearest = Some((distance, depth, axis));
            }
        }
        nearest.map(|(_, _, axis)| axis)
    }

    /// The pointer over a Move, Rotate or Resize handle shows an open hand;
    /// while a handle is held it shows a closed hand (OF-33).
    fn transform_handle_cursor(
        &self,
        rect: egui::Rect,
        pointer: Option<egui::Pos2>,
        pixels_per_point: f32,
        hovering: bool,
    ) -> Option<egui::CursorIcon> {
        match &self.viewport_drag {
            Some(ViewportDrag::Transform(_)) => return Some(egui::CursorIcon::Grabbing),
            Some(_) => return None,
            None => {}
        }
        let pointer = pointer.filter(|position| hovering && rect.contains(*position))?;
        let over_handle = match self.gizmo_mode() {
            GizmoMode::Resize => self
                .resize_handle_at_pointer(rect, pointer, pixels_per_point)
                .is_some(),
            mode => self
                .transform_axis_at_pointer(rect, pointer, pixels_per_point, mode)
                .is_some(),
        };
        over_handle.then_some(egui::CursorIcon::Grab)
    }

    fn resize_handle_at_pointer(
        &self,
        rect: egui::Rect,
        pointer: egui::Pos2,
        _pixels_per_point: f32,
    ) -> Option<ResizeHandle> {
        self.resize_overlay(rect, None)?.handle_at(pointer)
    }

    fn active_gizmo_axis_index(&self) -> Option<usize> {
        match self.viewport_drag.as_ref()? {
            ViewportDrag::Transform(gesture) => gesture.active_axis_index(),
            ViewportDrag::Orbit
            | ViewportDrag::Pan
            | ViewportDrag::Brush(_)
            | ViewportDrag::BoxSelect { .. } => None,
        }
    }

    fn active_resize_handle(&self) -> Option<ResizeHandle> {
        match self.viewport_drag.as_ref()? {
            ViewportDrag::Transform(gesture) => gesture.active_resize_handle(),
            ViewportDrag::Orbit
            | ViewportDrag::Pan
            | ViewportDrag::Brush(_)
            | ViewportDrag::BoxSelect { .. } => None,
        }
    }

    fn snap_view_context(&self, midpoint: [f32; 3]) -> Option<SnapViewContext> {
        let delta = subtract(self.camera.position(), midpoint);
        let depth = dot(delta, delta).sqrt();
        if !depth.is_finite() || depth <= f32::EPSILON {
            return None;
        }
        let world_per_normalized =
            depth * (self.camera.vertical_fov_degrees().to_radians() * 0.5).tan();
        SnapViewContext::new(world_per_normalized, midpoint).ok()
    }

    fn current_scene_radius(&self) -> Option<f32> {
        let extent = if let Some(bounds) = self
            .preview
            .as_ref()
            .and_then(|scene| scene.world_bounds().ok().flatten())
        {
            subtract(bounds.maximum, bounds.minimum)
        } else {
            self.gizmo_frame.as_ref()?.1.extents
        };
        Some((dot(extent, extent).sqrt() * 0.5).max(0.01))
    }

    fn handle_boolean_pick(&mut self, rect: egui::Rect, pointer: egui::Pos2) {
        let Some(pick_state) = self.boolean_pick else {
            return;
        };
        let Some(scene) = self.preview.as_ref() else {
            self.commands
                .set_status("Boolean pick has no visible scene");
            return;
        };
        let size = self
            .viewport_renderer
            .as_ref()
            .map_or(self.camera.viewport_size(), ViewportRenderer::size);
        let screen = viewport_pixel_position(pointer, rect, size);
        let hit = match pick_scene(scene, &self.camera, screen[0], screen[1]) {
            Ok(Some(hit)) => hit,
            Ok(None) => {
                self.commands
                    .set_status("Boolean pick missed; choose a visible mesh");
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
        if index == pick_state.source_index {
            self.commands
                .set_status("Choose a different object for the Boolean operation");
            return;
        }
        let (target, tool) = if pick_state.source_is_tool {
            (index, pick_state.source_index)
        } else {
            (pick_state.source_index, index)
        };
        self.apply_boolean_pair(
            target,
            tool,
            pick_state.operation,
            pick_state.clear_selection,
        );
    }

    fn object_under_pointer(&self, rect: egui::Rect, pointer: egui::Pos2) -> bool {
        let Some(scene) = self.preview.as_ref() else {
            return false;
        };
        let size = self
            .viewport_renderer
            .as_ref()
            .map_or(self.camera.viewport_size(), ViewportRenderer::size);
        let screen = viewport_pixel_position(pointer, rect, size);
        matches!(
            pick_scene(scene, &self.camera, screen[0], screen[1]),
            Ok(Some(_))
        )
    }

    /// Adds every visible object whose on-screen footprint overlaps the
    /// Shift box to the selection. Selection is not model history.
    fn apply_box_selection(&mut self, rect: egui::Rect, marquee: egui::Rect) {
        let Some(scene) = self.preview.as_ref() else {
            return;
        };
        let Ok(matrix) = self
            .camera
            .view_projection(self.current_scene_radius().unwrap_or(1.0))
        else {
            return;
        };
        let hits = objects_in_marquee(scene, matrix, rect, marquee);
        let mut selection = self.commands.scene().selection().clone();
        let before = selection.len();
        for object_id in &hits {
            if let Ok(index) = self.commands.scene().object_index(object_id) {
                selection.insert(index);
            }
        }
        let added = selection.len() - before;
        self.commands.set_selection_indices(selection);
        if !hits.is_empty() {
            self.surface = WorkspaceSurface::Scene;
        }
        self.commands.set_status(match hits.len() {
            0 => String::from("Box select: nothing inside the box"),
            count => format!("Box select: {count} object(s) in the box, {added} added"),
        });
    }

    fn paint_box_selection(&self, painter: &egui::Painter, palette: ThemePalette) {
        if let Some(ViewportDrag::BoxSelect { start, current }) = self.viewport_drag {
            let marquee = egui::Rect::from_two_pos(start, current);
            painter.rect_filled(marquee, 2.0, palette.accent_control.gamma_multiply(0.12));
            painter.rect_stroke(
                marquee,
                2.0,
                Stroke::new(1.0, palette.accent_control),
                egui::StrokeKind::Inside,
            );
        }
    }

    fn pick_viewport_object(
        &mut self,
        rect: egui::Rect,
        pointer: egui::Pos2,
        modifiers: egui::Modifiers,
    ) {
        let Some(scene) = self.preview.as_ref() else {
            return;
        };
        let size = self
            .viewport_renderer
            .as_ref()
            .map_or(self.camera.viewport_size(), ViewportRenderer::size);
        let screen = viewport_pixel_position(pointer, rect, size);
        match pick_scene(scene, &self.camera, screen[0], screen[1]) {
            Ok(Some(hit)) => match self.commands.scene().object_index(&hit.object_id) {
                Ok(index) => {
                    self.commands.select_index_with_modifiers(index, true, None);
                    self.surface = WorkspaceSurface::Scene;
                }
                Err(error) => self.commands.set_status(error.to_string()),
            },
            Ok(None) if !self.sticky_selection && !modifiers.command && !modifiers.ctrl => {
                self.commands.deselect_all();
                self.selection_anchor = None;
            }
            Ok(None) => {}
            Err(error) => self.commands.set_status(error.to_string()),
        }
    }

    fn render_brush_settings(&mut self, ctx: &egui::Context) {
        let Some(tool) = self.active_brush else {
            return;
        };
        if !self.brush_settings_open {
            return;
        }
        let mut open = self.brush_settings_open;
        let settings = &mut self.brush_settings;
        egui::Window::new(format!("{} settings", tool.label()))
            .id(egui::Id::unique("brush-settings"))
            .default_pos(egui::pos2(14.0, 160.0))
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Hold Shift and drag in the viewport to preview a stroke.");
                ui.separator();
                match tool {
                    ActiveBrush::Grab => {
                        ui.add(
                            egui::Slider::new(&mut settings.grab_radius_mm, 0.1..=1000.0)
                                .text("Radius mm"),
                        );
                        ui.add(
                            egui::Slider::new(&mut settings.grab_strength, 0.0..=1.0)
                                .text("Strength"),
                        );
                        brush_falloff_combo(
                            ui,
                            "Falloff",
                            &mut settings.grab_falloff,
                            "grab-falloff",
                        );
                        brush_symmetry_combo(ui, &mut settings.grab_symmetry);
                    }
                    ActiveBrush::Crease => {
                        ui.add(
                            egui::Slider::new(&mut settings.crease_radius_mm, 0.1..=1000.0)
                                .text("Radius mm"),
                        );
                        ui.add(
                            egui::Slider::new(&mut settings.crease_strength, 0.0..=1.0)
                                .text("Strength"),
                        );
                        // OF-59: 0 is the gentle crease; higher makes the
                        // groove at the line narrower and deeper.
                        ui.add(
                            egui::Slider::new(&mut settings.crease_sharpness, 0.0..=1.0)
                                .text("Sharpness"),
                        );
                        brush_direction_combo(
                            ui,
                            &mut settings.crease_direction,
                            "crease-direction",
                        );
                        brush_falloff_combo(
                            ui,
                            "Falloff",
                            &mut settings.crease_falloff,
                            "crease-falloff",
                        );
                    }
                    ActiveBrush::SmoothBrush => {
                        ui.add(
                            egui::Slider::new(&mut settings.smooth_radius_mm, 0.1..=1000.0)
                                .text("Radius mm"),
                        );
                        ui.add(
                            egui::Slider::new(&mut settings.smooth_strength, 0.0..=1.0)
                                .text("Strength"),
                        );
                        ui.add(
                            egui::Slider::new(&mut settings.smooth_shrinkage, 0.0..=1.0)
                                .text("Shrinkage"),
                        );
                        brush_falloff_combo(
                            ui,
                            "Falloff",
                            &mut settings.smooth_falloff,
                            "smooth-falloff",
                        );
                    }
                    ActiveBrush::Blob => {
                        ui.add(
                            egui::Slider::new(&mut settings.blob_radius_mm, 0.1..=1000.0)
                                .text("Radius mm"),
                        );
                        ui.add(
                            egui::Slider::new(&mut settings.blob_strength, 0.0..=1.0)
                                .text("Strength"),
                        );
                        brush_direction_combo(ui, &mut settings.blob_direction, "blob-direction");
                        brush_falloff_combo(
                            ui,
                            "Falloff",
                            &mut settings.blob_falloff,
                            "blob-falloff",
                        );
                    }
                    // Their brush size lives in the Subdivide / Simplify window.
                    ActiveBrush::SubdivideArea | ActiveBrush::SimplifyArea => {}
                }
            });
        self.brush_settings_open = open;
    }

    fn settings_work_area(&self, ctx: &egui::Context) -> egui::Rect {
        let screen = ctx.content_rect();
        let safe_top = screen.top() + 6.0;
        let safe_bottom = screen.bottom() - 6.0;
        let mut top = self
            .ribbon_rect
            .map_or(safe_top, |rect| rect.bottom() + 6.0)
            .max(safe_top);
        let mut bottom = safe_bottom;
        // Stop above the transform strip when there is room; on a short window
        // (or at Big size) use the full height below the ribbon.
        if let Some(rect) = self.transform_strip_rect
            && rect.top() - 8.0 - top >= 160.0
        {
            bottom = bottom.min(rect.top() - 8.0);
        }
        if bottom - top < 160.0 {
            top = safe_top;
            bottom = safe_bottom;
        }
        let left = screen.left() + 8.0;
        let width = (screen.width() - 16.0).clamp(240.0, 380.0);
        egui::Rect::from_min_max(
            egui::pos2(left, top),
            egui::pos2((left + width).min(screen.right() - 4.0), bottom),
        )
    }

    fn render_dialog(&mut self, ctx: &egui::Context) {
        if let Some(command) = self.placement_command {
            let mut apply = false;
            let mut cancel = false;
            let title = match command {
                CommandId::Mirror => "Mirror Axis",
                CommandId::Align => "Align Objects",
                _ => "Arrange Objects",
            };
            crate::dialogs::dialog_window(ctx, title).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Axis:");
                    // Letters in the same colours as the move arrows (OF-41).
                    for (axis, letter, color) in placement_axis_labels() {
                        ui.selectable_value(
                            &mut self.placement_axis,
                            axis,
                            RichText::new(letter).strong().color(color),
                        );
                    }
                });
                if command == CommandId::Align {
                    ui.horizontal(|ui| {
                        ui.label("Target:");
                        for (mode, label) in [
                            (Alignment::Minimum, "min"),
                            (Alignment::Center, "center"),
                            (Alignment::Maximum, "max"),
                        ] {
                            ui.selectable_value(&mut self.placement_alignment, mode, label);
                        }
                    });
                }
                if command == CommandId::Arrange {
                    ui.label("Gap (mm):");
                    ui.text_edit_singleline(&mut self.arrange_gap_field);
                }
                ui.horizontal(|ui| {
                    apply = ui.button("OK").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
                if let Some(error) = &self.transform_error {
                    ui.label(error);
                }
            });
            if cancel || ctx.input(|input| input.key_pressed(Key::Escape)) {
                self.placement_command = None;
                self.transform_error = None;
            } else if apply {
                let gap = if command == CommandId::Arrange {
                    parse_distance_mm(&self.arrange_gap_field, DisplayUnit::Millimetre)
                } else {
                    Ok(0.0)
                };
                match gap {
                    Ok(gap) if (0.0..=10000.0).contains(&gap) => {
                        match self.commands.place_selected(
                            command,
                            self.placement_axis,
                            self.placement_alignment,
                            gap,
                        ) {
                            Ok(_) => {
                                self.placement_command = None;
                                self.transform_error = None;
                                self.invalidate_presentation();
                            }
                            Err(error) => self.transform_error = Some(error.to_string()),
                        }
                    }
                    _ => {
                        self.transform_error =
                            Some(String::from("Gap must be between 0 and 10000 mm"))
                    }
                }
            }
        }
        let settings_work_area = self.settings_work_area(ctx);
        let selection_faces = self
            .commands
            .scene()
            .selection()
            .iter()
            .next()
            .and_then(|index| self.commands.scene().object(*index).ok())
            .map_or(0, |object| object.mesh().face_count());
        let Some(dialog) = self.dialog.as_mut() else {
            return;
        };
        dialog.set_selection_faces(selection_faces);
        dialog.sync_theme_mode(self.theme_mode);
        dialog.sync_ui_size(self.ui_size);
        if let Some(action) = dialog.show_with_settings_area(ctx, Some(settings_work_area)) {
            self.apply_dialog_action(action);
        }
    }

    fn apply_dialog_action(&mut self, action: DialogAction) {
        self.apply_dialog_action_with_overwrite(action, false);
    }

    fn update_torus_preview(&mut self, parameters: modeler3d_core::TorusParameters) {
        let Some(index) = self.commands.scene().selection().iter().next().copied() else {
            self.commands
                .set_status("Edit Torus requires a selected parametric torus");
            return;
        };
        let Ok((mesh, _)) = build_parametric_torus(parameters) else {
            self.commands
                .set_status("Torus parameters are outside the bounded range");
            return;
        };
        let Ok(object) = self.commands.scene().object(index) else {
            self.commands
                .set_status("Selected torus is no longer available");
            return;
        };
        self.preview_mesh_override = Some((object.object_id().to_owned(), mesh));
        self.preview_revision = u64::MAX;
        self.commands.set_status("Torus preview");
    }

    fn apply_dialog_action_with_overwrite(
        &mut self,
        action: DialogAction,
        overwrite_confirmed: bool,
    ) {
        if let DialogAction::TorusPreview(parameters) = &action {
            self.update_torus_preview(parameters.clone());
            return;
        }
        if matches!(action, DialogAction::Close) {
            if matches!(
                self.active_brush,
                Some(ActiveBrush::SubdivideArea | ActiveBrush::SimplifyArea)
            ) {
                self.active_brush = None;
                self.viewport_drag = None;
            }
            self.pending_overwrite = None;
            if self.pending_after_save.take().is_some() {
                self.commands.set_status("Cancelled");
            }
            self.dialog = None;
            self.dialog_generation = self.dialog_generation.wrapping_add(1);
            self.preview_mesh_override = None;
            self.invalidate_presentation();
            return;
        }
        if matches!(action, DialogAction::Torus(_)) {
            self.preview_mesh_override = None;
        }
        let pending_overwrite = match &action {
            DialogAction::Save(path, native_confirmed)
                if path.is_file() && !native_confirmed && !overwrite_confirmed =>
            {
                Some(PendingOverwrite::SaveProject(path.clone()))
            }
            DialogAction::Export(path, native_confirmed)
                if path.is_file() && !native_confirmed && !overwrite_confirmed =>
            {
                Some(PendingOverwrite::Export(path.clone()))
            }
            _ => None,
        };
        if let Some(pending_overwrite) = pending_overwrite {
            self.pending_overwrite = Some(pending_overwrite);
            return;
        }
        if let DialogAction::Open(path) = &action
            && self.commands.is_dirty()
        {
            self.dialog = None;
            self.confirm_action = Some(PendingReplacement::OpenProject(path.clone()));
            return;
        }
        let should_frame_on_success =
            matches!(&action, DialogAction::Open(_) | DialogAction::Import(_, _));
        let close_on_success = matches!(
            action,
            DialogAction::Close
                | DialogAction::Open(_)
                | DialogAction::Import(_, _)
                | DialogAction::Save(_, _)
                | DialogAction::Export(_, _)
                | DialogAction::Color(_)
                | DialogAction::ColorFromImage(_)
                | DialogAction::Texture(_, _)
                | DialogAction::RemoveTexture
                | DialogAction::Torus(_)
                | DialogAction::Emboss(_)
                | DialogAction::Hollow(_)
        );
        let result = match action {
            DialogAction::Close => Ok(String::new()),
            DialogAction::Open(path) => self.open_project(&path),
            DialogAction::Import(path, unit) => {
                self.commands.preferences_mut().remember_import_path(&path);
                self.persist_preferences();
                self.commands.import_from_path_with_unit(&path, unit)
            }
            DialogAction::Save(path, _) => self.commands.save_document_to_path(&path).map(|_| {
                self.commands.project_path().map_or_else(
                    || format!("Saved {}", path.display()),
                    |saved| format!("Saved {}", saved.display()),
                )
            }),
            DialogAction::Export(path, _) => {
                let is_project = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("3dp"));
                let exported = if is_project {
                    self.commands.export_project_copy(&path)
                } else {
                    self.commands.export_to_path(&path)
                };
                exported.map(|_| format!("Exported {}", path.display()))
            }
            DialogAction::Repair => {
                self.start_mesh_job(MeshJobOperation::Repair, true);
                return;
            }
            DialogAction::EditAction(command) => {
                if let Some(operation) = boolean_operation_for_command(command) {
                    self.dispatch_boolean(operation);
                } else if command == CommandId::Smooth {
                    self.start_mesh_job(MeshJobOperation::Smooth, true);
                }
                return;
            }
            DialogAction::Simplify(percent) => {
                self.start_mesh_job(MeshJobOperation::Simplify(percent), true);
                return;
            }
            DialogAction::SubdivideWhole => {
                self.start_mesh_job(MeshJobOperation::Subdivide, true);
                return;
            }
            DialogAction::SubdivideArea(Some(radius_mm)) => {
                self.brush_settings.subdivide_radius_mm = radius_mm;
                self.viewport_drag = None;
                self.active_brush = Some(ActiveBrush::SubdivideArea);
                self.brush_settings_open = false;
                self.invalidate_presentation();
                Ok(String::from(
                    "Subdivide: hold Shift and drag over the model to subdivide that area",
                ))
            }
            DialogAction::SimplifyArea(Some((radius_mm, percent))) => {
                self.brush_settings.simplify_radius_mm = radius_mm;
                self.brush_settings.simplify_percent = percent;
                self.viewport_drag = None;
                self.active_brush = Some(ActiveBrush::SimplifyArea);
                self.brush_settings_open = false;
                self.invalidate_presentation();
                Ok(String::from(
                    "Simplify: hold Shift and drag over the model to simplify that area",
                ))
            }
            DialogAction::SimplifyArea(None) => {
                if self.active_brush == Some(ActiveBrush::SimplifyArea) {
                    self.active_brush = None;
                    self.viewport_drag = None;
                    self.preview_mesh_override = None;
                    self.invalidate_presentation();
                }
                Ok(String::from("Simplify: whole object"))
            }
            DialogAction::SubdivideArea(None) => {
                if self.active_brush == Some(ActiveBrush::SubdivideArea) {
                    self.active_brush = None;
                    self.viewport_drag = None;
                    self.preview_mesh_override = None;
                    self.invalidate_presentation();
                }
                Ok(String::from("Subdivide: whole object"))
            }
            DialogAction::PrintPrep(bed_size_mm) => {
                self.start_print_prep_job(bed_size_mm);
                return;
            }
            DialogAction::DefaultImportUnit(unit) => {
                self.apply_default_import_unit(unit);
                let message = format!("Default import units set to {}", unit.label());
                self.commands.set_status(message.clone());
                Ok(message)
            }
            DialogAction::DisplayUnit(unit) => {
                self.apply_display_unit(unit);
                let message = format!("Transform display units set to {}", unit.label());
                self.commands.set_status(message.clone());
                Ok(message)
            }
            DialogAction::PreparePrint(slicer, remember) => {
                if remember {
                    self.apply_preferred_slicer(
                        Some(slicer.path.clone()),
                        Some(slicer.label.clone()),
                    );
                }
                self.start_print_staging_job(slicer);
                return;
            }
            DialogAction::BrowsePreferredSlicer => {
                let current = self
                    .commands
                    .preferences()
                    .preferred_slicer_path()
                    .map(Path::to_path_buf);
                match native_slicer_picker(current.as_deref()) {
                    Some(path) => match slicer_info_from_path(&path, None) {
                        Ok(slicer) => {
                            self.apply_preferred_slicer(
                                Some(slicer.path.clone()),
                                Some(slicer.label.clone()),
                            );
                            if let Some(dialog) = self.dialog.as_mut() {
                                dialog.set_message(format!(
                                    "Preferred slicer set to {}",
                                    slicer.label
                                ));
                            }
                            return;
                        }
                        Err(error) => {
                            if let Some(dialog) = self.dialog.as_mut() {
                                dialog.set_error(format!("Could not use selected slicer: {error}"));
                            }
                            return;
                        }
                    },
                    None => {
                        if let Some(dialog) = self.dialog.as_mut() {
                            dialog.set_message(
                                "Slicer selection cancelled; the saved preference is unchanged.",
                            );
                        }
                        return;
                    }
                }
            }
            DialogAction::BrowsePrintSlicer => {
                let current = self
                    .commands
                    .preferences()
                    .preferred_slicer_path()
                    .map(Path::to_path_buf);
                match native_slicer_picker(current.as_deref()) {
                    Some(path) => match slicer_info_from_path(&path, None) {
                        Ok(slicer) => {
                            if let Some(dialog) = self.dialog.as_mut() {
                                dialog.add_print_slicer(slicer.clone());
                                dialog.set_message(format!("Selected {}", slicer.label));
                            }
                            return;
                        }
                        Err(error) => {
                            if let Some(dialog) = self.dialog.as_mut() {
                                dialog.set_error(format!("Could not use selected slicer: {error}"));
                            }
                            return;
                        }
                    },
                    None => {
                        if let Some(dialog) = self.dialog.as_mut() {
                            dialog.set_message(
                                "Slicer selection cancelled; no preference was changed.",
                            );
                        }
                        return;
                    }
                }
            }
            DialogAction::ClearPreferredSlicer => {
                self.apply_preferred_slicer(None, None);
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.set_message("Preferred slicer cleared.");
                }
                return;
            }
            DialogAction::ClearRecentFiles => {
                self.clear_recent_files();
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.set_recent_file_count(0);
                    dialog.set_message("Recent files cleared. The source files were not changed.");
                }
                return;
            }
            DialogAction::LaunchPrint => {
                let prepared = self
                    .dialog
                    .as_ref()
                    .and_then(DialogState::prepared_print)
                    .cloned()
                    .ok_or(crate::print_handoff::PrintHandoffError::NoPreparedPrint);
                match prepared {
                    Ok(prepared) if !self.prepared_print_is_current(&prepared) => {
                        if let Some(dialog) = self.dialog.as_mut()
                            && let Err(error) = dialog.discard_prepared_print()
                        {
                            self.commands.set_status(format!(
                                "Staged print is stale; cleanup failed: {error}"
                            ));
                        }
                        Err(CommandError::StaleOperation)
                    }
                    Ok(prepared) => match launch_prepared_print(&prepared) {
                        Ok(launch) => {
                            let message = format!(
                                "Launched {} (process {})",
                                launch.slicer.label, launch.process_id
                            );
                            if let Some(dialog) = self.dialog.as_mut() {
                                dialog.mark_print_launched();
                            }
                            self.commands.set_status(message.clone());
                            Ok(message)
                        }
                        Err(error) => Err(error.into()),
                    },
                    Err(error) => Err(error.into()),
                }
            }
            DialogAction::Color(color) => self
                .commands
                .set_selected_color(color)
                .map(|_| self.commands.last_status().to_owned()),
            DialogAction::ColorFromImage(path) => {
                self.commands.set_selected_color_from_image(&path, 8)
            }
            DialogAction::Texture(path, tile_mm) => self
                .commands
                .apply_texture_to_selection(
                    (!path.as_os_str().is_empty()).then_some(path.as_path()),
                    tile_mm,
                )
                .map(|_| self.commands.last_status().to_owned()),
            DialogAction::RemoveTexture => self
                .commands
                .remove_texture_from_selection()
                .map(|_| self.commands.last_status().to_owned()),
            DialogAction::Torus(parameters) => self.commands.update_selected_torus(parameters),
            DialogAction::TorusPreview(parameters) => {
                self.update_torus_preview(parameters);
                Ok(String::from("Torus preview"))
            }
            DialogAction::Emboss(options) => self.commands.emboss_text_selected(options),
            DialogAction::Hollow(options) => {
                self.commands.hollow_selected(options.wall_thickness_mm)
            }
            DialogAction::Theme(mode) => {
                self.apply_theme_mode(mode);
                Ok(String::from("Theme updated"))
            }
            DialogAction::UiSize(size) => {
                self.apply_ui_size(size);
                Ok(String::from(match size {
                    UiSize::Big => "Size set to Big",
                    UiSize::Normal => "Size set to Normal",
                }))
            }
            DialogAction::ViewportLighting(lighting) => {
                self.apply_viewport_lighting(lighting);
                Ok(String::new())
            }
        };
        match result {
            Ok(message) => {
                if should_frame_on_success {
                    self.camera_auto_frame_pending = true;
                }
                if close_on_success {
                    self.dialog = None;
                } else if let Some(dialog) = self.dialog.as_mut() {
                    dialog.set_message(message);
                }
                self.synchronize_surface();
                self.persist_preferences();
                if let Some(pending) = self.pending_after_save.take() {
                    self.complete_replacement(pending);
                }
            }
            Err(error) => {
                self.commands.set_status(error.to_string());
                if let Some(dialog) = self.dialog.as_mut() {
                    dialog.set_error(error.to_string());
                }
            }
        }
    }

    fn render_confirmation(&mut self, ctx: &egui::Context) {
        let Some(action) = self.confirm_action.clone() else {
            return;
        };
        let prompt = match action {
            PendingReplacement::NewScene => "Save changes before creating a new scene?",
            PendingReplacement::OpenProject(_) => "Save changes before opening this project?",
            PendingReplacement::Exit => "Save changes before exiting 3D Modeler?",
        };
        let mut choice = None;
        crate::dialogs::dialog_window(ctx, "Unsaved changes").show(ctx, |ui| {
            ui.label(prompt);
            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    choice = Some(ConfirmationChoice::Save);
                }
                if ui.button("Discard").clicked() {
                    choice = Some(ConfirmationChoice::Discard);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(ConfirmationChoice::Cancel);
                }
            });
        });
        if let Some(choice) = choice {
            self.finish_confirmation(choice);
        }
    }

    fn render_overwrite_confirmation(&mut self, ctx: &egui::Context) {
        let Some(pending) = self.pending_overwrite.clone() else {
            return;
        };
        let path = match &pending {
            PendingOverwrite::SaveProject(path) | PendingOverwrite::Export(path) => path,
        };
        let mut overwrite = None;
        crate::dialogs::dialog_window(ctx, "Confirm overwrite").show(ctx, |ui| {
            ui.label("Replace the existing file?");
            ui.label(path.display().to_string());
            ui.horizontal(|ui| {
                if ui.button("Overwrite").clicked() {
                    overwrite = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    overwrite = Some(false);
                }
            });
        });
        if let Some(overwrite) = overwrite {
            self.finish_overwrite_confirmation(overwrite);
        }
    }

    fn render_ribbon_customizer(&mut self, ctx: &egui::Context) {
        if !self.customize_ribbon {
            return;
        }
        let mut open = true;
        let mut changed = false;
        let current = self.commands.preferences().ribbon_favorites().to_vec();
        let available = all_command_keys();
        let mut next = current.clone();
        let tab_snapshots = RibbonTab::all()
            .into_iter()
            .map(|tab| {
                let defaults = tab_commands(tab)
                    .into_iter()
                    .map(|command| command.label.to_owned())
                    .collect::<Vec<_>>();
                let requested = self
                    .commands
                    .preferences()
                    .ribbon_order(tab.label())
                    .map_or_else(Vec::new, ToOwned::to_owned);
                (tab, defaults, requested)
            })
            .collect::<Vec<_>>();
        let mut order_changes = Vec::new();
        egui::Window::new("Customize ribbon")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Choose favorites for the compact ribbon strip.");
                ui.separator();
                for key in &available {
                    let mut checked = next.iter().any(|item| item == key);
                    if ui.checkbox(&mut checked, key).changed() {
                        changed = true;
                        if checked {
                            next.push(key.clone());
                        } else {
                            next.retain(|item| item != key);
                        }
                    }
                }
                ui.separator();
                ui.label("Favorite order");
                let order_snapshot = next.clone();
                for key in order_snapshot {
                    ui.horizontal(|ui| {
                        ui.label(&key);
                        if ui.small_button("↑").clicked()
                            && let Some(moved) = move_favorite(&next, &key, -1)
                        {
                            next = moved;
                            changed = true;
                        }
                        if ui.small_button("↓").clicked()
                            && let Some(moved) = move_favorite(&next, &key, 1)
                        {
                            next = moved;
                            changed = true;
                        }
                    });
                }
                ui.separator();
                ui.label("Command order");
                for (tab, defaults, requested) in &tab_snapshots {
                    ui.collapsing(format!("{} tab", tab.label()), |ui| {
                        let mut order = normalized_tab_order(requested.clone(), defaults);
                        let order_snapshot = order.clone();
                        for key in order_snapshot {
                            ui.horizontal(|ui| {
                                ui.label(&key);
                                if ui.small_button("Up").clicked()
                                    && let Some(moved) = move_favorite(&order, &key, -1)
                                {
                                    order = moved;
                                }
                                if ui.small_button("Down").clicked()
                                    && let Some(moved) = move_favorite(&order, &key, 1)
                                {
                                    order = moved;
                                }
                            });
                        }
                        if ui.small_button("Reset tab order").clicked() {
                            order = defaults.clone();
                        }
                        if order != normalized_tab_order(requested.clone(), defaults) {
                            order_changes.push((tab.label().to_owned(), order));
                        }
                    });
                }
                ui.separator();
                if ui.button("Reset defaults").clicked() {
                    next = default_favorites();
                    changed = true;
                }
            });
        if changed {
            self.commands.preferences_mut().set_ribbon_favorites(next);
        }
        for (tab, order) in order_changes {
            self.commands.preferences_mut().set_ribbon_order(tab, order);
            changed = true;
        }
        if changed {
            self.persist_preferences();
        }
        self.customize_ribbon = open;
    }

    fn modal_active(&self) -> bool {
        self.dialog.as_ref().is_some_and(DialogState::is_modal)
            || self.placement_command.is_some()
            || self.confirm_action.is_some()
            || self.pending_overwrite.is_some()
            || self.customize_ribbon
    }

    fn interaction_blocked(&self) -> bool {
        self.modal_active() || self.menu_open
    }

    fn render_background(&mut self, ui: &mut Ui) {
        if self.modal_active() {
            ui.disable();
        }
        self.render_top(ui);
        self.render_operation_progress(ui.ctx());
        match self.surface {
            WorkspaceSurface::Welcome => {
                self.render_welcome(ui);
            }
            WorkspaceSurface::Scene => {
                self.render_objects(ui);
                self.render_viewport(ui);
            }
        }
        self.render_transform_strip(ui.ctx());
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.menu_open && ctx.input(|input| input.key_pressed(Key::Escape)) {
            self.menu_open = false;
            return;
        }
        let modal_or_menu = self.interaction_blocked();
        let file_command = ctx.input(|input| {
            if modal_or_menu {
                return None;
            }
            let command_modifier = input.modifiers.command || input.modifiers.ctrl;
            if command_modifier && input.modifiers.shift && input.key_pressed(Key::S) {
                Some(CommandId::SaveProjectAs)
            } else if command_modifier && input.key_pressed(Key::N) {
                Some(CommandId::NewScene)
            } else if command_modifier && input.key_pressed(Key::O) {
                Some(CommandId::OpenProject)
            } else if command_modifier && input.key_pressed(Key::S) {
                Some(CommandId::SaveProject)
            } else if command_modifier && input.key_pressed(Key::Q) {
                Some(CommandId::Exit)
            } else {
                None
            }
        });
        if let Some(command) = file_command {
            self.dispatch(command);
            return;
        }
        if modal_or_menu || ctx.egui_wants_keyboard_input() {
            return;
        }
        let command = ctx.input(|input| {
            let command_modifier = input.modifiers.command || input.modifiers.ctrl;
            if command_modifier && input.key_pressed(Key::S) {
                Some(CommandId::SaveProject)
            } else if command_modifier && input.key_pressed(Key::C) {
                Some(CommandId::Copy)
            } else if command_modifier && input.key_pressed(Key::X) {
                Some(CommandId::Cut)
            } else if command_modifier && input.key_pressed(Key::V) {
                Some(CommandId::Paste)
            } else if command_modifier && input.key_pressed(Key::D) {
                Some(CommandId::Duplicate)
            } else if command_modifier && input.key_pressed(Key::A) {
                Some(CommandId::SelectAll)
            } else if command_modifier && input.modifiers.shift && input.key_pressed(Key::Z) {
                Some(CommandId::Redo)
            } else if command_modifier && input.key_pressed(Key::Z) {
                Some(CommandId::Undo)
            } else if command_modifier && input.key_pressed(Key::Y) {
                Some(CommandId::Redo)
            } else if input.key_pressed(Key::Delete) {
                Some(CommandId::Delete)
            } else if input.key_pressed(Key::Escape) {
                self.menu_open = false;
                // Esc closes Measure wherever the pointer is (OF-55).
                if self.measure_mode.is_some() {
                    self.finish_measure_mode();
                }
                if self.active_brush.take().is_some() {
                    self.brush_settings_open = false;
                    self.preview_mesh_override = None;
                    self.invalidate_presentation();
                    self.commands.set_status("Brush deactivated");
                }
                None
            } else {
                None
            }
        });
        if let Some(command) = command {
            self.dispatch(command);
        }
    }
}

impl eframe::App for Modeler3dApp {
    fn raw_input_hook(&mut self, ctx: &egui::Context, input: &mut egui::RawInput) {
        if let Some([x, y]) = self.review.as_ref().and_then(|review| review.pointer()) {
            input
                .events
                .push(egui::Event::PointerMoved(egui::pos2(x, y)));
        }
        if !ctx.egui_wants_keyboard_input() {
            return;
        }
        // eframe's native clipboard translation is disabled so scene Ctrl+V
        // survives an empty OS clipboard. Text edits still use the platform
        // clipboard, via ordinary egui events and the native paste request.
        let mut events = Vec::with_capacity(input.events.len());
        for event in std::mem::take(&mut input.events) {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = &event
            else {
                events.push(event);
                continue;
            };
            if !(modifiers.command || modifiers.ctrl) {
                events.push(event);
                continue;
            }
            match key {
                Key::C => events.push(egui::Event::Copy),
                Key::X => events.push(egui::Event::Cut),
                Key::V => ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste),
                _ => events.push(event),
            }
        }
        input.events = events;
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let frame_started = std::time::Instant::now();
        let ctx = ui.ctx().clone();
        if ctx.input(|input| input.viewport().close_requested()) {
            self.handle_native_close_request(&ctx);
        }
        self.poll_mcp_bridge(&ctx);
        self.poll_desktop_job(&ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.window_title()));
        theme::apply(&ctx, self.theme_mode);
        #[cfg(windows)]
        theme::apply_native_caption(&ctx, _frame, self.theme_mode);
        theme::apply_size(&ctx, self.ui_size);
        self.handle_shortcuts(&ctx);
        if self.surface == WorkspaceSurface::Welcome {
            self.update_recent_previews(&ctx);
        }
        self.render_background(ui);
        self.render_brush_settings(&ctx);
        self.render_dialog(&ctx);
        self.render_overwrite_confirmation(&ctx);
        self.render_confirmation(&ctx);
        self.render_ribbon_customizer(&ctx);
        if self.confirm_action.is_none()
            && self.pending_after_save.is_none()
            && self.updater.show(&ctx)
        {
            self.updater.hide();
            self.request_exit();
        }
        if let Some(notice) = self.commands.take_notice() {
            self.toast = Some(crate::toast::Toast::new(
                notice,
                ctx.input(|input| input.time),
            ));
        }
        if let Some(toast) = &self.toast
            && !toast.show(&ctx, &ThemePalette::for_mode(self.theme_mode))
        {
            self.toast = None;
        }
        if self.close_requested {
            if let Err(error) = self.updater.finish_close() {
                self.close_requested = false;
                self.commands
                    .set_status(format!("Could not start the updater: {error}"));
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                return;
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if !self.close_requested {
            ctx.request_repaint_after(crate::diagnostics::UI_HEARTBEAT_INTERVAL);
        }
        crate::diagnostics::complete_ui_frame(frame_started.elapsed());
        if let Some(review) = self.review.as_mut()
            && review.advance(&ctx) == crate::review_capture::ReviewProgress::Finished
        {
            // The review scene is deliberately unsaved; ending the process
            // here avoids the unsaved-changes prompt blocking an unattended
            // capture. Nothing outside the isolated settings file is written.
            std::process::exit(0);
        }
    }
}

impl Drop for RunningDesktopJob {
    fn drop(&mut self) {
        if self.cancel_on_drop {
            self.handle.request_cancel();
        }
    }
}

fn native_import_picker(directory: &Path) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Import model")
        .set_directory(directory)
        .add_filter("All files", &["*"])
        .add_filter(
            "3D models",
            &["stl", "3mf", "obj", "ply", "gltf", "glb", "wrl", "3dp"],
        )
        .pick_file()
}

fn native_slicer_picker(current: Option<&Path>) -> Option<PathBuf> {
    let mut picker = rfd::FileDialog::new().set_title("Select 3D Slicer Executable");
    if let Some(path) = current
        && let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
    {
        picker = picker.set_directory(parent);
    }
    #[cfg(windows)]
    {
        picker = picker
            .add_filter("Slicer Application", &["exe"])
            .add_filter("All Files", &["*"]);
    }
    #[cfg(not(windows))]
    {
        picker = picker.add_filter("Applications", &["app", "bin"]);
    }
    picker.pick_file()
}

fn preserve_invalid_settings(path: &Path) -> std::io::Result<PathBuf> {
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "settings path has no file name",
        )
    })?;
    let file_name = file_name.to_string_lossy();
    for sequence in 0..100_u32 {
        let backup = path.with_file_name(format!(
            "{file_name}.corrupt-{}-{sequence}",
            std::process::id()
        ));
        if backup.exists() {
            continue;
        }
        match std::fs::copy(path, &backup) {
            Ok(_) => return Ok(backup),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "could not find an unused settings backup name",
    ))
}

const fn utility_accessible_label(command: CommandId) -> &'static str {
    match command {
        CommandId::SaveProject => "Save project",
        CommandId::SaveProjectAs => "Save project as",
        CommandId::Undo => "Undo",
        CommandId::Redo => "Redo",
        _ => "Command",
    }
}

fn discard_uncommitted_job_result(result: DesktopJobResult) {
    drop(result);
}

fn discard_unreceived_job_event(event: DesktopJobEvent) {
    drop(event);
}

fn recent_grid_layout(available_width: f32, item_spacing: f32) -> (usize, f32) {
    let columns = (available_width / 300.0).floor().clamp(1.0, 4.0) as usize;
    let gaps = item_spacing * columns.saturating_sub(1) as f32;
    let column_width = ((available_width - gaps) / columns as f32).max(0.0);
    (columns, column_width)
}

fn ribbon_separator(ui: &mut Ui, palette: ThemePalette) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(3.0, RIBBON_BUTTON_HEIGHT - 14.0), Sense::hover());
    ui.painter().rect_filled(rect, 1.0, palette.divider);
}

/// The accent "Slicer" button at the left of the top-right cluster.
fn slicer_button(ui: &mut Ui, tooltip: &str, palette: ThemePalette) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(SLICER_BUTTON_WIDTH, 30.0), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Slicer"));
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let painter = ui.painter_at(rect);
    let fill = if response.is_pointer_button_down_on() {
        palette.accent_dark
    } else if response.hovered() {
        palette.accent_control_hover
    } else {
        palette.accent_control
    };
    painter.rect_filled(rect, 6.0, fill);
    paint_slicer_symbol(
        &painter,
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 10.0, rect.center().y - 8.0),
            Vec2::splat(16.0),
        ),
        palette.accent_text,
    );
    painter.text(
        egui::pos2(rect.left() + 32.0, rect.center().y),
        Align2::LEFT_CENTER,
        "Slicer",
        FontId::proportional(12.5),
        palette.accent_text,
    );
    if response.has_focus() {
        painter.rect_stroke(
            rect,
            6.0,
            Stroke::new(1.5, palette.accent_text),
            egui::StrokeKind::Inside,
        );
    }
    response.on_hover_text(tooltip)
}

/// Three stacked layers: a model cut into slices.
fn paint_slicer_symbol(painter: &egui::Painter, rect: egui::Rect, color: Color32) {
    let stroke = Stroke::new(1.8, color);
    for (row, inset) in [(0.0_f32, 4.0_f32), (1.0, 2.0), (2.0, 0.0)] {
        let y = rect.top() + 3.0 + row * 5.0;
        painter.line_segment(
            [
                egui::pos2(rect.left() + inset, y),
                egui::pos2(rect.right() - inset, y),
            ],
            stroke,
        );
    }
}

/// Light/dark switch: a pill track with a knob carrying the current mode's
/// sun or moon. Returns true when clicked.
fn theme_toggle(ui: &mut Ui, mode: ThemeMode, palette: ThemePalette) -> bool {
    let (slot, response) =
        ui.allocate_exact_size(Vec2::new(THEME_TOGGLE_WIDTH, 36.0), Sense::click());
    let dark = mode == ThemeMode::Dark;
    let label = if dark {
        "Switch to light mode"
    } else {
        "Switch to dark mode"
    };
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, true, dark, "Dark mode"));
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let painter = ui.painter_at(slot);
    let track = egui::Rect::from_center_size(slot.center(), Vec2::new(44.0, 24.0));
    let track_fill = if response.hovered() {
        palette.pressed
    } else {
        palette.hover
    };
    painter.rect_filled(track, 12.0, track_fill);
    painter.rect_stroke(
        track,
        12.0,
        Stroke::new(1.0, palette.divider),
        egui::StrokeKind::Inside,
    );
    let left = egui::pos2(track.left() + 12.0, track.center().y);
    let right = egui::pos2(track.right() - 12.0, track.center().y);
    let (knob, other) = if dark { (right, left) } else { (left, right) };
    // The mode not in use, faint, on the empty side of the track.
    if dark {
        paint_sun(&painter, other, palette.muted.gamma_multiply(0.7));
    } else {
        paint_moon(
            &painter,
            other,
            palette.muted.gamma_multiply(0.7),
            track_fill,
        );
    }
    painter.circle_filled(knob, 9.5, palette.accent_control);
    if dark {
        paint_moon(&painter, knob, palette.accent_text, palette.accent_control);
    } else {
        paint_sun(&painter, knob, palette.accent_text);
    }
    if response.has_focus() {
        painter.rect_stroke(
            track,
            12.0,
            Stroke::new(1.5, palette.accent_border),
            egui::StrokeKind::Inside,
        );
    }
    response.on_hover_text(label).clicked()
}

fn paint_sun(painter: &egui::Painter, center: egui::Pos2, color: Color32) {
    painter.circle_filled(center, 3.0, color);
    let stroke = Stroke::new(1.3, color);
    for step in 0..8 {
        let angle = step as f32 * std::f32::consts::FRAC_PI_4;
        let direction = Vec2::new(angle.cos(), angle.sin());
        painter.line_segment([center + direction * 4.8, center + direction * 6.6], stroke);
    }
}

fn paint_moon(painter: &egui::Painter, center: egui::Pos2, color: Color32, background: Color32) {
    painter.circle_filled(center, 5.5, color);
    painter.circle_filled(center + Vec2::new(2.8, -2.2), 4.6, background);
}

/// A save icon with its name beside it ("Save" / "Save As").
/// Joins the active tab's tile to the open ribbon below it, like a folder
/// tab (OF-27): the tile's bottom is squared off and filled down through the
/// tab bar, and small concave corners at its base blend it into the bar.
fn attach_tab_to_ribbon(ui: &Ui, tab: egui::Rect, bar_bottom: f32, palette: ThemePalette) {
    if bar_bottom <= tab.bottom() {
        return;
    }
    let painter = ui.painter();
    let radius = RADIUS_CONTROL;
    painter.rect_filled(
        egui::Rect::from_min_max(
            egui::pos2(tab.left(), tab.bottom() - radius),
            egui::pos2(tab.right(), bar_bottom),
        ),
        0.0,
        palette.panel,
    );
    for (corner_x, centre_x) in [
        (tab.left() - radius, tab.left() - radius),
        (tab.right(), tab.right() + radius),
    ] {
        let square = egui::Rect::from_min_size(
            egui::pos2(corner_x, bar_bottom - radius),
            Vec2::splat(radius),
        );
        painter.rect_filled(square, 0.0, palette.panel);
        painter.with_clip_rect(square).circle_filled(
            egui::pos2(centre_x, bar_bottom - radius),
            radius,
            palette.navigation,
        );
    }
}

/// One full-width row of a drop-down menu. The row under the pointer is
/// tinted so the user can see where they are as they move down the menu
/// (OF-26); a pressed row is tinted deeper and a keyboard-focused row is
/// outlined.
fn menu_row(ui: &mut Ui, label: &str, palette: ThemePalette) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    let fill = if response.is_pointer_button_down_on() {
        Some(palette.pressed)
    } else if response.hovered() || response.has_focus() {
        Some(palette.hover)
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, RADIUS_CONTROL, fill);
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect,
            RADIUS_CONTROL,
            Stroke::new(1.5, palette.accent_border),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        egui::TextStyle::Button.resolve(ui.style()),
        palette.text,
    );
    response
}

fn labelled_save_button(
    ui: &mut Ui,
    label: &str,
    save_as: bool,
    tooltip: &str,
    palette: ThemePalette,
) -> bool {
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_owned(), FontId::proportional(12.5), palette.text);
    let width = 8.0 + 18.0 + 6.0 + galley.size().x + 10.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 36.0), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let painter = ui.painter_at(rect);
    if response.hovered() || response.has_focus() {
        painter.rect_filled(rect, 6.0, palette.hover);
    }
    if response.has_focus() {
        painter.rect_stroke(
            rect,
            6.0,
            Stroke::new(1.5, palette.accent_border),
            egui::StrokeKind::Inside,
        );
    }
    paint_save_symbol(
        &painter,
        egui::Rect::from_min_size(
            egui::pos2(rect.left() + 8.0, rect.center().y - 9.0),
            Vec2::splat(18.0),
        ),
        palette.muted,
        save_as,
    );
    painter.galley(
        egui::pos2(rect.left() + 32.0, rect.center().y - galley.size().y * 0.5),
        galley,
        palette.text,
    );
    response.on_hover_text(tooltip).clicked()
}

fn paint_save_symbol(painter: &egui::Painter, rect: egui::Rect, color: Color32, save_as: bool) {
    let stroke = Stroke::new(1.7, color);
    let body = rect.shrink(2.0);
    painter.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Inside);
    painter.line_segment(
        [
            egui::pos2(body.left() + 3.0, body.top() + 6.0),
            egui::pos2(body.right() - 3.0, body.top() + 6.0),
        ],
        stroke,
    );
    painter.rect_stroke(
        egui::Rect::from_min_max(
            egui::pos2(body.left() + 4.0, body.bottom() - 7.0),
            egui::pos2(body.right() - 4.0, body.bottom() - 2.0),
        ),
        1.0,
        stroke,
        egui::StrokeKind::Inside,
    );
    if save_as {
        painter.line_segment(
            [
                egui::pos2(body.right() - 1.0, body.top() + 2.0),
                egui::pos2(body.right() + 4.0, body.top() - 3.0),
            ],
            stroke,
        );
    }
}

fn all_command_keys() -> Vec<String> {
    ribbon_spec()
        .into_iter()
        .flat_map(|(tab, items)| {
            items.into_iter().filter_map(move |item| match item {
                RibbonItem::Command(command) => Some(command_key(tab, command)),
                RibbonItem::Separator => None,
            })
        })
        .collect()
}

fn find_command_key(key: &str) -> Option<(RibbonTab, CommandDescriptor)> {
    ribbon_spec().into_iter().find_map(|(tab, items)| {
        items.into_iter().find_map(|item| match item {
            RibbonItem::Command(command) if command_key(tab, command) == key => {
                Some((tab, command))
            }
            _ => None,
        })
    })
}

/// Commands that leave the Measure bar open (OF-55): history and the view.
const fn measure_keeps_running_for(command: CommandId) -> bool {
    matches!(
        command,
        CommandId::Undo
            | CommandId::Redo
            | CommandId::CenterView
            | CommandId::FrameSelection
            | CommandId::ResetView
            | CommandId::ViewTop
            | CommandId::ViewSide
            | CommandId::ToggleGrid
            | CommandId::ToggleWireframe
            | CommandId::ToggleXray
    )
}

const fn is_edit_tool(command: CommandId) -> bool {
    matches!(
        command,
        CommandId::Simplify
            | CommandId::Subdivide
            | CommandId::Smooth
            | CommandId::Grab
            | CommandId::Crease
            | CommandId::SmoothBrush
            | CommandId::Blob
            | CommandId::Merge
            | CommandId::Emboss
            | CommandId::Hollow
            | CommandId::Intersect
            | CommandId::Subtract
    )
}

fn modifier_dialog(command: CommandId) -> Option<DialogKind> {
    match command {
        CommandId::Simplify => Some(DialogKind::Simplify),
        CommandId::Subdivide => Some(DialogKind::Subdivide),
        CommandId::Hollow => Some(DialogKind::Hollow),
        CommandId::Emboss => Some(DialogKind::Emboss),
        CommandId::Smooth | CommandId::Merge | CommandId::Intersect | CommandId::Subtract => {
            Some(DialogKind::EditAction(command))
        }
        _ => None,
    }
}

fn command_enabled(command: CommandId, state: &CommandState) -> bool {
    if is_edit_tool(command) {
        return true;
    }
    let selected = state.selected_count();
    match command {
        CommandId::Paste => state.has_clipboard(),
        CommandId::SelectAll | CommandId::DeselectAll | CommandId::InvertSelection => {
            !state.scene().objects().is_empty()
        }
        CommandId::Delete
        | CommandId::Duplicate
        | CommandId::Copy
        | CommandId::Cut
        | CommandId::Settle
        | CommandId::Mirror
        | CommandId::Simplify
        | CommandId::Subdivide
        | CommandId::Smooth
        | CommandId::Emboss
        | CommandId::Hollow
        | CommandId::Repair
        | CommandId::FrameSelection => selected > 0,
        CommandId::Align | CommandId::Arrange => selected >= modeler3d_core::PLACEMENT_MIN_OBJECTS,
        // As in the Python build these stay clickable whenever there is
        // something to work on, and explain themselves when nothing is
        // selected instead of sitting greyed out.
        CommandId::Grab
        | CommandId::Crease
        | CommandId::SmoothBrush
        | CommandId::Blob
        | CommandId::Material
        | CommandId::Measure
        | CommandId::Paint => !state.scene().objects().is_empty(),
        CommandId::EditTorus => selected > 0 && state.has_selected_parametric_torus(),
        CommandId::Group => selected > 1,
        CommandId::Merge | CommandId::Intersect | CommandId::Subtract => selected > 0,
        CommandId::Ungroup => state.scene().selection_contains_group(),
        CommandId::Export => !state.scene().objects().is_empty(),
        _ => true,
    }
}

fn command_summary(command: CommandId) -> &'static str {
    match command {
        CommandId::Modeler3dMenu => "Start screen",
        CommandId::NewScene => "Blank scene",
        CommandId::OpenProject => "Open project",
        CommandId::ImportModel => "Add or open model",
        CommandId::SaveProject => "Save as STL",
        CommandId::SaveProjectAs => "Save as STL or 3MF",
        CommandId::Export => "Export to any format",
        CommandId::Repair => "Fix mesh issues",
        CommandId::PrintPrep => "Check for printing",
        CommandId::Settings => "App preferences",
        CommandId::Exit => "Close 3D Modeler",
        CommandId::AddPrimitive(_) => "Add a 3D shape",
        CommandId::EditTorus => "Edit torus shape",
        CommandId::Duplicate => "Make a copy",
        CommandId::Copy => "Copy selected models",
        CommandId::Paste => "Paste copied models",
        CommandId::Cut => "Cut selected models",
        CommandId::Delete => "Remove selected models",
        CommandId::Settle => "Place on floor",
        CommandId::Mirror => "Flip selected model",
        CommandId::Measure => "Measure with a ruler",
        CommandId::Align => "Line up models",
        CommandId::Arrange => "Space models evenly",
        CommandId::SelectAll => "Select every model",
        CommandId::DeselectAll => "Clear selection",
        CommandId::InvertSelection => "Select the rest",
        CommandId::Group => "Group selected models",
        CommandId::Ungroup => "Separate grouped models",
        CommandId::Simplify => "Reduce mesh detail",
        CommandId::Subdivide => "Add mesh detail",
        CommandId::Smooth => "Smooth model surface",
        CommandId::Grab => "Sculpt with Grab",
        CommandId::Crease => "Sculpt a crease",
        CommandId::SmoothBrush => "Smooth with brush",
        CommandId::Blob => "Add sculpted volume",
        CommandId::Merge => "Join selected models",
        CommandId::Emboss => "Raise a design",
        CommandId::Hollow => "Make a hollow shell",
        CommandId::Intersect => "Keep the overlap",
        CommandId::Subtract => "Cut one model from another",
        CommandId::Material => "Change material",
        CommandId::Paint => "Paint selected model",
        CommandId::CenterView => "Show the whole scene",
        CommandId::FrameSelection => "Show selected models",
        CommandId::ResetView => "Return to default view",
        CommandId::ViewTop => "Look from above",
        CommandId::ViewSide => "Look from the side",
        CommandId::ToggleGrid => "Show or hide floor grid",
        CommandId::ToggleWireframe => "Show mesh edges",
        CommandId::ToggleXray => "See through models",
        CommandId::ToggleSnap => "Snap moves to the floor",
        CommandId::Undo => "Undo last change",
        CommandId::Redo => "Redo last change",
    }
}

fn command_help(command: CommandId) -> &'static str {
    match command {
        CommandId::Modeler3dMenu => "Return to the start screen and recent projects.",
        CommandId::NewScene => "Start with a blank scene. Unsaved changes are checked first.",
        CommandId::OpenProject => "Open a saved 3D Modeler project without losing unsaved work.",
        CommandId::ImportModel => "Add a mesh or scene file to the current project.",
        CommandId::SaveProject => "Save the scene (STL by default).",
        CommandId::SaveProjectAs => "Save the scene to a new STL or 3MF file.",
        CommandId::Export => {
            "Export the selection (or every visible model) to a mesh, scene or project file."
        }
        CommandId::Repair => "Check the selected mesh and fix repairable problems.",
        CommandId::PrintPrep => "Check visible models and send them to an installed slicer.",
        CommandId::Settings => {
            "Change the theme, viewport lighting, units, import defaults, and ribbon settings."
        }
        CommandId::Exit => "Close 3D Modeler. Unsaved changes are checked first.",
        CommandId::AddPrimitive(_) => "Add a new 3D shape to the scene and select it.",
        CommandId::EditTorus => "Change the size and shape of the selected torus.",
        CommandId::Duplicate => "Create a copy of the selected models.",
        CommandId::Copy => "Copy the selected models to the 3D Modeler clipboard.",
        CommandId::Paste => "Place the copied models into the current scene.",
        CommandId::Cut => "Remove the selected models and copy them to the clipboard.",
        CommandId::Delete => "Remove the selected models from the scene. (Delete)",
        CommandId::Settle => "Move the selected models down to the build floor.",
        CommandId::Mirror => "Flip the selected models across the X axis.",
        CommandId::Measure => {
            "Click two points on your models to measure the distance between them."
        }
        CommandId::Align => "Line up the selected models with the first selected model.",
        CommandId::Arrange => "Space the selected models evenly along the X axis.",
        CommandId::SelectAll => "Select every model in the scene.",
        CommandId::DeselectAll => "Clear the current model selection.",
        CommandId::InvertSelection => "Select unselected models and clear the current selection.",
        CommandId::Group => "Put the selected models into one group without joining their meshes.",
        CommandId::Ungroup => "Separate the children of selected groups.",
        CommandId::Simplify => "Reduce the selected mesh detail while keeping its shape close.",
        CommandId::Subdivide => "Add faces to the selected mesh for more detail.",
        CommandId::Smooth => "Smooth the selected model surface.",
        CommandId::Grab => "Sculpt the selected model by pulling its surface.",
        CommandId::Crease => "Sculpt a sharper groove into the selected model.",
        CommandId::SmoothBrush => "Smooth the selected model where you brush.",
        CommandId::Blob => "Add or remove rounded volume where you brush.",
        CommandId::Merge => "Join the selected models into one solid.",
        CommandId::Emboss => "Raise a design on the selected model.",
        CommandId::Hollow => "Create a hollow shell from the selected model.",
        CommandId::Intersect => "Keep only the area where selected models overlap.",
        CommandId::Subtract => "Cut the second selected model out of the first.",
        CommandId::Material => "Choose a material for the selected model.",
        CommandId::Paint => "Paint the selected model in the viewport.",
        CommandId::CenterView => "Move the camera to show all visible models.",
        CommandId::FrameSelection => "Move the camera to show the selected models.",
        CommandId::ResetView => "Restore the default camera angle and distance.",
        CommandId::ViewTop => "Look down at the scene from above.",
        CommandId::ViewSide => "Look across the scene from the side.",
        CommandId::ToggleGrid => "Show or hide the metric build-floor grid.",
        CommandId::ToggleWireframe => "Show the edges of the model meshes.",
        CommandId::ToggleXray => "Make models translucent so you can see through them.",
        CommandId::ToggleSnap => "Snap moves to nearby models and the build floor.",
        CommandId::Undo => "Undo the last model change. (Ctrl+Z)",
        CommandId::Redo => "Redo the last undone model change. (Ctrl+Y)",
    }
}

fn command_disabled_help(command: CommandId, state: &CommandState) -> &'static str {
    match command {
        CommandId::Paste if !state.has_clipboard() => "Copy or cut a model before pasting.",
        CommandId::Group if state.selected_count() < 2 => {
            "Select at least two models to make a group."
        }
        CommandId::Align | CommandId::Arrange
            if state.selected_count() < modeler3d_core::PLACEMENT_MIN_OBJECTS =>
        {
            modeler3d_core::PLACEMENT_NEEDS_TWO_MESSAGE
        }
        CommandId::Ungroup => "Select a group to ungroup it.",
        CommandId::EditTorus => "Select a torus to change its size and shape.",
        CommandId::Measure => "Add or import a model to measure.",
        CommandId::Export if state.scene().objects().is_empty() => {
            "Add or import a model before exporting."
        }
        CommandId::SelectAll | CommandId::DeselectAll | CommandId::InvertSelection => {
            "Add or import a model before changing the selection."
        }
        CommandId::Delete
        | CommandId::Duplicate
        | CommandId::Copy
        | CommandId::Cut
        | CommandId::Settle
        | CommandId::Mirror
        | CommandId::Align
        | CommandId::Arrange
        | CommandId::Simplify
        | CommandId::Subdivide
        | CommandId::Smooth
        | CommandId::Grab
        | CommandId::Crease
        | CommandId::SmoothBrush
        | CommandId::Blob
        | CommandId::Emboss
        | CommandId::Hollow
        | CommandId::Material
        | CommandId::Paint
        | CommandId::Repair
        | CommandId::FrameSelection => "Select a model first.",
        _ => command_help(command),
    }
}

fn brush_falloff_combo(ui: &mut Ui, label: &str, value: &mut BrushFalloff, id: &'static str) {
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(id)
            .selected_text(brush_falloff_label(*value))
            .show_ui(ui, |ui| {
                for option in [
                    BrushFalloff::Smooth,
                    BrushFalloff::Linear,
                    BrushFalloff::Constant,
                ] {
                    ui.selectable_value(value, option, brush_falloff_label(option));
                }
            });
    });
}

fn brush_direction_combo(ui: &mut Ui, value: &mut BrushDirection, id: &'static str) {
    ui.horizontal(|ui| {
        ui.label("Direction");
        egui::ComboBox::from_id_salt(id)
            .selected_text(brush_direction_label(*value))
            .show_ui(ui, |ui| {
                for option in [BrushDirection::Inward, BrushDirection::Outward] {
                    ui.selectable_value(value, option, brush_direction_label(option));
                }
            });
    });
}

fn brush_symmetry_combo(ui: &mut Ui, value: &mut BrushSymmetry) {
    ui.horizontal(|ui| {
        ui.label("Symmetry");
        egui::ComboBox::from_id_salt("grab-symmetry")
            .selected_text(brush_symmetry_label(*value))
            .show_ui(ui, |ui| {
                for option in [
                    BrushSymmetry::None,
                    BrushSymmetry::X,
                    BrushSymmetry::Y,
                    BrushSymmetry::Z,
                    BrushSymmetry::Xy,
                    BrushSymmetry::Xz,
                    BrushSymmetry::Yz,
                    BrushSymmetry::Xyz,
                ] {
                    ui.selectable_value(value, option, brush_symmetry_label(option));
                }
            });
    });
}

const fn brush_falloff_label(value: BrushFalloff) -> &'static str {
    match value {
        BrushFalloff::Smooth => "Smooth",
        BrushFalloff::Linear => "Linear",
        BrushFalloff::Constant => "Constant",
    }
}

const fn brush_direction_label(value: BrushDirection) -> &'static str {
    match value {
        BrushDirection::Inward => "Inward",
        BrushDirection::Outward => "Outward",
    }
}

const fn brush_symmetry_label(value: BrushSymmetry) -> &'static str {
    match value {
        BrushSymmetry::None => "None",
        BrushSymmetry::X => "X",
        BrushSymmetry::Y => "Y",
        BrushSymmetry::Z => "Z",
        BrushSymmetry::Xy => "X/Y",
        BrushSymmetry::Xz => "X/Z",
        BrushSymmetry::Yz => "Y/Z",
        BrushSymmetry::Xyz => "X/Y/Z",
    }
}

const fn boolean_operation_for_command(
    command: CommandId,
) -> Option<modeler3d_core::BooleanOperation> {
    match command {
        CommandId::Merge => Some(modeler3d_core::BooleanOperation::Union),
        CommandId::Intersect => Some(modeler3d_core::BooleanOperation::Intersection),
        CommandId::Subtract => Some(modeler3d_core::BooleanOperation::Difference),
        _ => None,
    }
}

const fn boolean_label(operation: modeler3d_core::BooleanOperation) -> &'static str {
    match operation {
        modeler3d_core::BooleanOperation::Union => "Merge",
        modeler3d_core::BooleanOperation::Intersection => "Intersect",
        modeler3d_core::BooleanOperation::Difference => "Subtract",
    }
}

fn discover_capability_profile_blocking() -> Result<CapabilityProfile, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("could not initialize adapter discovery: {error}"))?;
    runtime
        .block_on(discover_capability_profile())
        .map_err(|error| format!("could not discover graphics adapters: {error}"))
}

fn viewport_input(ctx: &egui::Context) -> ViewportInput {
    ctx.input(|input| {
        let pointer = &input.pointer;
        ViewportInput {
            pointer_position: pointer.interact_pos(),
            pointer_delta: pointer.delta(),
            primary_pressed: pointer.button_pressed(egui::PointerButton::Primary),
            primary_down: pointer.button_down(egui::PointerButton::Primary),
            primary_released: pointer.button_released(egui::PointerButton::Primary),
            secondary_pressed: pointer.button_pressed(egui::PointerButton::Secondary),
            secondary_down: pointer.button_down(egui::PointerButton::Secondary),
            secondary_released: pointer.button_released(egui::PointerButton::Secondary),
            middle_pressed: pointer.button_pressed(egui::PointerButton::Middle),
            middle_down: pointer.button_down(egui::PointerButton::Middle),
            middle_released: pointer.button_released(egui::PointerButton::Middle),
            scroll_y: input.smooth_scroll_delta.y,
            pinch_zoom_factor: input.zoom_delta(),
            escape_pressed: input.key_pressed(Key::Escape),
            focused: input.focused,
            modifiers: input.modifiers,
        }
    })
}

/// Whether an Items row is free of the "needs repair" marker. A group is
/// separate pieces by design, so it needs repair only when one of its parts
/// does (nested groups are not inspected).
fn item_is_healthy(object: &modeler3d_core::SceneObject) -> bool {
    let mesh_healthy = |mesh: &modeler3d_core::Mesh| {
        modeler3d_core::inspect_mesh(mesh).is_ok_and(|report| report.valid)
    };
    if object.children().is_empty() {
        mesh_healthy(object.mesh())
    } else {
        object
            .children()
            .iter()
            .all(|child| !child.children().is_empty() || mesh_healthy(child.mesh()))
    }
}

/// The axis choices of the Mirror, Align and Arrange dialogs: upper-case
/// letters coloured like the move arrows (OF-41, OF-42).
fn placement_axis_labels() -> [(Axis, &'static str, Color32); 3] {
    let [x, y, z] = crate::chrome::AXIS_COLORS;
    [(Axis::X, "X", x), (Axis::Y, "Y", y), (Axis::Z, "Z", z)]
}

fn viewport_pointer_interactions_allowed(
    interaction_blocked: bool,
    pointer_over_viewport: bool,
    drag_active: bool,
) -> bool {
    !interaction_blocked && (pointer_over_viewport || drag_active)
}

fn physical_viewport_size(size_points: Vec2, pixels_per_point: f32) -> (u32, u32) {
    let width = (size_points.x * pixels_per_point).round().max(1.0) as u32;
    let height = (size_points.y * pixels_per_point).round().max(1.0) as u32;
    (width, height)
}

fn project_to_viewport(
    matrix: Mat4,
    rect: egui::Rect,
    point: [f32; 3],
) -> Option<(egui::Pos2, f32)> {
    let clip = matrix.transform4([point[0], point[1], point[2], 1.]);
    if clip.iter().any(|v| !v.is_finite()) || clip[3] <= 0. {
        return None;
    }
    Some((
        egui::pos2(
            rect.left() + (clip[0] / clip[3] + 1.) * rect.width() * 0.5,
            rect.top() + (1. - clip[1] / clip[3]) * rect.height() * 0.5,
        ),
        clip[2] / clip[3],
    ))
}

/// Ids of objects whose projected world bounds overlap `marquee` (screen
/// points inside `rect`). Corners behind the camera are skipped; an object
/// with no corner in front of it is never hit.
fn objects_in_marquee(
    scene: &RenderScene,
    matrix: Mat4,
    rect: egui::Rect,
    marquee: egui::Rect,
) -> Vec<String> {
    scene
        .objects()
        .iter()
        .filter(|object| {
            let Ok(bounds) = object.world_bounds() else {
                return false;
            };
            let mut footprint = egui::Rect::NOTHING;
            for corner in 0..8 {
                let point = [
                    if corner & 1 == 0 {
                        bounds.minimum[0]
                    } else {
                        bounds.maximum[0]
                    },
                    if corner & 2 == 0 {
                        bounds.minimum[1]
                    } else {
                        bounds.maximum[1]
                    },
                    if corner & 4 == 0 {
                        bounds.minimum[2]
                    } else {
                        bounds.maximum[2]
                    },
                ];
                if let Some((screen, _)) = project_to_viewport(matrix, rect, point) {
                    footprint.extend_with(screen);
                }
            }
            footprint.is_positive() && footprint.intersects(marquee)
        })
        .map(|object| object.object_id().to_owned())
        .collect()
}

// Egui draws these overlays outside the GPU viewport pipeline, so clip them in
// homogeneous space before dividing by w to avoid streaks at the near plane.
fn project_gizmo_point(matrix: Mat4, rect: egui::Rect, point: [f32; 3]) -> Option<egui::Pos2> {
    let clip = matrix.transform4([point[0], point[1], point[2], 1.0]);
    if clip.iter().any(|value| !value.is_finite())
        || (0..6).any(|plane| clip_plane_distance(clip, plane) < 0.0)
    {
        return None;
    }
    clip_point_to_viewport(rect, clip)
}

fn project_gizmo_line(
    matrix: Mat4,
    rect: egui::Rect,
    start: [f32; 3],
    end: [f32; 3],
) -> Option<[egui::Pos2; 2]> {
    let mut points = [
        matrix.transform4([start[0], start[1], start[2], 1.0]),
        matrix.transform4([end[0], end[1], end[2], 1.0]),
    ];
    if points.iter().flatten().any(|value| !value.is_finite()) {
        return None;
    }
    for plane in 0..6 {
        let first_distance = clip_plane_distance(points[0], plane);
        let second_distance = clip_plane_distance(points[1], plane);
        if !first_distance.is_finite() || !second_distance.is_finite() {
            return None;
        }
        if first_distance < 0.0 && second_distance < 0.0 {
            return None;
        }
        if first_distance < 0.0 {
            let amount = first_distance / (first_distance - second_distance);
            points[0] = interpolate_clip_point(points[0], points[1], amount);
        } else if second_distance < 0.0 {
            let amount = first_distance / (first_distance - second_distance);
            points[1] = interpolate_clip_point(points[0], points[1], amount);
        }
    }
    Some([
        clip_point_to_viewport(rect, points[0])?,
        clip_point_to_viewport(rect, points[1])?,
    ])
}

fn paint_gizmo_shaft(
    painter: &egui::Painter,
    start: egui::Pos2,
    end: egui::Pos2,
    axis_color: Color32,
    width_pixels: f32,
    emphasized: bool,
) {
    let pixels_per_point = painter.ctx().pixels_per_point().max(1.0e-6);
    let segment = end - start;
    let segment_length = segment.length();
    if !segment_length.is_finite() || segment_length <= 1.0e-3 {
        return;
    }
    let direction = segment / segment_length;
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let pixels_to_points = 1.0 / pixels_per_point;
    let outline = Color32::from_rgba_unmultiplied(18, 24, 36, 232);
    if emphasized {
        painter.line_segment(
            [start, end],
            Stroke::new(
                (width_pixels + 7.0) * pixels_to_points,
                with_alpha(axis_color, 44),
            ),
        );
    }
    painter.line_segment(
        [start, end],
        Stroke::new((width_pixels + 2.0) * pixels_to_points, outline),
    );
    painter.line_segment(
        [start, end],
        Stroke::new(width_pixels * pixels_to_points, axis_color),
    );
    let highlight = axis_highlight_color(axis_color);
    let highlight_offset = perpendicular * (width_pixels * 0.16 * pixels_to_points);
    painter.line_segment(
        [start + highlight_offset, end + highlight_offset],
        Stroke::new(0.8 * pixels_to_points, with_alpha(highlight, 220)),
    );
}

fn paint_curved_rotate_arrows(
    painter: &egui::Painter,
    matrix: Mat4,
    rect: egui::Rect,
    ring_lines: &[GizmoLine],
    emphasized_axis: Option<usize>,
) {
    let ring_segments = ROTATE_RING_SEGMENTS as usize;
    let pixels_per_point = painter.ctx().pixels_per_point().max(1.0e-6);
    let size_scale = MOVE_GIZMO_SCALE;
    let head_length = MOVE_ARROW_HEAD_LENGTH_PIXELS * size_scale / pixels_per_point;
    let head_half_width = MOVE_ARROW_HEAD_HALF_WIDTH_PIXELS * size_scale / pixels_per_point;
    let visible_rect = rect.shrink(head_half_width + 4.0 / pixels_per_point);
    let mut arrow_tips = Vec::new();

    for (ring_index, ring) in ring_lines.chunks_exact(ring_segments).enumerate() {
        let preferred = (ring_index * ring_segments / 3 + ring_segments / 8) % ring_segments;
        let mut best: Option<(f32, usize, egui::Pos2, Vec2)> = None;
        for (segment_index, line) in ring.iter().enumerate() {
            let Some([start, end]) = project_gizmo_line(matrix, rect, line.start, line.end) else {
                continue;
            };
            let tangent = end - start;
            let tangent_length = tangent.length();
            if !tangent_length.is_finite()
                || tangent_length <= 1.0e-3
                || !visible_rect.contains(end)
            {
                continue;
            }
            let separation = arrow_tips
                .iter()
                .map(|tip: &egui::Pos2| tip.distance(end))
                .fold(150.0_f32, f32::min);
            let center_distance = end.distance(rect.center());
            let difference = segment_index.abs_diff(preferred);
            let preferred_distance = difference.min(ring_segments - difference) as f32;
            let score = tangent_length + center_distance * 0.15 + separation * 0.2
                - preferred_distance * 0.05;
            if best
                .as_ref()
                .is_none_or(|(current, _, _, _)| score > *current)
            {
                best = Some((score, segment_index, end, tangent / tangent_length));
            }
        }

        let Some((_, tip_segment, tip, direction)) = best else {
            continue;
        };
        let emphasized = emphasized_axis == Some(ring_index);
        let axis_color = Color32::from(egui::Rgba::from_rgba_unmultiplied(
            ring[tip_segment].color[0],
            ring[tip_segment].color[1],
            ring[tip_segment].color[2],
            ring[tip_segment].color[3],
        ));

        // Thicken a short section of the existing ring so the arrow follows
        // its curve into the tangent arrowhead without replacing the ring.
        let mut curve_length = 0.0;
        let mut step = 0;
        while step < ring.len() && (curve_length < head_length * 1.25 || step < 4) {
            let segment_index = (tip_segment + ring.len() - step) % ring.len();
            let line = ring[segment_index];
            if let Some([start, end]) = project_gizmo_line(matrix, rect, line.start, line.end) {
                curve_length += (end - start).length();
                paint_gizmo_shaft(
                    painter,
                    start,
                    end,
                    axis_color,
                    line.width_pixels,
                    emphasized,
                );
            }
            step += 1;
        }

        paint_gizmo_arrowhead(
            painter,
            tip - direction * head_length,
            direction,
            axis_color,
            size_scale,
            emphasized,
        );
        arrow_tips.push(tip);
    }
}

fn distance_to_screen_segment(point: egui::Pos2, start: egui::Pos2, end: egui::Pos2) -> f32 {
    let segment = end - start;
    let length_squared = segment.length_sq();
    if !length_squared.is_finite() || length_squared <= 1.0e-6 {
        return point.distance(start);
    }
    let parameter = ((point - start).dot(segment) / length_squared).clamp(0.0, 1.0);
    point.distance(start + segment * parameter)
}

fn gizmo_axis_index(axis: GizmoAxis) -> usize {
    match axis {
        GizmoAxis::X => 0,
        GizmoAxis::Y => 1,
        GizmoAxis::Z => 2,
    }
}

fn with_alpha(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

fn axis_highlight_color(color: Color32) -> Color32 {
    let highlight = |channel: u8| channel + ((u8::MAX - channel) / 2);
    Color32::from_rgb(
        highlight(color.r()),
        highlight(color.g()),
        highlight(color.b()),
    )
}

fn axis_shadow_color(color: Color32) -> Color32 {
    Color32::from_rgb(
        (u16::from(color.r()) * 68 / 100) as u8,
        (u16::from(color.g()) * 68 / 100) as u8,
        (u16::from(color.b()) * 68 / 100) as u8,
    )
}

fn paint_gizmo_arrowhead(
    painter: &egui::Painter,
    base: egui::Pos2,
    axis_direction: Vec2,
    axis_color: Color32,
    size_scale: f32,
    emphasized: bool,
) {
    let pixels_per_point = painter.ctx().pixels_per_point().max(1.0e-6);
    let direction_length = axis_direction.length();
    if !direction_length.is_finite() || direction_length <= 1.0e-3 {
        return;
    }
    let direction = axis_direction / direction_length;
    let perpendicular = Vec2::new(-direction.y, direction.x);
    let pixels_to_points = size_scale / pixels_per_point;
    let head_length = MOVE_ARROW_HEAD_LENGTH_PIXELS * pixels_to_points;
    let head_half_width = MOVE_ARROW_HEAD_HALF_WIDTH_PIXELS * pixels_to_points;
    let tip = base + direction * head_length;
    let outer_left = base + perpendicular * head_half_width;
    let outer_right = base - perpendicular * head_half_width;
    let outline = Color32::from_rgba_unmultiplied(18, 24, 36, 240);
    painter.add(egui::Shape::convex_polygon(
        vec![tip, outer_left, outer_right],
        outline,
        Stroke::NONE,
    ));

    let inset = 1.5 / pixels_per_point;
    let inner_half_width = head_half_width - inset;
    if inner_half_width <= 0.0 {
        return;
    }
    let inner_tip = tip - direction * inset;
    let inner_base = base + direction * inset;
    let face_color = if emphasized {
        axis_highlight_color(axis_color)
    } else {
        axis_color
    };
    painter.add(egui::Shape::convex_polygon(
        vec![
            inner_tip,
            inner_base + perpendicular * inner_half_width,
            inner_base - perpendicular * inner_half_width,
        ],
        face_color,
        Stroke::NONE,
    ));
    painter.add(egui::Shape::convex_polygon(
        vec![
            inner_tip,
            inner_base,
            inner_base - perpendicular * inner_half_width,
        ],
        axis_shadow_color(face_color),
        Stroke::NONE,
    ));
    painter.line_segment(
        [
            inner_tip,
            inner_base + perpendicular * inner_half_width * 0.85,
        ],
        Stroke::new(
            0.9 / pixels_per_point,
            with_alpha(Color32::WHITE, if emphasized { 245 } else { 190 }),
        ),
    );
}

fn clip_gizmo_triangle(matrix: Mat4, rect: egui::Rect, triangle: [[f32; 3]; 3]) -> Vec<egui::Pos2> {
    let mut polygon = triangle
        .into_iter()
        .map(|point| matrix.transform4([point[0], point[1], point[2], 1.0]))
        .collect::<Vec<_>>();
    if polygon.iter().flatten().any(|value| !value.is_finite()) {
        return Vec::new();
    }
    for plane in 0..6 {
        let input = std::mem::take(&mut polygon);
        let Some(mut previous) = input.last().copied() else {
            return Vec::new();
        };
        let mut previous_distance = clip_plane_distance(previous, plane);
        if !previous_distance.is_finite() {
            return Vec::new();
        }
        for current in input {
            let current_distance = clip_plane_distance(current, plane);
            if !current_distance.is_finite() {
                return Vec::new();
            }
            let previous_inside = previous_distance >= 0.0;
            let current_inside = current_distance >= 0.0;
            if previous_inside != current_inside {
                let amount = previous_distance / (previous_distance - current_distance);
                polygon.push(interpolate_clip_point(previous, current, amount));
            }
            if current_inside {
                polygon.push(current);
            }
            previous = current;
            previous_distance = current_distance;
        }
    }
    let mut projected = Vec::with_capacity(polygon.len());
    for point in polygon {
        let Some(point) = clip_point_to_viewport(rect, point) else {
            return Vec::new();
        };
        projected.push(point);
    }
    projected
}

fn clip_plane_distance(point: [f32; 4], plane: usize) -> f32 {
    match plane {
        0 => point[3] + point[0],
        1 => point[3] - point[0],
        2 => point[3] + point[1],
        3 => point[3] - point[1],
        4 => point[2],
        5 => point[3] - point[2],
        _ => f32::NEG_INFINITY,
    }
}

fn interpolate_clip_point(first: [f32; 4], second: [f32; 4], amount: f32) -> [f32; 4] {
    [
        first[0] + (second[0] - first[0]) * amount,
        first[1] + (second[1] - first[1]) * amount,
        first[2] + (second[2] - first[2]) * amount,
        first[3] + (second[3] - first[3]) * amount,
    ]
}

fn clip_point_to_viewport(rect: egui::Rect, point: [f32; 4]) -> Option<egui::Pos2> {
    if point.iter().any(|value| !value.is_finite()) || point[3] <= 0.0 {
        return None;
    }
    let inverse_w = point[3].recip();
    let x = point[0] * inverse_w;
    let y = point[1] * inverse_w;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some(egui::pos2(
        rect.left() + (x + 1.0) * rect.width() * 0.5,
        rect.top() + (1.0 - y) * rect.height() * 0.5,
    ))
}

fn viewport_pixel_position(position: egui::Pos2, rect: egui::Rect, size: (u32, u32)) -> [f32; 2] {
    let x = ((position.x - rect.left()) / rect.width().max(1.0e-6)) * size.0 as f32;
    let y = ((position.y - rect.top()) / rect.height().max(1.0e-6)) * size.1 as f32;
    [x.clamp(0.0, size.0 as f32), y.clamp(0.0, size.1 as f32)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences::Preferences;
    use modeler3d_core::PrimitiveKind;
    use serde_json::Value;

    #[test]
    fn import_picker_starts_in_documents_remembers_folder_and_cancel_preserves_scene() {
        let directory = test_directory("import-picker");
        std::fs::create_dir_all(&directory).expect("directory");
        let path = directory.join("model.stl");
        modeler3d_core::save_mesh(&modeler3d_core::make_box([2., 4., 6.]).expect("box"), &path)
            .expect("fixture");
        let mut app = Modeler3dApp {
            settings_path: directory.join("settings.json"),
            ..Modeler3dApp::default()
        };
        app.open_import_with_picker(|start| {
            assert_eq!(start, modeler3d_platform::documents_directory());
            Some(path.clone())
        });
        assert_eq!(app.commands.scene().objects().len(), 1);
        assert!(app.dialog.is_none());
        let history = app.commands.scene().history_state_id();
        let mut restarted = Modeler3dApp {
            settings_path: app.settings_path.clone(),
            ..Modeler3dApp::default()
        };
        restarted.load_preferences();
        restarted.open_import_with_picker(|start| {
            assert_eq!(start, directory);
            None
        });
        assert!(restarted.commands.scene().objects().is_empty());
        app.open_import_with_picker(|start| {
            assert_eq!(start, directory);
            None
        });
        assert_eq!(app.commands.scene().history_state_id(), history);
        assert_eq!(app.commands.scene().objects().len(), 1);
        let bad = directory.join("unrelated.txt");
        std::fs::write(&bad, "not a model").expect("invalid fixture");
        app.open_import_with_picker(|_| Some(bad));
        assert!(app.dialog.is_some());
        assert_eq!(app.commands.scene().history_state_id(), history);
        std::fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn grab_brush_shift_drag_on_a_mesh_changes_it_in_one_undo_entry() {
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("of8-grab-stroke").join("settings.json"),
            ..Default::default()
        };
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
            .expect("sphere");
        app.refresh_preview();
        app.camera.set_viewport_size(800, 600).expect("size");
        app.frame_camera_if_pending();
        // Choose the brush first, then select the mesh without rearming it.
        app.commands.set_selection_indices(std::iter::empty());
        assert!(command_enabled(CommandId::Grab, &app.commands));
        app.dispatch(CommandId::Grab);
        assert_eq!(app.active_brush, Some(ActiveBrush::Grab));
        app.commands.set_selection_indices([0]);
        assert_eq!(app.active_brush, Some(ActiveBrush::Grab));
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.));
        let center = rect.center();
        let before = app.commands.scene().objects()[0].mesh().vertices.clone();
        let history = app.commands.scene().history_state_id();
        let shift = egui::Modifiers {
            shift: true,
            ..Default::default()
        };
        let frame = |app: &mut Modeler3dApp, position: egui::Pos2, delta: Vec2, phase: u8| {
            let input = ViewportInput {
                pointer_position: Some(position),
                pointer_delta: delta,
                primary_pressed: phase == 0,
                primary_down: phase < 2,
                primary_released: phase == 2,
                secondary_pressed: false,
                secondary_down: false,
                secondary_released: false,
                middle_pressed: false,
                middle_down: false,
                middle_released: false,
                scroll_y: 0.,
                pinch_zoom_factor: 1.,
                escape_pressed: false,
                focused: true,
                modifiers: shift,
            };
            app.route_viewport_input(rect, 1., input, true);
            app.refresh_preview();
        };
        frame(&mut app, center, Vec2::ZERO, 0);
        assert!(
            matches!(app.viewport_drag, Some(ViewportDrag::Brush(_))),
            "Shift-press on the mesh starts a stroke: {}",
            app.commands.last_status()
        );
        for step in 1..=6 {
            let position = center + Vec2::new(0.0, -8.0 * step as f32);
            frame(&mut app, position, Vec2::new(0.0, -8.0), 1);
        }
        frame(&mut app, center + Vec2::new(0.0, -48.0), Vec2::ZERO, 2);
        assert!(app.viewport_drag.is_none());
        let after = &app.commands.scene().objects()[0].mesh().vertices;
        assert_ne!(&before, after, "{}", app.commands.last_status());
        app.commands.dispatch(CommandId::Undo).expect("undo");
        assert_eq!(app.commands.scene().history_state_id(), history);
    }

    #[test]
    fn grab_pulls_the_grabbed_point_with_the_pointer_even_off_the_object() {
        // OF-48: like Blender's Grab, the grabbed surface point follows the
        // pointer across a camera-facing plane, including past the silhouette,
        // instead of sliding along the surface it was picked on.
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("of48-grab-pull").join("settings.json"),
            ..Default::default()
        };
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
            .expect("sphere");
        app.refresh_preview();
        app.camera.set_viewport_size(800, 600).expect("size");
        app.frame_camera_if_pending();
        app.commands.set_selection_indices([0]);
        app.dispatch(CommandId::Grab);
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.));
        let center = rect.center();
        let shift = egui::Modifiers {
            shift: true,
            ..Default::default()
        };
        let frame = |app: &mut Modeler3dApp, position: egui::Pos2, phase: u8| {
            let input = ViewportInput {
                pointer_position: Some(position),
                pointer_delta: if phase == 0 {
                    Vec2::ZERO
                } else {
                    Vec2::new(20.0, 0.0)
                },
                primary_pressed: phase == 0,
                primary_down: phase < 2,
                primary_released: phase == 2,
                secondary_pressed: false,
                secondary_down: false,
                secondary_released: false,
                middle_pressed: false,
                middle_down: false,
                middle_released: false,
                scroll_y: 0.,
                pinch_zoom_factor: 1.,
                escape_pressed: false,
                focused: true,
                modifiers: shift,
            };
            app.route_viewport_input(rect, 1., input, true);
            app.refresh_preview();
        };
        let before = app.commands.scene().objects()[0].mesh().vertices.clone();
        frame(&mut app, center, 0);
        assert!(matches!(app.viewport_drag, Some(ViewportDrag::Brush(_))));
        // Drag far to the right, well past the sphere's outline.
        let release = center + Vec2::new(260.0, 0.0);
        for step in 1..=13 {
            frame(&mut app, center + Vec2::new(20.0 * step as f32, 0.0), 1);
        }
        frame(&mut app, release, 2);
        let object = &app.commands.scene().objects()[0];
        let after = &object.mesh().vertices;
        let (moved, _) = before
            .iter()
            .zip(after)
            .enumerate()
            .map(|(index, (first, second))| {
                let delta = subtract(*second, *first);
                (index, dot(delta, delta))
            })
            .fold(
                (0, 0.0_f32),
                |best, item| if item.1 > best.1 { item } else { best },
            );
        let world = object.transform().transform_point(after[moved]);
        let projection = app.camera.view_projection(1.0).expect("projection");
        let ndc = projection.transform_point(world).expect("ndc");
        let screen_x = (ndc[0] + 1.0) * 0.5 * 800.0;
        let screen_y = (1.0 - ndc[1]) * 0.5 * 600.0;
        assert!(
            (screen_x - release.x).abs() < 30.0 && (screen_y - release.y).abs() < 30.0,
            "the grabbed point lands under the pointer: ({screen_x}, {screen_y}) vs {release:?}"
        );
    }

    #[test]
    fn paint_tab_explains_itself_without_a_selection_and_colors_with_one() {
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("of7-paint").join("settings.json"),
            ..Default::default()
        };
        for command in [CommandId::Material, CommandId::Paint] {
            assert!(!command_enabled(command, &app.commands), "empty scene");
        }
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        app.commands.set_selection_indices(std::iter::empty());
        assert!(command_enabled(CommandId::Material, &app.commands));
        app.dispatch(CommandId::Material);
        assert!(app.dialog.is_none());
        assert_eq!(app.commands.last_status(), "Select an object to paint");
        // Paint (OF-44) needs no selection: it opens the Paint bar.
        assert!(command_enabled(CommandId::Paint, &app.commands));
        app.dispatch(CommandId::Paint);
        assert!(app.dialog.is_none());
        assert!(app.paint_mode.is_some());
        app.dispatch(CommandId::Paint);
        assert!(app.paint_mode.is_none());
        app.commands.set_selection_indices([0]);
        app.dispatch(CommandId::Material);
        assert!(
            app.dialog
                .as_ref()
                .is_some_and(|dialog| dialog.kind() == DialogKind::Material)
        );
        let history = app.commands.scene().history_state_id();
        app.apply_dialog_action(DialogAction::Color([0.9, 0.2, 0.1, 1.0]));
        assert_eq!(
            app.commands.scene().objects()[0].color(),
            [0.9, 0.2, 0.1, 1.0]
        );
        app.commands.dispatch(CommandId::Undo).expect("undo");
        assert_eq!(app.commands.scene().history_state_id(), history);
    }

    fn test_directory(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        std::env::temp_dir().join(format!(
            "modeler3d-shell-{label}-{}-{unique}",
            std::process::id()
        ))
    }

    fn shell_for_jobs(label: &str) -> (Modeler3dApp, PathBuf) {
        let directory = test_directory(label);
        assert!(std::fs::create_dir_all(&directory).is_ok());
        let app = Modeler3dApp {
            settings_path: directory.join("settings.json"),
            ..Modeler3dApp::default()
        };
        (app, directory)
    }

    #[test]
    fn shortcut_bar_starts_with_the_owner_set_and_then_keeps_what_the_person_chose() {
        let (mut app, directory) = shell_for_jobs("shortcut-bar-defaults-kept");
        let legacy = modeler3d_platform::LegacySettings::default();
        let restart = |app: &Modeler3dApp| {
            let mut next = Modeler3dApp {
                settings_path: app.settings_path.clone(),
                ..Modeler3dApp::default()
            };
            next.load_preferences_from_legacy(&legacy);
            next.commands.preferences().ribbon_favorites().to_vec()
        };

        // First start: no saved bar, so the default six.
        app.load_preferences_from_legacy(&legacy);
        let defaults = [
            "Insert:Cube",
            "Insert:Sphere",
            "Object:Settle",
            "Object:Mirror",
            "Edit:Merge",
            "Edit:Subtract",
        ];
        assert_eq!(app.commands.preferences().ribbon_favorites(), defaults);

        // A change is kept across a restart.
        app.set_favorite("Object:Mirror", false);
        app.set_favorite("Insert:Torus", true);
        let changed = app.commands.preferences().ribbon_favorites().to_vec();
        assert!(!changed.iter().any(|key| key == "Object:Mirror"));
        assert_eq!(restart(&app), changed);

        // Changing it again replaces the earlier choice.
        app.set_favorite("Insert:Cube", false);
        let changed_again = app.commands.preferences().ribbon_favorites().to_vec();
        assert_ne!(changed_again, changed);
        assert_eq!(restart(&app), changed_again);

        // An emptied bar stays empty; the defaults do not come back.
        for key in changed_again {
            app.set_favorite(&key, false);
        }
        assert!(restart(&app).is_empty());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn slicer_button_without_a_saved_slicer_opens_the_chooser_and_starts_nothing() {
        let (mut app, directory) = shell_for_jobs("of11-slicer-chooser");
        app.slicer_button_clicked();
        assert!(app.slicer_chooser_open);
        assert!(app.desktop_job.is_none());
        assert!(app.dialog.is_none());
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn slicer_button_with_a_missing_saved_slicer_points_to_settings() {
        let (mut app, directory) = shell_for_jobs("of11-slicer-missing");
        app.commands
            .preferences_mut()
            .set_preferred_slicer(Some(directory.join("gone.exe")), Some("Gone".into()));
        app.slicer_button_clicked();
        assert!(!app.slicer_chooser_open);
        assert!(app.desktop_job.is_none());
        assert!(app.commands.last_status().contains("change it in Settings"));
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn slicer_button_with_a_saved_slicer_sends_straight_to_it_without_a_dialog() {
        let (mut app, directory) = shell_for_jobs("of11-slicer-direct");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        // Not a runnable program, so the launch step fails after staging and
        // the staged files must be discarded.
        let slicer = directory.join("not-a-slicer.exe");
        assert!(std::fs::write(&slicer, b"").is_ok());
        app.commands
            .preferences_mut()
            .set_preferred_slicer(Some(slicer), Some("Test Slicer".into()));
        app.slicer_button_clicked();
        assert!(!app.slicer_chooser_open);
        assert!(app.dialog.is_none());
        let job = app
            .desktop_job
            .as_ref()
            .expect("send-to-slicer job started");
        assert!(job.launch_when_staged);
        assert!(job.resume_dialog.is_none());
        wait_for_desktop_job(&mut app);
        assert!(app.dialog.is_none());
        assert!(
            app.commands
                .last_status()
                .starts_with("Send to slicer failed"),
            "{}",
            app.commands.last_status()
        );
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn shift_box_selects_objects_it_covers_without_history() {
        let (mut app, directory) = shell_for_jobs("of13-box-select");
        for kind in [PrimitiveKind::Cube, PrimitiveKind::Sphere] {
            assert!(app.commands.dispatch(CommandId::AddPrimitive(kind)).is_ok());
        }
        app.commands.deselect_all();
        let history = app.commands.scene().history_state_id();
        app.refresh_preview();
        let bounds = app
            .preview
            .as_ref()
            .expect("preview")
            .objects()
            .iter()
            .filter_map(|object| object.world_bounds().ok())
            .collect::<Vec<_>>();
        assert!(app.camera.set_viewport_size(1280, 840).is_ok());
        assert!(app.camera.frame_bounds(&bounds).is_ok());
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1280.0, 840.0));

        // A box in an empty corner selects nothing.
        let corner = egui::Rect::from_min_size(egui::pos2(2.0, 2.0), Vec2::splat(10.0));
        app.apply_box_selection(viewport, corner);
        assert!(app.commands.scene().selection().is_empty());

        // A box over the whole view selects both objects.
        app.apply_box_selection(viewport, viewport.shrink(1.0));
        assert_eq!(app.commands.scene().selection().len(), 2);
        assert_eq!(app.commands.scene().history_state_id(), history);
        assert!(app.commands.last_status().contains("2 object(s)"));
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn theme_toggle_flips_the_mode_and_saves_it() {
        let (mut app, directory) = shell_for_jobs("of14-theme-toggle");
        app.theme_mode = ThemeMode::Light;
        app.toggle_theme();
        assert_eq!(app.theme_mode, ThemeMode::Dark);
        assert_eq!(app.commands.preferences().theme_mode(), ThemeMode::Dark);
        assert!(directory.join("settings.json").is_file());
        app.toggle_theme();
        assert_eq!(app.theme_mode, ThemeMode::Light);
        assert_eq!(app.commands.preferences().theme_mode(), ThemeMode::Light);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn operation_controls_appear_only_while_work_is_active() {
        fn painted_text(app: &mut Modeler3dApp, ctx: &egui::Context) -> String {
            fn collect(shape: &egui::Shape, text: &mut String) {
                match shape {
                    egui::Shape::Text(shape) => {
                        text.push_str(shape.galley.text());
                        text.push('|');
                    }
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            collect(shape, text);
                        }
                    }
                    _ => {}
                }
            }
            let mut text = String::new();
            for _ in 0..3 {
                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    app.render_operation_progress(ui.ctx());
                });
                output.textures_delta.clear();
                text.clear();
                for clipped in &output.shapes {
                    collect(&clipped.shape, &mut text);
                }
            }
            text
        }
        let (mut app, directory) = shell_for_jobs("operation-controls");
        let ctx = egui::Context::default();
        // Disable window fade animations so terminal-state checks are deterministic.
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            ctx.style_mut_of(theme, |style| style.animation_time = 0.0);
        }
        assert!(painted_text(&mut app, &ctx).is_empty());
        let operation = app
            .commands
            .operations()
            .start("Repair model", "repair", None)
            .expect("start operation");
        let text = painted_text(&mut app, &ctx);
        assert!(text.contains("Repair model"), "{text}");
        assert!(text.contains("Cancel"), "{text}");
        app.commands
            .operations()
            .complete(&operation.operation_id, None)
            .expect("complete operation");
        assert!(painted_text(&mut app, &ctx).is_empty());
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    /// Renders the transform strip once and returns its painted rectangles and
    /// text.
    fn transform_strip_shapes(app: &mut Modeler3dApp) -> (Vec<egui::epaint::RectShape>, String) {
        let context = egui::Context::default();
        let mut rects = Vec::new();
        let mut text = String::new();
        for _ in 0..2 {
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                app.render_transform_strip(&ui.ctx().clone());
            });
            output.textures_delta.clear();
            rects.clear();
            text.clear();
            for clipped in &output.shapes {
                match &clipped.shape {
                    egui::Shape::Rect(rect) => rects.push(rect.clone()),
                    egui::Shape::Text(shape) => {
                        text.push_str(shape.galley.text());
                        text.push('|');
                    }
                    _ => {}
                }
            }
        }
        (rects, text)
    }

    #[test]
    fn transform_strip_has_no_count_and_each_tab_is_centred_over_its_box() {
        let (mut app, directory) = shell_for_jobs("of65-transform-strip");
        app.dispatch(CommandId::AddPrimitive(modeler3d_core::PrimitiveKind::Cube));
        assert_eq!(app.commands.selected_count(), 1);
        for (axis, mode) in [
            TransformPanelMode::Move,
            TransformPanelMode::Rotate,
            TransformPanelMode::Resize,
        ]
        .into_iter()
        .enumerate()
        {
            app.transform_panel_mode = mode;
            let (rects, text) = transform_strip_shapes(&mut app);
            assert!(!text.contains("selected"), "count still shown: {text}");
            // The tab row is the top row; only the chosen tab is filled.
            let top = rects
                .iter()
                .map(|rect| rect.rect.min.y)
                .fold(f32::MAX, f32::min);
            let tabs: Vec<&egui::epaint::RectShape> = rects
                .iter()
                .filter(|rect| (rect.rect.min.y - top).abs() < 0.5)
                .collect();
            assert_eq!(tabs.len(), 3, "{mode:?}: three tabs");
            let filled: Vec<egui::Rect> = tabs
                .iter()
                .filter(|rect| rect.fill != Color32::TRANSPARENT)
                .map(|rect| rect.rect)
                .collect();
            assert_eq!(filled.len(), 1, "{mode:?}: one highlighted tab");
            let tab = filled[0];
            // The axis boxes are the stroked frames below the tab row.
            let mut boxes: Vec<egui::Rect> = rects
                .iter()
                .map(|rect| rect.rect)
                .filter(|rect| {
                    rect.min.y > tab.max.y && (rect.width() - TRANSFORM_FIELD_WIDTH).abs() < 1.0
                })
                .collect();
            boxes.sort_by(|a, b| a.min.x.total_cmp(&b.min.x));
            boxes.dedup_by(|a, b| (a.min.x - b.min.x).abs() < 0.5);
            assert_eq!(boxes.len(), 3, "{mode:?}: {boxes:?}");
            assert!(
                (tab.max.x - boxes[axis].max.x).abs() < 0.5,
                "{mode:?}: tab right {} vs box right {}",
                tab.max.x,
                boxes[axis].max.x
            );
            assert!(
                (tab.min.x - boxes[axis].min.x).abs() < 0.5,
                "{mode:?}: tab left {} vs box left {}",
                tab.min.x,
                boxes[axis].min.x
            );
        }
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    /// Renders one menu row at the top-left with the pointer at `pointer`
    /// and returns the fill colours painted that frame.
    fn menu_row_fills(pointer: egui::Pos2) -> Vec<Color32> {
        let palette = ThemePalette::for_mode(ThemeMode::Light);
        let context = egui::Context::default();
        let mut fills = Vec::new();
        for _ in 0..3 {
            let input = egui::RawInput {
                events: vec![egui::Event::PointerMoved(pointer)],
                ..Default::default()
            };
            let mut output = context.run_ui(input, |ui| {
                ui.set_width(200.0);
                let _ = menu_row(ui, "Export", palette);
            });
            output.textures_delta.clear();
            fills = output
                .shapes
                .iter()
                .filter_map(|clipped| match &clipped.shape {
                    egui::Shape::Rect(rect) => Some(rect.fill),
                    _ => None,
                })
                .collect();
        }
        fills
    }

    #[test]
    fn menu_row_under_the_pointer_is_highlighted() {
        let palette = ThemePalette::for_mode(ThemeMode::Light);
        assert!(menu_row_fills(egui::pos2(40.0, 12.0)).contains(&palette.hover));
        assert!(!menu_row_fills(egui::pos2(40.0, 400.0)).contains(&palette.hover));
    }

    #[test]
    fn active_tab_is_joined_to_the_ribbon_below() {
        let palette = ThemePalette::for_mode(ThemeMode::Light);
        let tab = egui::Rect::from_min_size(egui::pos2(100.0, 4.0), Vec2::new(62.0, 34.0));
        let bar_bottom = 41.0;
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            attach_tab_to_ribbon(ui, tab, bar_bottom, palette);
        });
        output.textures_delta.clear();
        let bridged = output.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Rect(rect) => {
                rect.fill == palette.panel
                    && (rect.rect.left() - tab.left()).abs() < 0.01
                    && (rect.rect.right() - tab.right()).abs() < 0.01
                    && rect.rect.top() < tab.bottom()
                    && (rect.rect.bottom() - bar_bottom).abs() < 0.01
            }
            _ => false,
        });
        assert!(bridged, "the active tab must reach down to the ribbon");
    }

    #[test]
    fn big_size_doubles_the_shell_and_survives_a_restart() {
        let (mut app, directory) = shell_for_jobs("of20-ui-size");
        app.open_dialog(DialogKind::Settings);
        app.apply_dialog_action(DialogAction::UiSize(UiSize::Big));
        assert_eq!(app.ui_size, UiSize::Big);
        assert_eq!(app.commands.preferences().ui_size(), UiSize::Big);
        assert!(
            app.dialog.is_some(),
            "Size, like Theme, keeps Settings open"
        );

        let context = egui::Context::default();
        for _ in 0..2 {
            let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                theme::apply_size(ui.ctx(), app.ui_size);
            });
            output.textures_delta.clear();
        }
        assert!((context.zoom_factor() - 2.0).abs() < f32::EPSILON);

        let settings = directory.join("settings.json");
        assert!(app.commands.preferences().save_to_path(&settings).is_ok());
        let reloaded =
            crate::preferences::Preferences::load_from_path(&settings).expect("settings reload");
        assert_eq!(reloaded.ui_size(), UiSize::Big);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn open_settings_shows_the_theme_picked_with_the_top_bar_toggle() {
        let (mut app, directory) = shell_for_jobs("of16-theme-sync");
        app.theme_mode = ThemeMode::Light;
        app.surface = WorkspaceSurface::Scene;
        app.open_dialog(DialogKind::Settings);
        app.toggle_theme();
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            app.render_dialog(ui.ctx());
        });
        output.textures_delta.clear();
        assert!(app.dialog.as_ref().is_some_and(|dialog| {
            dialog.kind() == DialogKind::Settings && dialog.theme_mode() == ThemeMode::Dark
        }));
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn repair_of_a_clean_model_says_so_and_changes_nothing() {
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("of40-clean-repair").join("settings.json"),
            ..Default::default()
        };
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        let history = app.commands.scene().history_state_id();
        let revision = app.commands.scene().revision();
        app.open_dialog(DialogKind::Repair);
        app.apply_dialog_action(DialogAction::Repair);
        wait_for_desktop_job(&mut app);
        assert_eq!(
            app.commands.scene().history_state_id(),
            history,
            "no undo entry"
        );
        assert_eq!(app.commands.scene().revision(), revision, "scene untouched");
        assert_eq!(
            app.commands.last_status(),
            modeler3d_core::REPAIR_NOTHING_TO_FIX_MESSAGE
        );
        assert!(app.dialog.as_ref().is_some_and(|dialog| {
            dialog.kind() == DialogKind::Repair
                && dialog.message() == modeler3d_core::REPAIR_NOTHING_TO_FIX_MESSAGE
                && dialog.dismiss_label() == "Close"
        }));

        // A model with a hole is still repaired, in one undo step.
        let mut open = app.commands.scene().objects()[0].mesh().clone();
        open.faces.pop();
        let object_id = app.commands.scene().objects()[0].object_id().to_owned();
        let generation = app.commands.document_generation();
        let revision = app.commands.scene().revision();
        app.commands
            .replace_mesh_if_current(generation, revision, &object_id, open)
            .expect("open the cube");
        let history = app.commands.scene().history_state_id();
        app.apply_dialog_action(DialogAction::Repair);
        wait_for_desktop_job(&mut app);
        assert_eq!(app.commands.scene().history_state_id(), history + 1);
        assert_ne!(
            app.commands.last_status(),
            modeler3d_core::REPAIR_NOTHING_TO_FIX_MESSAGE
        );
    }

    fn wait_for_desktop_job(app: &mut Modeler3dApp) {
        let context = egui::Context::default();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while app.desktop_job.is_some() && std::time::Instant::now() < deadline {
            app.poll_desktop_job(&context);
            if app.desktop_job.is_some() {
                thread::sleep(Duration::from_millis(5));
            }
        }
        assert!(
            app.desktop_job.is_none(),
            "desktop operation did not finish"
        );
    }

    fn fixture(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../modeler3d-core/tests/fixtures")
            .join(relative)
    }

    fn repository_test_directory(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/rb03-shell-overwrite-tests")
            .join(format!("{label}-{}-{unique}", std::process::id()));
        assert!(std::fs::create_dir_all(&path).is_ok());
        path
    }

    #[test]
    fn recent_grid_allocates_a_visible_width_for_every_column() {
        let available_width = 1280.0;
        let item_spacing = 8.0;
        let (columns, column_width) = recent_grid_layout(available_width, item_spacing);

        assert_eq!(columns, 4);
        assert_eq!(column_width, 314.0);
        assert!(column_width >= 280.0);
        assert!(
            column_width * columns as f32 + item_spacing * (columns - 1) as f32 <= available_width
        );
    }

    fn make_dirty(shell: &mut Modeler3dApp) {
        assert!(matches!(
            shell
                .commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube)),
            Ok(CommandOutcome::Applied(_))
        ));
    }

    #[test]
    fn default_shell_starts_on_a_safe_welcome_surface() {
        let shell = Modeler3dApp::default();
        assert_eq!(shell.active_tab, RibbonTab::Insert);
        assert_eq!(shell.surface, WorkspaceSurface::Welcome);
        assert!(!shell.ribbon_pinned);
        assert!(!shell.commands.preferences().ribbon_pinned());
        assert!(shell.commands.display_state().grid_visible);
    }

    #[test]
    fn mcp_scene_edit_synchronizes_the_welcome_surface() {
        let mut shell = Modeler3dApp::default();
        let project_id = shell.commands.scene().project_id().to_owned();
        let revision = shell.commands.scene().revision();
        make_dirty(&mut shell);

        shell.synchronize_mcp_presentation(&project_id, revision, false);

        assert_eq!(shell.surface, WorkspaceSurface::Scene);
    }

    #[test]
    fn mcp_project_replacement_enters_the_scene_surface_when_empty() {
        let (mut shell, directory) = shell_for_jobs("rb09-mcp-project-replacement");
        let project_id = shell.commands.scene().project_id().to_owned();
        let revision = shell.commands.scene().revision();

        shell.synchronize_mcp_presentation(&project_id, revision, true);

        assert_eq!(shell.surface, WorkspaceSurface::Scene);
        assert!(shell.commands.scene().objects().is_empty());
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn clear_recent_files_keeps_sources_and_invalidates_pending_preview_results() {
        let (mut app, directory) = shell_for_jobs("rb08-clear-recents");
        let source = directory.join("kept-model.stl");
        std::fs::write(&source, b"the source file remains").expect("source fixture");
        app.commands
            .preferences_mut()
            .recent_files_mut()
            .record(source.clone());
        app.recent_preview_paths = vec![source.clone()];
        app.recent_preview_pending.insert(source.clone());
        app.recent_preview_generation = 4;
        app.recent_preview_generation_signal
            .store(4, Ordering::Relaxed);
        app.open_dialog(DialogKind::Settings);

        app.apply_dialog_action(DialogAction::ClearRecentFiles);

        assert!(app.commands.preferences().recent_files().paths().is_empty());
        assert!(
            source.is_file(),
            "clearing recents must not delete source files"
        );
        assert!(app.recent_preview_paths.is_empty());
        assert!(app.recent_preview_pending.is_empty());
        assert_eq!(app.recent_preview_generation, 5);
        assert_eq!(
            app.recent_preview_generation_signal.load(Ordering::Relaxed),
            5
        );
        assert!(app.dialog.as_ref().is_some_and(|dialog| {
            dialog.kind() == DialogKind::Settings
                && dialog.message().contains("source files were not changed")
        }));
        let saved: Value =
            serde_json::from_slice(&std::fs::read(&app.settings_path).expect("settings saved"))
                .expect("settings JSON");
        assert_eq!(saved["recent_files"]["paths"], serde_json::json!([]));
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn stale_recent_preview_result_cannot_replace_the_current_fallback() {
        let (mut app, directory) = shell_for_jobs("rb08-stale-preview");
        let path = fixture("formats/python_box.stl");
        let stale_image = crate::recent_preview::load_recent_preview(&path)
            .expect("valid preview for the stale worker result");
        app.commands
            .preferences_mut()
            .recent_files_mut()
            .record(path.clone());
        app.recent_preview_paths = vec![path.clone()];
        app.recent_preview_pending.insert(path.clone());
        app.recent_preview_generation = 10;
        app.recent_preview_generation_signal
            .store(10, Ordering::Relaxed);
        app.recent_preview_result_sender
            .as_ref()
            .expect("test result sender")
            .send(RecentPreviewResult {
                generation: 9,
                path: path.clone(),
                image: Some(stale_image),
            })
            .expect("queue stale result");
        let context = egui::Context::default();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            app.update_recent_previews(ui.ctx());
        });
        output.textures_delta.clear();

        assert!(!app.recent_preview_textures.contains_key(&path));
        assert!(app.recent_preview_pending.contains(&path));
        assert_eq!(app.recent_preview_generation, 10);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn file_shortcuts_are_available_in_text_fields_and_dirty_guards_still_apply() {
        fn send_key(
            app: &mut Modeler3dApp,
            ctx: &egui::Context,
            text: &mut String,
            key: Key,
            shift: bool,
        ) {
            let modifiers = egui::Modifiers {
                ctrl: true,
                command: true,
                shift,
                ..Default::default()
            };
            let input = egui::RawInput {
                events: vec![
                    egui::Event::ModifiersChanged(modifiers),
                    egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed: true,
                        repeat: false,
                        modifiers,
                    },
                ],
                ..Default::default()
            };
            let mut output = ctx.run_ui(input, |ui| {
                app.handle_shortcuts(ui.ctx());
                ui.text_edit_singleline(text).request_focus();
            });
            output.textures_delta.clear();
        }
        let (mut app, directory) = shell_for_jobs("rb08-file-shortcuts");
        let project_path = directory.join("saved.3dp");
        app.commands
            .save_to_path(&project_path)
            .expect("seed a clean saved project");
        let ctx = egui::Context::default();
        let mut text = String::from("focused field");
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.text_edit_singleline(&mut text).request_focus();
        });
        output.textures_delta.clear();
        send_key(&mut app, &ctx, &mut text, Key::S, false);
        assert!(
            app.dialog.is_none(),
            "Ctrl+S saves the active project from a text field"
        );
        assert!(app.commands.last_status().contains("Saved"));

        send_key(&mut app, &ctx, &mut text, Key::S, true);
        assert!(
            app.dialog
                .as_ref()
                .is_some_and(|dialog| dialog.kind() == DialogKind::SaveProject)
        );

        app.dialog = None;
        send_key(&mut app, &ctx, &mut text, Key::O, false);
        assert!(
            app.dialog
                .as_ref()
                .is_some_and(|dialog| dialog.kind() == DialogKind::OpenProject)
        );

        app.dialog = None;
        make_dirty(&mut app);
        send_key(&mut app, &ctx, &mut text, Key::N, false);
        assert!(
            app.confirm_action.is_some(),
            "Ctrl+N must retain the dirty-scene guard"
        );
        assert_eq!(app.commands.scene().objects().len(), 1);
        app.confirm_action = None;
        send_key(&mut app, &ctx, &mut text, Key::Q, false);
        assert!(
            app.confirm_action.is_some(),
            "Ctrl+Q must retain the dirty-exit guard"
        );
        assert!(!app.close_requested);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn material_opens_the_color_dialog_and_there_is_no_color_button() {
        let (mut app, directory) = shell_for_jobs("appearance-routes");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        app.dispatch(CommandId::Material);
        assert!(
            app.dialog
                .as_ref()
                .is_some_and(|dialog| dialog.kind() == DialogKind::Material)
        );
        app.apply_dialog_action(DialogAction::Close);
        // The owner removed the Color button (OF-45): the Paint tab has
        // Material and Paint only.
        let paint_labels = crate::commands::ribbon_spec()
            .into_iter()
            .filter(|(tab, _)| *tab == RibbonTab::Paint)
            .flat_map(|(_, items)| items)
            .filter_map(|item| match item {
                RibbonItem::Command(command) => Some(command.label),
                RibbonItem::Separator => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(paint_labels, ["Material", "Paint"]);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn image_sampling_failure_keeps_dialog_and_scene_ready_for_retry() {
        let (mut app, directory) = shell_for_jobs("image-sample-retry");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        let original_color = app.commands.scene().objects()[0].color();
        let revision_before = app.commands.scene().revision();
        let history_before = app.commands.scene().history_state_id();
        app.open_dialog(DialogKind::Material);
        let missing = directory.join("missing.png");

        app.apply_dialog_action(DialogAction::ColorFromImage(missing));

        assert!(app.dialog.as_ref().is_some_and(DialogState::has_error));
        assert_eq!(app.commands.scene().objects()[0].color(), original_color);
        assert_eq!(app.commands.scene().revision(), revision_before);
        assert_eq!(app.commands.scene().history_state_id(), history_before);

        let image = directory.join("retry.ppm");
        assert!(std::fs::write(&image, b"P3\n1 1\n255\n0 255 0\n").is_ok());
        app.apply_dialog_action(DialogAction::ColorFromImage(image));
        assert!(app.dialog.is_none());
        assert_eq!(
            app.commands.scene().objects()[0].color(),
            [0.0, 1.0, 0.0, 1.0]
        );
        assert!(
            app.commands
                .last_status()
                .contains("no texture was created")
        );
        assert_ne!(app.commands.scene().history_state_id(), history_before);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn shell_mesh_job_runs_off_thread_and_commits_one_undoable_result() {
        let (mut app, directory) = shell_for_jobs("mesh-job-success");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        let original = app.commands.scene().objects()[0].mesh().clone();
        let history_before = app.commands.scene().history_state_id();
        let revision_before = app.commands.scene().revision();

        app.dispatch(CommandId::Smooth);

        let job_id = app
            .desktop_job
            .as_ref()
            .map(|job| job.handle.operation_id.clone())
            .expect("worker was scheduled");
        assert!(
            app.commands
                .operations()
                .active_status()
                .is_ok_and(|status| {
                    status.is_some_and(|status| status.phase == OperationPhase::Running)
                })
        );
        assert_eq!(app.commands.scene().history_state_id(), history_before);
        wait_for_desktop_job(&mut app);

        assert_eq!(
            app.commands
                .operations()
                .status(&job_id)
                .map(|status| status.phase),
            Ok(OperationPhase::Completed)
        );
        assert!(app.commands.scene().revision() > revision_before);
        assert_ne!(app.commands.scene().objects()[0].mesh(), &original);
        assert_ne!(app.commands.scene().history_state_id(), history_before);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn duplicate_job_request_is_rejected_with_a_visible_busy_status() {
        let (mut app, directory) = shell_for_jobs("mesh-job-busy");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        app.dispatch(CommandId::Smooth);
        let job_id = app
            .desktop_job
            .as_ref()
            .map(|job| job.handle.operation_id.clone())
            .expect("first worker was scheduled");

        app.start_mesh_job(MeshJobOperation::Subdivide, false);

        assert!(
            app.commands
                .last_status()
                .contains("another operation is still running")
        );
        assert_eq!(
            app.commands
                .operations()
                .active_status()
                .map(|status| { status.map(|status| status.operation_id) }),
            Ok(Some(job_id.clone()))
        );
        wait_for_desktop_job(&mut app);
        assert_eq!(
            app.commands
                .operations()
                .status(&job_id)
                .map(|status| status.phase),
            Ok(OperationPhase::Completed)
        );
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn shell_mesh_job_keeps_its_target_when_selection_changes() {
        let (mut app, directory) = shell_for_jobs("mesh-job-selection");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .is_ok()
        );
        app.commands.set_selection_indices([0]);
        let first_before = app.commands.scene().objects()[0].mesh().clone();
        let second_before = app.commands.scene().objects()[1].mesh().clone();
        app.dispatch(CommandId::Smooth);
        app.commands.set_selection_indices([1]);
        wait_for_desktop_job(&mut app);

        assert_ne!(app.commands.scene().objects()[0].mesh(), &first_before);
        assert_eq!(app.commands.scene().objects()[1].mesh(), &second_before);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn cancelled_shell_mesh_job_does_not_commit_and_a_retry_succeeds() {
        let (mut app, directory) = shell_for_jobs("mesh-job-cancel");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        let mesh_before = app.commands.scene().objects()[0].mesh().clone();
        let revision_before = app.commands.scene().revision();
        let history_before = app.commands.scene().history_state_id();

        app.start_mesh_job(MeshJobOperation::Subdivide, false);

        let first_id = app
            .desktop_job
            .as_ref()
            .map(|job| job.handle.operation_id.clone())
            .expect("worker was scheduled");
        assert!(app.commands.operations().cancel(&first_id).is_ok());
        wait_for_desktop_job(&mut app);
        assert_eq!(
            app.commands
                .operations()
                .status(&first_id)
                .map(|status| status.phase),
            Ok(OperationPhase::Cancelled)
        );
        assert_eq!(app.commands.scene().objects()[0].mesh(), &mesh_before);
        assert_eq!(app.commands.scene().revision(), revision_before);
        assert_eq!(app.commands.scene().history_state_id(), history_before);

        app.dispatch(CommandId::Smooth);
        let retry_id = app
            .desktop_job
            .as_ref()
            .map(|job| job.handle.operation_id.clone())
            .expect("retry was scheduled");
        wait_for_desktop_job(&mut app);
        assert_eq!(
            app.commands
                .operations()
                .status(&retry_id)
                .map(|status| status.phase),
            Ok(OperationPhase::Completed)
        );
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn changed_scene_rejects_worker_result_without_overwriting_new_history() {
        let (mut app, directory) = shell_for_jobs("mesh-job-stale");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        let first_mesh = app.commands.scene().objects()[0].mesh().clone();
        app.start_mesh_job(MeshJobOperation::Subdivide, false);
        let job_id = app
            .desktop_job
            .as_ref()
            .map(|job| job.handle.operation_id.clone())
            .expect("worker was scheduled");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .is_ok()
        );
        let revision_after_edit = app.commands.scene().revision();
        let history_after_edit = app.commands.scene().history_state_id();
        wait_for_desktop_job(&mut app);

        assert_eq!(
            app.commands
                .operations()
                .status(&job_id)
                .map(|status| status.phase),
            Ok(OperationPhase::Failed)
        );
        assert_eq!(app.commands.scene().objects()[0].mesh(), &first_mesh);
        assert_eq!(app.commands.scene().revision(), revision_after_edit);
        assert_eq!(app.commands.scene().history_state_id(), history_after_edit);
        assert!(app.commands.last_status().contains("result was discarded"));
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn print_check_failure_is_visible_retryable_and_scene_preserving() {
        let (mut app, directory) = shell_for_jobs("print-prep-job");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        app.open_dialog(DialogKind::PrintPrep);
        let revision_before = app.commands.scene().revision();
        let history_before = app.commands.scene().history_state_id();

        app.start_print_prep_job(0.0);
        let failed_id = app
            .desktop_job
            .as_ref()
            .map(|job| job.handle.operation_id.clone())
            .expect("print check was scheduled");
        wait_for_desktop_job(&mut app);

        assert_eq!(
            app.commands
                .operations()
                .status(&failed_id)
                .map(|status| status.phase),
            Ok(OperationPhase::Failed)
        );
        assert!(app.dialog.as_ref().is_some_and(DialogState::has_error));
        assert!(
            app.dialog
                .as_ref()
                .is_some_and(|dialog| dialog.message().contains("positive finite"))
        );
        assert_eq!(app.commands.scene().revision(), revision_before);
        assert_eq!(app.commands.scene().history_state_id(), history_before);

        app.start_print_prep_job(200.0);
        let retry_id = app
            .desktop_job
            .as_ref()
            .map(|job| job.handle.operation_id.clone())
            .expect("print check retry was scheduled");
        wait_for_desktop_job(&mut app);
        assert_eq!(
            app.commands
                .operations()
                .status(&retry_id)
                .map(|status| status.phase),
            Ok(OperationPhase::Completed)
        );
        assert!(app.dialog.as_ref().is_some_and(|dialog| {
            dialog.kind() == DialogKind::PrintPrep
                && dialog.message().contains("Print prep (200.0 mm bed)")
                && !dialog.has_error()
        }));
        assert_eq!(app.commands.scene().revision(), revision_before);
        assert_eq!(app.commands.scene().history_state_id(), history_before);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn background_print_staging_reopens_output_and_dialog_close_cleans_it() {
        let (mut app, directory) = shell_for_jobs("print-stage-worker");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        let Some(slicer) = crate::print_handoff::discover_print_slicers()
            .into_iter()
            .next()
        else {
            assert!(std::fs::remove_dir_all(directory).is_ok());
            return;
        };
        let temp_root = directory.join("staging");
        assert!(std::fs::create_dir_all(&temp_root).is_ok());
        app.open_dialog(DialogKind::PrintPrep);
        app.start_desktop_job(
            "Prepare slicer handoff",
            DesktopJobRequest::PreparePrint {
                snapshot: app.commands.scene().snapshot(),
                slicer: slicer.clone(),
                temp_root: temp_root.clone(),
                document_generation: app.commands.document_generation(),
            },
            None,
            true,
        );
        let job_id = app
            .desktop_job
            .as_ref()
            .map(|job| job.handle.operation_id.clone())
            .expect("print staging worker was scheduled");
        wait_for_desktop_job(&mut app);

        assert_eq!(
            app.commands
                .operations()
                .status(&job_id)
                .map(|status| status.phase),
            Ok(OperationPhase::Completed)
        );
        let prepared = app
            .dialog
            .as_ref()
            .and_then(DialogState::prepared_print)
            .expect("prepared handoff is shown in the native dialog");
        assert_eq!(prepared.source_revision, app.commands.scene().revision());
        assert_eq!(
            prepared.source_document_generation,
            Some(app.commands.document_generation())
        );
        assert_eq!(
            prepared.format,
            if slicer.stl_fallback() { "stl" } else { "3mf" }
        );
        assert!(!prepared.paths.is_empty());
        assert!(
            prepared
                .paths
                .iter()
                .all(|path| { std::fs::metadata(path).is_ok_and(|metadata| metadata.len() > 0) })
        );
        app.apply_dialog_action(DialogAction::Close);
        assert!(std::fs::read_dir(&temp_root).is_ok_and(|entries| entries.count() == 0));
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn stale_background_print_staging_removes_its_partial_handoff() {
        let (mut app, directory) = shell_for_jobs("print-stage-stale");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        let Some(slicer) = crate::print_handoff::discover_print_slicers()
            .into_iter()
            .next()
        else {
            assert!(std::fs::remove_dir_all(directory).is_ok());
            return;
        };
        let temp_root = directory.join("staging");
        assert!(std::fs::create_dir_all(&temp_root).is_ok());
        app.open_dialog(DialogKind::PrintPrep);
        app.start_desktop_job(
            "Prepare slicer handoff",
            DesktopJobRequest::PreparePrint {
                snapshot: app.commands.scene().snapshot(),
                slicer,
                temp_root: temp_root.clone(),
                document_generation: app.commands.document_generation(),
            },
            None,
            true,
        );
        let job_id = app
            .desktop_job
            .as_ref()
            .map(|job| job.handle.operation_id.clone())
            .expect("print staging worker was scheduled");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .is_ok()
        );
        wait_for_desktop_job(&mut app);

        assert_eq!(
            app.commands
                .operations()
                .status(&job_id)
                .map(|status| status.phase),
            Ok(OperationPhase::Failed)
        );
        assert!(std::fs::read_dir(&temp_root).is_ok_and(|entries| entries.count() == 0));
        assert!(
            app.dialog.as_ref().is_none_or(|dialog| {
                dialog.kind() != DialogKind::PrintPrep || dialog.has_error()
            })
        );
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn simplify_window_removes_a_notch_of_faces_and_stays_open() {
        // OF-51: Simplify opens a window with a 10 % slider; applying it
        // runs the same core function as MCP `simplify` with `percent`.
        let (mut app, directory) = shell_for_jobs("of51-simplify");
        app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere));
        let source = app.commands.scene().objects()[0].mesh().clone();
        let history = app.commands.scene().history_state_id();
        app.dispatch(CommandId::Simplify);
        assert_eq!(
            app.dialog.as_ref().map(DialogState::kind),
            Some(DialogKind::Simplify)
        );
        app.apply_dialog_action(crate::dialogs::DialogAction::Simplify(10));
        wait_for_desktop_job(&mut app);
        let expected = modeler3d_core::simplify_mesh_by_percent(
            &source,
            10,
            modeler3d_core::MeshOperationLimits::default(),
        )
        .expect("core simplify");
        assert_eq!(app.commands.scene().objects()[0].mesh(), &expected.mesh);
        assert_eq!(app.commands.scene().history_state_id(), history + 1);
        let message = app
            .dialog
            .as_ref()
            .map(|dialog| dialog.message().to_owned());
        assert_eq!(
            message.as_deref(),
            Some(
                format!(
                    "Simplify: {} \u{2192} {} faces",
                    crate::desktop_jobs::group_thousands(source.face_count()),
                    crate::desktop_jobs::group_thousands(expected.mesh.face_count())
                )
                .as_str()
            ),
            "the window stays open and shows before \u{2192} after"
        );
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn subdivide_window_cuts_the_whole_object_into_four_times_the_faces() {
        // OF-52: Whole object is an even 1-to-4 split, one Undo entry.
        let (mut app, directory) = shell_for_jobs("of52-subdivide-whole");
        app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere));
        let faces = app.commands.scene().objects()[0].mesh().face_count();
        let history = app.commands.scene().history_state_id();
        app.dispatch(CommandId::Subdivide);
        assert_eq!(
            app.dialog.as_ref().map(DialogState::kind),
            Some(DialogKind::Subdivide)
        );
        app.apply_dialog_action(crate::dialogs::DialogAction::SubdivideWhole);
        wait_for_desktop_job(&mut app);
        assert_eq!(
            app.commands.scene().objects()[0].mesh().face_count(),
            faces * 4
        );
        assert_eq!(app.commands.scene().history_state_id(), history + 1);
        assert!(
            app.dialog
                .as_ref()
                .is_some_and(|dialog| dialog.message().starts_with("Subdivide: ")),
            "the window stays open with the result"
        );
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn subdivide_area_brush_refines_only_where_the_stroke_went() {
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("of52-subdivide-area").join("settings.json"),
            ..Default::default()
        };
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
            .expect("sphere");
        app.refresh_preview();
        app.camera.set_viewport_size(800, 600).expect("size");
        app.frame_camera_if_pending();
        app.commands.set_selection_indices([0]);
        app.dispatch(CommandId::Subdivide);
        app.apply_dialog_action(crate::dialogs::DialogAction::SubdivideArea(Some(4.0)));
        assert_eq!(app.active_brush, Some(ActiveBrush::SubdivideArea));
        let faces = app.commands.scene().objects()[0].mesh().face_count();
        let history = app.commands.scene().history_state_id();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.));
        let center = rect.center();
        let shift = egui::Modifiers {
            shift: true,
            ..Default::default()
        };
        let frame = |app: &mut Modeler3dApp, position: egui::Pos2, phase: u8| {
            let input = ViewportInput {
                pointer_position: Some(position),
                pointer_delta: if phase == 0 {
                    Vec2::ZERO
                } else {
                    Vec2::new(6.0, 0.0)
                },
                primary_pressed: phase == 0,
                primary_down: phase < 2,
                primary_released: phase == 2,
                secondary_pressed: false,
                secondary_down: false,
                secondary_released: false,
                middle_pressed: false,
                middle_down: false,
                middle_released: false,
                scroll_y: 0.,
                pinch_zoom_factor: 1.,
                escape_pressed: false,
                focused: true,
                modifiers: shift,
            };
            app.route_viewport_input(rect, 1., input, true);
            app.refresh_preview();
        };
        frame(&mut app, center, 0);
        for step in 1..=5 {
            frame(&mut app, center + Vec2::new(6.0 * step as f32, 0.0), 1);
        }
        frame(&mut app, center + Vec2::new(30.0, 0.0), 2);
        let after = app.commands.scene().objects()[0].mesh().face_count();
        assert!(after > faces, "{}", app.commands.last_status());
        assert!(
            after < faces * 2,
            "only the brushed area: {faces} -> {after}"
        );
        assert_eq!(app.commands.scene().history_state_id(), history + 1);
        // Closing the window turns the area brush off.
        app.apply_dialog_action(crate::dialogs::DialogAction::Close);
        assert_eq!(app.active_brush, None);
    }

    #[test]
    fn simplify_on_a_cube_says_it_cannot_and_adds_no_history() {
        let (mut app, directory) = shell_for_jobs("of51-simplify-cube");
        app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
        let history = app.commands.scene().history_state_id();
        app.dispatch(CommandId::Simplify);
        app.apply_dialog_action(crate::dialogs::DialogAction::Simplify(10));
        wait_for_desktop_job(&mut app);
        assert_eq!(app.commands.scene().history_state_id(), history);
        assert!(
            app.dialog
                .as_ref()
                .is_some_and(|dialog| dialog.message().contains("can't lose any faces")),
            "{:?}",
            app.dialog.as_ref().map(DialogState::message)
        );
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn reopening_dialog_during_a_job_keeps_the_new_dialog_in_place() {
        let (mut app, directory) = shell_for_jobs("job-dialog-reopen");
        assert!(
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        app.open_dialog(DialogKind::Repair);
        app.start_mesh_job(MeshJobOperation::Repair, true);
        let job_id = app
            .desktop_job
            .as_ref()
            .map(|job| job.handle.operation_id.clone())
            .expect("repair was scheduled");
        app.open_dialog(DialogKind::Settings);
        app.dialog
            .as_mut()
            .expect("new dialog")
            .set_message("new dialog stays open");

        wait_for_desktop_job(&mut app);

        assert_eq!(
            app.commands
                .operations()
                .status(&job_id)
                .map(|status| status.phase),
            Ok(OperationPhase::Completed)
        );
        assert!(app.dialog.as_ref().is_some_and(|dialog| {
            dialog.kind() == DialogKind::Settings && dialog.message() == "new dialog stays open"
        }));
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn recent_project_and_model_dispatch_persist_across_app_restart() {
        let directory = test_directory("recent-dispatch");
        assert!(std::fs::create_dir_all(&directory).is_ok());
        let settings = directory.join("settings.json");
        let project = fixture("projects/python-current.3dp");
        let model = fixture("formats/python_box.stl");
        let mut shell = Modeler3dApp::default();
        shell.settings_path.clone_from(&settings);
        shell.open_recent(&project);

        assert_eq!(shell.commands.project_path(), Some(project.as_path()));
        assert!(!shell.commands.is_dirty());
        let project_object_count = shell.commands.scene().objects().len();
        assert!(project_object_count > 0);
        assert_eq!(
            shell.commands.preferences().recent_files().paths().first(),
            Some(&project)
        );

        shell.open_recent(&model);
        assert_eq!(
            shell.commands.scene().objects().len(),
            project_object_count + 1
        );
        assert_eq!(shell.commands.project_path(), Some(project.as_path()));
        assert!(shell.commands.is_dirty());
        assert_eq!(shell.commands.selected_count(), 1);
        assert_eq!(
            shell.commands.preferences().recent_files().paths()[..2],
            [model.clone(), project.clone()]
        );

        let mut restarted = Modeler3dApp::default();
        restarted.settings_path.clone_from(&settings);
        restarted.load_preferences();
        assert_eq!(
            restarted.commands.preferences().recent_files().paths()[..2],
            [model, project]
        );
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn failed_recent_project_preserves_state_and_wrl_imports_additively() {
        let directory = test_directory("recent-failures");
        assert!(std::fs::create_dir_all(&directory).is_ok());
        let prior_project = directory.join("prior.3dp");
        let corrupt_project = directory.join("corrupt.3dp");
        assert!(std::fs::write(&corrupt_project, b"not a project archive").is_ok());
        let mut shell = Modeler3dApp::default();
        make_dirty(&mut shell);
        assert!(shell.commands.save_to_path(&prior_project).is_ok());
        shell
            .commands
            .preferences_mut()
            .recent_files_mut()
            .record(corrupt_project.clone());
        let scene_before = shell.commands.scene().objects().to_vec();
        let history_before = shell.commands.scene().history_state_id();
        let recent_before = shell.commands.preferences().recent_files().paths().to_vec();

        shell.open_recent(&corrupt_project);
        assert_eq!(shell.commands.scene().objects().len(), scene_before.len());
        assert_eq!(
            shell.commands.scene().objects()[0].object_id(),
            scene_before[0].object_id()
        );
        assert_eq!(
            shell.commands.scene().objects()[0].mesh().vertices,
            scene_before[0].mesh().vertices
        );
        assert_eq!(shell.commands.scene().history_state_id(), history_before);
        assert_eq!(shell.commands.project_path(), Some(prior_project.as_path()));
        assert!(!shell.commands.is_dirty());
        assert_eq!(
            shell.commands.preferences().recent_files().paths(),
            recent_before.as_slice()
        );
        assert!(!shell.commands.last_status().is_empty());

        let wrl = fixture("wrl/python_reference.wrl");
        assert_eq!(recent_entry(&wrl).kind, RecentEntryKind::Supported);
        shell.open_recent(&wrl);
        assert_eq!(
            shell.commands.scene().objects().len(),
            scene_before.len() + 2
        );
        assert_eq!(
            shell.commands.scene().objects()[0].object_id(),
            scene_before[0].object_id()
        );
        assert_eq!(shell.commands.project_path(), Some(prior_project.as_path()));
        assert!(shell.commands.is_dirty());
        assert_eq!(
            shell.commands.preferences().recent_files().paths().first(),
            Some(&wrl)
        );
        shell
            .commands
            .dispatch(CommandId::Undo)
            .expect("undo WRL import");
        assert_eq!(shell.commands.scene().objects().len(), scene_before.len());
        assert_eq!(
            shell.commands.scene().objects()[0].mesh().vertices,
            scene_before[0].mesh().vertices
        );
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn opening_a_missing_recent_removes_and_persists_it_without_scene_changes() {
        let directory = test_directory("recent-missing");
        assert!(std::fs::create_dir_all(&directory).is_ok());
        let settings = directory.join("settings.json");
        let missing = directory.join("gone.stl");
        let mut shell = Modeler3dApp::default();
        shell.settings_path.clone_from(&settings);
        make_dirty(&mut shell);
        let history_before = shell.commands.scene().history_state_id();
        shell
            .commands
            .preferences_mut()
            .recent_files_mut()
            .record(missing.clone());

        shell.open_recent(&missing);
        assert_eq!(shell.commands.scene().objects().len(), 1);
        assert_eq!(shell.commands.scene().history_state_id(), history_before);
        assert!(shell.commands.is_dirty());
        assert!(
            shell
                .commands
                .preferences()
                .recent_files()
                .paths()
                .is_empty()
        );
        assert!(shell.commands.last_status().contains("File not found"));
        let persisted = Preferences::load_from_path(&settings).expect("saved recent list");
        assert!(persisted.recent_files().paths().is_empty());
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn dirty_new_guard_covers_cancel_discard_and_save_as() {
        let directory = test_directory("new-guard");
        let save_path = directory.join("saved-before-new");
        let mut shell = Modeler3dApp::default();
        make_dirty(&mut shell);
        let original_history = shell.commands.scene().history_state_id();
        let original_revision = shell.commands.scene().revision();

        shell.confirm_action = Some(PendingReplacement::NewScene);
        shell.finish_confirmation(ConfirmationChoice::Cancel);
        assert_eq!(shell.commands.scene().objects().len(), 1);
        assert_eq!(shell.commands.scene().history_state_id(), original_history);
        assert_eq!(shell.commands.scene().revision(), original_revision);
        assert!(shell.commands.is_dirty());

        shell.confirm_action = Some(PendingReplacement::NewScene);
        shell.finish_confirmation(ConfirmationChoice::Discard);
        assert!(shell.commands.scene().objects().is_empty());
        assert!(shell.commands.project_path().is_none());
        assert!(!shell.commands.is_dirty());

        make_dirty(&mut shell);
        let dirty_history = shell.commands.scene().history_state_id();
        shell.confirm_action = Some(PendingReplacement::NewScene);
        shell.finish_confirmation(ConfirmationChoice::Save);
        assert!(shell.dialog.is_some());
        assert_eq!(shell.pending_after_save, Some(PendingReplacement::NewScene));
        assert!(shell.commands.is_dirty());

        shell.apply_dialog_action(DialogAction::Save(save_path.clone(), false));
        let saved_path = save_path.with_extension("stl");
        assert!(saved_path.is_file());
        assert!(shell.commands.scene().objects().is_empty());
        assert!(shell.commands.project_path().is_none());
        assert!(!shell.commands.is_dirty());
        assert!(shell.pending_after_save.is_none());
        assert_ne!(shell.commands.scene().history_state_id(), dirty_history);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn failed_save_keeps_the_confirmation_and_dirty_scene_open() {
        let directory = test_directory("failed-save");
        let project_path = directory.join("blocked.3dp");
        let mut shell = Modeler3dApp::default();
        make_dirty(&mut shell);
        assert!(shell.commands.save_to_path(&project_path).is_ok());
        make_dirty(&mut shell);
        assert!(std::fs::remove_file(&project_path).is_ok());
        assert!(std::fs::create_dir(&project_path).is_ok());

        let history = shell.commands.scene().history_state_id();
        let revision = shell.commands.scene().revision();
        shell.confirm_action = Some(PendingReplacement::NewScene);
        shell.finish_confirmation(ConfirmationChoice::Save);

        assert_eq!(shell.confirm_action, Some(PendingReplacement::NewScene));
        assert!(shell.commands.is_dirty());
        assert_eq!(shell.commands.scene().history_state_id(), history);
        assert_eq!(shell.commands.scene().revision(), revision);
        assert_eq!(shell.commands.project_path(), Some(project_path.as_path()));
        assert_eq!(shell.commands.scene().objects().len(), 2);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn typed_export_overwrite_cancel_preserves_destination_and_scene_state() {
        let directory = repository_test_directory("export-cancel");
        let destination = directory.join("existing.ply");
        let prior_bytes = b"existing export must survive cancellation";
        assert!(std::fs::write(&destination, prior_bytes).is_ok());

        let mut shell = Modeler3dApp::default();
        make_dirty(&mut shell);
        shell.dialog = Some(DialogState::new(DialogKind::Export, ThemeMode::Dark));
        let selection = shell.commands.scene().selection().clone();
        let revision = shell.commands.scene().revision();
        let history = shell.commands.scene().history_state_id();
        let dirty = shell.commands.is_dirty();
        let project_path = shell.commands.project_path().map(Path::to_path_buf);
        let recents = shell.commands.preferences().recent_files().paths().to_vec();

        shell.apply_dialog_action(DialogAction::Export(destination.clone(), false));
        assert_eq!(
            shell.pending_overwrite,
            Some(PendingOverwrite::Export(destination.clone()))
        );
        assert_eq!(
            std::fs::read(&destination).ok().as_deref(),
            Some(&prior_bytes[..])
        );
        shell.finish_overwrite_confirmation(false);

        assert!(shell.pending_overwrite.is_none());
        assert_eq!(
            std::fs::read(&destination).ok().as_deref(),
            Some(&prior_bytes[..])
        );
        assert!(shell.dialog.is_some());
        assert_eq!(shell.commands.scene().selection(), &selection);
        assert_eq!(shell.commands.scene().revision(), revision);
        assert_eq!(shell.commands.scene().history_state_id(), history);
        assert_eq!(shell.commands.project_path(), project_path.as_deref());
        assert_eq!(shell.commands.is_dirty(), dirty);
        assert_eq!(
            shell.commands.preferences().recent_files().paths(),
            recents.as_slice()
        );

        shell.apply_dialog_action(DialogAction::Export(destination.clone(), false));
        shell.finish_overwrite_confirmation(true);
        assert!(modeler3d_core::load_mesh_asset(&destination).is_ok());
        assert!(shell.dialog.is_none());
        assert_eq!(shell.commands.scene().selection(), &selection);
        assert_eq!(shell.commands.scene().revision(), revision);
        assert_eq!(shell.commands.scene().history_state_id(), history);
        assert_eq!(shell.commands.project_path(), project_path.as_deref());
        assert_eq!(shell.commands.is_dirty(), dirty);
        assert_eq!(
            shell.commands.preferences().recent_files().paths(),
            recents.as_slice()
        );
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn typed_save_as_overwrite_cancel_keeps_the_existing_project_and_live_scene() {
        let directory = repository_test_directory("project-cancel");
        let destination = directory.join("existing.3dp");
        let mut prior = CommandState::default();
        assert!(matches!(
            prior.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere)),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(prior.save_to_path(&destination).is_ok());
        let prior_bytes = std::fs::read(&destination);
        assert!(prior_bytes.is_ok());

        let mut shell = Modeler3dApp::default();
        make_dirty(&mut shell);
        shell.dialog = Some(DialogState::new(DialogKind::SaveProject, ThemeMode::Dark));
        let selection = shell.commands.scene().selection().clone();
        let revision = shell.commands.scene().revision();
        let history = shell.commands.scene().history_state_id();
        let dirty = shell.commands.is_dirty();

        shell.apply_dialog_action(DialogAction::Save(destination.clone(), false));
        assert_eq!(
            shell.pending_overwrite,
            Some(PendingOverwrite::SaveProject(destination.clone()))
        );
        shell.finish_overwrite_confirmation(false);

        assert_eq!(std::fs::read(&destination).ok(), prior_bytes.ok());
        assert!(shell.dialog.is_some());
        assert!(shell.commands.project_path().is_none());
        assert_eq!(shell.commands.scene().selection(), &selection);
        assert_eq!(shell.commands.scene().revision(), revision);
        assert_eq!(shell.commands.scene().history_state_id(), history);
        assert_eq!(shell.commands.is_dirty(), dirty);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn dirty_project_open_supports_cancel_discard_and_save_as() {
        let directory = test_directory("open-guard");
        let target = directory.join("replacement.3dp");
        let mut replacement = CommandState::default();
        assert!(matches!(
            replacement.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere)),
            Ok(CommandOutcome::Applied(_))
        ));
        let expected_name = replacement.scene().objects()[0].name().to_owned();
        assert!(replacement.save_to_path(&target).is_ok());

        let mut shell = Modeler3dApp::default();
        make_dirty(&mut shell);
        let history = shell.commands.scene().history_state_id();
        shell.apply_dialog_action(DialogAction::Open(target.clone()));
        assert_eq!(
            shell.confirm_action,
            Some(PendingReplacement::OpenProject(target.clone()))
        );
        shell.finish_confirmation(ConfirmationChoice::Cancel);

        assert_eq!(shell.commands.scene().objects().len(), 1);
        assert_eq!(shell.commands.scene().history_state_id(), history);
        assert!(shell.commands.project_path().is_none());
        assert!(shell.commands.is_dirty());

        shell.confirm_action = Some(PendingReplacement::OpenProject(target.clone()));
        shell.finish_confirmation(ConfirmationChoice::Discard);
        assert_eq!(shell.commands.scene().objects().len(), 1);
        assert_eq!(shell.commands.scene().objects()[0].name(), expected_name);
        assert_eq!(shell.commands.project_path(), Some(target.as_path()));
        assert!(!shell.commands.is_dirty());
        assert!(shell.camera_auto_frame_pending);

        assert!(matches!(
            shell.commands.dispatch(CommandId::NewScene),
            Ok(CommandOutcome::Applied(_))
        ));
        make_dirty(&mut shell);
        let save_path = directory.join("saved-before-open");
        shell.apply_dialog_action(DialogAction::Open(target.clone()));
        assert_eq!(
            shell.confirm_action,
            Some(PendingReplacement::OpenProject(target.clone()))
        );
        shell.finish_confirmation(ConfirmationChoice::Save);
        assert!(shell.dialog.is_some());
        assert_eq!(
            shell.pending_after_save,
            Some(PendingReplacement::OpenProject(target.clone()))
        );
        assert!(shell.commands.is_dirty());

        shell.apply_dialog_action(DialogAction::Save(save_path.clone(), false));
        assert!(save_path.with_extension("stl").is_file());
        assert_eq!(shell.commands.scene().objects().len(), 1);
        assert_eq!(shell.commands.scene().objects()[0].name(), expected_name);
        assert_eq!(shell.commands.project_path(), Some(target.as_path()));
        assert!(!shell.commands.is_dirty());
        assert!(shell.pending_after_save.is_none());
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn dirty_exit_guard_cancel_and_discard_match_file_menu_and_window_close() {
        let mut cancelled = Modeler3dApp::default();
        make_dirty(&mut cancelled);
        cancelled.request_exit();
        assert_eq!(cancelled.confirm_action, Some(PendingReplacement::Exit));
        cancelled.finish_confirmation(ConfirmationChoice::Cancel);
        assert!(!cancelled.close_requested);
        assert!(cancelled.commands.is_dirty());

        let mut discarded = Modeler3dApp::default();
        make_dirty(&mut discarded);
        discarded.request_exit();
        discarded.finish_confirmation(ConfirmationChoice::Discard);
        assert!(discarded.close_requested);
        assert!(discarded.commands.is_dirty());

        discarded.handle_native_close_request(&egui::Context::default());
        assert!(discarded.confirm_action.is_none());
        assert!(discarded.close_requested);
    }

    #[test]
    fn dispatch_moves_to_scene_without_putting_model_logic_in_the_shell() {
        let mut shell = Modeler3dApp::default();
        shell.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
        assert_eq!(shell.commands.scene().objects().len(), 1);
        assert_eq!(shell.surface, WorkspaceSurface::Scene);
        assert!(shell.preview.is_none());
    }

    #[test]
    fn disabled_command_policy_requires_the_relevant_selection() {
        let state = CommandState::default();
        assert!(!command_enabled(CommandId::Delete, &state));
        assert!(!command_enabled(CommandId::Paste, &state));
        assert!(command_enabled(CommandId::NewScene, &state));
    }

    #[test]
    fn brush_buttons_activate_persistent_mode_without_model_history() {
        let mut app = Modeler3dApp::default();
        app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
        let history = app.commands.scene().history_state_id();
        let revision = app.commands.scene().revision();
        app.dispatch(CommandId::Blob);
        assert_eq!(app.active_brush, Some(ActiveBrush::Blob));
        assert!(app.brush_settings_open);
        assert_eq!(app.commands.scene().history_state_id(), history);
        assert_eq!(app.commands.scene().revision(), revision);
        app.dispatch(CommandId::Blob);
        assert_eq!(app.active_brush, None);
        assert_eq!(app.commands.scene().history_state_id(), history);
    }

    #[test]
    fn clicking_off_a_model_allows_simplify_smooth_blob_and_selection_resume() {
        let mut app = Modeler3dApp::default();
        app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere));
        app.refresh_preview();
        app.camera.set_viewport_size(800, 600).expect("viewport");
        app.frame_camera_if_pending();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800., 600.));
        let history = app.commands.scene().history_state_id();
        app.dispatch(CommandId::Simplify);
        app.pick_viewport_object(rect, egui::pos2(1., 1.), egui::Modifiers::default());
        assert_eq!(app.commands.selected_count(), 0);
        app.dispatch(CommandId::Smooth);
        assert!(app.command_selected(CommandId::Smooth));
        app.dispatch(CommandId::Blob);
        assert!(app.command_selected(CommandId::Blob));
        assert!(app.dialog.is_none());
        app.pick_viewport_object(rect, rect.center(), egui::Modifiers::default());
        assert_eq!(app.commands.selected_count(), 1);
        assert!(app.command_selected(CommandId::Blob));
        app.dispatch(CommandId::Blob);
        assert!(!app.brush_settings_open);
        assert_eq!(app.commands.scene().history_state_id(), history);
    }

    #[test]
    fn every_edit_tool_switches_and_dismisses_without_selection_or_history() {
        let buttons = tab_commands(RibbonTab::Edit);
        for populated in [false, true] {
            for from in &buttons {
                for to in &buttons {
                    let mut app = Modeler3dApp::default();
                    if populated {
                        app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
                        app.commands.deselect_all();
                    }
                    let history = app.commands.scene().history_state_id();
                    let revision = app.commands.scene().revision();
                    // Begin from no active tool, including when a Boolean ran.
                    app.apply_dialog_action(DialogAction::Close);
                    app.active_brush = None;
                    app.boolean_pick = None;
                    assert!(command_enabled(from.id, &app.commands));
                    app.dispatch(from.id);
                    assert!(app.command_selected(from.id), "{from:?}");
                    if from.id != to.id {
                        app.dispatch(to.id);
                        assert!(!app.command_selected(from.id), "{from:?} -> {to:?}");
                    }
                    assert!(app.command_selected(to.id), "{to:?}");
                    app.dispatch(to.id);
                    assert!(!app.command_selected(to.id));
                    assert!(app.dialog.is_none());
                    assert!(!app.brush_settings_open);
                    assert_eq!(app.commands.scene().history_state_id(), history);
                    assert_eq!(app.commands.scene().revision(), revision);
                }
            }
        }
    }

    #[test]
    fn edit_buttons_accept_pointer_clicks_in_ribbon_and_favorites_without_selection() {
        for favorite in [false, true] {
            for button in tab_commands(RibbonTab::Edit) {
                let (mut app, directory) = shell_for_jobs("edit-tool-buttons");
                app.commands
                    .preferences_mut()
                    .set_ribbon_favorites(vec![command_key(RibbonTab::Edit, button)]);
                let ctx = egui::Context::default();
                let frame = |app: &mut Modeler3dApp, events| {
                    let mut output = ctx.run_ui(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                Vec2::new(1280., 840.),
                            )),
                            events,
                            ..Default::default()
                        },
                        |ui| {
                            if favorite {
                                app.render_favorites(ui, ThemePalette::dark());
                            } else {
                                app.render_command_button(ui, button, ThemePalette::dark());
                            }
                        },
                    );
                    output.textures_delta.clear();
                };
                frame(&mut app, vec![]);
                let pointer = egui::pos2(20., 15.);
                for selected in [true, false] {
                    frame(&mut app, vec![egui::Event::PointerMoved(pointer)]);
                    for pressed in [true, false] {
                        frame(
                            &mut app,
                            vec![egui::Event::PointerButton {
                                pos: pointer,
                                button: egui::PointerButton::Primary,
                                pressed,
                                modifiers: egui::Modifiers::default(),
                            }],
                        );
                    }
                    assert_eq!(
                        app.command_selected(button.id),
                        selected,
                        "favorite={favorite}, {button:?}"
                    );
                }
                assert!(std::fs::remove_dir_all(directory).is_ok());
            }
        }
    }

    #[test]
    fn chosen_whole_object_tool_applies_after_selection_with_one_undo() {
        for command in [CommandId::Simplify, CommandId::Subdivide, CommandId::Smooth] {
            let (mut app, directory) = shell_for_jobs("edit-tool-resume");
            app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere));
            app.commands.deselect_all();
            let before = app.commands.scene().objects()[0].mesh().clone();
            let history = app.commands.scene().history_state_id();
            app.dispatch(command);
            let action = match command {
                CommandId::Simplify => DialogAction::Simplify(10),
                CommandId::Subdivide => DialogAction::SubdivideWhole,
                _ => DialogAction::EditAction(command),
            };
            app.apply_dialog_action(action.clone());
            assert!(app.desktop_job.is_none());
            assert_eq!(app.commands.scene().history_state_id(), history);
            app.commands.select_index(0);
            app.apply_dialog_action(action);
            wait_for_desktop_job(&mut app);
            assert_ne!(app.commands.scene().objects()[0].mesh(), &before);
            app.dispatch(CommandId::Undo);
            assert_eq!(app.commands.scene().objects()[0].mesh(), &before);
            assert_eq!(app.commands.scene().history_state_id(), history);
            app.dispatch(CommandId::Redo);
            assert_ne!(app.commands.scene().objects()[0].mesh(), &before);
            assert!(std::fs::remove_dir_all(directory).is_ok());
        }
    }

    #[test]
    fn active_brush_buttons_close_settings_after_selection_is_lost() {
        for command in [
            CommandId::Grab,
            CommandId::Blob,
            CommandId::Crease,
            CommandId::SmoothBrush,
        ] {
            let mut app = Modeler3dApp::default();
            app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
            let history = app.commands.scene().history_state_id();
            app.dispatch(command);
            assert!(app.command_selected(command));
            assert!(app.brush_settings_open);
            app.commands.deselect_all();
            app.dispatch(command);
            assert!(!app.command_selected(command));
            assert!(app.active_brush.is_none());
            assert!(!app.brush_settings_open);
            assert!(app.viewport_drag.is_none());
            assert!(app.preview_mesh_override.is_none());
            assert_eq!(app.commands.scene().history_state_id(), history);
        }
    }

    #[test]
    fn clicking_the_brush_button_again_closes_its_window() {
        let mut app = Modeler3dApp::default();
        app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
        let button = tab_commands(RibbonTab::Edit)
            .into_iter()
            .find(|button| button.id == CommandId::Grab)
            .expect("Grab button");
        let ctx = egui::Context::default();
        let frame = |app: &mut Modeler3dApp, events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1280.0, 840.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    app.render_command_button(ui, button, ThemePalette::dark());
                    app.render_brush_settings(ui.ctx());
                },
            );
            output.textures_delta.clear();
        };
        let pointer = egui::pos2(30.0, 30.0);
        frame(&mut app, vec![]);
        for click in 0..2 {
            frame(&mut app, vec![egui::Event::PointerMoved(pointer)]);
            for pressed in [true, false] {
                frame(
                    &mut app,
                    vec![egui::Event::PointerButton {
                        pos: pointer,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::default(),
                    }],
                );
            }
            assert_eq!(app.command_selected(CommandId::Grab), click == 0);
            assert_eq!(app.brush_settings_open, click == 0);
            app.commands.deselect_all();
        }
    }

    #[test]
    fn modifier_dialog_buttons_toggle_closed_without_applying_changes() {
        for command in [
            CommandId::Simplify,
            CommandId::Subdivide,
            CommandId::Hollow,
            CommandId::Emboss,
            CommandId::Mirror,
            CommandId::Align,
            CommandId::Arrange,
        ] {
            let mut app = Modeler3dApp::default();
            app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
            app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
            app.commands.select_all();
            let history = app.commands.scene().history_state_id();
            app.dispatch(command);
            assert!(app.command_selected(command), "{command:?}");
            app.commands.deselect_all();
            app.dispatch(command);
            assert!(!app.command_selected(command), "{command:?}");
            assert!(app.dialog.is_none());
            assert!(app.placement_command.is_none());
            assert_eq!(app.commands.scene().history_state_id(), history);
        }
    }

    #[test]
    fn one_selection_boolean_starts_a_picker_and_does_not_mutate_history() {
        let mut app = Modeler3dApp::default();
        app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
        app.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
        app.commands.select_index(0);
        let history = app.commands.scene().history_state_id();
        let revision = app.commands.scene().revision();
        app.dispatch(CommandId::Intersect);
        assert!(app.boolean_pick.is_some());
        assert_eq!(app.commands.scene().objects().len(), 2);
        assert_eq!(app.commands.scene().history_state_id(), history);
        assert_eq!(app.commands.scene().revision(), revision);
        app.boolean_pick = None;
    }

    #[test]
    fn physical_viewport_mapping_tracks_scale_and_client_coordinates() {
        assert_eq!(
            physical_viewport_size(Vec2::new(640.0, 420.0), 2.0),
            (1280, 840)
        );
        let rect = egui::Rect::from_min_size(egui::pos2(24.0, 36.0), Vec2::new(640.0, 420.0));
        assert_eq!(
            viewport_pixel_position(egui::pos2(344.0, 246.0), rect, (1280, 840)),
            [640.0, 420.0]
        );
    }

    #[test]
    fn viewport_router_applies_camera_gestures_and_honors_capture() {
        let mut app = Modeler3dApp::default();
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube insertion should succeed");
        app.refresh_preview();
        assert!(app.camera.set_viewport_size(800, 600).is_ok());
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        let neutral = ViewportInput {
            pointer_position: None,
            pointer_delta: Vec2::ZERO,
            primary_pressed: false,
            primary_down: false,
            primary_released: false,
            secondary_pressed: false,
            secondary_down: false,
            secondary_released: false,
            middle_pressed: false,
            middle_down: false,
            middle_released: false,
            scroll_y: 0.0,
            pinch_zoom_factor: 1.0,
            escape_pressed: false,
            focused: true,
            modifiers: egui::Modifiers::default(),
        };
        let before_orbit = app.camera.state();
        let mut press = neutral;
        press.pointer_position = Some(egui::pos2(20.0, 20.0));
        press.primary_pressed = true;
        press.primary_down = true;
        app.route_viewport_input(rect, 1.0, press, true);
        let mut move_pointer = neutral;
        move_pointer.pointer_position = Some(egui::pos2(32.0, 28.0));
        move_pointer.pointer_delta = Vec2::new(12.0, 8.0);
        move_pointer.primary_down = true;
        app.route_viewport_input(rect, 1.0, move_pointer, true);
        let mut release = neutral;
        release.pointer_position = move_pointer.pointer_position;
        release.primary_released = true;
        app.route_viewport_input(rect, 1.0, release, true);
        assert_ne!(app.camera.state().yaw_degrees, before_orbit.yaw_degrees);
        assert_ne!(app.camera.state().pitch_degrees, before_orbit.pitch_degrees);

        let before_pan = app.camera.center();
        let mut pan = neutral;
        pan.pointer_position = Some(egui::pos2(250.0, 260.0));
        pan.middle_pressed = true;
        pan.middle_down = true;
        app.route_viewport_input(rect, 1.0, pan, true);
        pan.pointer_delta = Vec2::new(10.0, -6.0);
        app.route_viewport_input(rect, 1.0, pan, true);
        pan.middle_released = true;
        pan.middle_down = false;
        app.route_viewport_input(rect, 1.0, pan, true);
        assert_ne!(app.camera.center(), before_pan);

        let before_zoom = app.camera.distance();
        let mut zoom = neutral;
        zoom.pointer_position = Some(egui::pos2(250.0, 260.0));
        zoom.scroll_y = 120.0;
        app.route_viewport_input(rect, 1.0, zoom, true);
        assert_ne!(app.camera.distance(), before_zoom);

        let before_captured_zoom = app.camera.distance();
        zoom.scroll_y = 120.0;
        app.route_viewport_input(rect, 1.0, zoom, false);
        assert_eq!(app.camera.distance(), before_captured_zoom);
    }

    #[test]
    fn gizmo_release_commits_once_and_cancel_or_noop_keeps_history() {
        let mut app = Modeler3dApp::default();
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube insertion should succeed");
        app.refresh_preview();
        assert!(app.camera.set_viewport_size(800, 600).is_ok());
        let start_position = app.commands.scene().objects()[0].position();
        let moved_position = [
            start_position[0] + 10.0,
            start_position[1],
            start_position[2],
        ];
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        let neutral = ViewportInput {
            pointer_position: None,
            pointer_delta: Vec2::ZERO,
            primary_pressed: false,
            primary_down: false,
            primary_released: false,
            secondary_pressed: false,
            secondary_down: false,
            secondary_released: false,
            middle_pressed: false,
            middle_down: false,
            middle_released: false,
            scroll_y: 0.0,
            pinch_zoom_factor: 1.0,
            escape_pressed: false,
            focused: true,
            modifiers: egui::Modifiers::default(),
        };
        let history_before = app.commands.scene().history_state_id();

        let make_drag = |app: &Modeler3dApp, distance: f32| {
            let snapshot = app.commands.capture_transform_selection().expect("capture");
            let mut drag = TransformGesture::move_axis(
                snapshot,
                [0.; 3],
                [1., 0., 0.],
                modeler3d_viewport::Ray::new([0., 10., 0.], [0., -1., 0.]).expect("start ray"),
            )
            .expect("start");
            drag.update(
                modeler3d_viewport::Ray::new([distance, 10., 0.], [0., -1., 0.]).expect("ray"),
            )
            .expect("preview");
            ViewportDrag::Transform(Box::new(drag))
        };
        app.viewport_drag = Some(make_drag(&app, 10.));
        let mut cancel = neutral;
        cancel.escape_pressed = true;
        app.route_viewport_input(rect, 1.0, cancel, true);
        assert!(app.viewport_drag.is_none());
        assert_eq!(app.commands.scene().history_state_id(), history_before);
        assert_eq!(app.commands.scene().objects()[0].position(), start_position);

        app.viewport_drag = Some(make_drag(&app, 0.0));
        let mut release = neutral;
        release.primary_released = true;
        app.route_viewport_input(rect, 1.0, release, true);
        assert_eq!(app.commands.scene().history_state_id(), history_before);
        assert_eq!(app.commands.scene().objects()[0].position(), start_position);

        app.viewport_drag = Some(make_drag(&app, 10.0));
        app.route_viewport_input(rect, 1.0, release, true);
        let after_commit = app.commands.scene().history_state_id();
        assert_ne!(after_commit, history_before);
        assert_eq!(app.commands.scene().objects()[0].position(), moved_position);
        assert!(matches!(
            app.commands.dispatch(CommandId::Undo),
            Ok(CommandOutcome::Applied(_))
        ));
        assert_eq!(app.commands.scene().objects()[0].position(), start_position);
        assert!(matches!(
            app.commands.dispatch(CommandId::Redo),
            Ok(CommandOutcome::Applied(_))
        ));
        assert_eq!(app.commands.scene().objects()[0].position(), moved_position);
    }

    #[test]
    fn clipboard_enables_paste_without_requiring_a_current_selection() {
        let mut state = CommandState::default();
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(state.dispatch(CommandId::Copy).is_ok());
        assert!(state.dispatch(CommandId::DeselectAll).is_ok());
        assert!(command_enabled(CommandId::Paste, &state));
    }

    #[test]
    fn items_anchor_follows_identity_after_an_earlier_row_is_deleted() {
        let mut app = Modeler3dApp::default();
        for _ in 0..4 {
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("cube");
        }
        app.commands.deselect_all();
        app.select_item_row(2, egui::Modifiers::default());
        let anchor = app.selection_anchor.clone().expect("anchor");
        app.commands.set_selection_indices([0]);
        app.commands
            .dispatch(CommandId::Delete)
            .expect("remove earlier row");
        let history = app.commands.scene().history_state_id();
        let dirty = app.commands.is_dirty();
        app.select_item_row(
            2,
            egui::Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        assert_eq!(
            app.commands
                .scene()
                .selection()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(app.commands.scene().objects()[1].object_id(), anchor);
        assert_eq!(app.commands.scene().history_state_id(), history);
        assert_eq!(app.commands.is_dirty(), dirty);
        app.select_item_row(1, egui::Modifiers::default()); // plain Items click toggles
        assert_eq!(
            app.commands
                .scene()
                .selection()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            [2]
        );
        app.select_item_row(
            1,
            egui::Modifiers {
                ctrl: true,
                ..Default::default()
            },
        );
        assert_eq!(app.commands.selected_count(), 2);
    }

    #[test]
    fn replacing_a_project_at_the_same_revision_resets_items_and_gizmo_caches() {
        let directory = repository_test_directory("rb04-project-cache");
        let mut app = Modeler3dApp {
            settings_path: directory.join("settings.json"),
            ..Default::default()
        };
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        app.commands
            .set_selected_position([100., 20., 30.])
            .expect("move");
        let file = directory.join("same-revision.3dp");
        app.commands.save_to_path(&file).expect("save");
        app.refresh_preview();
        app.update_gizmo_frame();
        let id = app.commands.scene().objects()[0].object_id().to_owned();
        app.item_revision = app.commands.scene().revision();
        app.item_health.insert(id, false);
        assert!(app.gizmo_frame.is_some());
        let revision = app.commands.scene().revision();
        app.open_project(&file).expect("reopen");
        assert_eq!(app.commands.scene().revision(), revision);
        assert_eq!(app.item_revision, u64::MAX);
        assert!(app.item_health.is_empty());
        assert!(app.gizmo_frame.is_none());
        app.commands.set_selection_indices([0]);
        app.update_gizmo_frame();
        assert_eq!(
            app.gizmo_frame.as_ref().expect("new frame").1.center,
            [100., 20., 30.]
        );
        app.finish_new_scene_presentation();
        assert!(app.gizmo_frame.is_none());
    }

    #[test]
    fn paint_clicks_are_one_undo_each_and_cancel_removes_them() {
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("of44-paint").join("settings.json"),
            ..Default::default()
        };
        for _ in 0..2 {
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("cube");
        }
        let second = app.commands.scene().objects()[1].object_id().to_owned();
        app.commands
            .set_object_position_by_id(&second, [70., 0., 0.])
            .expect("separate");
        app.commands.deselect_all();
        app.refresh_preview();
        app.frame_camera_if_pending();
        app.camera.set_viewport_size(1, 1).expect("size");
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(1.));
        let a = app
            .project_handle(app.commands.scene().objects()[0].position(), rect)
            .expect("project")
            .0;
        let original = app.commands.scene().objects()[0].color();
        let history = app.commands.scene().history_state_id();

        app.dispatch(CommandId::Paint);
        let red = [1.0, 0.0, 0.0, 1.0];
        if let Some(mode) = app.paint_mode.as_mut() {
            mode.color = red;
            mode.coverage = 1.0;
        }
        app.paint_click(rect, a);
        assert_eq!(app.commands.scene().objects()[0].color(), red);
        assert_eq!(app.commands.scene().history_state_id(), history + 1);
        // Painting is not selecting.
        assert_eq!(app.commands.selected_count(), 0);
        // A low-coverage click paints only part of the object.
        if let Some(mode) = app.paint_mode.as_mut() {
            mode.color = [0.0, 0.0, 1.0, 1.0];
            mode.coverage = 0.0;
        }
        app.refresh_preview();
        app.paint_click(rect, a);
        let colors = modeler3d_core::object_face_colors(&app.commands.scene().objects()[0])
            .expect("region paint");
        assert!(colors.contains(&[0, 0, 255, 255]) && colors.contains(&[255, 0, 0, 255]));
        assert_eq!(app.commands.scene().history_state_id(), history + 2);
        // Cancel removes both paint clicks; Paint remembers the colour.
        app.finish_paint_mode(false);
        assert!(app.paint_mode.is_none());
        assert_eq!(app.commands.scene().objects()[0].color(), original);
        assert!(modeler3d_core::object_face_colors(&app.commands.scene().objects()[0]).is_none());
        assert_eq!(app.commands.scene().history_state_id(), history);
        assert_eq!(app.paint_color, [0.0, 0.0, 1.0, 1.0]);
        // Done keeps the paint.
        app.dispatch(CommandId::Paint);
        app.refresh_preview();
        app.paint_click(rect, a);
        app.dispatch(CommandId::Paint);
        assert!(app.paint_mode.is_none());
        assert_ne!(app.commands.scene().history_state_id(), history);
        assert!(modeler3d_core::object_face_colors(&app.commands.scene().objects()[0]).is_some());
    }

    #[test]
    fn measure_ruler_clicks_two_points_snaps_corners_and_changes_nothing() {
        // OF-55: Measure is a ruler in the view, not a dialog.
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("of55-measure").join("settings.json"),
            ..Default::default()
        };
        for _ in 0..2 {
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("cube");
        }
        let second = app.commands.scene().objects()[1].object_id().to_owned();
        app.commands
            .set_object_position_by_id(&second, [70., 0., 0.])
            .expect("separate");
        app.commands.set_selection_indices([1]);
        app.refresh_preview();
        app.frame_camera_if_pending();
        app.camera.set_viewport_size(800, 600).expect("size");
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800., 600.));
        let centre = |app: &Modeler3dApp, index: usize| {
            let bounds = world_bounds(app.commands.scene(), &[index]).expect("bounds");
            let middle = [0, 1, 2].map(|axis| (bounds.minimum[axis] + bounds.maximum[axis]) * 0.5);
            app.project_handle(middle, rect).expect("project").0
        };
        let (a, b) = (centre(&app, 0), centre(&app, 1));
        let history = app.commands.scene().history_state_id();
        let revision = app.commands.scene().revision();

        app.dispatch(CommandId::Measure);
        assert!(app.measure_mode.is_some());
        assert!(!app.modal_active(), "Measure is not a dialog");
        app.measure_click(rect, a, true);
        app.measure_click(rect, b, true);
        let mode = app.measure_mode.as_ref().expect("measuring");
        let (start, end) = (
            mode.start.expect("first end"),
            mode.end.expect("second end"),
        );
        let (delta, distance) = measure_ruler::ruler_measurement(start.world, end.world);
        assert!(distance > 30.0, "{distance}");
        // MCP `measure` between the same two points gives the same numbers.
        let version = app.commands.mcp_document_version();
        let server = modeler3d_mcp::R4Server::new(modeler3d_mcp::R4Config::for_profile(
            modeler3d_mcp::Profile::Observe,
            modeler3d_mcp::ApprovedRoots::from_paths(Vec::new()).expect("deny-all roots"),
        ));
        let mut document = Some(
            app.commands
                .take_mcp_document(modeler3d_mcp::ViewState::default()),
        );
        server.replace_document(&mut document).expect("attach");
        let mcp = server
            .call_value(
                "measure",
                serde_json::json!({"from_point_mm":start.world,"to_point_mm":end.world}),
            )
            .expect("MCP measure");
        app.commands
            .install_mcp_document(server.take_document().expect("detach"), &version)
            .expect("install");
        assert!((mcp["distance_mm"].as_f64().unwrap_or(0.0) as f32 - distance).abs() < 1e-4);
        for axis in 0..3 {
            let part = mcp["delta_mm"][axis].as_f64().unwrap_or(f64::NAN) as f32;
            assert!((part - delta[axis]).abs() < 1e-4);
        }
        assert!(app.commands.last_status().contains("Measured"));
        // Measuring is not selecting and adds nothing to Undo.
        assert_eq!(app.commands.scene().history_state_id(), history);
        assert_eq!(app.commands.scene().revision(), revision);
        assert_eq!(app.commands.scene().selection().len(), 1);

        // Near a visible corner the pointer snaps exactly onto it.
        let cube = app.commands.scene().objects()[0]
            .transformed_mesh()
            .expect("world cube");
        let snapped = cube.vertices.iter().any(|vertex| {
            let Some((corner, _)) = app.project_handle(*vertex, rect) else {
                return false;
            };
            let towards_middle = corner + (a - corner).normalized() * 4.0;
            if let Some(mode) = app.measure_mode.as_mut() {
                mode.start = None;
                mode.end = None;
            }
            app.measure_click(rect, towards_middle, false);
            app.measure_mode
                .as_ref()
                .and_then(|mode| mode.start)
                .is_some_and(|point| {
                    point.snap == measure_ruler::RulerSnap::Corner && point.world == *vertex
                })
        });
        assert!(snapped, "no visible cube corner caught the pointer");

        // One Escape closes Measure, even with a half-placed ruler.
        if let Some(mode) = app.measure_mode.as_mut() {
            mode.end = None;
        }
        assert!(
            app.measure_mode
                .as_ref()
                .is_some_and(|mode| mode.start.is_some())
        );
        app.measure_escape();
        assert!(app.measure_mode.is_none());
        // View commands keep Measure open; other commands close it.
        app.dispatch(CommandId::Measure);
        app.dispatch(CommandId::ToggleGrid);
        assert!(app.measure_mode.is_some());
        app.dispatch(CommandId::SelectAll);
        assert!(app.measure_mode.is_none());
        assert_eq!(app.commands.scene().history_state_id(), history);
    }

    #[test]
    fn viewport_picks_toggle_plain_ctrl_shift_and_empty_sticky_without_history() {
        let mut app = Modeler3dApp::default();
        for _ in 0..2 {
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("cube");
        }
        let second = app.commands.scene().objects()[1].object_id().to_owned();
        app.commands
            .set_object_position_by_id(&second, [70., 0., 0.])
            .expect("separate");
        app.commands.deselect_all();
        app.refresh_preview();
        app.frame_camera_if_pending();
        app.camera.set_viewport_size(1, 1).expect("size");
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(1.));
        let a = app
            .project_handle(app.commands.scene().objects()[0].position(), rect)
            .expect("project")
            .0;
        let b = app
            .project_handle(app.commands.scene().objects()[1].position(), rect)
            .expect("project")
            .0;
        let history = app.commands.scene().history_state_id();
        let revision = app.commands.scene().revision();
        let dirty = app.commands.is_dirty();
        app.pick_viewport_object(rect, a, egui::Modifiers::default());
        assert_eq!(app.commands.selected_count(), 1);
        app.pick_viewport_object(rect, b, egui::Modifiers::default());
        assert_eq!(app.commands.selected_count(), 2);
        app.pick_viewport_object(
            rect,
            a,
            egui::Modifiers {
                ctrl: true,
                ..Default::default()
            },
        );
        assert_eq!(
            app.commands
                .scene()
                .selection()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            [1]
        );
        app.pick_viewport_object(
            rect,
            b,
            egui::Modifiers {
                shift: true,
                ..Default::default()
            },
        );
        assert_eq!(app.commands.selected_count(), 0);
        app.pick_viewport_object(rect, a, egui::Modifiers::default());
        app.sticky_selection = true;
        app.pick_viewport_object(rect, rect.left_top(), egui::Modifiers::default());
        assert_eq!(app.commands.selected_count(), 1);
        app.sticky_selection = false;
        app.pick_viewport_object(
            rect,
            rect.left_top(),
            egui::Modifiers {
                ctrl: true,
                ..Default::default()
            },
        );
        assert_eq!(app.commands.selected_count(), 1);
        app.pick_viewport_object(rect, rect.left_top(), egui::Modifiers::default());
        assert_eq!(app.commands.selected_count(), 0);
        assert_eq!(history, app.commands.scene().history_state_id());
        assert_eq!(revision, app.commands.scene().revision());
        assert_eq!(dirty, app.commands.is_dirty());
    }

    #[test]
    fn viewport_orbit_mouse_input_is_independent_of_keyboard_focus() {
        let context = egui::Context::default();
        let mut text = String::new();
        let mut output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    Vec2::new(800.0, 600.0),
                )),
                ..Default::default()
            },
            |ui| {
                ui.add(egui::TextEdit::singleline(&mut text))
                    .request_focus();
            },
        );
        output.textures_delta.clear();

        assert!(context.egui_wants_keyboard_input());
        assert!(viewport_pointer_interactions_allowed(false, true, false));
        assert!(!viewport_pointer_interactions_allowed(false, false, false));
        assert!(viewport_pointer_interactions_allowed(false, false, true));
        assert!(!viewport_pointer_interactions_allowed(true, true, true));
    }

    #[test]
    fn resize_boxes_follow_pointer_preview_cancel_and_one_step_commit() {
        for unit in [DisplayUnit::Millimetre, DisplayUnit::Inch] {
            for object_basis in [false, true] {
                for handle in [
                    ResizeHandle::Face { axis: 0, side: 1.0 },
                    ResizeHandle::Face { axis: 1, side: 1.0 },
                    ResizeHandle::Face { axis: 2, side: 1.0 },
                    ResizeHandle::Corner([1.; 3]),
                ] {
                    let mut app = Modeler3dApp {
                        settings_path: repository_test_directory("resize-live-boxes")
                            .join("settings.json"),
                        ..Default::default()
                    };
                    app.commands
                        .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                        .expect("cube");
                    app.commands
                        .set_selected_size([30., 40., 50.])
                        .expect("size");
                    if object_basis {
                        app.commands
                            .set_selected_rotation([15., 25., 35.])
                            .expect("rotate");
                    }
                    app.commands.preferences_mut().set_display_unit(unit);
                    app.transform_panel_mode = TransformPanelMode::Resize;
                    app.object_resize_basis = object_basis;
                    app.refresh_preview();
                    app.camera.set_viewport_size(800, 600).expect("viewport");
                    app.frame_camera_if_pending();
                    app.update_gizmo_frame();
                    let frame = app.gizmo_frame.as_ref().expect("frame").1;
                    let original = world_bounds(app.commands.scene(), &[0]).expect("bounds");
                    let original_fields = original.extents().map(|v| format_distance(v, unit));
                    let pose = *app.commands.scene().objects()[0].transform();
                    let history = app.commands.scene().history_state_id();
                    let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800., 600.));
                    // Use a ray through the handle's model-space point. This also
                    // exercises the depth axis when its chevron is foreshortened.
                    let sides = match handle {
                        ResizeHandle::Face { axis, side } => {
                            let mut sides = [0.; 3];
                            sides[axis] = side;
                            sides
                        }
                        ResizeHandle::Corner(sides) => sides,
                        _ => unreachable!(),
                    };
                    let start = frame.point(sides);
                    let direction =
                        modeler3d_viewport::normalize(subtract(start, frame.center), "resize test")
                            .expect("direction");
                    let ray = |distance: f32| {
                        let point = modeler3d_viewport::add(
                            start,
                            modeler3d_viewport::scale(direction, distance),
                        );
                        modeler3d_viewport::Ray::new(
                            modeler3d_viewport::add(point, [100., 100., 100.]),
                            [-1., -1., -1.],
                        )
                        .expect("ray")
                    };
                    let input = ViewportInput {
                        pointer_position: None,
                        pointer_delta: Vec2::ZERO,
                        primary_pressed: false,
                        primary_down: false,
                        primary_released: false,
                        secondary_pressed: false,
                        secondary_down: false,
                        secondary_released: false,
                        middle_pressed: false,
                        middle_down: false,
                        middle_released: false,
                        scroll_y: 0.,
                        pinch_zoom_factor: 1.,
                        escape_pressed: false,
                        focused: true,
                        modifiers: egui::Modifiers::default(),
                    };
                    for finish in 0..4 {
                        let cancel = finish != 3;
                        let mut gesture = TransformGesture::resize(
                            app.commands
                                .capture_transform_selection()
                                .expect("snapshot"),
                            frame,
                            handle,
                            ray(0.),
                        )
                        .expect("gesture");
                        let mut previous = original_fields.clone();
                        for distance in [-8., 6., 14.] {
                            gesture
                                .update_with_snap(ray(distance), false)
                                .expect("motion");
                            app.viewport_drag =
                                Some(ViewportDrag::Transform(Box::new(gesture.clone())));
                            app.sync_resize_size_fields();
                            assert_ne!(app.size_fields, previous, "every motion updates sizes");
                            previous = app.size_fields.clone();
                            assert_eq!(*app.commands.scene().objects()[0].transform(), pose);
                            assert_eq!(app.commands.scene().history_state_id(), history);
                            if matches!(handle, ResizeHandle::Corner(_)) {
                                for (live, original) in app.size_fields.iter().zip(&original_fields)
                                {
                                    assert_ne!(live, original);
                                }
                            }
                        }
                        let live = app.size_fields.clone();
                        let mut end = input;
                        end.escape_pressed = finish == 0;
                        end.focused = finish != 1;
                        end.primary_released = !cancel;
                        app.route_viewport_input(rect, 1., end, finish != 2);
                        app.sync_resize_size_fields();
                        assert!(app.viewport_drag.is_none());
                        let committed = world_bounds(app.commands.scene(), &[0]).expect("bounds");
                        assert_eq!(
                            app.size_fields,
                            committed.extents().map(|v| format_distance(v, unit))
                        );
                        if cancel {
                            assert_eq!(app.size_fields, original_fields);
                            assert_eq!(*app.commands.scene().objects()[0].transform(), pose);
                            assert_eq!(app.commands.scene().history_state_id(), history);
                        } else {
                            assert_eq!(app.size_fields, live, "no jump on release");
                            app.commands.dispatch(CommandId::Undo).expect("undo");
                            assert_eq!(*app.commands.scene().objects()[0].transform(), pose);
                            assert_eq!(app.commands.scene().history_state_id(), history);
                            app.commands.dispatch(CommandId::Redo).expect("redo");
                            assert_eq!(
                                world_bounds(app.commands.scene(), &[0]).expect("bounds"),
                                committed
                            );
                        }
                    }
                    app.size_fields[0] = "a numeric edit in progress".into();
                    app.sync_resize_size_fields();
                    assert_eq!(app.size_fields[0], "a numeric edit in progress");
                }
            }
        }
    }

    #[test]
    fn reference_faces_and_corners_start_real_gestures_and_no_motion_is_noop() {
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("rb04-handle-click").join("settings.json"),
            ..Default::default()
        };
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        app.commands
            .set_selected_rotation([15., 25., 35.])
            .expect("rotate");
        app.refresh_preview();
        app.camera.set_viewport_size(800, 600).expect("size");
        app.frame_camera_if_pending();
        app.transform_panel_mode = TransformPanelMode::Resize;
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.));
        let history = app.commands.scene().history_state_id();
        for object_basis in [false, true] {
            app.object_resize_basis = object_basis;
            app.update_gizmo_frame();
            let overlay = app.resize_overlay(rect, None).expect("overlay");
            assert_eq!(
                app.transform_handle_cursor(rect, Some(egui::pos2(4., 4.)), 1.0, true),
                None,
                "ordinary pointer away from the handles"
            );
            let grabs = overlay.grab_points().collect::<Vec<_>>();
            assert!(grabs.len() >= 8, "four corner dots and the visible faces");
            for (handle, position) in grabs {
                assert!(rect.contains(position));
                assert_eq!(
                    app.resize_handle_at_pointer(rect, position, 1.0),
                    Some(handle)
                );
                assert_eq!(
                    app.transform_handle_cursor(rect, Some(position), 1.0, true),
                    Some(egui::CursorIcon::Grab),
                    "open hand over every handle (OF-33)"
                );
                assert_eq!(
                    app.transform_handle_cursor(rect, Some(position), 1.0, false),
                    None,
                    "no hand when a panel covers the viewport"
                );
                let drag = app
                    .begin_transform_drag(rect, position, 1.0)
                    .expect("handle starts gesture");
                assert!(matches!(drag, ViewportDrag::Transform(_)));
                app.viewport_drag = Some(drag);
                assert_eq!(
                    app.transform_handle_cursor(rect, Some(position), 1.0, true),
                    Some(egui::CursorIcon::Grabbing),
                    "closed hand while a handle is held"
                );
                let input = ViewportInput {
                    pointer_position: Some(position),
                    pointer_delta: Vec2::ZERO,
                    primary_pressed: false,
                    primary_down: false,
                    primary_released: true,
                    secondary_pressed: false,
                    secondary_down: false,
                    secondary_released: false,
                    middle_pressed: false,
                    middle_down: false,
                    middle_released: false,
                    scroll_y: 0.,
                    pinch_zoom_factor: 1.,
                    escape_pressed: false,
                    focused: true,
                    modifiers: egui::Modifiers::default(),
                };
                app.route_viewport_input(rect, 1., input, true);
                assert_eq!(history, app.commands.scene().history_state_id());
            }
        }
    }

    #[test]
    fn align_and_arrange_are_disabled_with_a_hint_under_two_objects() {
        let mut state = CommandState::default();
        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        state.set_selection_indices([0]);
        for command in [CommandId::Align, CommandId::Arrange] {
            assert!(!command_enabled(command, &state));
            assert_eq!(
                command_disabled_help(command, &state),
                modeler3d_core::PLACEMENT_NEEDS_TWO_MESSAGE
            );
        }
        assert!(
            command_enabled(CommandId::Mirror, &state),
            "Mirror works on one"
        );
        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("second cube");
        state.set_selection_indices([0, 1]);
        for command in [CommandId::Align, CommandId::Arrange] {
            assert!(command_enabled(command, &state));
        }
    }

    #[test]
    fn placement_axis_letters_use_the_arrow_colours() {
        assert_eq!(
            placement_axis_labels(),
            [
                (Axis::X, "X", crate::chrome::AXIS_COLORS[0]),
                (Axis::Y, "Y", crate::chrome::AXIS_COLORS[1]),
                (Axis::Z, "Z", crate::chrome::AXIS_COLORS[2]),
            ]
        );
    }

    #[test]
    fn a_group_of_sound_parts_is_not_marked_for_repair() {
        let mut state = CommandState::default();
        for _ in 0..2 {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("cube");
        }
        state.set_selection_indices([0, 1]);
        state.dispatch(CommandId::Group).expect("group");
        let group = &state.scene().objects()[0];
        assert_eq!(group.children().len(), 2);
        // The combined mesh is two shells, which a single object would be
        // flagged for; the group is not.
        assert!(!modeler3d_core::inspect_mesh(group.mesh()).is_ok_and(|report| report.valid));
        assert!(item_is_healthy(group));
    }

    #[test]
    fn move_and_rotate_handles_show_the_open_hand() {
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("of33-handle-cursor").join("settings.json"),
            ..Default::default()
        };
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        app.refresh_preview();
        app.camera.set_viewport_size(800, 600).expect("size");
        app.frame_camera_if_pending();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800., 600.));
        for mode in [TransformPanelMode::Move, TransformPanelMode::Rotate] {
            app.transform_panel_mode = mode;
            app.update_gizmo_frame();
            let gizmo = app.gizmo_mode();
            let mut on_handle = 0;
            let mut off_handle = 0;
            for y in (0..600).step_by(6) {
                for x in (0..800).step_by(6) {
                    let point = egui::pos2(x as f32, y as f32);
                    let over = app
                        .transform_axis_at_pointer(rect, point, 1.0, gizmo)
                        .is_some();
                    let cursor = app.transform_handle_cursor(rect, Some(point), 1.0, true);
                    if over {
                        on_handle += 1;
                        assert_eq!(
                            cursor,
                            Some(egui::CursorIcon::Grab),
                            "{mode:?} at {point:?}"
                        );
                    } else {
                        off_handle += 1;
                        assert_eq!(cursor, None, "{mode:?} at {point:?}");
                    }
                }
            }
            assert!(on_handle > 0, "{mode:?} handles were found");
            assert!(off_handle > on_handle);
        }
    }

    #[test]
    fn viewport_motion_cancels_on_escape_focus_modal_or_scene_change() {
        let mut app = Modeler3dApp::default();
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        let pose = *app.commands.scene().objects()[0].transform();
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(800.));
        for condition in 0..5 {
            let snapshot = app
                .commands
                .capture_transform_selection()
                .expect("snapshot");
            let mut drag = TransformGesture::move_axis(
                snapshot,
                [0.; 3],
                [1., 0., 0.],
                modeler3d_viewport::Ray::new([0., 10., 0.], [0., -1., 0.]).expect("ray"),
            )
            .expect("start");
            drag.update(modeler3d_viewport::Ray::new([25., 10., 0.], [0., -1., 0.]).expect("ray"))
                .expect("preview");
            app.viewport_drag = Some(ViewportDrag::Transform(Box::new(drag)));
            if condition == 4 {
                app.commands.deselect_all();
            }
            if condition == 3 {
                app.commands
                    .set_visibility(0, !app.commands.scene().objects()[0].is_visible())
                    .expect("change scene");
            }
            let history = app.commands.scene().history_state_id();
            let input = ViewportInput {
                pointer_position: None,
                pointer_delta: Vec2::ZERO,
                primary_pressed: false,
                primary_down: true,
                primary_released: false,
                secondary_pressed: false,
                secondary_down: false,
                secondary_released: false,
                middle_pressed: false,
                middle_down: false,
                middle_released: false,
                scroll_y: 0.,
                pinch_zoom_factor: 1.,
                escape_pressed: condition == 0,
                focused: condition != 1,
                modifiers: egui::Modifiers::default(),
            };
            app.route_viewport_input(rect, 1., input, condition != 2);
            assert!(app.viewport_drag.is_none());
            assert_eq!(*app.commands.scene().objects()[0].transform(), pose);
            assert_eq!(history, app.commands.scene().history_state_id());
        }
    }

    #[test]
    fn numeric_apply_preserves_unedited_exact_axes_and_noop_history() {
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("rb04-exact-axes").join("settings.json"),
            ..Default::default()
        };
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        app.commands
            .set_selected_position([0., 15., 0.])
            .expect("position");
        let history = app.commands.scene().history_state_id();
        app.position_fields = ["0.000 in".into(), "0.591 in".into(), "0.000 in".into()];
        app.apply_transform_fields(0, DisplayUnit::Inch);
        assert_eq!(app.commands.scene().history_state_id(), history);
        app.position_fields[0] = "1 in".into();
        app.numeric_edited_axes[0] = true;
        app.apply_transform_fields(0, DisplayUnit::Inch);
        assert_eq!(
            app.commands.scene().objects()[0].position(),
            [25.4, 15., 0.]
        );
        app.commands
            .set_selected_size([20., 30., 40.])
            .expect("size");
        app.transform_panel_mode = TransformPanelMode::Resize;
        app.proportional_resize = false;
        app.size_fields = ["1 in".into(), "1.181 in".into(), "1.575 in".into()];
        app.numeric_edited_axes[0] = true;
        app.apply_transform_fields(0, DisplayUnit::Inch);
        let size = world_bounds(app.commands.scene(), &[0])
            .expect("bounds")
            .extents();
        assert!((size[0] - 25.4).abs() < 0.000_01);
        assert_eq!([size[1], size[2]], [30., 40.]);
        app.commands
            .set_selected_rotation([12.123_456, 23.234_567, 34.345_68])
            .expect("rotation");
        let before = crate::rotation::degrees(app.commands.scene().objects()[0].transform())
            .expect("angles");
        app.rotation_fields = ["12.123 deg".into(), "23.235 deg".into(), "45 deg".into()];
        app.numeric_edited_axes[2] = true;
        app.apply_rotation_fields();
        let after = crate::rotation::degrees(app.commands.scene().objects()[0].transform())
            .expect("angles");
        assert!((before[0] - after[0]).abs() < 0.000_01);
        assert!((before[1] - after[1]).abs() < 0.000_01);
        assert!((after[2] - 45.).abs() < 0.000_01);
    }

    #[test]
    fn proportional_numeric_resize_uses_edited_y_in_inches_and_rejects_invalid_input() {
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("rb04-numeric").join("settings.json"),
            ..Default::default()
        };
        for _ in 0..2 {
            app.commands
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("cube");
        }
        app.commands.set_selection_indices([0, 1]);
        let before = world_bounds(app.commands.scene(), &[0, 1])
            .expect("bounds")
            .extents();
        let history = app.commands.scene().history_state_id();
        app.transform_panel_mode = TransformPanelMode::Resize;
        app.proportional_resize = true;
        app.last_numeric_axis = Some(1);
        app.numeric_edited_axes[1] = true;
        app.position_fields = ["invalid unused position".into(), "".into(), "".into()];
        app.size_fields = std::array::from_fn(|i| {
            format!("{:.8} in", before[i] * if i == 1 { 2. } else { 1. } / 25.4)
        });
        app.apply_transform_fields(0, DisplayUnit::Inch);
        assert!(app.transform_error.is_none());
        let after = world_bounds(app.commands.scene(), &[0, 1])
            .expect("bounds")
            .extents();
        for i in 0..3 {
            assert!((after[i] - before[i] * 2.).abs() < 0.001);
        }
        app.commands.dispatch(CommandId::Undo).expect("undo");
        assert_eq!(app.commands.scene().history_state_id(), history);
        app.size_fields[1] = "NaN in".into();
        app.numeric_edited_axes[1] = true;
        app.apply_transform_fields(0, DisplayUnit::Inch);
        assert!(app.transform_error.is_some());
        assert_eq!(app.commands.scene().history_state_id(), history);
        assert_eq!(
            world_bounds(app.commands.scene(), &[0, 1])
                .expect("bounds")
                .extents(),
            before
        );
    }

    /// The whole shell at `size` with Settings open: wheels over Settings and
    /// returns how far its "Restore 3D Builder defaults" button moved up.
    fn shell_settings_wheel_scroll(size: UiSize) -> f32 {
        let mut app = Modeler3dApp::default();
        app.dispatch(CommandId::AddPrimitive(modeler3d_core::PrimitiveKind::Cube));
        app.ui_size = size;
        app.open_dialog(DialogKind::Settings);
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        // Short enough that Settings overflows at either size.
        let window = match size {
            UiSize::Normal => Vec2::new(1280.0, 640.0),
            UiSize::Big => Vec2::new(1920.0, 1040.0),
        };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, window / size.zoom_factor());
        let frame = |app: &mut Modeler3dApp, events: Vec<egui::Event>| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ui| {
                    theme::apply(ui.ctx(), app.theme_mode);
                    theme::apply_size(ui.ctx(), app.ui_size);
                    app.render_background(ui);
                    app.render_dialog(ui.ctx());
                },
            );
            output.textures_delta.clear();
            output.platform_output.accesskit_update.and_then(|update| {
                update.nodes.iter().find_map(|(_, node)| {
                    (node.label() == Some("Restore 3D Builder defaults"))
                        .then(|| node.bounds().map(|bounds| bounds.y0 as f32))
                        .flatten()
                })
            })
        };
        let over = egui::pos2(120.0, screen.height() * 0.5);
        let mut before = None;
        for _ in 0..4 {
            before = frame(&mut app, vec![egui::Event::PointerMoved(over)]).or(before);
        }
        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -200.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::default(),
            source: egui::MouseWheelSource::Unknown,
        };
        let distance = app.camera.distance();
        let mut after = frame(&mut app, vec![egui::Event::PointerMoved(over), wheel]);
        for _ in 0..30 {
            after = frame(&mut app, vec![egui::Event::PointerMoved(over)]).or(after);
        }
        assert!(
            (app.camera.distance() - distance).abs() < f32::EPSILON,
            "wheeling over Settings must not zoom the 3D view"
        );
        before.unwrap_or_default() - after.unwrap_or_default()
    }

    #[test]
    fn settings_scrolls_and_the_view_stays_put_at_normal_and_big() {
        for size in [UiSize::Normal, UiSize::Big] {
            let scrolled = shell_settings_wheel_scroll(size);
            assert!(scrolled > 20.0, "{size:?} Settings scrolled {scrolled}");
        }
    }

    #[test]
    fn modal_pointer_input_cannot_duplicate_or_apply_background_transform() {
        fn frame(
            app: &mut Modeler3dApp,
            ctx: &egui::Context,
            events: Vec<egui::Event>,
        ) -> [Option<egui::Pos2>; 2] {
            fn label(shape: &egui::Shape, text: &str) -> Option<egui::Pos2> {
                match shape {
                    egui::Shape::Text(shape) if shape.galley.text() == text => {
                        Some(shape.pos + shape.galley.size() * 0.5)
                    }
                    egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| label(shape, text)),
                    _ => None,
                }
            }
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1280., 840.),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    theme::apply(ui.ctx(), app.theme_mode);
                    app.render_background(ui);
                    app.render_dialog(ui.ctx());
                },
            );
            output.textures_delta.clear();
            ["Duplicate", "Apply"].map(|text| {
                output
                    .shapes
                    .iter()
                    .find_map(|shape| label(&shape.shape, text))
            })
        }
        fn click(app: &mut Modeler3dApp, ctx: &egui::Context, point: egui::Pos2) {
            for pressed in [true, false] {
                frame(
                    app,
                    ctx,
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::default(),
                        },
                    ],
                );
            }
        }
        // Shortcut-bar buttons are icon-only (OF-10), so target the labelled
        // Duplicate button on the Object ribbon tab.
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("rb04-modal-pointer").join("settings.json"),
            surface: WorkspaceSurface::Scene,
            active_tab: RibbonTab::Object,
            ..Default::default()
        };
        app.commands
            .preferences_mut()
            .set_ribbon_favorites(default_favorites());
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        let history = app.commands.scene().history_state_id();
        let pose = *app.commands.scene().objects()[0].transform();
        let ctx = egui::Context::default();
        for _ in 0..3 {
            frame(&mut app, &ctx, Vec::new());
        }
        // Settings is deliberately non-modal (see
        // `settings_stays_non_modal_until_about_is_opened`), so block with a
        // modal dialog instead.
        app.open_dialog(DialogKind::SaveProject);
        assert!(app.modal_active());
        let blocked = frame(&mut app, &ctx, Vec::new());
        app.position_fields[0] = "12 mm".into();
        app.numeric_edited_axes[0] = true;
        click(&mut app, &ctx, blocked[0].expect("Duplicate label"));
        click(&mut app, &ctx, blocked[1].expect("Apply label"));
        assert_eq!(app.commands.scene().objects().len(), 1);
        assert_eq!(*app.commands.scene().objects()[0].transform(), pose);
        assert_eq!(app.commands.scene().history_state_id(), history);
        frame(
            &mut app,
            &ctx,
            vec![egui::Event::Key {
                key: Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
        );
        assert!(app.dialog.is_none());
        for _ in 0..2 {
            frame(&mut app, &ctx, Vec::new());
        }
        let enabled = frame(&mut app, &ctx, Vec::new());
        click(&mut app, &ctx, enabled[0].expect("Duplicate label"));
        assert_eq!(
            app.commands.scene().objects().len(),
            2,
            "positive control reaches the real Duplicate button"
        );
        let enabled = frame(&mut app, &ctx, Vec::new());
        app.position_fields[0] = "12 mm".into();
        app.numeric_edited_axes[0] = true;
        click(&mut app, &ctx, enabled[1].expect("Apply label"));
        assert_eq!(
            app.commands.scene().objects()[1].position()[0],
            12.,
            "positive control reaches real Apply"
        );
    }

    #[test]
    fn native_clipboard_keys_paste_objects_and_keep_text_clipboard_routes() {
        use eframe::App as _;
        fn key(key: Key) -> egui::RawInput {
            let modifiers = egui::Modifiers {
                ctrl: true,
                command: true,
                ..Default::default()
            };
            egui::RawInput {
                events: vec![
                    egui::Event::ModifiersChanged(modifiers),
                    egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed: true,
                        repeat: false,
                        modifiers,
                    },
                ],
                ..Default::default()
            }
        }
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("rb04-native-clipboard").join("settings.json"),
            ..Default::default()
        };
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        let original = app.commands.scene().objects()[0].object_id().to_owned();
        let ctx = egui::Context::default();
        for pressed in [Key::C, Key::V] {
            let mut input = key(pressed);
            app.raw_input_hook(&ctx, &mut input);
            let mut output = ctx.run_ui(input, |ui| app.handle_shortcuts(ui.ctx()));
            output.textures_delta.clear();
            if pressed == Key::C {
                app.commands.deselect_all();
            }
        }
        assert_eq!(app.commands.scene().objects().len(), 2);
        assert_eq!(app.commands.scene().objects()[0].object_id(), original);
        assert_ne!(app.commands.scene().objects()[1].object_id(), original);
        // OF-43 replaced the Python offsets: copies sit 10 mm beside the original.
        assert_eq!(app.commands.scene().objects()[1].position(), [50., 20., 0.]);
        let history = app.commands.scene().history_state_id();
        let mut text = String::from("editable name");
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.add(egui::TextEdit::singleline(&mut text).id(egui::Id::unique("clipboard-text")))
                .request_focus();
        });
        output.textures_delta.clear();
        for pressed in [Key::A, Key::X, Key::V] {
            let mut input = key(pressed);
            app.raw_input_hook(&ctx, &mut input);
            let mut output = ctx.run_ui(input, |ui| {
                app.handle_shortcuts(ui.ctx());
                ui.add(
                    egui::TextEdit::singleline(&mut text).id(egui::Id::unique("clipboard-text")),
                );
            });
            output.textures_delta.clear();
            if pressed == Key::X {
                assert!(text.is_empty());
                assert!(output.platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::CopyText(value) if value == "editable name")));
            }
            if pressed == Key::V {
                assert!(output.viewport_output.values().any(|viewport| {
                    viewport
                        .commands
                        .contains(&egui::ViewportCommand::RequestPaste)
                }));
            }
            assert_eq!(app.commands.scene().objects().len(), 2);
            assert_eq!(app.commands.scene().history_state_id(), history);
        }
        let mut input = egui::RawInput {
            events: vec![egui::Event::Paste("pasted field".into())],
            ..Default::default()
        };
        app.raw_input_hook(&ctx, &mut input);
        let mut output = ctx.run_ui(input, |ui| {
            app.handle_shortcuts(ui.ctx());
            ui.add(egui::TextEdit::singleline(&mut text).id(egui::Id::unique("clipboard-text")));
        });
        output.textures_delta.clear();
        assert_eq!(text, "pasted field");
        assert_eq!(app.commands.scene().objects().len(), 2);
        assert_eq!(app.commands.scene().history_state_id(), history);
    }

    #[test]
    fn editing_shortcuts_respect_text_focus_and_modal_boundaries() {
        fn run_without_presentation(
            ctx: &egui::Context,
            input: egui::RawInput,
            ui: impl FnMut(&mut Ui),
        ) {
            let mut output = ctx.run_ui(input, ui);
            output.textures_delta.clear();
        }
        let mut app = Modeler3dApp {
            settings_path: repository_test_directory("rb04-focus").join("settings.json"),
            ..Default::default()
        };
        app.commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        app.commands.dispatch(CommandId::Copy).expect("clipboard");
        let id = app.commands.scene().objects()[0].object_id().to_owned();
        let history = app.commands.scene().history_state_id();
        let ctx = egui::Context::default();
        let mut text = String::from("editable name");
        run_without_presentation(&ctx, egui::RawInput::default(), |ui| {
            ui.text_edit_singleline(&mut text).request_focus();
        });
        for (key, modifier) in [
            (Key::Delete, false),
            (Key::X, true),
            (Key::V, true),
            (Key::D, true),
            (Key::Z, true),
            (Key::A, true),
        ] {
            let modifiers = egui::Modifiers {
                ctrl: modifier,
                command: modifier,
                ..Default::default()
            };
            let input = egui::RawInput {
                events: vec![
                    egui::Event::ModifiersChanged(modifiers),
                    egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers,
                    },
                ],
                ..Default::default()
            };
            run_without_presentation(&ctx, input, |ui| {
                app.handle_shortcuts(ui.ctx());
                ui.text_edit_singleline(&mut text);
            });
            assert_eq!(app.commands.scene().objects().len(), 1);
            assert_eq!(app.commands.scene().objects()[0].object_id(), id);
            assert_eq!(app.commands.scene().history_state_id(), history);
        }
        let delete_input = || egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::Delete,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }],
            ..Default::default()
        };
        for modal in [CommandId::Mirror, CommandId::Align, CommandId::Arrange] {
            app.placement_command = Some(modal);
            run_without_presentation(&egui::Context::default(), delete_input(), |ui| {
                app.handle_shortcuts(ui.ctx())
            });
            assert_eq!(app.commands.scene().history_state_id(), history);
        }
        app.placement_command = None;
        run_without_presentation(&egui::Context::default(), delete_input(), |ui| {
            app.handle_shortcuts(ui.ctx())
        });
        assert!(app.commands.scene().objects().is_empty());
        app.commands.dispatch(CommandId::Undo).expect("undo Delete");
        assert_eq!(app.commands.scene().objects()[0].object_id(), id);
    }
}
