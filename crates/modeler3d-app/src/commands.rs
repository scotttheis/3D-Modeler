//! Desktop command descriptors and the first application-side command seam.
//!
//! The descriptors mirror the private Python ribbon contract, while
//! [`CommandState`] owns only application state.  Mesh edits remain delegated
//! to `modeler3d-core`; the shell never mutates a mesh directly.

use std::path::{Path, PathBuf};

use modeler3d_core::{
    BlobOptions, BooleanOperation, BooleanOptions, Command as CoreCommand, CoreError,
    CreasePolishOptions, ElasticGrabOptions, GroupedCommand, HollowOptions, Mesh, PrimitiveKind,
    ReplaceMeshCommand, Scene, SceneObject, SetColorCommand, SetMetadataCommand,
    SmoothBrushOptions, TextEmbossOptions, TorusParameters, Transform, TransformObjectCommand,
    build_parametric_torus, load_project, repair_mesh, save_project, torus_metadata,
    torus_parameters_from_metadata,
};
use modeler3d_mcp::{R4Document, ViewState as McpViewState};
use modeler3d_viewport::{
    SNAP_TOLERANCE_FIXED_MM, SnapBounds, SnapError, SnapQuery, SnapSceneSnapshot, SnapTarget,
    SnapTargetKind, select_snap_correction,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::preferences::{Preferences, PreferencesError};
use crate::print_handoff::PrintHandoffError;
use crate::transforms::{self, Alignment, Axis, GroupTransform, TransformError};
use crate::{export, export::ExportError};
use crate::{image_color, image_color::ImageColorError};
use crate::{operations::OperationRegistry, print_prep, print_prep::PrintPrepError};
use crate::{view_state, view_state::ViewStateError};

/// Upper bound on estimated document data cloned into one isolated MCP operation.
pub const MAX_MCP_SCENE_SNAPSHOT_BYTES: usize = 512 * 1024 * 1024;

/// The top-level tabs in the 3D Modeler ribbon.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RibbonTab {
    /// Primitive insertion and asset import commands.
    Insert,
    /// Object-level arrangement commands.
    Object,
    /// Mesh editing and sculpt commands.
    Edit,
    /// Material and color commands.
    Paint,
    /// Camera and display commands.
    View,
}

impl RibbonTab {
    /// Returns tabs in the reference order.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Insert,
            Self::Object,
            Self::Edit,
            Self::Paint,
            Self::View,
        ]
    }

    /// Returns the exact user-facing tab label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Insert => "Insert",
            Self::Object => "Object",
            Self::Edit => "Edit",
            Self::Paint => "Paint",
            Self::View => "View",
        }
    }
}

/// Small artwork keys shared by the ribbon and later theme catalogs.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IconKey {
    /// Folder/import artwork.
    Folder,
    /// Generic object artwork.
    Object,
    /// Primitive artwork.
    Primitive(PrimitiveKind),
    /// Edit/transform artwork.
    Edit,
    /// View artwork.
    View,
    /// Save artwork.
    Save,
    /// Undo artwork.
    Undo,
    /// Redo artwork.
    Redo,
    /// Settings artwork.
    Settings,
    /// Close artwork.
    Close,
    /// X-Ray artwork.
    ExRay,
    /// Boolean subtract artwork.
    Subtract,
}

/// Unit override for formats whose files do not declare physical units.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ImportUnit {
    /// One source coordinate represents one micrometre.
    #[serde(alias = "um", alias = "\u{00b5}m", alias = "\u{03bc}m")]
    Micrometre,
    /// One source coordinate represents one millimetre.
    Millimetre,
    /// One source coordinate represents one centimetre.
    Centimetre,
    /// One source coordinate represents one metre.
    Metre,
    /// One source coordinate represents one inch.
    Inch,
    /// One source coordinate represents one foot.
    Foot,
}

impl ImportUnit {
    /// Returns the bounded unit choices in the reference order.
    #[must_use]
    pub const fn all() -> [Self; 6] {
        [
            Self::Micrometre,
            Self::Millimetre,
            Self::Centimetre,
            Self::Inch,
            Self::Foot,
            Self::Metre,
        ]
    }

    /// Returns the compact UI label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Micrometre => "um",
            Self::Millimetre => "mm",
            Self::Centimetre => "cm",
            Self::Metre => "m",
            Self::Inch => "in",
            Self::Foot => "ft",
        }
    }

    const fn scale_mm(self) -> f32 {
        match self {
            Self::Micrometre => 0.001,
            Self::Millimetre => 1.0,
            Self::Centimetre => 10.0,
            Self::Metre => 1_000.0,
            Self::Inch => 25.4,
            Self::Foot => 304.8,
        }
    }
}

/// A command that can be dispatched by a desktop adapter.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CommandId {
    /// 3D Modeler application menu.
    Modeler3dMenu,
    /// Start a new scene.
    NewScene,
    /// Open a `.3dp` project.
    OpenProject,
    /// Import one supported mesh/model file into the current scene.
    ImportModel,
    /// Save the current project.
    SaveProject,
    /// Save the current project to a selected path.
    SaveProjectAs,
    /// Export the selected object as a mesh.
    Export,
    /// Repair the selected mesh.
    Repair,
    /// Run the print-preparation check.
    PrintPrep,
    /// Open settings.
    Settings,
    /// Close the application.
    Exit,
    /// Insert one catalog primitive.
    AddPrimitive(PrimitiveKind),
    /// Edit a torus using its persisted parameters.
    EditTorus,
    /// Duplicate the current selection.
    Duplicate,
    /// Copy the current selection.
    Copy,
    /// Paste the application clipboard.
    Paste,
    /// Cut the current selection into the application clipboard.
    Cut,
    /// Delete the current selection.
    Delete,
    /// Move the selection onto the build plane.
    Settle,
    /// Mirror the selection on X.
    Mirror,
    /// Measure the selection.
    Measure,
    /// Align the selection to the first selected object.
    Align,
    /// Arrange the selection along X.
    Arrange,
    /// Select every top-level scene object.
    SelectAll,
    /// Clear the top-level scene selection.
    DeselectAll,
    /// Invert the top-level scene selection.
    InvertSelection,
    /// Group the current selection without merging its semantic children.
    Group,
    /// Restore the children of selected groups.
    Ungroup,
    /// Simplify the selected mesh.
    Simplify,
    /// Subdivide the selected mesh.
    Subdivide,
    /// Smooth the selected mesh.
    Smooth,
    /// Activate the persistent elastic-grab brush.
    Grab,
    /// Activate the persistent crease-polish brush.
    Crease,
    /// Activate the persistent smooth brush.
    SmoothBrush,
    /// Activate the persistent blob brush.
    Blob,
    /// Boolean-union the selected target and tool objects.
    Merge,
    /// Apply an emboss operation through the application editor.
    Emboss,
    /// Hollow one selected solid through the application editor.
    Hollow,
    /// Intersect the first two selected objects.
    Intersect,
    /// Subtract the second selected object from the first.
    Subtract,
    /// Open the material editor.
    Material,
    /// Open the paint tool.
    Paint,
    /// Center the camera on the scene.
    CenterView,
    /// Frame the selected objects.
    FrameSelection,
    /// Restore the default camera.
    ResetView,
    /// Look down the top axis.
    ViewTop,
    /// Look down the side axis.
    ViewSide,
    /// Toggle the floor grid.
    ToggleGrid,
    /// Toggle wireframe display.
    ToggleWireframe,
    /// Toggle X-ray display.
    ToggleXray,
    /// Toggle object-landmark and build-floor snapping for Move gestures.
    ToggleSnap,
    /// Undo the last model command.
    Undo,
    /// Redo the last model command.
    Redo,
}

/// A command presentation item in a ribbon row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RibbonItem {
    /// A visible command button.
    Command(CommandDescriptor),
    /// A visual separator between command groups.
    Separator,
}

/// Static presentation information for one command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandDescriptor {
    /// Semantic command identifier.
    pub id: CommandId,
    /// Exact user-facing label.
    pub label: &'static str,
    /// Artwork family used by the theme catalog.
    pub icon: IconKey,
    /// Whether the button keeps a selected state.
    pub checkable: bool,
    /// Initial checked state for checkable buttons.
    pub default_checked: bool,
}

impl CommandDescriptor {
    const fn button(id: CommandId, label: &'static str, icon: IconKey) -> Self {
        Self {
            id,
            label,
            icon,
            checkable: false,
            default_checked: false,
        }
    }

    const fn checked(id: CommandId, label: &'static str, icon: IconKey, checked: bool) -> Self {
        Self {
            id,
            label,
            icon,
            checkable: true,
            default_checked: checked,
        }
    }
}

/// A modal request emitted by a command dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DialogKind {
    /// Select a target before applying a whole-object Edit action.
    EditAction(CommandId),
    /// Select a project to open.
    OpenProject,
    /// Select a mesh or scene file to import.
    ImportModel,
    /// Choose a project save destination.
    SaveProject,
    /// Choose a mesh export destination.
    Export,
    /// Edit application settings.
    Settings,
    /// Review repair parameters.
    Repair,
    /// Review print-preparation results.
    PrintPrep,
    /// Edit a material/color value.
    Material,
    /// Edit torus parameters.
    Torus,
    /// Enter bounded emboss parameters.
    Emboss,
    /// Enter bounded hollow parameters.
    Hollow,
    /// Choose how much to simplify the selected mesh (OF-51).
    Simplify,
    /// Choose whole-object or brushed-area subdivision (OF-52).
    Subdivide,
}

/// Result of dispatching one desktop command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandOutcome {
    /// The command changed model or application state.
    Applied(String),
    /// The command did not mutate state but produced useful status.
    Noop(String),
    /// The shell must present a typed dialog.
    OpenDialog(DialogKind),
    /// The shell must close its native window.
    ExitRequested,
}

/// Errors at the application command boundary.
#[derive(Debug, Error)]
pub enum CommandError {
    /// A portable core command rejected its input.
    #[error(transparent)]
    Core(#[from] CoreError),
    /// Project or export I/O failed.
    #[error("application file operation failed: {0}")]
    Io(#[from] std::io::Error),
    /// A command requiring a selection was invoked without one.
    #[error("{0} requires at least one selected object")]
    SelectionRequired(&'static str),
    /// A command requiring two objects was invoked without two selections.
    #[error("{0} requires at least two selected objects")]
    PairRequired(&'static str),
    /// A torus-only editor was invoked for a non-parametric object.
    #[error("{0} requires a selected parametric torus")]
    InvalidTorusSelection(&'static str),
    /// A path did not meet the bounded desktop contract.
    #[error("invalid application path: {0}")]
    InvalidPath(String),
    /// A multi-object transform rejected its bounded input.
    #[error(transparent)]
    Transform(#[from] TransformError),
    /// The shared viewport snap solver rejected its bounded input.
    #[error(transparent)]
    Snap(#[from] SnapError),
    /// The application camera/view service rejected its bounded input.
    #[error(transparent)]
    ViewState(#[from] ViewStateError),
    /// An isolated desktop worker completed against an obsolete document.
    #[error("active document changed while the operation was running; result was discarded")]
    StaleOperation,
    /// The image-color sampler rejected its bounded input.
    #[error(transparent)]
    ImageColor(#[from] ImageColorError),
    /// The application export selection or format route rejected its input.
    #[error(transparent)]
    Export(#[from] ExportError),
    /// The print-preparation checks rejected their bounded input.
    #[error(transparent)]
    PrintPrep(#[from] PrintPrepError),
    /// Scene-to-slicer staging or launch rejected the request.
    #[error(transparent)]
    PrintHandoff(#[from] PrintHandoffError),
    /// Application settings could not be loaded or saved.
    #[error(transparent)]
    Preferences(#[from] PreferencesError),
}

/// One committed local-space sculpt stroke.
///
/// The shell uses the same value for preview-independent commit routing.  It
/// keeps the operation choice and its bounded parameters together so a brush
/// path can create exactly one scene-history entry.
#[derive(Clone, Debug)]
pub enum BrushStroke {
    /// Elastic Grab center and final local displacement.
    Grab {
        /// Local brush center.
        center_mm: [f32; 3],
        /// Local displacement.
        offset_mm: [f32; 3],
        /// Bounded options.
        options: ElasticGrabOptions,
    },
    /// Crease Polish path.
    Crease {
        /// Local path centers.
        centers_mm: Vec<[f32; 3]>,
        /// Bounded options.
        options: CreasePolishOptions,
    },
    /// Smooth Brush path.
    SmoothBrush {
        /// Local path centers.
        centers_mm: Vec<[f32; 3]>,
        /// Bounded options.
        options: SmoothBrushOptions,
    },
    /// Blob path.
    Blob {
        /// Local path centers.
        centers_mm: Vec<[f32; 3]>,
        /// Bounded options.
        options: BlobOptions,
    },
    /// Simplify of the faces under a brushed path (OF-60): about `percent`
    /// of them go, the area's border stays put.
    SimplifyArea {
        /// Local path centers.
        centers_mm: Vec<[f32; 3]>,
        /// Faces whose centre lies this close to the path are simplified.
        radius_mm: f32,
        /// Share of the area's faces to remove (1-95).
        percent: u32,
    },
    /// Even subdivision of the faces under a brushed path (OF-52).
    SubdivideArea {
        /// Local path centers.
        centers_mm: Vec<[f32; 3]>,
        /// Faces whose centre lies this close to the path are cut.
        radius_mm: f32,
    },
}

impl BrushStroke {
    const fn label(&self) -> &'static str {
        match self {
            Self::Grab { .. } => "Grab",
            Self::Crease { .. } => "Crease",
            Self::SmoothBrush { .. } => "Smooth Brush",
            Self::Blob { .. } => "Blob",
            Self::SubdivideArea { .. } => "Subdivide",
            Self::SimplifyArea { .. } => "Simplify",
        }
    }
}

/// The portable state owned by the desktop adapter.
pub struct CommandState {
    scene: Scene,
    document_generation: u64,
    clipboard: Vec<SceneObject>,
    project_path: Option<PathBuf>,
    dirty: bool,
    saved_history_state_id: u64,
    grid_visible: bool,
    wireframe: bool,
    xray: bool,
    snap_enabled: bool,
    view: ViewOrientation,
    operations: OperationRegistry,
    view_state: view_state::ViewStateController,
    preferences: Preferences,
    last_status: String,
    /// A brief notice for the user, shown once as a toast and then cleared.
    notice: Option<String>,
}

impl Default for CommandState {
    fn default() -> Self {
        Self {
            scene: Scene::new(),
            document_generation: 0,
            clipboard: Vec::new(),
            project_path: None,
            dirty: false,
            saved_history_state_id: 0,
            grid_visible: true,
            wireframe: false,
            xray: false,
            snap_enabled: true,
            view: ViewOrientation::Perspective,
            operations: OperationRegistry::default(),
            view_state: view_state::ViewStateController::default(),
            preferences: Preferences::default(),
            last_status: String::from("Ready"),
            notice: None,
        }
    }
}

/// Identity and save-path state captured before an MCP request attaches or
/// copies the desktop document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct McpDocumentVersion {
    pub(crate) project_id: String,
    pub(crate) revision: u64,
    pub(crate) generation: u64,
    pub(crate) project_path: Option<PathBuf>,
}

/// How a completed isolated MCP operation updates application document state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum McpOperationCommit {
    /// Apply a scene edit through one undoable command and retain the path.
    SceneUpdate,
    /// Replace the opened project and reset document history.
    ProjectReplace,
    /// Record a successful save and its destination path.
    ProjectSave,
}

impl CommandState {
    /// Returns the authoritative scene for renderer/application adapters.
    #[must_use]
    pub const fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Captures the semantic document identity used by MCP stale checks.
    #[must_use]
    pub(crate) fn mcp_document_version(&self) -> McpDocumentVersion {
        McpDocumentVersion {
            project_id: self.scene.project_id().to_owned(),
            revision: self.scene.revision(),
            generation: self.document_generation,
            project_path: self.project_path.clone(),
        }
    }

    /// Moves the live scene into the MCP adapter for one UI-thread request.
    /// The scene is restored before control returns to the event loop, so the
    /// core undo/redo stack remains the same object history for both adapters.
    pub(crate) fn take_mcp_document(&mut self, view: McpViewState) -> R4Document {
        R4Document {
            scene: std::mem::replace(&mut self.scene, Scene::new()),
            project_path: self.project_path.take(),
            dirty: self.dirty,
            view,
            snapping: self.snap_enabled,
            clipboard: if self.clipboard.is_empty() {
                None
            } else {
                Some(std::mem::take(&mut self.clipboard))
            },
        }
    }

    /// Takes an isolated snapshot for a long MCP operation. Long workers never
    /// receive the live scene or its history stack.
    pub(crate) fn snapshot_mcp_document(
        &self,
        view: McpViewState,
    ) -> Result<R4Document, CommandError> {
        let estimated_bytes = estimated_scene_snapshot_bytes(
            &self.scene,
            &self.clipboard,
            self.project_path.as_deref(),
        )
        .ok_or_else(|| CoreError::OperationResourceLimit {
            operation: "MCP scene snapshot".to_owned(),
            reason: "estimated scene size overflowed".to_owned(),
        })?;
        if estimated_bytes > MAX_MCP_SCENE_SNAPSHOT_BYTES {
            return Err(CoreError::OperationResourceLimit {
                operation: "MCP scene snapshot".to_owned(),
                reason: format!(
                    "scene snapshot requires {estimated_bytes} bytes, limit is {MAX_MCP_SCENE_SNAPSHOT_BYTES}"
                ),
            }
            .into());
        }
        let scene = self.scene.isolated_copy();
        Ok(R4Document {
            scene,
            project_path: self.project_path.clone(),
            dirty: self.dirty,
            view,
            snapping: self.snap_enabled,
            clipboard: if self.clipboard.is_empty() {
                None
            } else {
                Some(self.clipboard.clone())
            },
        })
    }

    /// Installs a document returned by a synchronous MCP request after the
    /// adapter relinquishes it.
    pub(crate) fn install_mcp_document(
        &mut self,
        document: R4Document,
        expected: &McpDocumentVersion,
    ) -> Result<(), CommandError> {
        if expected.generation != self.document_generation {
            return Err(CommandError::StaleOperation);
        }
        let next_project_id = document.scene.project_id().to_owned();
        self.scene = document.scene;
        self.project_path = document.project_path;
        self.clipboard = document.clipboard.unwrap_or_default();
        self.snap_enabled = document.snapping;
        self.preferences
            .set_object_snapping_enabled(document.snapping);
        self.grid_visible = document.view.show_grid;
        self.wireframe = document.view.wireframe;
        self.xray = document.view.xray;
        self.view_state.set_state(view_state::ViewState {
            yaw_deg: document.view.yaw_deg,
            pitch_deg: document.view.pitch_deg,
            distance_mm: document.view.distance_mm,
            center_mm: document.view.center_mm,
            display: view_state::ViewDisplayState {
                show_grid: document.view.show_grid,
                wireframe: document.view.wireframe,
                xray: document.view.xray,
            },
        })?;
        if expected.project_id != next_project_id {
            self.document_generation = expected.generation.wrapping_add(1);
            self.saved_history_state_id = self.scene.history_state_id();
            self.dirty = document.dirty;
        } else if !document.dirty {
            self.saved_history_state_id = self.scene.history_state_id();
            self.dirty = false;
        } else {
            self.dirty = self.scene.history_state_id() != self.saved_history_state_id;
        }
        Ok(())
    }

    /// Commits a completed isolated MCP scene on the UI owner thread after
    /// checking the desktop document generation as well as core identity and
    /// revision.
    pub(crate) fn commit_mcp_operation_document(
        &mut self,
        expected: &McpDocumentVersion,
        commit: McpOperationCommit,
        document: R4Document,
    ) -> Result<(), CommandError> {
        if self.document_generation != expected.generation {
            return Err(CommandError::StaleOperation);
        }
        if self.scene.project_id() != expected.project_id
            || self.scene.revision() != expected.revision
            || self.project_path != expected.project_path
        {
            return Err(CommandError::StaleOperation);
        }
        let next_project_id = document.scene.project_id().to_owned();
        if commit == McpOperationCommit::ProjectReplace || next_project_id != expected.project_id {
            self.document_generation = self.document_generation.wrapping_add(1);
            self.scene = document.scene;
            self.project_path = document.project_path;
            self.saved_history_state_id = self.scene.history_state_id();
            self.dirty = document.dirty;
            self.status("Opened MCP project");
        } else {
            let snapshot = document.scene.into_snapshot();
            match snapshot.revision {
                revision if revision == expected.revision => {}
                revision if revision == expected.revision.saturating_add(1) => {
                    self.scene.commit_snapshot_from_operation(
                        &expected.project_id,
                        expected.revision,
                        snapshot,
                    )?;
                }
                _ => return Err(CommandError::StaleOperation),
            }
            self.dirty = self.scene.history_state_id() != self.saved_history_state_id;
            if commit == McpOperationCommit::ProjectSave && !document.dirty {
                self.saved_history_state_id = self.scene.history_state_id();
                self.dirty = false;
                self.project_path = document.project_path;
            }
            self.status("MCP operation completed");
        }
        Ok(())
    }

    /// Returns the document generation captured by isolated desktop jobs.
    #[must_use]
    pub(crate) const fn document_generation(&self) -> u64 {
        self.document_generation
    }

    /// Returns whether the shell has unsaved semantic changes.
    #[must_use]
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Returns the current project path, if one has been selected.
    #[must_use]
    pub fn project_path(&self) -> Option<&Path> {
        self.project_path.as_deref()
    }

    /// Returns the selected top-level object count.
    #[must_use]
    pub fn selected_count(&self) -> usize {
        self.scene.selection().len()
    }

    /// Returns whether the application clipboard has objects available to paste.
    #[must_use]
    pub fn has_clipboard(&self) -> bool {
        !self.clipboard.is_empty()
    }

    /// Selects one top-level object from the object panel without creating a
    /// model-history entry.
    pub fn select_index(&mut self, index: usize) {
        self.scene.set_selection([index]);
    }

    /// Updates top-level selection without creating model history.
    pub fn set_selection_indices(&mut self, indices: impl IntoIterator<Item = usize>) {
        self.scene.set_selection(indices);
    }

    /// Applies ordinary click, Ctrl/Command-click, and Shift-range selection.
    ///
    /// Selection is presentation state, so the operation intentionally does
    /// not create an undo entry or mark the scene dirty.
    pub fn select_index_with_modifiers(
        &mut self,
        index: usize,
        additive: bool,
        range_anchor: Option<usize>,
    ) {
        if index >= self.scene.objects().len() {
            return;
        }
        if let Some(anchor) = range_anchor
            && !additive
            && anchor < self.scene.objects().len()
        {
            let (start, end) = if anchor <= index {
                (anchor, index)
            } else {
                (index, anchor)
            };
            self.scene.set_selection(start..=end);
            return;
        }
        let mut next = if additive {
            self.scene.selection().clone()
        } else {
            std::collections::BTreeSet::new()
        };
        if additive && !next.insert(index) {
            next.remove(&index);
        } else {
            next.insert(index);
        }
        self.scene.set_selection(next);
    }

    /// Sets one object's visibility through the core undoable command.
    pub fn set_visibility(
        &mut self,
        index: usize,
        visible: bool,
    ) -> Result<CommandOutcome, CommandError> {
        let result = self.scene.set_visible(index, visible)?;
        if !result.changed {
            return Ok(CommandOutcome::Noop(String::from("Visibility unchanged")));
        }
        self.dirty = true;
        let message = if visible { "Shown" } else { "Hidden" };
        let message = format!("{message} {}", self.scene.object(index)?.name());
        self.status(message.clone());
        Ok(CommandOutcome::Applied(message))
    }

    /// Selects every top-level object without creating model history.
    pub fn select_all(&mut self) -> CommandOutcome {
        let before = self.scene.selection().clone();
        let object_count = self.scene.objects().len();
        self.scene.set_selection(0..object_count);
        self.selection_outcome("Select all", before != *self.scene.selection())
    }

    /// Clears the top-level selection without creating model history.
    pub fn deselect_all(&mut self) -> CommandOutcome {
        let changed = !self.scene.selection().is_empty();
        self.scene.clear_selection();
        self.selection_outcome("Deselect all", changed)
    }

    /// Inverts the top-level selection without creating model history.
    pub fn invert_selection(&mut self) -> CommandOutcome {
        let before = self.scene.selection().clone();
        let next = (0..self.scene.objects().len())
            .filter(|index| !before.contains(index))
            .collect::<Vec<_>>();
        self.scene.set_selection(next);
        self.selection_outcome("Invert selection", before != *self.scene.selection())
    }

    /// Returns display toggles used by the viewport adapter.
    #[must_use]
    pub const fn display_state(&self) -> DisplayState {
        DisplayState {
            grid_visible: self.grid_visible,
            wireframe: self.wireframe,
            xray: self.xray,
            snap_enabled: self.snap_enabled,
        }
    }

    /// Returns the current shell view orientation.
    #[must_use]
    pub const fn view_orientation(&self) -> ViewOrientation {
        self.view
    }

    /// Returns the latest bounded status text.
    #[must_use]
    pub fn last_status(&self) -> &str {
        &self.last_status
    }

    /// Returns the bounded registry used by desktop background operations.
    #[must_use]
    pub const fn operations(&self) -> &OperationRegistry {
        &self.operations
    }

    /// Returns application preferences owned by this command state.
    #[must_use]
    pub const fn preferences(&self) -> &Preferences {
        &self.preferences
    }

    /// Returns mutable preferences for a persistence adapter.
    pub const fn preferences_mut(&mut self) -> &mut Preferences {
        &mut self.preferences
    }

    /// Returns the live Move snapping preference used by the native shell.
    #[must_use]
    pub const fn snap_enabled(&self) -> bool {
        self.snap_enabled
    }

    /// Applies a persisted Move snapping preference without creating model
    /// history. Desktop settings use this during startup.
    pub fn set_snap_enabled(&mut self, enabled: bool) {
        self.snap_enabled = enabled;
        self.preferences.set_object_snapping_enabled(enabled);
    }

    /// Loads persisted application preferences through the typed settings seam.
    pub fn load_preferences_from_path(&mut self, path: &Path) -> Result<(), CommandError> {
        self.preferences = Preferences::load_from_path(path)?;
        Ok(())
    }

    /// Saves application preferences through the typed settings seam.
    pub fn save_preferences_to_path(&self, path: &Path) -> Result<(), CommandError> {
        self.preferences.save_to_path(path)?;
        Ok(())
    }

    /// Returns the semantic camera and display state.
    #[must_use]
    pub fn view_state(&self) -> view_state::ViewState {
        self.view_state.state()
    }

    /// Frames the scene through the shared camera service.
    pub fn frame_view(
        &mut self,
        target: view_state::FrameTarget,
    ) -> Result<view_state::ViewState, CommandError> {
        Ok(self.view_state.frame(&self.scene, target)?)
    }

    /// Dispatches a semantic command without knowing anything about egui.
    pub fn dispatch(&mut self, command: CommandId) -> Result<CommandOutcome, CommandError> {
        match command {
            CommandId::Modeler3dMenu => Ok(CommandOutcome::Noop(String::from("3D Modeler menu"))),
            CommandId::NewScene => {
                self.document_generation = self.document_generation.wrapping_add(1);
                self.scene = Scene::new();
                self.project_path = None;
                self.saved_history_state_id = self.scene.history_state_id();
                self.dirty = false;
                self.status("New scene");
                Ok(CommandOutcome::Applied(String::from("New scene")))
            }
            CommandId::OpenProject => Ok(CommandOutcome::OpenDialog(DialogKind::OpenProject)),
            CommandId::ImportModel => Ok(CommandOutcome::OpenDialog(DialogKind::ImportModel)),
            CommandId::SaveProject => {
                if let Some(path) = self.project_path.clone() {
                    self.save_document_to_path(&path)?;
                    Ok(CommandOutcome::Applied(format!("Saved {}", path.display())))
                } else {
                    Ok(CommandOutcome::OpenDialog(DialogKind::SaveProject))
                }
            }
            CommandId::SaveProjectAs => Ok(CommandOutcome::OpenDialog(DialogKind::SaveProject)),
            CommandId::Export => Ok(CommandOutcome::OpenDialog(DialogKind::Export)),
            CommandId::Repair => Ok(CommandOutcome::OpenDialog(DialogKind::Repair)),
            CommandId::PrintPrep => Ok(CommandOutcome::OpenDialog(DialogKind::PrintPrep)),
            CommandId::Settings => Ok(CommandOutcome::OpenDialog(DialogKind::Settings)),
            CommandId::Exit => Ok(CommandOutcome::ExitRequested),
            CommandId::AddPrimitive(kind) => self.add_primitive(kind),
            CommandId::Duplicate => self.mutate_selected("Duplicate", Scene::duplicate_selected),
            CommandId::Copy => {
                self.clipboard = self.scene.copy_selected()?;
                self.status("Copied selection");
                Ok(CommandOutcome::Applied(String::from("Copied selection")))
            }
            CommandId::Paste => {
                if self.clipboard.is_empty() {
                    return Err(CommandError::SelectionRequired("Paste"));
                }
                // Beside the original, then beside each previous copy (OF-43).
                self.scene.paste_beside(&mut self.clipboard)?;
                self.dirty = true;
                self.status("Paste");
                Ok(CommandOutcome::Applied(String::from("Paste")))
            }
            CommandId::Cut => self.cut_selected(),
            CommandId::Delete => self.mutate_selected("Delete", Scene::remove_selected),
            CommandId::Settle => self.settle_selected(),
            CommandId::Mirror => self.mirror_selected(),
            CommandId::Measure => self.measure_selected(),
            CommandId::Align => self.align_selected(),
            CommandId::Arrange => self.arrange_selected(),
            CommandId::SelectAll => Ok(self.select_all()),
            CommandId::DeselectAll => Ok(self.deselect_all()),
            CommandId::InvertSelection => Ok(self.invert_selection()),
            CommandId::Group => self.group_selected(),
            CommandId::Ungroup => self.ungroup_selected(),
            CommandId::Simplify => {
                self.first_selected("Simplify")?;
                Ok(CommandOutcome::OpenDialog(DialogKind::Simplify))
            }
            CommandId::Subdivide => {
                self.first_selected("Subdivide")?;
                Ok(CommandOutcome::OpenDialog(DialogKind::Subdivide))
            }
            CommandId::Smooth => self.mutate_one_selected("Smooth", |scene, index| {
                scene.apply_smooth(index, 1).map(|(result, _)| result)
            }),
            CommandId::Grab => self.apply_center_brush("Grab", |scene, index, center| {
                scene
                    .apply_elastic_grab(
                        index,
                        center,
                        [0.0, 0.0, 2.0],
                        ElasticGrabOptions::default(),
                    )
                    .map(|(result, _)| result)
            }),
            CommandId::Crease => self.apply_center_brush("Crease", |scene, index, center| {
                scene
                    .apply_crease_polish(index, &[center], CreasePolishOptions::default())
                    .map(|(result, _)| result)
            }),
            CommandId::SmoothBrush => {
                self.apply_center_brush("Smooth Brush", |scene, index, center| {
                    scene
                        .apply_smooth_brush(index, &[center], SmoothBrushOptions::default())
                        .map(|(result, _)| result)
                })
            }
            CommandId::Blob => self.apply_center_brush("Blob", |scene, index, center| {
                scene
                    .apply_blob(index, &[center], BlobOptions::default())
                    .map(|(result, _)| result)
            }),
            CommandId::Merge => self.boolean_selected(BooleanOperation::Union),
            CommandId::Emboss => Ok(CommandOutcome::OpenDialog(DialogKind::Emboss)),
            CommandId::Hollow => Ok(CommandOutcome::OpenDialog(DialogKind::Hollow)),
            CommandId::Intersect => self.boolean_selected(BooleanOperation::Intersection),
            CommandId::Subtract => self.boolean_selected(BooleanOperation::Difference),
            CommandId::Material | CommandId::Paint => {
                Ok(CommandOutcome::OpenDialog(DialogKind::Material))
            }
            CommandId::EditTorus => {
                self.selected_torus_parameters()?;
                Ok(CommandOutcome::OpenDialog(DialogKind::Torus))
            }
            CommandId::CenterView => {
                self.frame_view(view_state::FrameTarget::All)?;
                self.view = ViewOrientation::Perspective;
                self.status("Center View");
                Ok(CommandOutcome::Applied(String::from("Center View")))
            }
            CommandId::FrameSelection => {
                if self.scene.selection().is_empty() {
                    return Err(CommandError::SelectionRequired("Frame"));
                }
                self.frame_view(view_state::FrameTarget::Selected)?;
                self.view = ViewOrientation::FrameSelection;
                self.status("Frame");
                Ok(CommandOutcome::Applied(String::from("Frame")))
            }
            CommandId::ResetView => {
                self.view_state.reset();
                self.view = ViewOrientation::Perspective;
                self.status("Reset View");
                Ok(CommandOutcome::Applied(String::from("Reset View")))
            }
            CommandId::ViewTop => {
                self.view_state.preset("top")?;
                self.view = ViewOrientation::Top;
                self.status("Top");
                Ok(CommandOutcome::Applied(String::from("Top")))
            }
            CommandId::ViewSide => {
                self.view_state.preset("side")?;
                self.view = ViewOrientation::Side;
                self.status("Side");
                Ok(CommandOutcome::Applied(String::from("Side")))
            }
            CommandId::Undo => {
                if self.scene.undo()?.is_some() {
                    self.dirty = self.scene.history_state_id() != self.saved_history_state_id;
                    self.status("Undo");
                    Ok(CommandOutcome::Applied(String::from("Undo")))
                } else {
                    Ok(CommandOutcome::Noop(String::from("Nothing to undo")))
                }
            }
            CommandId::Redo => {
                if self.scene.redo()?.is_some() {
                    self.dirty = self.scene.history_state_id() != self.saved_history_state_id;
                    self.status("Redo");
                    Ok(CommandOutcome::Applied(String::from("Redo")))
                } else {
                    Ok(CommandOutcome::Noop(String::from("Nothing to redo")))
                }
            }
            CommandId::ToggleGrid => {
                self.grid_visible = !self.grid_visible;
                self.sync_view_display();
                self.status(if self.grid_visible {
                    "Grid on"
                } else {
                    "Grid off"
                });
                Ok(CommandOutcome::Applied(self.last_status.clone()))
            }
            CommandId::ToggleWireframe => {
                self.wireframe = !self.wireframe;
                self.sync_view_display();
                self.status(if self.wireframe {
                    "Wireframe on"
                } else {
                    "Wireframe off"
                });
                Ok(CommandOutcome::Applied(self.last_status.clone()))
            }
            CommandId::ToggleXray => {
                self.xray = !self.xray;
                self.sync_view_display();
                self.status(if self.xray { "X-Ray on" } else { "X-Ray off" });
                Ok(CommandOutcome::Applied(self.last_status.clone()))
            }
            CommandId::ToggleSnap => {
                self.snap_enabled = !self.snap_enabled;
                self.preferences
                    .set_object_snapping_enabled(self.snap_enabled);
                self.status(if self.snap_enabled {
                    "Snap on"
                } else {
                    "Snap off"
                });
                Ok(CommandOutcome::Applied(self.last_status.clone()))
            }
        }
    }

    /// Opens a project through the portable core loader.
    pub fn open_from_path(&mut self, path: &Path) -> Result<(), CommandError> {
        validate_path(path)?;
        if !has_extension(path, "3dp") {
            return Err(CommandError::InvalidPath(
                "only .3dp project files can be opened".to_owned(),
            ));
        }
        let loaded = load_project(path)?;
        self.document_generation = self.document_generation.wrapping_add(1);
        self.scene = loaded;
        self.project_path = Some(path.to_path_buf());
        self.preferences.recent_files_mut().record(path);
        self.saved_history_state_id = self.scene.history_state_id();
        self.dirty = false;
        self.status(format!("Opened {}", path.display()));
        Ok(())
    }

    /// Imports a supported mesh or scene file as one atomic, selected history
    /// command.  The portable core owns format parsing; this desktop seam only
    /// maps imported items to scene objects and preserves their millimetre
    /// transforms and representative colors.
    pub fn import_from_path(&mut self, path: &Path) -> Result<String, CommandError> {
        self.import_from_path_with_unit(path, self.preferences.default_import_unit())
    }

    /// Imports a path using an explicit unit for unitless mesh formats.
    pub fn import_from_path_with_unit(
        &mut self,
        path: &Path,
        unit: ImportUnit,
    ) -> Result<String, CommandError> {
        validate_path(path)?;
        // A model opened into an empty, unsaved scene is the document: Save
        // writes it back (Save As is for a new file). Importing into a scene
        // that already has content just adds to it.
        let opens_as_document = self.scene.objects().is_empty()
            && self.project_path.is_none()
            && (has_extension(path, "stl") || has_extension(path, "3mf"));
        let imported = modeler3d_core::import_model(&mut self.scene, path, unit.scale_mm())?;
        let count = imported.len();
        self.preferences.recent_files_mut().record(path);
        if opens_as_document {
            self.project_path = Some(path.to_path_buf());
        }
        self.dirty = true;
        let message = format!("Imported {count} object(s)");
        self.status(message.clone());
        Ok(message)
    }

    /// Saves the current project through the portable core writer.
    pub fn save_to_path(&mut self, path: &Path) -> Result<(), CommandError> {
        let path = project_save_path(path)?;
        save_project(&self.scene, &path)?;
        self.project_path = Some(path.clone());
        self.preferences.recent_files_mut().record(&path);
        self.saved_history_state_id = self.scene.history_state_id();
        self.dirty = false;
        self.status(format!("Saved {}", path.display()));
        Ok(())
    }

    /// Saves the document in the format its extension names (owner requests
    /// OF-23/OF-24): `.stl` (also the default when no extension is given) or
    /// `.3mf` write every object as one mesh file; `.3dp` writes a project.
    /// The saved path becomes the document path, so a later Save rewrites it
    /// in the same format.
    pub fn save_document_to_path(&mut self, path: &Path) -> Result<(), CommandError> {
        let path = document_save_path(path)?;
        if has_extension(&path, "3dp") {
            return self.save_to_path(&path);
        }
        if self.scene.objects().is_empty() {
            return Err(ExportError::EmptyScene.into());
        }
        // Like a project save, create a missing parent folder.
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        export::save_scene_document(&self.scene, &path)?;
        self.project_path = Some(path.clone());
        self.preferences.recent_files_mut().record(&path);
        self.saved_history_state_id = self.scene.history_state_id();
        self.dirty = false;
        self.status(format!("Saved {}", path.display()));
        Ok(())
    }

    /// Writes a `.3dp` project copy without changing the document path or the
    /// saved state; this is Export's "3D Modeler project" type (OF-25).
    pub fn export_project_copy(&mut self, path: &Path) -> Result<(), CommandError> {
        let path = project_save_path(path)?;
        save_project(&self.scene, &path)?;
        self.status(format!("Exported {}", path.display()));
        Ok(())
    }

    /// Exports the first selected object through the core mesh writer.
    pub fn export_to_path(&mut self, path: &Path) -> Result<(), CommandError> {
        let report = export::export_scene(&self.scene, path)?;
        self.status(format!("Exported {}", path.display()));
        debug_assert!(report.objects_exported > 0);
        Ok(())
    }

    /// Repairs the first selected mesh through the bounded core repair path.
    pub fn repair_selected(&mut self) -> Result<String, CommandError> {
        let index = self.first_selected("Repair")?;
        let source = self.scene.object(index)?.mesh().clone();
        let result = repair_mesh(&source)?;
        let unchanged_message = if result.report.nothing_to_fix() {
            Some(modeler3d_core::REPAIR_NOTHING_TO_FIX_MESSAGE)
        } else if result.report.refused() {
            Some(modeler3d_core::REPAIR_REFUSED_MESSAGE)
        } else {
            None
        };
        if let Some(message) = unchanged_message {
            self.status(message);
            return Ok(message.to_owned());
        }
        let command_result = self.scene.replace_mesh(index, result.mesh)?;
        let message = if command_result.changed {
            self.dirty = true;
            format!("Repair: {} stage(s)", result.report.stages.len())
        } else {
            String::from("Repair: no changes")
        };
        self.status(message.clone());
        Ok(message)
    }

    /// Applies one bounded RGBA material color to the complete selection as a
    /// single undoable action.
    pub fn set_selected_color(&mut self, color: [f32; 4]) -> Result<(), CommandError> {
        let indices = self.selected_indices("Color")?;
        let commands = indices
            .iter()
            .map(|index| Box::new(SetColorCommand::new(*index, color)) as Box<dyn CoreCommand>)
            .collect::<Vec<_>>();
        let outcome = self.scene.execute(GroupedCommand::new(commands))?;
        if outcome.changed {
            self.dirty = true;
            self.status(format!("Color applied to {} object(s)", indices.len()));
        } else {
            self.status("Color unchanged");
        }
        Ok(())
    }

    /// One Paint click (OF-44): paints the object at `index` (coverage 1) or
    /// the region around `face` that `coverage` selects, as one undo entry.
    /// MCP `paint` makes the same core call.
    pub fn paint_at(
        &mut self,
        index: usize,
        face: usize,
        coverage: f32,
        color: [f32; 4],
    ) -> Result<CommandOutcome, CommandError> {
        let result = if coverage >= 1.0 {
            self.scene.paint_object(index, color)?
        } else {
            self.scene.paint_region(index, face, coverage, color)?
        };
        let name = self.scene.object(index)?.name().to_owned();
        if !result.changed {
            let message = format!("{name} already has that colour there");
            self.status(message.clone());
            return Ok(CommandOutcome::Noop(message));
        }
        self.dirty = true;
        let message = format!("Painted {name}");
        self.status(message.clone());
        Ok(CommandOutcome::Applied(message))
    }

    /// Commits an isolated mesh result only while its document, revision, and
    /// durable target identity still match the snapshot used by the worker.
    pub(crate) fn replace_mesh_if_current(
        &mut self,
        expected_generation: u64,
        expected_revision: u64,
        object_id: &str,
        mesh: Mesh,
    ) -> Result<(), CommandError> {
        if self.document_generation != expected_generation
            || self.scene.revision() != expected_revision
        {
            return Err(CommandError::StaleOperation);
        }
        let index = self
            .scene
            .object_index(object_id)
            .map_err(|_| CommandError::StaleOperation)?;
        if self.scene.objects()[index].object_id() != object_id {
            return Err(CommandError::StaleOperation);
        }
        let outcome = self.scene.replace_mesh(index, mesh)?;
        if outcome.changed {
            self.dirty = true;
        }
        Ok(())
    }

    /// Samples an image's average color and applies it to the complete
    /// selection as one ordinary scene-history command. This does not map a
    /// texture.
    pub fn set_selected_color_from_image(
        &mut self,
        path: &Path,
        sample_step: usize,
    ) -> Result<String, CommandError> {
        validate_path(path)?;
        let color = image_color::average_image_color(path, sample_step)?;
        self.set_selected_color(color)?;
        let message = format!(
            "Average image color applied to {} object(s); no texture was created",
            self.selected_count()
        );
        self.status(message.clone());
        Ok(message)
    }

    /// Covers the whole selection with a picture (OF-46) as one undo entry.
    /// With no path, the selection's current texture gets the new repeat
    /// size. MCP `apply_texture` makes the same core calls.
    pub fn apply_texture_to_selection(
        &mut self,
        path: Option<&Path>,
        tile_mm: f32,
    ) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices("Texture")?;
        let image = match path {
            Some(path) => {
                validate_path(path)?;
                modeler3d_core::import_texture_file(path)?
            }
            None => indices
                .iter()
                .find_map(|index| {
                    self.scene
                        .object(*index)
                        .ok()
                        .and_then(|object| modeler3d_core::resolved_texture(object.metadata()))
                })
                .map(|(image, _)| image)
                .ok_or_else(|| {
                    CommandError::Core(CoreError::InvalidTexture(
                        "choose a picture first".to_owned(),
                    ))
                })?,
        };
        let result = self.scene.apply_texture(&indices, &image, tile_mm)?;
        if !result.changed {
            let message = String::from("Texture unchanged");
            self.status(message.clone());
            return Ok(CommandOutcome::Noop(message));
        }
        self.dirty = true;
        let message = format!(
            "Texture applied to {} object(s), repeating every {:.0} mm",
            indices.len(),
            modeler3d_core::clamp_tile_mm(tile_mm)
        );
        self.status(message.clone());
        Ok(CommandOutcome::Applied(message))
    }

    /// Removes the selection's textures as one undo entry; their colour
    /// shows again.
    pub fn remove_texture_from_selection(&mut self) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices("Remove texture")?;
        let result = self.scene.remove_texture(&indices)?;
        if !result.changed {
            let message = String::from("No texture to remove");
            self.status(message.clone());
            return Ok(CommandOutcome::Noop(message));
        }
        self.dirty = true;
        let message = format!("Texture removed from {} object(s)", indices.len());
        self.status(message.clone());
        Ok(CommandOutcome::Applied(message))
    }

    /// Applies one atomic world-space transform to the current selection.
    pub fn transform_selected(
        &mut self,
        operation: GroupTransform,
        pivot_mm: Option<[f32; 3]>,
    ) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices("Transform")?;
        let changed =
            transforms::apply_group_transform(&mut self.scene, &indices, operation, pivot_mm)?;
        if !changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to transform")));
        }
        self.dirty = true;
        self.status("Transform");
        Ok(CommandOutcome::Applied(String::from("Transform")))
    }

    pub(crate) fn capture_transform_selection(
        &self,
    ) -> Result<crate::transform_gesture::TransformSelectionSnapshot, CommandError> {
        let mut indices = self.selected_indices("Transform")?;
        indices.sort_unstable();
        if indices.len() > modeler3d_viewport::MAX_RENDER_OBJECTS {
            return Err(CoreError::InvalidTransform("too many transform targets".into()).into());
        }
        let objects = indices
            .into_iter()
            .map(|i| {
                self.scene
                    .object(i)
                    .map(|object| (object.object_id().to_owned(), *object.transform()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(crate::transform_gesture::TransformSelectionSnapshot {
            revision: self.scene.revision(),
            document_generation: self.document_generation,
            selection: self.scene.selection().clone(),
            objects,
        })
    }

    /// Captures the immutable object/floor landmark set used by a Move drag.
    /// Mesh bounds are measured once here; pointer updates never rescan mesh
    /// vertices and moving IDs are excluded from the target set.
    pub(crate) fn capture_viewport_snap_snapshot(
        &self,
    ) -> Result<modeler3d_viewport::SnapSceneSnapshot, CommandError> {
        let indices = self.selected_indices("Snap")?;
        let moving_bounds = transforms::world_bounds(&self.scene, &indices)?;
        let moving_ids = indices
            .iter()
            .map(|index| {
                self.scene
                    .object(*index)
                    .map(|object| object.object_id().to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let selected = indices
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let mut targets = Vec::new();
        for (scene_order, object) in self.scene.objects().iter().enumerate() {
            if selected.contains(&scene_order) || !object.is_visible() {
                continue;
            }
            let bounds = transforms::world_bounds(&self.scene, &[scene_order])?;
            targets.push(SnapTarget {
                object_id: object.object_id().to_owned(),
                scene_order,
                bounds: SnapBounds::new(bounds.minimum, bounds.maximum)?,
            });
        }
        Ok(SnapSceneSnapshot::new(
            SnapBounds::new(moving_bounds.minimum, moving_bounds.maximum)?,
            moving_ids,
            targets,
            true,
        ))
    }

    pub(crate) fn commit_transform_snapshot(
        &mut self,
        snapshot: &crate::transform_gesture::TransformSelectionSnapshot,
        delta: Transform,
    ) -> Result<CommandOutcome, CommandError> {
        if snapshot.revision != self.scene.revision()
            || snapshot.document_generation != self.document_generation
        {
            return Err(CoreError::InvalidTransform(
                "scene changed during viewport gesture; gesture cancelled".into(),
            )
            .into());
        }
        if snapshot.selection.len() != snapshot.objects.len() {
            return Err(
                CoreError::InvalidTransform("invalid gesture target snapshot".into()).into(),
            );
        }
        let indices = snapshot
            .selection
            .iter()
            .copied()
            .zip(&snapshot.objects)
            .map(|(index, (id, transform))| {
                // An unchanged generation/revision retains ordering; validate
                // durable identity as well as the starting pose in linear time.
                if self.scene.object(index)?.object_id() != id {
                    return Err(CoreError::ObjectId(
                        "gesture target no longer exists".into(),
                    ));
                }
                if self.scene.object(index)?.transform() != transform {
                    return Err(CoreError::InvalidTransform(
                        "transform changed during gesture".into(),
                    ));
                }
                Ok(index)
            })
            .collect::<Result<Vec<_>, CoreError>>()?;
        if !transforms::apply_world_matrix(&mut self.scene, &indices, delta)? {
            return Ok(CommandOutcome::Noop("Transform unchanged".into()));
        }
        self.dirty = true;
        self.status("Transform");
        Ok(CommandOutcome::Applied("Transform".into()))
    }

    /// Sets the selected object's world-space position through one history
    /// command.
    pub fn set_selected_position(
        &mut self,
        position_mm: [f32; 3],
    ) -> Result<CommandOutcome, CommandError> {
        let index = self.first_selected("Position")?;
        if self.selected_count() > 1 {
            let indices = self.selected_indices("Position")?;
            let center = transforms::world_bounds(&self.scene, &indices)?.center();
            return self.transform_selected(
                GroupTransform::Translation(std::array::from_fn(|i| position_mm[i] - center[i])),
                None,
            );
        }
        if !transforms::set_position(&mut self.scene, index, position_mm)? {
            return Ok(CommandOutcome::Noop(String::from("Nothing to position")));
        }
        self.dirty = true;
        self.status("Position");
        Ok(CommandOutcome::Applied(String::from("Position")))
    }

    /// Sets one object by its durable ID through one history command.
    ///
    /// The lookup is performed at commit time so viewport gestures remain
    /// attached to the object selected at drag start even if scene order
    /// changes while the gesture is active.
    pub fn set_object_position_by_id(
        &mut self,
        object_id: &str,
        position_mm: [f32; 3],
    ) -> Result<CommandOutcome, CommandError> {
        let index = self.scene.object_index(object_id)?;
        if !transforms::set_position(&mut self.scene, index, position_mm)? {
            return Ok(CommandOutcome::Noop(String::from("Nothing to position")));
        }
        self.dirty = true;
        self.status("Move");
        Ok(CommandOutcome::Applied(String::from("Move")))
    }

    /// Sets the selected object's world-space size through one history
    /// command.
    pub fn set_selected_size(&mut self, size_mm: [f32; 3]) -> Result<CommandOutcome, CommandError> {
        let index = self.first_selected("Size")?;
        if self.selected_count() > 1 {
            let indices = self.selected_indices("Size")?;
            let extent = transforms::world_bounds(&self.scene, &indices)?.extents();
            if extent.iter().any(|v| *v <= f32::EPSILON) {
                return Err(CoreError::InvalidTransform(
                    "cannot resize a zero world dimension".into(),
                )
                .into());
            }
            return self.transform_selected(
                GroupTransform::Scale(std::array::from_fn(|i| size_mm[i] / extent[i])),
                None,
            );
        }
        if !transforms::set_size(&mut self.scene, index, size_mm)? {
            return Ok(CommandOutcome::Noop(String::from("Nothing to size")));
        }
        self.dirty = true;
        self.status("Size");
        Ok(CommandOutcome::Applied(String::from("Size")))
    }

    /// Sizes along the chosen frame, using the existing atomic transform command.
    pub(crate) fn set_selected_size_in_basis(
        &mut self,
        size_mm: [f32; 3],
        object_basis: bool,
    ) -> Result<CommandOutcome, CommandError> {
        if !object_basis {
            return self.set_selected_size(size_mm);
        }
        let snapshot = self.capture_transform_selection()?;
        // Match the model order used by viewport bounds and drag snapshots.
        let indices: Vec<_> = snapshot.selection.iter().copied().collect();
        let frame =
            crate::transform_gesture::ResizeFrame::resize_selection(&self.scene, &indices, true)?;
        let pivot = if indices.len() == 1 {
            self.scene.object(indices[0])?.position()
        } else {
            frame.center
        };
        let delta = frame.resize_delta(size_mm, pivot)?;
        self.commit_transform_snapshot(&snapshot, delta)
    }

    /// Sets the selected object's position and size as one history command.
    pub fn set_selected_position_and_size(
        &mut self,
        position_mm: [f32; 3],
        size_mm: [f32; 3],
    ) -> Result<CommandOutcome, CommandError> {
        let index = self.first_selected("Transform")?;
        if !transforms::set_position_and_size(&mut self.scene, index, position_mm, size_mm)? {
            return Ok(CommandOutcome::Noop(String::from("Nothing to transform")));
        }
        self.dirty = true;
        self.status("Transform");
        Ok(CommandOutcome::Applied(String::from("Transform")))
    }

    /// Rotates the selected objects by one bounded world-space delta.
    pub fn rotate_selected(
        &mut self,
        rotation_delta_deg: [f32; 3],
    ) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices("Rotate")?;
        let changed = transforms::apply_group_transform(
            &mut self.scene,
            &indices,
            GroupTransform::Rotation(rotation_delta_deg),
            None,
        )?;
        if !changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to rotate")));
        }
        self.dirty = true;
        self.status("Rotate");
        Ok(CommandOutcome::Applied(String::from("Rotate")))
    }

    /// Renames the selected object as one identity-preserving history action.
    pub fn rename_selected(&mut self, name: String) -> Result<CommandOutcome, CommandError> {
        let index = self.first_selected("Rename")?;
        let changed = self
            .scene
            .execute(modeler3d_core::RenameObjectCommand::new(index, name))?
            .changed;
        if !changed {
            return Ok(CommandOutcome::Noop(String::from("Name unchanged")));
        }
        self.dirty = true;
        Ok(CommandOutcome::Applied(String::from("Renamed")))
    }

    /// Sets absolute Roll/Pitch/Yaw while retaining signed stretch and position.
    pub fn set_selected_rotation(
        &mut self,
        degrees: [f32; 3],
    ) -> Result<CommandOutcome, CommandError> {
        let index = self.first_selected("Rotate")?;
        let current = *self.scene.object(index)?.transform();
        if self.selected_count() > 1 {
            let angles = crate::rotation::degrees(&current)?;
            return self.rotate_selected(std::array::from_fn(|i| degrees[i] - angles[i]));
        }
        let target = crate::rotation::with_degrees(&current, degrees)?;
        if current
            .rows()
            .iter()
            .flatten()
            .zip(target.rows().iter().flatten())
            .all(|(a, b)| (*a - *b).abs() <= 1e-5 * a.abs().max(1.0))
        {
            return Ok(CommandOutcome::Noop(String::from("Nothing to rotate")));
        }
        self.scene
            .execute(TransformObjectCommand::new(index, target))?;
        self.dirty = true;
        Ok(CommandOutcome::Applied(String::from("Rotate")))
    }

    /// Scales the selected objects by one bounded world-space factor.
    pub fn scale_selected(
        &mut self,
        scale_factor: [f32; 3],
    ) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices("Scale")?;
        let changed = transforms::apply_group_transform(
            &mut self.scene,
            &indices,
            GroupTransform::Scale(scale_factor),
            None,
        )?;
        if !changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to scale")));
        }
        self.dirty = true;
        self.status("Scale");
        Ok(CommandOutcome::Applied(String::from("Scale")))
    }

    /// Arranges the selected objects along X with a user-supplied gap.
    pub fn arrange_selected_with_gap(
        &mut self,
        gap_mm: f32,
    ) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices("Arrange")?;
        if indices.len() < 2 {
            return Ok(CommandOutcome::Noop(String::from("Nothing to arrange")));
        }
        let changed = transforms::arrange(&mut self.scene, &indices, Axis::X, gap_mm)?;
        if !changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to arrange")));
        }
        self.dirty = true;
        self.status("Arrange");
        Ok(CommandOutcome::Applied(String::from("Arrange")))
    }

    /// Applies reference axis/landmark placement as one undoable command.
    pub fn place_selected(
        &mut self,
        command: CommandId,
        axis: Axis,
        alignment: Alignment,
        gap_mm: f32,
    ) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices("Placement")?;
        if matches!(command, CommandId::Align | CommandId::Arrange)
            && indices.len() < modeler3d_core::PLACEMENT_MIN_OBJECTS
        {
            return Ok(CommandOutcome::Noop(
                modeler3d_core::PLACEMENT_NEEDS_TWO_MESSAGE.to_owned(),
            ));
        }
        let changed = match command {
            CommandId::Mirror => transforms::mirror(&mut self.scene, &indices, axis)?,
            CommandId::Align => transforms::align(&mut self.scene, &indices, axis, alignment)?,
            CommandId::Arrange => transforms::arrange(&mut self.scene, &indices, axis, gap_mm)?,
            _ => return Ok(CommandOutcome::Noop(String::from("No placement requested"))),
        };
        if !changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to place")));
        }
        self.dirty = true;
        Ok(CommandOutcome::Applied(String::from("Placement applied")))
    }

    /// Applies one shared viewport snap correction to the current selection.
    pub fn snap_selected_translation(
        &mut self,
        axis: Axis,
        proposed_delta_mm: f32,
        tolerance_mm: Option<f32>,
    ) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices("Snap")?;
        let moving_bounds = transforms::world_bounds(&self.scene, &indices)?;
        let moving_ids = indices
            .iter()
            .map(|index| {
                self.scene
                    .object(*index)
                    .map(|object| object.object_id().to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let selected = indices
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let mut targets = Vec::new();
        for (index, object) in self.scene.objects().iter().enumerate() {
            if selected.contains(&index) || !object.is_visible() {
                continue;
            }
            let bounds = transforms::world_bounds(&self.scene, &[index])?;
            targets.push(SnapTarget {
                object_id: object.object_id().to_owned(),
                scene_order: index,
                bounds: SnapBounds::new(bounds.minimum, bounds.maximum)?,
            });
        }
        let snapshot = SnapSceneSnapshot::new(
            SnapBounds::new(moving_bounds.minimum, moving_bounds.maximum)?,
            moving_ids,
            targets,
            true,
        );
        let mut vector = [0.0; 3];
        vector[axis.index()] = 1.0;
        let result = select_snap_correction(SnapQuery {
            snapshot: &snapshot,
            axis: vector,
            proposed_delta: proposed_delta_mm,
            previous_delta: 0.0,
            tolerance_mm,
            tolerance_mode: SNAP_TOLERANCE_FIXED_MM,
            view_context: None,
            floor_y: Some(0.0),
            excluded: None,
        })?;
        vector[axis.index()] = result.applied_delta;
        let changed = transforms::apply_group_transform(
            &mut self.scene,
            &indices,
            GroupTransform::Translation(vector),
            None,
        )?;
        if !changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to snap")));
        }
        self.dirty = true;
        let message = if result.snapped {
            match result.target_kind {
                Some(SnapTargetKind::Floor) => String::from("Snapped to floor"),
                Some(SnapTargetKind::Origin) => String::from("Snapped to origin"),
                Some(SnapTargetKind::Object) | None => String::from("Snapped to object"),
            }
        } else {
            String::from("Translated")
        };
        self.status(message.clone());
        Ok(CommandOutcome::Applied(message))
    }

    /// Cuts the current selection into the application clipboard with one
    /// grouped scene-history entry.
    pub fn cut_selected(&mut self) -> Result<CommandOutcome, CommandError> {
        self.selected_indices("Cut")?;
        let clipboard = self.scene.copy_selected()?;
        let result = self.scene.remove_selected()?;
        if !result.changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to cut")));
        }
        let count = clipboard.len();
        self.clipboard = clipboard;
        self.dirty = true;
        let message = format!("Cut {count} object(s)");
        self.status(message.clone());
        Ok(CommandOutcome::Applied(message))
    }

    /// Groups the current selection through the portable scene command.
    pub fn group_selected(&mut self) -> Result<CommandOutcome, CommandError> {
        self.selected_indices("Group")?;
        let result = self.scene.group_selected()?;
        if !result.changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to group")));
        }
        self.dirty = true;
        self.status("Group");
        Ok(CommandOutcome::Applied(String::from("Group")))
    }

    /// Ungroups selected group objects through the portable scene command.
    pub fn ungroup_selected(&mut self) -> Result<CommandOutcome, CommandError> {
        self.selected_indices("Ungroup")?;
        if !self.scene.selection_contains_group() {
            return Ok(CommandOutcome::Noop(String::from(
                "Nothing to ungroup: select a group",
            )));
        }
        let result = self.scene.ungroup_selected()?;
        if !result.changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to ungroup")));
        }
        self.dirty = true;
        self.status("Ungroup");
        Ok(CommandOutcome::Applied(String::from("Ungroup")))
    }

    /// Returns the selected object's persisted parametric torus parameters.
    pub fn selected_torus_parameters(&self) -> Result<TorusParameters, CommandError> {
        let index = self.first_selected("Edit Torus")?;
        torus_parameters_from_metadata(self.scene.object(index)?.metadata())?
            .ok_or(CommandError::InvalidTorusSelection("Edit Torus"))
    }

    /// Returns whether the current first selection has editable torus metadata.
    #[must_use]
    pub fn has_selected_parametric_torus(&self) -> bool {
        self.scene
            .selection()
            .iter()
            .next()
            .and_then(|index| self.scene.object(*index).ok())
            .and_then(|object| torus_parameters_from_metadata(object.metadata()).ok())
            .flatten()
            .is_some()
    }

    /// Rebuilds a selected torus and commits mesh plus metadata as one core
    /// history command.
    pub fn update_selected_torus(
        &mut self,
        parameters: TorusParameters,
    ) -> Result<String, CommandError> {
        let index = self.first_selected("Edit Torus")?;
        if torus_parameters_from_metadata(self.scene.object(index)?.metadata())?.is_none() {
            return Err(CommandError::InvalidTorusSelection("Edit Torus"));
        }
        let (mesh, canonical) = build_parametric_torus(parameters)?;
        let base_metadata = self.scene.object(index)?.metadata().clone();
        let metadata = torus_metadata(canonical, Some(base_metadata))?;
        let commands: Vec<Box<dyn CoreCommand>> = vec![
            Box::new(ReplaceMeshCommand::new(index, mesh)),
            Box::new(SetMetadataCommand::new(index, metadata)),
        ];
        self.scene.execute(GroupedCommand::new(commands))?;
        self.dirty = true;
        self.status("Torus updated");
        Ok(String::from("Torus updated"))
    }

    /// Commits one sculpt stroke as one history command.
    pub fn apply_brush_stroke(
        &mut self,
        stroke: BrushStroke,
    ) -> Result<CommandOutcome, CommandError> {
        let label = stroke.label();
        let index = self.first_selected(label)?;
        let result = match stroke {
            BrushStroke::Grab {
                center_mm,
                offset_mm,
                options,
            } => self
                .scene
                .apply_elastic_grab(index, center_mm, offset_mm, options)
                .map(|(result, report)| (result, report.changed, report.affected_vertices)),
            BrushStroke::Crease {
                centers_mm,
                options,
            } => self
                .scene
                .apply_crease_polish(index, &centers_mm, options)
                .map(|(result, report)| (result, report.changed, report.affected_vertices)),
            BrushStroke::SmoothBrush {
                centers_mm,
                options,
            } => self
                .scene
                .apply_smooth_brush(index, &centers_mm, options)
                .map(|(result, report)| (result, report.changed, report.affected_vertices)),
            BrushStroke::Blob {
                centers_mm,
                options,
            } => self
                .scene
                .apply_blob(index, &centers_mm, options)
                .map(|(result, report)| (result, report.changed, report.affected_vertices)),
            BrushStroke::SimplifyArea {
                centers_mm,
                radius_mm,
                percent,
            } => {
                let faces = modeler3d_core::faces_near_path(
                    self.scene.object(index)?.mesh(),
                    &centers_mm,
                    radius_mm,
                );
                if faces.is_empty() {
                    return Ok(CommandOutcome::Noop(
                        "Simplify: the stroke touched no faces".to_owned(),
                    ));
                }
                let (result, report) = self.scene.apply_simplify_area(index, percent, &faces)?;
                if !result.changed {
                    return Ok(CommandOutcome::Noop(format!(
                        "Simplify: this area can't lose any faces without changing its shape ({} faces)",
                        faces.len()
                    )));
                }
                self.dirty = true;
                let mut message = format!(
                    "Simplify: {} \u{2192} {} faces",
                    crate::desktop_jobs::group_thousands(report.before_faces),
                    crate::desktop_jobs::group_thousands(report.after_faces)
                );
                if report.after_faces > report.target_faces {
                    message.push_str(" (stopped early to keep the shape)");
                }
                self.status(message.clone());
                return Ok(CommandOutcome::Applied(message));
            }
            BrushStroke::SubdivideArea {
                centers_mm,
                radius_mm,
            } => {
                let faces = modeler3d_core::faces_near_path(
                    self.scene.object(index)?.mesh(),
                    &centers_mm,
                    radius_mm,
                );
                if faces.is_empty() {
                    return Ok(CommandOutcome::Noop(
                        "Subdivide: the stroke touched no faces".to_owned(),
                    ));
                }
                self.scene
                    .apply_subdivide_even(index, Some(&faces))
                    .map(|(result, report)| {
                        (
                            result,
                            report.changed,
                            report.after_faces - report.before_faces,
                        )
                    })
            }
        }?;
        if !result.0.changed || !result.1 {
            return Ok(CommandOutcome::Noop(format!("{label}: no geometry change")));
        }
        self.dirty = true;
        let message = format!("{label}: {} affected", result.2);
        self.status(message.clone());
        Ok(CommandOutcome::Applied(message))
    }

    /// Applies a boolean to explicit target/tool indices.  The shell uses
    /// this seam for the reference pick workflow; ordinary command dispatch
    /// still obtains the ordered pair from selection.
    pub fn apply_boolean_pair(
        &mut self,
        target_index: usize,
        tool_index: usize,
        operation: BooleanOperation,
        clear_selection: bool,
    ) -> Result<CommandOutcome, CommandError> {
        let outcome = self.apply_product_boolean_pair(target_index, tool_index, operation)?;
        if clear_selection && matches!(outcome, CommandOutcome::Applied(_)) {
            self.scene.set_selection(std::iter::empty::<usize>());
        }
        Ok(outcome)
    }

    fn apply_product_boolean_pair(
        &mut self,
        target_index: usize,
        tool_index: usize,
        operation: BooleanOperation,
    ) -> Result<CommandOutcome, CommandError> {
        // The core owns the OF-9 Merge color rule and the D8 rule for
        // objects that are not touching, so MCP gets the same result.
        let outcome = self.scene.apply_product_boolean(
            target_index,
            tool_index,
            operation,
            BooleanOptions::default(),
        )?;
        self.finish_boolean(operation, outcome)
    }

    /// Copies a boolean's operands so the desktop can compute it on a worker
    /// thread. Nothing changes until [`Self::commit_boolean_pair`].
    pub(crate) fn prepare_boolean_pair(
        &self,
        target_index: usize,
        tool_index: usize,
        operation: BooleanOperation,
    ) -> Result<modeler3d_core::PreparedBoolean, CommandError> {
        Ok(self.scene.prepare_product_boolean(
            target_index,
            tool_index,
            operation,
            BooleanOptions::default(),
        )?)
    }

    /// Applies a boolean computed on a worker thread, exactly as
    /// [`Self::apply_boolean_pair`] would have. Fails without changes if the
    /// scene changed while it ran.
    pub(crate) fn commit_boolean_pair(
        &mut self,
        prepared: modeler3d_core::PreparedBoolean,
        result: modeler3d_core::BooleanResult,
        clear_selection: bool,
    ) -> Result<CommandOutcome, CommandError> {
        let operation = prepared.operation();
        let outcome = self.scene.commit_boolean(prepared, result)?;
        let outcome = self.finish_boolean(operation, outcome)?;
        if clear_selection && matches!(outcome, CommandOutcome::Applied(_)) {
            self.scene.set_selection(std::iter::empty::<usize>());
        }
        Ok(outcome)
    }

    fn finish_boolean(
        &mut self,
        operation: BooleanOperation,
        outcome: modeler3d_core::ProductBooleanOutcome,
    ) -> Result<CommandOutcome, CommandError> {
        let (command_result, report) = match outcome {
            modeler3d_core::ProductBooleanOutcome::Applied { result, report, .. } => {
                (result, report)
            }
            modeler3d_core::ProductBooleanOutcome::GroupedApart { result, .. } => {
                if !result.changed {
                    return Ok(CommandOutcome::Noop(String::from("Nothing to group")));
                }
                self.dirty = true;
                let message = modeler3d_core::MERGE_NOT_TOUCHING_MESSAGE.to_owned();
                self.notice = Some(message.clone());
                self.status(message.clone());
                return Ok(CommandOutcome::Applied(message));
            }
        };
        log_boolean_timings(&report);
        if !command_result.changed {
            crate::diagnostics::record_warning(
                "boolean no change",
                "the core reported no geometry change; the objects may not overlap",
            );
            return Ok(CommandOutcome::Noop(String::from(
                "Boolean made no geometry change",
            )));
        }
        self.dirty = true;
        let label = boolean_label(operation);
        let message = format!("{label}: {} face(s)", report.result_after.triangles);
        self.status(message.clone());
        Ok(CommandOutcome::Applied(message))
    }

    /// Applies the reference text relief through one core history command.
    pub fn emboss_text_selected(
        &mut self,
        options: TextEmbossOptions,
    ) -> Result<String, CommandError> {
        let index = self.first_selected("Emboss")?;
        let (_, report) = self.scene.apply_text_emboss(index, options)?;
        self.dirty = true;
        let message = format!(
            "Embossed {} at {:.2} mm",
            report.occupied_cells, report.depth_mm
        );
        self.status(message.clone());
        Ok(message)
    }

    /// Applies the reference constant-thickness shell through one core
    /// history command.
    pub fn hollow_selected(&mut self, wall_thickness_mm: f32) -> Result<String, CommandError> {
        let index = self.first_selected("Hollow")?;
        let (_, report) = self.scene.apply_hollow(
            index,
            HollowOptions {
                wall_thickness_mm,
                ..HollowOptions::default()
            },
        )?;
        self.dirty = true;
        let message = format!("Hollowed with a {:.1} mm wall", report.wall_thickness_mm);
        self.status(message.clone());
        Ok(message)
    }

    /// Runs the complete print-preparation report with a selected bed size.
    pub fn print_prep_report(
        &self,
        bed_size_mm: f32,
    ) -> Result<print_prep::PrintPrepReport, CommandError> {
        Ok(print_prep::full_check(&self.scene, bed_size_mm)?)
    }

    /// Returns a deterministic print-preparation summary for the dialog.
    pub fn print_prep_summary(&mut self) -> Result<String, CommandError> {
        let report = self.print_prep_report(print_prep::DEFAULT_BED_SIZE_MM)?;
        let message = format!("Print prep: {}", report.summary());
        self.status(message.clone());
        Ok(message)
    }

    /// Replaces the project path after a successful export dialog action.
    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status(message);
    }

    fn add_primitive(&mut self, kind: PrimitiveKind) -> Result<CommandOutcome, CommandError> {
        let (_, _) = self.scene.add_primitive(kind, None, None)?;
        let index = self.scene.objects().len().saturating_sub(1);
        self.scene.set_selection([index]);
        self.dirty = true;
        let label = primitive_label(kind);
        self.status(format!("Added {label}"));
        if kind == PrimitiveKind::Torus {
            Ok(CommandOutcome::OpenDialog(DialogKind::Torus))
        } else {
            Ok(CommandOutcome::Applied(self.last_status.clone()))
        }
    }

    fn settle_selected(&mut self) -> Result<CommandOutcome, CommandError> {
        let mut indices = self.selected_indices("Settle")?;
        indices.sort_unstable();
        let changed = transforms::settle(&mut self.scene, &indices)?;
        if !changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to settle")));
        }
        self.dirty = true;
        self.status("Settle");
        Ok(CommandOutcome::Applied(String::from("Settle")))
    }

    fn measure_selected(&mut self) -> Result<CommandOutcome, CommandError> {
        let mut indices = self.selected_indices("Measure")?;
        indices.sort_unstable();
        let mut parts = Vec::with_capacity(indices.len());
        for index in indices {
            let object = self.scene.object(index)?;
            let dimensions = transforms::world_bounds(&self.scene, &[index])?.extents();
            parts.push(format!(
                "{}\n  Width: {:.2} mm\n  Depth: {:.2} mm\n  Height: {:.2} mm",
                object.name(),
                dimensions[0],
                dimensions[2],
                dimensions[1]
            ));
        }
        let message = parts.join("\n\n");
        self.status(message.clone());
        Ok(CommandOutcome::Applied(message))
    }

    fn align_selected(&mut self) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices("Align")?;
        let changed = transforms::align(&mut self.scene, &indices, Axis::X, Alignment::Center)?;
        if !changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to align")));
        }
        self.dirty = true;
        self.status("Align");
        Ok(CommandOutcome::Applied(String::from("Align")))
    }

    fn arrange_selected(&mut self) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices("Arrange")?;
        if indices.len() < 2 {
            return Ok(CommandOutcome::Noop(String::from("Nothing to arrange")));
        }
        let changed = transforms::arrange(&mut self.scene, &indices, Axis::X, 0.0)?;
        if !changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to arrange")));
        }
        self.dirty = true;
        self.status("Arrange");
        Ok(CommandOutcome::Applied(String::from("Arrange")))
    }

    fn mirror_selected(&mut self) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices("Mirror")?;
        let changed = transforms::mirror(&mut self.scene, &indices, Axis::X)?;
        if !changed {
            return Ok(CommandOutcome::Noop(String::from("Nothing to mirror")));
        }
        self.dirty = true;
        self.status("Mirror");
        Ok(CommandOutcome::Applied(String::from("Mirror")))
    }

    fn boolean_selected(
        &mut self,
        operation: BooleanOperation,
    ) -> Result<CommandOutcome, CommandError> {
        let indices = self.selected_indices(match operation {
            BooleanOperation::Intersection => "Intersect",
            BooleanOperation::Difference => "Subtract",
            BooleanOperation::Union => "Merge",
        })?;
        if indices.len() < 2 {
            return Err(CommandError::PairRequired(match operation {
                BooleanOperation::Intersection => "Intersect",
                BooleanOperation::Difference => "Subtract",
                BooleanOperation::Union => "Merge",
            }));
        }
        self.apply_boolean_pair(indices[0], indices[1], operation, false)
    }

    fn mutate_one_selected<F>(
        &mut self,
        label: &'static str,
        operation: F,
    ) -> Result<CommandOutcome, CommandError>
    where
        F: FnOnce(&mut Scene, usize) -> Result<modeler3d_core::CommandResult, CoreError>,
    {
        let index = self.first_selected(label)?;
        if !operation(&mut self.scene, index)?.changed {
            return Ok(CommandOutcome::Noop(format!("{label}: no geometry change")));
        }
        self.dirty = true;
        self.status(label);
        Ok(CommandOutcome::Applied(String::from(label)))
    }

    fn apply_center_brush<F>(
        &mut self,
        label: &'static str,
        operation: F,
    ) -> Result<CommandOutcome, CommandError>
    where
        F: FnOnce(&mut Scene, usize, [f32; 3]) -> Result<modeler3d_core::CommandResult, CoreError>,
    {
        let index = self.first_selected(label)?;
        let mesh = self.scene.object(index)?.mesh();
        let Some((minimum, maximum)) = mesh.bounds() else {
            return Err(CoreError::InvalidMesh(String::from("selected mesh has no bounds")).into());
        };
        let center = [
            (minimum[0] + maximum[0]) * 0.5,
            (minimum[1] + maximum[1]) * 0.5,
            (minimum[2] + maximum[2]) * 0.5,
        ];
        if !operation(&mut self.scene, index, center)?.changed {
            return Ok(CommandOutcome::Noop(format!("{label}: no geometry change")));
        }
        self.dirty = true;
        self.status(label);
        Ok(CommandOutcome::Applied(String::from(label)))
    }

    fn first_selected(&self, label: &'static str) -> Result<usize, CommandError> {
        self.scene
            .selection()
            .iter()
            .next()
            .copied()
            .ok_or(CommandError::SelectionRequired(label))
    }

    fn selected_indices(&self, label: &'static str) -> Result<Vec<usize>, CommandError> {
        let indices = self.scene.ordered_selection();
        if indices.is_empty() {
            return Err(CommandError::SelectionRequired(label));
        }
        Ok(indices)
    }

    fn mutate_selected<F>(
        &mut self,
        label: &'static str,
        operation: F,
    ) -> Result<CommandOutcome, CommandError>
    where
        F: FnOnce(&mut Scene) -> Result<modeler3d_core::CommandResult, CoreError>,
    {
        if self.scene.selection().is_empty() {
            return Err(CommandError::SelectionRequired(label));
        }
        if !operation(&mut self.scene)?.changed {
            return Ok(CommandOutcome::Noop(format!("{label}: no change")));
        }
        self.dirty = true;
        self.status(label);
        Ok(CommandOutcome::Applied(String::from(label)))
    }

    fn status(&mut self, message: impl Into<String>) {
        self.last_status = message.into();
    }

    /// Takes the pending one-off notice, if any, for the shell to show as a
    /// toast.
    pub(crate) fn take_notice(&mut self) -> Option<String> {
        self.notice.take()
    }

    fn sync_view_display(&mut self) {
        self.view_state.set_display(view_state::ViewDisplayState {
            show_grid: self.grid_visible,
            wireframe: self.wireframe,
            xray: self.xray,
        });
    }

    fn selection_outcome(&mut self, label: &'static str, changed: bool) -> CommandOutcome {
        if changed {
            self.status(label);
            CommandOutcome::Applied(String::from(label))
        } else {
            CommandOutcome::Noop(format!("Nothing to {label}"))
        }
    }
}

/// Viewport display flags owned by the application adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DisplayState {
    /// Whether the build-plane grid is visible.
    pub grid_visible: bool,
    /// Whether wireframe overlay is requested.
    pub wireframe: bool,
    /// Whether translucent X-ray display is requested.
    pub xray: bool,
    /// Whether Move landmark/floor snapping is enabled.
    pub snap_enabled: bool,
}

/// Camera intent owned by the shell and consumed by the viewport adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViewOrientation {
    /// The default perspective view.
    Perspective,
    /// A top-axis view.
    Top,
    /// A side-axis view.
    Side,
    /// A selection-framed perspective view.
    FrameSelection,
}

/// Returns the menu commands from the Python reference contract.
#[must_use]
pub fn file_menu_spec() -> Vec<RibbonItem> {
    vec![
        command(CommandId::Modeler3dMenu, "3D Modeler", IconKey::Object),
        RibbonItem::Separator,
        command(CommandId::ImportModel, "Import", IconKey::Folder),
        command(CommandId::NewScene, "New Scene", IconKey::Object),
        command(CommandId::OpenProject, "Open Project", IconKey::Folder),
        command(CommandId::SaveProject, "Save", IconKey::Save),
        command(CommandId::SaveProjectAs, "Save As", IconKey::Save),
        command(CommandId::Export, "Export", IconKey::Save),
        RibbonItem::Separator,
        command(CommandId::Repair, "Repair", IconKey::Edit),
        command(CommandId::PrintPrep, "Print Prep Check", IconKey::Edit),
        RibbonItem::Separator,
        command(CommandId::Settings, "Settings", IconKey::Settings),
        RibbonItem::Separator,
        command(CommandId::Exit, "Exit", IconKey::Close),
    ]
}

/// Returns the complete ribbon specification in the reference order.
#[must_use]
pub fn ribbon_spec() -> Vec<(RibbonTab, Vec<RibbonItem>)> {
    vec![
        (
            RibbonTab::Insert,
            vec![
                command(CommandId::ImportModel, "Add/Open", IconKey::Folder),
                RibbonItem::Separator,
                primitive_command(PrimitiveKind::Cube, "Cube"),
                primitive_command(PrimitiveKind::Cylinder, "Cylinder"),
                primitive_command(PrimitiveKind::Pyramid, "Pyramid"),
                primitive_command(PrimitiveKind::Cone, "Cone"),
                primitive_command(PrimitiveKind::Sphere, "Sphere"),
                primitive_command(PrimitiveKind::Torus, "Torus"),
                primitive_command(PrimitiveKind::Hexagon, "Hexagon"),
                primitive_command(PrimitiveKind::Wedge, "Wedge"),
            ],
        ),
        (
            RibbonTab::Object,
            vec![
                command(CommandId::EditTorus, "Edit Torus", IconKey::Edit),
                command(CommandId::Repair, "Repair", IconKey::Edit),
                command(CommandId::Duplicate, "Duplicate", IconKey::Edit),
                command(CommandId::Copy, "Copy", IconKey::Edit),
                command(CommandId::Paste, "Paste", IconKey::Edit),
                command(CommandId::Delete, "Delete", IconKey::Close),
                RibbonItem::Separator,
                command(CommandId::Settle, "Settle", IconKey::Edit),
                command(CommandId::Mirror, "Mirror", IconKey::Edit),
                command(CommandId::Measure, "Measure", IconKey::Edit),
                command(CommandId::Align, "Align", IconKey::Edit),
                command(CommandId::Arrange, "Arrange", IconKey::Edit),
            ],
        ),
        (
            RibbonTab::Edit,
            vec![
                command(CommandId::Simplify, "Simplify", IconKey::Edit),
                command(CommandId::Subdivide, "Subdivide", IconKey::Edit),
                checked_command(CommandId::Grab, "Grab", IconKey::Edit),
                checked_command(CommandId::Crease, "Crease", IconKey::Edit),
                checked_command(CommandId::SmoothBrush, "Smooth Brush", IconKey::Edit),
                checked_command(CommandId::Blob, "Blob", IconKey::Edit),
                // Normalize was removed by the owner (OF-60): Simplify and
                // Subdivide by area cover it. MCP keeps normalize_face_density.
                RibbonItem::Separator,
                command(CommandId::Merge, "Merge", IconKey::Edit),
                // Emboss is hidden until it works correctly (owner, 2026-11-22).
                command(CommandId::Hollow, "Hollow", IconKey::Edit),
                command(CommandId::Intersect, "Intersect", IconKey::Edit),
                command(CommandId::Subtract, "Subtract", IconKey::Subtract),
                command(CommandId::Smooth, "Smooth", IconKey::Edit),
            ],
        ),
        (
            RibbonTab::Paint,
            vec![
                command(CommandId::Material, "Material", IconKey::Edit),
                // The Color button was removed by the owner (OF-45); image
                // colour sampling stays in the Material dialog and MCP.
                command(CommandId::Paint, "Paint", IconKey::Edit),
            ],
        ),
        (
            RibbonTab::View,
            vec![
                command(CommandId::CenterView, "Center View", IconKey::View),
                command(CommandId::FrameSelection, "Frame", IconKey::View),
                command(CommandId::ResetView, "Reset View", IconKey::View),
                RibbonItem::Separator,
                command(CommandId::ViewTop, "Top", IconKey::View),
                command(CommandId::ViewSide, "Side", IconKey::View),
                RibbonItem::Separator,
                checked_command_with_default(CommandId::ToggleGrid, "Grid", IconKey::View, true),
                checked_command(CommandId::ToggleWireframe, "Wireframe", IconKey::View),
                checked_command(CommandId::ToggleXray, "X-Ray", IconKey::ExRay),
                checked_command_with_default(CommandId::ToggleSnap, "Snap", IconKey::View, true),
            ],
        ),
    ]
}

fn command(id: CommandId, label: &'static str, icon: IconKey) -> RibbonItem {
    RibbonItem::Command(CommandDescriptor::button(id, label, icon))
}

fn checked_command(id: CommandId, label: &'static str, icon: IconKey) -> RibbonItem {
    RibbonItem::Command(CommandDescriptor::checked(id, label, icon, false))
}

fn checked_command_with_default(
    id: CommandId,
    label: &'static str,
    icon: IconKey,
    checked: bool,
) -> RibbonItem {
    RibbonItem::Command(CommandDescriptor::checked(id, label, icon, checked))
}

fn primitive_command(kind: PrimitiveKind, label: &'static str) -> RibbonItem {
    command(
        CommandId::AddPrimitive(kind),
        label,
        IconKey::Primitive(kind),
    )
}

fn primitive_label(kind: PrimitiveKind) -> &'static str {
    match kind {
        PrimitiveKind::Cube => "Cube",
        PrimitiveKind::Cylinder => "Cylinder",
        PrimitiveKind::Cone => "Cone",
        PrimitiveKind::Sphere => "Sphere",
        PrimitiveKind::Pyramid => "Pyramid",
        PrimitiveKind::Torus => "Torus",
        PrimitiveKind::Hexagon => "Hexagon",
        PrimitiveKind::Wedge => "Wedge",
        PrimitiveKind::Tetrahedron => "Tetrahedron",
    }
}

/// Writes one log line naming how long each boolean step took, so a slow
/// Merge, Subtract or Intersect shows which step was slow.
fn log_boolean_timings(report: &modeler3d_core::BooleanReport) {
    let steps = report
        .step_timings
        .iter()
        .map(|(step, duration)| format!("{step} {} ms", duration.as_millis()))
        .collect::<Vec<_>>()
        .join("; ");
    crate::diagnostics::record_info(
        "boolean timing",
        format!(
            "{}: {} + {} -> {} faces; {steps}",
            report.operation,
            report.target_before.triangles,
            report.tool_before.triangles,
            report.result_after.triangles
        ),
    );
}

fn boolean_label(operation: BooleanOperation) -> &'static str {
    match operation {
        BooleanOperation::Intersection => "Intersect",
        BooleanOperation::Difference => "Subtract",
        BooleanOperation::Union => "Merge",
    }
}

fn estimated_scene_snapshot_bytes(
    scene: &Scene,
    clipboard: &[SceneObject],
    project_path: Option<&Path>,
) -> Option<usize> {
    let selection_bytes = scene
        .selection()
        .len()
        .checked_mul(std::mem::size_of::<usize>().checked_mul(4)?)?;
    let mut total = scene
        .source_archive_bytes_len()
        .checked_add(scene.project_id().len())?
        .checked_add(selection_bytes)?
        .checked_add(project_path.map_or(0, |path| path.as_os_str().len()))?;
    for object in scene.objects().iter().chain(clipboard) {
        total = estimated_object_snapshot_bytes(object, total)?;
    }
    Some(total)
}

fn estimated_object_snapshot_bytes(object: &SceneObject, total: usize) -> Option<usize> {
    let total = total.checked_add(std::mem::size_of::<SceneObject>())?;
    let total = estimate_mesh_and_metadata_bytes(
        object.mesh(),
        object.object_id(),
        object.name(),
        object.metadata(),
        total,
    )?;
    object.children().iter().try_fold(total, |total, child| {
        estimated_child_snapshot_bytes(child, total)
    })
}

fn estimated_child_snapshot_bytes(
    child: &modeler3d_core::SceneChild,
    total: usize,
) -> Option<usize> {
    let total = total.checked_add(std::mem::size_of::<modeler3d_core::SceneChild>())?;
    let total = estimate_mesh_and_metadata_bytes(
        child.mesh(),
        child.object_id(),
        child.name(),
        child.metadata(),
        total,
    )?;
    child.children().iter().try_fold(total, |total, nested| {
        estimated_child_snapshot_bytes(nested, total)
    })
}

fn estimate_mesh_and_metadata_bytes(
    mesh: &Mesh,
    object_id: &str,
    name: &str,
    metadata: &Value,
    total: usize,
) -> Option<usize> {
    let geometry_bytes = mesh
        .vertex_count()
        .checked_mul(std::mem::size_of::<[f32; 3]>())?
        .checked_add(
            mesh.face_count()
                .checked_mul(std::mem::size_of::<[u32; 3]>())?,
        )?;
    let metadata_bytes = serde_json::to_vec(metadata).ok()?.len();
    total
        .checked_add(std::mem::size_of::<Mesh>())?
        .checked_add(geometry_bytes)?
        .checked_add(metadata_bytes)?
        .checked_add(object_id.len())?
        .checked_add(name.len())
}

fn validate_path(path: &Path) -> Result<(), CommandError> {
    let value = path.to_string_lossy();
    if value.trim().is_empty() || value.len() > 4096 {
        return Err(CommandError::InvalidPath(value.into_owned()));
    }
    Ok(())
}

fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

/// Document saves accept `.stl`, `.3mf` and `.3dp`; no extension means `.stl`.
fn document_save_path(path: &Path) -> Result<PathBuf, CommandError> {
    validate_path(path)?;
    if ["stl", "3mf", "3dp"]
        .iter()
        .any(|extension| has_extension(path, extension))
    {
        return Ok(path.to_path_buf());
    }
    if path.extension().is_some() {
        return Err(CommandError::InvalidPath(
            "Save writes .stl or .3mf files; use Export for other formats".to_owned(),
        ));
    }
    let mut normalized = path.to_path_buf();
    normalized.set_extension("stl");
    Ok(normalized)
}

fn project_save_path(path: &Path) -> Result<PathBuf, CommandError> {
    validate_path(path)?;
    if has_extension(path, "3dp") {
        return Ok(path.to_path_buf());
    }
    if path.extension().is_some() {
        return Err(CommandError::InvalidPath(
            "project files must use the .3dp extension".to_owned(),
        ));
    }
    let mut normalized = path.to_path_buf();
    normalized.set_extension("3dp");
    Ok(normalized)
}

/// Temporary folder for tests. On macOS `$TMPDIR` is under the `/var` symbolic
/// link, which the MCP approved-roots check refuses, so resolve it first.
#[cfg(test)]
fn test_temp_dir() -> std::path::PathBuf {
    let dir = std::env::temp_dir();
    if cfg!(windows) {
        dir
    } else {
        dir.canonicalize().unwrap_or(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn import_fixture(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../modeler3d-core/tests/fixtures")
            .join(relative)
    }

    fn assert_vec3_close(actual: [f32; 3], expected: [f32; 3]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            let tolerance = 1.0e-5_f32.max(expected.abs() * 1.0e-5);
            assert!(
                (actual - expected).abs() <= tolerance,
                "{actual} != {expected} (tolerance {tolerance})"
            );
        }
    }

    fn assert_color_close(actual: [f32; 4], expected: [f32; 4]) {
        for (actual, expected) in actual.into_iter().zip(expected) {
            assert!((actual - expected).abs() <= 0.005, "{actual} != {expected}");
        }
    }

    /// Runs one MCP tool against a desktop `CommandState`, the way the live
    /// bridge does: attach the document, call, hand the document back.
    fn run_mcp(
        commands: &mut CommandState,
        tool: &str,
        arguments: serde_json::Value,
    ) -> serde_json::Value {
        let version = commands.mcp_document_version();
        let roots = modeler3d_mcp::ApprovedRoots::from_paths(Vec::new()).expect("deny-all roots");
        let server = modeler3d_mcp::R4Server::new(modeler3d_mcp::R4Config::for_profile(
            modeler3d_mcp::Profile::Full,
            roots,
        ));
        let mut document = Some(commands.take_mcp_document(modeler3d_mcp::ViewState::default()));
        server.replace_document(&mut document).expect("attach");
        let result = server.call_value(tool, arguments).expect("MCP call");
        let document = server.take_document().expect("detach");
        commands
            .install_mcp_document(document, &version)
            .expect("install");
        result
    }

    fn one_cube_pair() -> (CommandState, CommandState) {
        let mut desktop = CommandState::default();
        let mut mcp = CommandState::default();
        for state in [&mut desktop, &mut mcp] {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("add cube");
        }
        desktop.select_index(0);
        (desktop, mcp)
    }

    #[test]
    fn every_export_format_imports_atomically_and_matches_mcp() {
        let directory = temporary_import_directory("all-export-import");
        let mut source = CommandState::default();
        for _ in 0..2 {
            source
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("cube");
        }
        source.select_all();
        for extension in ["stl", "obj", "ply", "3mf", "gltf", "glb", "wrl", "3dp"] {
            let path = directory.join(format!("model.{extension}"));
            if extension == "3dp" {
                source.export_project_copy(&path).expect("export project");
            } else {
                source.export_to_path(&path).expect("export model");
            }
            let mut desktop = CommandState::default();
            desktop
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .expect("existing object");
            let mut mcp = CommandState::default();
            mcp.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .expect("existing object");
            desktop
                .import_from_path(&path)
                .unwrap_or_else(|e| panic!("{extension}: {e}"));
            let expected_count = if matches!(extension, "stl" | "obj" | "ply") {
                1
            } else {
                2
            };
            assert_eq!(desktop.selected_count(), expected_count, "{extension}");
            let roots =
                modeler3d_mcp::ApprovedRoots::from_paths(vec![directory.clone()]).expect("roots");
            let server = modeler3d_mcp::R4Server::new(modeler3d_mcp::R4Config::for_profile(
                modeler3d_mcp::Profile::Full,
                roots,
            ));
            let version = mcp.mcp_document_version();
            let mut document = Some(mcp.take_mcp_document(modeler3d_mcp::ViewState::default()));
            server.replace_document(&mut document).expect("attach");
            let started = server
                .call_value("import_file", serde_json::json!({"path":path,"unit":"mm"}))
                .expect("MCP import");
            let mut status = serde_json::Value::Null;
            for _ in 0..6000 {
                status = server
                    .call_value(
                        "operation_status",
                        serde_json::json!({"operation_id":started["operation_id"]}),
                    )
                    .expect("status");
                if matches!(
                    status["state"].as_str(),
                    Some("completed" | "failed" | "cancelled")
                ) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert_eq!(status["state"], "completed", "{extension}: {status}");
            mcp.install_mcp_document(server.take_document().expect("detach"), &version)
                .expect("install");
            assert_eq!(
                mcp.selected_count(),
                desktop.selected_count(),
                "{extension}"
            );
            for (a, b) in desktop.scene().objects().iter().zip(mcp.scene().objects()) {
                assert_eq!(a.mesh().vertices, b.mesh().vertices, "{extension}");
                assert_eq!(a.mesh().faces, b.mesh().faces, "{extension}");
                assert_eq!(a.transform(), b.transform(), "{extension}");
                assert_eq!(a.color(), b.color(), "{extension}");
            }
            for state in [&mut desktop, &mut mcp] {
                assert_eq!(state.scene().objects().len(), expected_count + 1);
                state.dispatch(CommandId::Undo).expect("undo");
                assert_eq!(state.scene().objects().len(), 1, "{extension}");
                state.dispatch(CommandId::Redo).expect("redo");
                assert_eq!(state.scene().objects().len(), expected_count + 1);
                assert_eq!(state.project_path(), None);
            }
        }
        std::fs::remove_dir_all(directory).expect("cleanup");
    }

    #[test]
    fn mcp_paint_matches_the_desktop_paint_click() {
        let (mut desktop, mut mcp) = one_cube_pair();
        let history = desktop.scene().history_state_id();
        let red = [1.0, 0.0, 0.0, 1.0];
        // A low-coverage click on face 0 paints one side of the cube.
        assert!(matches!(
            desktop.paint_at(0, 0, 0.0, red),
            Ok(CommandOutcome::Applied(_))
        ));
        let result = run_mcp(
            &mut mcp,
            "paint",
            serde_json::json!({"index":0,"face":0,"coverage":0.0,"color":[1.0,0.0,0.0,1.0]}),
        );
        assert_eq!(
            result["changed_object_ids"].as_array().map(Vec::len),
            Some(1)
        );
        for state in [&desktop, &mcp] {
            let colors = modeler3d_core::object_face_colors(&state.scene().objects()[0])
                .expect("region colours");
            assert_eq!(colors.iter().filter(|c| **c == [255, 0, 0, 255]).count(), 2);
            assert_eq!(state.scene().history_state_id(), history + 1);
        }
        // The same click again changes nothing on either side.
        assert!(matches!(
            desktop.paint_at(0, 0, 0.0, red),
            Ok(CommandOutcome::Noop(_))
        ));
        run_mcp(
            &mut mcp,
            "paint",
            serde_json::json!({"index":0,"face":0,"coverage":0.0,"color":[1.0,0.0,0.0,1.0]}),
        );
        // Full coverage paints the whole object one colour.
        let blue = [0.0, 0.0, 1.0, 1.0];
        assert!(desktop.paint_at(0, 3, 1.0, blue).is_ok());
        run_mcp(
            &mut mcp,
            "paint",
            serde_json::json!({"index":0,"color":[0.0,0.0,1.0,1.0]}),
        );
        for state in [&mut desktop, &mut mcp] {
            assert_eq!(state.scene().objects()[0].color(), blue);
            assert!(modeler3d_core::object_face_colors(&state.scene().objects()[0]).is_none());
            assert_eq!(state.scene().history_state_id(), history + 2);
            assert!(state.dispatch(CommandId::Undo).is_ok());
            assert!(modeler3d_core::object_face_colors(&state.scene().objects()[0]).is_some());
        }
    }

    fn run_mcp_in_root(
        commands: &mut CommandState,
        root: &Path,
        tool: &str,
        arguments: serde_json::Value,
    ) -> serde_json::Value {
        let version = commands.mcp_document_version();
        let roots =
            modeler3d_mcp::ApprovedRoots::from_paths([root.to_path_buf()]).expect("approved root");
        let server = modeler3d_mcp::R4Server::new(modeler3d_mcp::R4Config::for_profile(
            modeler3d_mcp::Profile::Full,
            roots,
        ));
        let mut document = Some(commands.take_mcp_document(modeler3d_mcp::ViewState::default()));
        server.replace_document(&mut document).expect("attach");
        let result = server.call_value(tool, arguments).expect("MCP call");
        let document = server.take_document().expect("detach");
        commands
            .install_mcp_document(document, &version)
            .expect("install");
        result
    }

    #[test]
    fn mcp_texture_tools_match_the_desktop_material_texture() {
        let root = test_temp_dir().join(format!("modeler3d-of46-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&root);
        let picture = root.join("stripes.png");
        let image = image::RgbaImage::from_fn(16, 16, |x, _| {
            if x < 8 {
                image::Rgba([200, 40, 40, 255])
            } else {
                image::Rgba([40, 40, 200, 255])
            }
        });
        assert!(image.save(&picture).is_ok());
        let (mut desktop, mut mcp) = one_cube_pair();
        let history = desktop.scene().history_state_id();
        assert!(matches!(
            desktop.apply_texture_to_selection(Some(&picture), 30.0),
            Ok(CommandOutcome::Applied(_))
        ));
        let result = run_mcp_in_root(
            &mut mcp,
            &root,
            "apply_texture",
            serde_json::json!({"path":picture.display().to_string(),"tile_mm":30.0,"indices":[0]}),
        );
        assert_eq!(
            result["changed_object_ids"].as_array().map(Vec::len),
            Some(1)
        );
        assert_eq!(result["objects"][0]["texture"]["tile_mm"], 30.0);
        let texture_of = |state: &CommandState| {
            modeler3d_core::object_texture(state.scene().objects()[0].metadata())
        };
        assert!(texture_of(&desktop).is_some());
        assert_eq!(texture_of(&desktop), texture_of(&mcp));
        // No picture: the current texture takes the new repeat size.
        assert!(desktop.apply_texture_to_selection(None, 60.0).is_ok());
        run_mcp_in_root(
            &mut mcp,
            &root,
            "apply_texture",
            serde_json::json!({"tile_mm":60.0,"indices":[0]}),
        );
        assert_eq!(texture_of(&desktop), texture_of(&mcp));
        assert!(texture_of(&mcp).is_some_and(|texture| (texture.tile_mm - 60.0).abs() < 1e-4));
        assert!(matches!(
            desktop.remove_texture_from_selection(),
            Ok(CommandOutcome::Applied(_))
        ));
        run_mcp(
            &mut mcp,
            "remove_texture",
            serde_json::json!({"indices":[0]}),
        );
        for state in [&mut desktop, &mut mcp] {
            assert!(texture_of(state).is_none());
            assert_eq!(state.scene().history_state_id(), history + 3);
            assert!(state.dispatch(CommandId::Undo).is_ok());
            assert!(texture_of(state).is_some());
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn mcp_crease_polish_matches_the_desktop_crease_stroke() {
        // OF-49: the reworked Crease is one core function behind both.
        let mut desktop = CommandState::default();
        let mut mcp = CommandState::default();
        for state in [&mut desktop, &mut mcp] {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .expect("add sphere");
        }
        desktop.select_index(0);
        let top = desktop.scene().objects()[0]
            .mesh()
            .vertices
            .iter()
            .copied()
            .fold([0.0_f32; 3], |best, vertex| {
                if vertex[1] > best[1] { vertex } else { best }
            });
        let centers = vec![top, [top[0] + 2.0, top[1] - 0.2, top[2]]];
        let options = CreasePolishOptions {
            radius_mm: 6.0,
            strength: 0.8,
            // OF-59: the Sharpness slider reaches MCP too.
            sharpness: 0.7,
            ..CreasePolishOptions::default()
        };
        assert!(matches!(
            desktop.apply_brush_stroke(BrushStroke::Crease {
                centers_mm: centers.clone(),
                options,
            }),
            Ok(CommandOutcome::Applied(_))
        ));
        run_mcp(
            &mut mcp,
            "crease_polish",
            serde_json::json!({
                "index": 0,
                "centers_mm": centers,
                "radius_mm": 6.0,
                "strength": 0.8,
                "sharpness": 0.7,
            }),
        );
        assert_eq!(
            desktop.scene().objects()[0].mesh(),
            mcp.scene().objects()[0].mesh()
        );
        assert_eq!(
            desktop.scene().history_state_id(),
            mcp.scene().history_state_id()
        );
    }

    #[test]
    fn mcp_smooth_brush_matches_the_desktop_stroke_and_keeps_the_cylinder() {
        // OF-50: one core function behind both; the stock cylinder keeps
        // its height under a strong stroke over its rim.
        let mut desktop = CommandState::default();
        let mut mcp = CommandState::default();
        for state in [&mut desktop, &mut mcp] {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cylinder))
                .expect("add cylinder");
        }
        desktop.select_index(0);
        let mesh = desktop.scene().objects()[0].mesh().clone();
        let height = |mesh: &Mesh| {
            let values = mesh.vertices.iter().map(|vertex| vertex[1]);
            values.clone().fold(f32::MIN, f32::max) - values.fold(f32::MAX, f32::min)
        };
        let rim = mesh
            .vertices
            .iter()
            .copied()
            .fold([0.0_f32; 3], |best, vertex| {
                if vertex[1] + vertex[0] > best[1] + best[0] {
                    vertex
                } else {
                    best
                }
            });
        let centers = vec![rim, [rim[0] - 4.0, rim[1], rim[2] + 4.0]];
        let options = SmoothBrushOptions {
            radius_mm: 15.0,
            strength: 1.0,
            shrinkage: 1.0,
            ..SmoothBrushOptions::default()
        };
        assert!(
            desktop
                .apply_brush_stroke(BrushStroke::SmoothBrush {
                    centers_mm: centers.clone(),
                    options,
                })
                .is_ok()
        );
        run_mcp(
            &mut mcp,
            "smooth_brush",
            serde_json::json!({
                "index": 0,
                "centers_mm": centers,
                "radius_mm": 15.0,
                "strength": 1.0,
                "shrinkage": 1.0,
            }),
        );
        assert_eq!(
            desktop.scene().objects()[0].mesh(),
            mcp.scene().objects()[0].mesh()
        );
        assert!(height(desktop.scene().objects()[0].mesh()) >= height(&mesh) * 0.97);
    }

    #[test]
    fn mcp_rename_object_matches_the_desktop_rename() {
        let (mut desktop, mut mcp) = one_cube_pair();
        let history = desktop.scene().history_state_id();
        assert_eq!(history, mcp.scene().history_state_id());
        let originals = [
            desktop.scene().objects()[0].name().to_owned(),
            mcp.scene().objects()[0].name().to_owned(),
        ];

        assert!(desktop.rename_selected("Bracket".to_owned()).is_ok());
        let result = run_mcp(
            &mut mcp,
            "rename_object",
            serde_json::json!({"index":0,"name":"Bracket"}),
        );
        assert_eq!(
            result["changed_object_ids"].as_array().map(Vec::len),
            Some(1)
        );

        for state in [&desktop, &mcp] {
            assert_eq!(state.scene().objects()[0].name(), "Bracket");
            assert_eq!(state.scene().history_state_id(), history + 1);
            assert!(state.is_dirty());
        }
        // The same name again is a no-op on both sides: no history entry.
        assert!(matches!(
            desktop.rename_selected("Bracket".to_owned()),
            Ok(CommandOutcome::Noop(_))
        ));
        run_mcp(
            &mut mcp,
            "rename_object",
            serde_json::json!({"index":0,"name":"Bracket"}),
        );
        assert_eq!(desktop.scene().history_state_id(), history + 1);
        assert_eq!(mcp.scene().history_state_id(), history + 1);
        // One undo restores the name on both.
        for (state, original) in [&mut desktop, &mut mcp].into_iter().zip(&originals) {
            assert!(state.dispatch(CommandId::Undo).is_ok());
            assert_eq!(state.scene().objects()[0].name(), original.as_str());
        }
    }

    /// Runs a long-running MCP tool to completion against a desktop
    /// `CommandState` and returns the operation's result.
    fn run_mcp_operation(
        commands: &mut CommandState,
        tool: &str,
        arguments: serde_json::Value,
    ) -> serde_json::Value {
        let version = commands.mcp_document_version();
        let roots = modeler3d_mcp::ApprovedRoots::from_paths(Vec::new()).expect("deny-all roots");
        let server = modeler3d_mcp::R4Server::new(modeler3d_mcp::R4Config::for_profile(
            modeler3d_mcp::Profile::Full,
            roots,
        ));
        let mut document = Some(commands.take_mcp_document(modeler3d_mcp::ViewState::default()));
        server.replace_document(&mut document).expect("attach");
        let started = server.call_value(tool, arguments).expect("MCP call");
        let operation_id = started["operation_id"]
            .as_str()
            .expect("operation ID")
            .to_owned();
        let mut status = serde_json::Value::Null;
        for _ in 0..6000 {
            status = server
                .call_value(
                    "operation_status",
                    serde_json::json!({"operation_id": operation_id}),
                )
                .expect("operation status");
            if matches!(
                status["state"].as_str(),
                Some("completed" | "failed" | "cancelled")
            ) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(status["state"], serde_json::json!("completed"), "{status}");
        let document = server.take_document().expect("detach");
        commands
            .install_mcp_document(document, &version)
            .expect("install");
        status["result"].clone()
    }

    #[test]
    fn mcp_repair_of_a_clean_model_matches_the_desktop_and_changes_nothing() {
        let (_, mut mcp) = one_cube_pair();
        let history = mcp.scene().history_state_id();
        let id = mcp.scene().objects()[0].object_id().to_owned();
        let result = run_mcp_operation(&mut mcp, "repair", serde_json::json!({"object_id": id}));
        assert_eq!(result["changed"], serde_json::json!(false), "{result}");
        assert_eq!(
            result["message"].as_str(),
            Some(modeler3d_core::REPAIR_NOTHING_TO_FIX_MESSAGE)
        );
        assert_eq!(mcp.scene().history_state_id(), history);
    }

    fn model_with_unrepairable_hole() -> CommandState {
        let (mut state, _) = one_cube_pair();
        let mut mesh = modeler3d_core::make_sphere(15.0, 1).expect("sphere");
        mesh.faces.retain(|face| !face.contains(&12));
        assert_eq!(
            modeler3d_core::inspect_mesh(&mesh)
                .expect("health")
                .boundary_edges,
            6
        );
        state.scene.replace_mesh(0, mesh).expect("open sphere");
        state.dirty = false;
        state
    }

    #[test]
    fn refused_repair_leaves_desktop_mesh_and_history_unchanged() {
        let mut desktop = model_with_unrepairable_hole();
        let source = desktop.scene().objects()[0].mesh().clone();
        let history = desktop.scene().history_state_id();
        let revision = desktop.scene().revision();
        assert_eq!(
            desktop.repair_selected().expect("refused repair"),
            modeler3d_core::REPAIR_REFUSED_MESSAGE
        );
        assert_eq!(desktop.scene().history_state_id(), history);
        assert_eq!(desktop.scene().revision(), revision);
        assert_eq!(desktop.scene().objects()[0].mesh(), &source);
        assert!(!desktop.is_dirty());
    }

    #[test]
    fn mcp_refused_repair_matches_desktop_and_leaves_history_unchanged() {
        let mut mcp = model_with_unrepairable_hole();
        let source = mcp.scene().objects()[0].mesh().clone();
        let history = mcp.scene().history_state_id();
        let revision = mcp.scene().revision();
        let id = mcp.scene().objects()[0].object_id().to_owned();
        let result = run_mcp_operation(&mut mcp, "repair", serde_json::json!({"object_id": id}));
        assert_eq!(result["changed"], serde_json::json!(false), "{result}");
        assert_eq!(
            result["message"].as_str(),
            Some(modeler3d_core::REPAIR_REFUSED_MESSAGE)
        );
        assert_eq!(result["report"]["verdict"], "NoBetter");
        assert_eq!(
            result["report"]["score_before"],
            result["report"]["score_after"]
        );
        assert_eq!(mcp.scene().history_state_id(), history);
        assert_eq!(mcp.scene().revision(), revision);
        assert_eq!(mcp.scene().objects()[0].mesh(), &source);
        assert!(!mcp.is_dirty());
    }

    #[test]
    fn mcp_simplify_percent_matches_the_desktop_simplify_slider() {
        // OF-51: MCP `percent` runs the desktop slider's core function.
        let mut mcp = CommandState::default();
        mcp.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
            .expect("sphere");
        let source = mcp.scene().objects()[0].mesh().clone();
        let id = mcp.scene().objects()[0].object_id().to_owned();
        let history = mcp.scene().history_state_id();
        let result = run_mcp_operation(
            &mut mcp,
            "simplify",
            serde_json::json!({"object_id": id, "percent": 10}),
        );
        let expected = modeler3d_core::simplify_mesh_by_percent(
            &source,
            10,
            modeler3d_core::MeshOperationLimits::default(),
        )
        .expect("core simplify");
        assert_eq!(result["changed"], serde_json::json!(true), "{result}");
        assert_eq!(mcp.scene().objects()[0].mesh(), &expected.mesh);
        assert_eq!(mcp.scene().history_state_id(), history + 1);

        // A cube cannot lose faces without changing: no change, no history.
        let (_, mut cube) = one_cube_pair();
        let id = cube.scene().objects()[0].object_id().to_owned();
        let history = cube.scene().history_state_id();
        let result = run_mcp_operation(
            &mut cube,
            "simplify",
            serde_json::json!({"object_id": id, "percent": 10}),
        );
        assert_eq!(result["changed"], serde_json::json!(false), "{result}");
        assert_eq!(cube.scene().history_state_id(), history);
    }

    #[test]
    fn mcp_subdivide_modes_match_the_desktop_subdivide_window() {
        // OF-52: `mode: "whole"` is the desktop Whole object; `mode: "area"`
        // with the same path and size is the desktop area brush.
        let mut mcp = CommandState::default();
        mcp.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
            .expect("sphere");
        let source = mcp.scene().objects()[0].mesh().clone();
        let id = mcp.scene().objects()[0].object_id().to_owned();
        let result = run_mcp_operation(
            &mut mcp,
            "subdivide",
            serde_json::json!({"object_id": id, "mode": "whole"}),
        );
        assert_eq!(result["changed"], serde_json::json!(true), "{result}");
        let whole = modeler3d_core::subdivide_mesh_even(
            &source,
            None,
            modeler3d_core::MeshOperationLimits::default(),
        )
        .expect("whole");
        assert_eq!(mcp.scene().objects()[0].mesh(), &whole.mesh);

        let mut desktop = CommandState::default();
        let mut mcp = CommandState::default();
        for state in [&mut desktop, &mut mcp] {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .expect("sphere");
        }
        desktop.select_index(0);
        let top = desktop.scene().objects()[0]
            .mesh()
            .vertices
            .iter()
            .copied()
            .fold([0.0_f32; 3], |best, vertex| {
                if vertex[1] > best[1] { vertex } else { best }
            });
        let centers = vec![top, [top[0] + 3.0, top[1] - 0.3, top[2]]];
        assert!(matches!(
            desktop.apply_brush_stroke(BrushStroke::SubdivideArea {
                centers_mm: centers.clone(),
                radius_mm: 4.0,
            }),
            Ok(CommandOutcome::Applied(_))
        ));
        let id = mcp.scene().objects()[0].object_id().to_owned();
        run_mcp_operation(
            &mut mcp,
            "subdivide",
            serde_json::json!({"object_id": id, "mode": "area", "centers_mm": centers, "radius_mm": 4.0}),
        );
        assert_eq!(
            desktop.scene().objects()[0].mesh(),
            mcp.scene().objects()[0].mesh()
        );
    }

    #[test]
    fn mcp_hollow_of_a_stock_torus_matches_the_desktop_hollow() {
        // OF-54: the stock torus (concave, 20,000 faces) used to exceed the
        // resource limit; both adapters run the same core function.
        let mut desktop = CommandState::default();
        let mut mcp = CommandState::default();
        for state in [&mut desktop, &mut mcp] {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Torus))
                .expect("torus");
        }
        desktop.select_index(0);
        let message = desktop.hollow_selected(1.5).expect("desktop hollow");
        assert_eq!(message, "Hollowed with a 1.5 mm wall");
        let id = mcp.scene().objects()[0].object_id().to_owned();
        let result = run_mcp_operation(
            &mut mcp,
            "hollow",
            serde_json::json!({"object_id": id, "wall_thickness_mm": 1.5}),
        );
        assert_eq!(result["changed"], serde_json::json!(true), "{result}");
        assert_eq!(
            desktop.scene().objects()[0].mesh(),
            mcp.scene().objects()[0].mesh()
        );
    }

    #[test]
    fn hollow_defaults_to_a_2_mm_wall_on_the_desktop_and_in_mcp() {
        // OF-61: the owner's working wall is the default everywhere.
        assert!((HollowOptions::default().wall_thickness_mm - 2.0).abs() < f32::EPSILON);
        let mut desktop = CommandState::default();
        let mut mcp = CommandState::default();
        for state in [&mut desktop, &mut mcp] {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .expect("sphere");
        }
        desktop.select_index(0);
        let message = desktop
            .hollow_selected(HollowOptions::default().wall_thickness_mm)
            .expect("desktop hollow");
        assert_eq!(message, "Hollowed with a 2.0 mm wall");
        let id = mcp.scene().objects()[0].object_id().to_owned();
        let result = run_mcp_operation(&mut mcp, "hollow", serde_json::json!({"object_id": id}));
        assert_eq!(result["changed"], serde_json::json!(true), "{result}");
        assert_eq!(
            desktop.scene().objects()[0].mesh(),
            mcp.scene().objects()[0].mesh()
        );
    }

    #[test]
    fn mcp_simplify_area_matches_the_desktop_simplify_area_brush() {
        // OF-60: `mode: "area"` with the same path, size and percent is the
        // desktop Simplify window's Area I brush over; one Undo entry.
        let mut desktop = CommandState::default();
        let mut mcp = CommandState::default();
        for state in [&mut desktop, &mut mcp] {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .expect("sphere");
        }
        desktop.select_index(0);
        let source = desktop.scene().objects()[0].mesh().clone();
        let top = source
            .vertices
            .iter()
            .copied()
            .fold([0.0_f32; 3], |best, vertex| {
                if vertex[1] > best[1] { vertex } else { best }
            });
        let centers = vec![top, [top[0] + 4.0, top[1] - 0.5, top[2]]];
        let history = desktop.scene().history_state_id();
        assert!(matches!(
            desktop.apply_brush_stroke(BrushStroke::SimplifyArea {
                centers_mm: centers.clone(),
                radius_mm: 8.0,
                percent: 50,
            }),
            Ok(CommandOutcome::Applied(_))
        ));
        assert_ne!(desktop.scene().history_state_id(), history);
        let after = desktop.scene().objects()[0].mesh().face_count();
        assert!(after < source.face_count(), "the area lost faces");
        desktop.dispatch(CommandId::Undo).expect("undo");
        assert_eq!(desktop.scene().objects()[0].mesh(), &source);
        desktop.dispatch(CommandId::Redo).expect("redo");
        let id = mcp.scene().objects()[0].object_id().to_owned();
        run_mcp_operation(
            &mut mcp,
            "simplify",
            serde_json::json!({"object_id": id, "mode": "area", "percent": 50, "centers_mm": centers, "radius_mm": 8.0}),
        );
        assert_eq!(
            desktop.scene().objects()[0].mesh(),
            mcp.scene().objects()[0].mesh()
        );
    }

    #[test]
    fn align_and_arrange_need_two_objects_on_the_desktop_and_in_mcp() {
        let (mut desktop, mut mcp) = one_cube_pair();
        mcp.set_selection_indices([0]);
        let history = desktop.scene().history_state_id();
        for command in [CommandId::Align, CommandId::Arrange] {
            assert!(matches!(
                desktop.place_selected(command, Axis::X, Alignment::Center, 0.0),
                Ok(CommandOutcome::Noop(message))
                    if message == modeler3d_core::PLACEMENT_NEEDS_TWO_MESSAGE
            ));
        }
        for tool in ["align", "arrange"] {
            let refused = try_mcp(&mut mcp, tool, serde_json::json!({"axis": "x"}));
            assert!(
                refused
                    .as_ref()
                    .is_err_and(|error| error.contains(modeler3d_core::PLACEMENT_NEEDS_TWO_MESSAGE)),
                "{tool}: {refused:?}"
            );
        }
        assert_eq!(desktop.scene().history_state_id(), history);
        assert_eq!(mcp.scene().history_state_id(), history);

        // With two objects both sides align.
        for state in [&mut desktop, &mut mcp] {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("second cube");
            let id = state.scene().objects()[1].object_id().to_owned();
            state
                .set_object_position_by_id(&id, [60.0, 20.0, 30.0])
                .expect("offset");
            state.set_selection_indices([0, 1]);
        }
        assert!(matches!(
            desktop.place_selected(CommandId::Align, Axis::Z, Alignment::Center, 0.0),
            Ok(CommandOutcome::Applied(_))
        ));
        let aligned = try_mcp(
            &mut mcp,
            "align",
            serde_json::json!({"axis": "z", "mode": "center"}),
        );
        assert!(aligned.is_ok(), "{aligned:?}");
        for state in [&desktop, &mcp] {
            let z = |index: usize| state.scene().objects()[index].position()[2];
            assert!((z(0) - z(1)).abs() < 1.0e-4, "aligned on Z");
        }
    }

    #[test]
    fn paste_and_duplicate_place_copies_beside_desktop_and_mcp() {
        let (mut desktop, mut mcp) = one_cube_pair();
        mcp.set_selection_indices([0]);
        let start = desktop.scene().objects()[0].position();
        let step = 40.0 + modeler3d_core::COPY_GAP_MM;
        assert!(desktop.dispatch(CommandId::Copy).is_ok());
        run_mcp(&mut mcp, "copy_selected", serde_json::json!({}));
        for _ in 0..2 {
            assert!(desktop.dispatch(CommandId::Paste).is_ok());
            run_mcp(&mut mcp, "paste", serde_json::json!({}));
        }
        for state in [&desktop, &mcp] {
            let positions = state
                .scene()
                .objects()
                .iter()
                .map(|object| object.position())
                .collect::<Vec<_>>();
            assert_eq!(
                positions,
                vec![
                    start,
                    [start[0] + step, start[1], start[2]],
                    [start[0] + 2.0 * step, start[1], start[2]],
                ],
                "beside the original, then beside the previous copy"
            );
        }
        // Duplicate does the same with the selection, and steps on when the
        // copy (now selected) is duplicated again.
        for state in [&mut desktop, &mut mcp] {
            state.set_selection_indices([0]);
        }
        assert!(desktop.dispatch(CommandId::Duplicate).is_ok());
        assert!(desktop.dispatch(CommandId::Duplicate).is_ok());
        run_mcp(&mut mcp, "duplicate_selected", serde_json::json!({}));
        run_mcp(&mut mcp, "duplicate_selected", serde_json::json!({}));
        for state in [&desktop, &mcp] {
            let objects = state.scene().objects();
            assert_eq!(objects.len(), 5);
            assert_eq!(objects[3].position(), [start[0] + step, start[1], start[2]]);
            assert_eq!(
                objects[4].position(),
                [start[0] + 2.0 * step, start[1], start[2]]
            );
        }
    }

    /// Two cubes 100 mm apart on X (not touching), both selected.
    fn two_cubes_apart() -> CommandState {
        let mut state = CommandState::default();
        for _ in 0..2 {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("add cube");
        }
        let id = state.scene().objects()[1].object_id().to_owned();
        let position = state.scene().objects()[1].position();
        state
            .set_object_position_by_id(&id, [100.0, position[1], position[2]])
            .expect("move apart");
        state.set_selection_indices([0, 1]);
        state
    }

    #[test]
    fn merge_of_objects_not_touching_groups_them_with_a_notice_desktop_and_mcp() {
        let mut desktop = two_cubes_apart();
        let mut mcp = two_cubes_apart();
        let history = desktop.scene().history_state_id();

        let outcome = desktop.apply_boolean_pair(0, 1, BooleanOperation::Union, false);
        assert!(
            matches!(&outcome, Ok(CommandOutcome::Applied(message))
                if message == modeler3d_core::MERGE_NOT_TOUCHING_MESSAGE),
            "{outcome:?}"
        );
        assert_eq!(
            desktop.take_notice().as_deref(),
            Some(modeler3d_core::MERGE_NOT_TOUCHING_MESSAGE)
        );
        assert_eq!(desktop.take_notice(), None, "the notice is shown once");

        let ids = [
            mcp.scene().objects()[0].object_id().to_owned(),
            mcp.scene().objects()[1].object_id().to_owned(),
        ];
        let result = run_mcp_operation(
            &mut mcp,
            "boolean_merge",
            serde_json::json!({"target_object_id": ids[0], "tool_object_id": ids[1]}),
        );
        assert_eq!(result["grouped"], serde_json::json!(true), "{result}");
        assert_eq!(
            result["message"].as_str(),
            Some(modeler3d_core::MERGE_NOT_TOUCHING_MESSAGE)
        );

        for state in [&desktop, &mcp] {
            // One group holding both cubes, selected, in one history entry.
            assert_eq!(state.scene().objects().len(), 1);
            assert_eq!(state.scene().objects()[0].children().len(), 2);
            assert!(state.scene().selection_contains_group());
            assert_eq!(state.scene().history_state_id(), history + 1);
        }
        assert_eq!(
            result["object_id"].as_str(),
            Some(mcp.scene().objects()[0].object_id())
        );
        // One undo brings both cubes back on both sides.
        for state in [&mut desktop, &mut mcp] {
            assert!(state.dispatch(CommandId::Undo).is_ok());
            assert_eq!(state.scene().objects().len(), 2);
        }
    }

    #[test]
    fn merge_of_overlapping_objects_still_merges() {
        let mut state = CommandState::default();
        for _ in 0..2 {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("add cube");
        }
        let id = state.scene().objects()[1].object_id().to_owned();
        let position = state.scene().objects()[1].position();
        state
            .set_object_position_by_id(&id, [20.0, position[1], position[2]])
            .expect("overlap");
        let outcome = state.apply_boolean_pair(0, 1, BooleanOperation::Union, false);
        assert!(matches!(outcome, Ok(CommandOutcome::Applied(_))));
        assert_eq!(state.take_notice(), None);
        assert_eq!(state.scene().objects().len(), 1);
        assert!(state.scene().objects()[0].children().is_empty());
    }

    #[test]
    fn ungroup_needs_a_group_on_the_desktop_and_in_mcp() {
        let mut desktop = CommandState::default();
        let mut mcp = CommandState::default();
        for state in [&mut desktop, &mut mcp] {
            for _ in 0..2 {
                state
                    .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                    .expect("add cube");
            }
            state.set_selection_indices([0, 1]);
            assert!(state.dispatch(CommandId::Merge).is_ok());
            // After Merge there is one object and it is not a group.
            assert_eq!(state.scene().objects().len(), 1);
            state.set_selection_indices([0]);
            assert!(!state.scene().selection_contains_group());
        }
        let history = desktop.scene().history_state_id();
        assert!(matches!(
            desktop.dispatch(CommandId::Ungroup),
            Ok(CommandOutcome::Noop(_))
        ));
        assert_eq!(desktop.scene().history_state_id(), history);
        let refused = try_mcp(&mut mcp, "ungroup_selected", serde_json::json!({}));
        assert!(
            refused.as_ref().is_err_and(|error| error.contains("group")),
            "{refused:?}"
        );
        assert_eq!(mcp.scene().history_state_id(), history);

        // With a real group selected both sides ungroup it.
        for state in [&mut desktop, &mut mcp] {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .expect("add sphere");
            state.set_selection_indices([0, 1]);
            assert!(state.dispatch(CommandId::Group).is_ok());
            assert!(state.scene().selection_contains_group());
        }
        assert!(desktop.dispatch(CommandId::Ungroup).is_ok());
        let result = try_mcp(&mut mcp, "ungroup_selected", serde_json::json!({}));
        assert!(result.is_ok(), "{result:?}");
        for state in [&desktop, &mcp] {
            assert_eq!(state.scene().objects().len(), 2);
            assert!(!state.scene().selection_contains_group());
        }
    }

    /// Like [`run_mcp`], but returns the tool's error text instead of
    /// panicking when the call is refused.
    fn try_mcp(
        commands: &mut CommandState,
        tool: &str,
        arguments: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let version = commands.mcp_document_version();
        let roots = modeler3d_mcp::ApprovedRoots::from_paths(Vec::new()).expect("deny-all roots");
        let server = modeler3d_mcp::R4Server::new(modeler3d_mcp::R4Config::for_profile(
            modeler3d_mcp::Profile::Full,
            roots,
        ));
        let mut document = Some(commands.take_mcp_document(modeler3d_mcp::ViewState::default()));
        server.replace_document(&mut document).expect("attach");
        let result = server
            .call_value(tool, arguments)
            .map_err(|error| error.to_string());
        let document = server.take_document().expect("detach");
        commands
            .install_mcp_document(document, &version)
            .expect("install");
        result
    }

    #[test]
    fn mcp_set_visibility_matches_the_desktop_show_hide() {
        let (mut desktop, mut mcp) = one_cube_pair();
        let history = desktop.scene().history_state_id();

        assert!(desktop.set_visibility(0, false).is_ok());
        let result = run_mcp(
            &mut mcp,
            "set_visibility",
            serde_json::json!({"index":0,"visible":false}),
        );
        assert_eq!(
            result["changed_object_ids"].as_array().map(Vec::len),
            Some(1)
        );
        for state in [&desktop, &mcp] {
            assert!(!state.scene().objects()[0].is_visible());
            assert_eq!(state.scene().history_state_id(), history + 1);
        }
        // Hiding a hidden object changes nothing on either side.
        assert!(matches!(
            desktop.set_visibility(0, false),
            Ok(CommandOutcome::Noop(_))
        ));
        run_mcp(
            &mut mcp,
            "set_visibility",
            serde_json::json!({"index":0,"visible":false}),
        );
        assert_eq!(desktop.scene().history_state_id(), history + 1);
        assert_eq!(mcp.scene().history_state_id(), history + 1);
        // Show again, then undo twice to the start, on both.
        assert!(desktop.set_visibility(0, true).is_ok());
        run_mcp(
            &mut mcp,
            "set_visibility",
            serde_json::json!({"index":0,"visible":true}),
        );
        for state in [&mut desktop, &mut mcp] {
            assert!(state.scene().objects()[0].is_visible());
            assert!(state.dispatch(CommandId::Undo).is_ok());
            assert!(!state.scene().objects()[0].is_visible());
        }
    }

    #[test]
    fn mcp_scene_attachment_returns_the_live_history_to_command_state() {
        let mut commands = CommandState::default();
        commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("add desktop cube");
        let version = commands.mcp_document_version();
        let initial_history = commands.scene().history_state_id();
        let roots = modeler3d_mcp::ApprovedRoots::from_paths(Vec::new()).expect("deny-all roots");
        let server = modeler3d_mcp::R4Server::new(modeler3d_mcp::R4Config::for_profile(
            modeler3d_mcp::Profile::Full,
            roots,
        ));
        let view = modeler3d_mcp::ViewState::default();
        let mut document = Some(commands.take_mcp_document(view));
        server
            .replace_document(&mut document)
            .expect("attach desktop scene");
        server
            .call_value(
                "translate",
                serde_json::json!({"index":0,"offset_mm":[2,0,0]}),
            )
            .expect("MCP edit");
        let document = server.take_document().expect("return desktop scene");
        commands
            .install_mcp_document(document, &version)
            .expect("restore app command state");

        assert_eq!(commands.scene().history_state_id(), initial_history + 1);
        assert!(commands.is_dirty());
        assert!(commands.dispatch(CommandId::Undo).is_ok());
        assert_eq!(commands.scene().history_state_id(), initial_history);
        assert_eq!(commands.scene().objects().len(), 1);
    }

    #[test]
    fn completed_mcp_open_replaces_a_same_identity_document() {
        let mut commands = CommandState::default();
        commands
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("add desktop cube");
        let version = commands.mcp_document_version();

        let mut opened = Scene::new();
        opened
            .restore_snapshot(commands.scene().snapshot())
            .expect("copy project identity");
        opened
            .add_primitive(PrimitiveKind::Sphere, None, None)
            .expect("add first opened object");
        opened
            .add_primitive(PrimitiveKind::Cylinder, None, None)
            .expect("add second opened object");
        let archive =
            modeler3d_core::save_project_bytes(&opened).expect("serialize opened project");
        let opened = modeler3d_core::load_project_bytes(&archive)
            .expect("load opened project with cleared history");
        assert_eq!(opened.project_id(), version.project_id);
        assert_ne!(opened.revision(), version.revision + 1);

        let document = R4Document {
            scene: opened,
            project_path: Some(PathBuf::from("same-identity.3dp")),
            dirty: false,
            view: McpViewState::default(),
            snapping: false,
            clipboard: None,
        };
        commands
            .commit_mcp_operation_document(&version, McpOperationCommit::ProjectReplace, document)
            .expect("commit the opened project");

        assert_eq!(commands.document_generation(), version.generation + 1);
        assert_eq!(commands.scene().objects().len(), 3);
        assert_eq!(
            commands.project_path(),
            Some(Path::new("same-identity.3dp"))
        );
        assert!(!commands.is_dirty());
        assert!(commands.dispatch(CommandId::Undo).is_ok());
        assert_eq!(commands.scene().objects().len(), 3);
    }

    #[test]
    fn unrelated_mcp_operation_keeps_path_and_rejects_a_stale_path() {
        let mut commands = CommandState {
            project_path: Some(PathBuf::from("current.3dp")),
            ..CommandState::default()
        };
        let version = commands.mcp_document_version();
        let document = R4Document {
            scene: commands.scene().isolated_copy(),
            project_path: Some(PathBuf::from("captured.3dp")),
            dirty: false,
            view: McpViewState::default(),
            snapping: false,
            clipboard: None,
        };

        commands
            .commit_mcp_operation_document(&version, McpOperationCommit::SceneUpdate, document)
            .expect("commit a read-only operation");
        assert_eq!(commands.project_path(), Some(Path::new("current.3dp")));

        commands.project_path = Some(PathBuf::from("later.3dp"));
        let document = R4Document {
            scene: commands.scene().isolated_copy(),
            project_path: Some(PathBuf::from("captured.3dp")),
            dirty: false,
            view: McpViewState::default(),
            snapping: false,
            clipboard: None,
        };
        assert!(matches!(
            commands.commit_mcp_operation_document(
                &version,
                McpOperationCommit::SceneUpdate,
                document,
            ),
            Err(CommandError::StaleOperation)
        ));
        assert_eq!(commands.project_path(), Some(Path::new("later.3dp")));
    }

    fn temporary_import_directory(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let path = test_temp_dir().join(format!(
            "modeler3d-import-{label}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("temporary import directory");
        path
    }

    fn commands(items: &[RibbonItem]) -> Vec<CommandDescriptor> {
        items
            .iter()
            .filter_map(|item| match item {
                RibbonItem::Command(command) => Some(*command),
                RibbonItem::Separator => None,
            })
            .collect()
    }

    #[test]
    fn ribbon_matches_reference_tab_and_command_order() {
        let tabs = ribbon_spec();
        let names = tabs.iter().map(|(tab, _)| tab.label()).collect::<Vec<_>>();
        assert_eq!(names, vec!["Insert", "Object", "Edit", "Paint", "View"]);
        let insert = commands(&tabs[0].1);
        assert_eq!(insert[0].label, "Add/Open");
        assert_eq!(insert[0].id, CommandId::ImportModel);
        assert_eq!(insert[1].label, "Cube");
        assert_eq!(insert[8].label, "Wedge");
        let view = commands(&tabs[4].1);
        assert_eq!(view.last().map(|item| item.label), Some("Snap"));
        assert!(
            view.iter()
                .any(|item| item.label == "Grid" && item.default_checked)
        );
    }

    #[test]
    fn file_menu_keeps_shell_contract_and_no_removed_actions() {
        let labels = commands(&file_menu_spec())
            .into_iter()
            .map(|command| command.label)
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec![
                "3D Modeler",
                "Import",
                "New Scene",
                "Open Project",
                "Save",
                "Save As",
                "Export",
                "Repair",
                "Print Prep Check",
                "Settings",
                "Exit"
            ]
        );
        assert!(!labels.contains(&"Save Project"));
        assert!(!labels.contains(&"Open File"));
    }

    #[test]
    fn primitive_command_changes_scene_and_selection() {
        let mut state = CommandState::default();
        let outcome = state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
        assert!(matches!(outcome, Ok(CommandOutcome::Applied(_))));
        assert_eq!(state.scene().objects().len(), 1);
        assert_eq!(state.selected_count(), 1);
        assert!(state.is_dirty());
    }

    #[test]
    fn durable_id_move_survives_scene_reordering_and_has_one_undo_entry() {
        let mut state = CommandState::default();
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .is_ok()
        );
        let target_id = state.scene().objects()[1].object_id().to_owned();
        let removed = state.scene.remove_object(0);
        assert!(removed.is_ok());
        assert_eq!(state.scene().objects()[0].object_id(), target_id);

        let before = state.scene().history_state_id();
        let moved = state.set_object_position_by_id(&target_id, [12.0, 3.0, -4.0]);

        assert!(matches!(moved, Ok(CommandOutcome::Applied(_))));
        assert_eq!(state.scene().objects()[0].position(), [12.0, 3.0, -4.0]);
        assert_ne!(state.scene().history_state_id(), before);
        let after_move = state.scene().history_state_id();
        assert!(matches!(
            state.set_object_position_by_id(&target_id, [12.0, 3.0, -4.0]),
            Ok(CommandOutcome::Noop(_))
        ));
        assert_eq!(state.scene().history_state_id(), after_move);
        assert!(matches!(state.scene.undo(), Ok(Some(result)) if result.changed));
        assert_eq!(state.scene().objects()[0].position(), [0.0, 20.0, 0.0]);
        assert!(matches!(state.scene.redo(), Ok(Some(result)) if result.changed));
        assert_eq!(state.scene().objects()[0].position(), [12.0, 3.0, -4.0]);
    }

    #[test]
    fn center_view_runs_with_and_without_objects_and_adds_no_undo_entry() {
        let mut state = CommandState::default();
        let empty_history = state.scene.history_state_id();
        let empty = state.dispatch(CommandId::CenterView);
        assert!(matches!(empty, Ok(CommandOutcome::Applied(ref text)) if text == "Center View"));
        assert_eq!(state.view_orientation(), ViewOrientation::Perspective);
        assert_eq!(state.last_status(), "Center View");
        assert_eq!(state.scene.history_state_id(), empty_history);

        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("add cube");
        state.dispatch(CommandId::ViewTop).expect("top view");
        let history = state.scene.history_state_id();
        let filled = state.dispatch(CommandId::CenterView);
        assert!(matches!(filled, Ok(CommandOutcome::Applied(ref text)) if text == "Center View"));
        assert_eq!(state.view_orientation(), ViewOrientation::Perspective);
        assert_eq!(state.last_status(), "Center View");
        assert_eq!(state.scene.history_state_id(), history);
    }

    #[test]
    fn frame_selection_requires_a_selection_and_adds_no_undo_entry() {
        let mut state = CommandState::default();
        assert!(matches!(
            state.dispatch(CommandId::FrameSelection),
            Err(CommandError::SelectionRequired("Frame"))
        ));
        assert_eq!(state.view_orientation(), ViewOrientation::Perspective);

        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("add cube");
        state.select_index(0);
        let history = state.scene.history_state_id();
        let framed = state.dispatch(CommandId::FrameSelection);
        assert!(matches!(framed, Ok(CommandOutcome::Applied(ref text)) if text == "Frame"));
        assert_eq!(state.view_orientation(), ViewOrientation::FrameSelection);
        assert_eq!(state.last_status(), "Frame");
        assert_eq!(state.scene.history_state_id(), history);
    }

    #[test]
    fn snap_toggle_flips_persists_to_preferences_and_adds_no_undo_entry() {
        let mut state = CommandState::default();
        assert!(state.snap_enabled());
        assert!(state.preferences().object_snapping_enabled());
        let history = state.scene.history_state_id();

        let off = state.dispatch(CommandId::ToggleSnap);
        assert!(matches!(off, Ok(CommandOutcome::Applied(ref text)) if text == "Snap off"));
        assert!(!state.snap_enabled());
        assert!(!state.preferences().object_snapping_enabled());
        assert_eq!(state.last_status(), "Snap off");

        let directory = temporary_import_directory("snap-toggle");
        let path = directory.join("settings.json");
        state
            .save_preferences_to_path(&path)
            .expect("save settings");
        let mut reloaded = CommandState::default();
        reloaded
            .load_preferences_from_path(&path)
            .expect("load settings");
        let _ = std::fs::remove_dir_all(&directory);
        assert!(!reloaded.preferences().object_snapping_enabled());

        let on = state.dispatch(CommandId::ToggleSnap);
        assert!(matches!(on, Ok(CommandOutcome::Applied(ref text)) if text == "Snap on"));
        assert!(state.snap_enabled());
        assert!(state.preferences().object_snapping_enabled());
        assert_eq!(state.scene.history_state_id(), history);
    }

    #[test]
    fn empty_selection_is_a_typed_error() {
        let mut state = CommandState::default();
        let result = state.dispatch(CommandId::Delete);
        assert!(matches!(
            result,
            Err(CommandError::SelectionRequired("Delete"))
        ));
    }

    #[test]
    fn core_backed_object_commands_change_state_without_shell_access() {
        let mut state = CommandState::default();
        let added = state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube));
        assert!(matches!(added, Ok(CommandOutcome::Applied(_))));
        assert!(state.dispatch(CommandId::Measure).is_ok());
        assert!(state.dispatch(CommandId::Mirror).is_ok());
        assert!(state.dispatch(CommandId::Settle).is_ok());
        assert!(state.dispatch(CommandId::Arrange).is_ok());
        assert!(state.dispatch(CommandId::Copy).is_ok());
        assert!(state.dispatch(CommandId::Paste).is_ok());
        assert_eq!(state.scene().objects().len(), 2);
        assert!(state.dispatch(CommandId::Delete).is_ok());
        assert_eq!(state.scene().objects().len(), 1);
    }

    #[test]
    fn view_commands_are_typed_state_changes() {
        let mut state = CommandState::default();
        assert!(state.dispatch(CommandId::ViewTop).is_ok());
        assert_eq!(state.view_orientation(), ViewOrientation::Top);
        assert!(state.dispatch(CommandId::ViewSide).is_ok());
        assert_eq!(state.view_orientation(), ViewOrientation::Side);
        assert!(state.dispatch(CommandId::ResetView).is_ok());
        assert_eq!(state.view_orientation(), ViewOrientation::Perspective);
    }

    #[test]
    fn torus_and_shell_dialog_actions_have_real_core_paths() {
        let mut state = CommandState::default();
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Torus))
                .is_ok()
        );
        let parameters_result = state.selected_torus_parameters();
        assert!(parameters_result.is_ok());
        let mut parameters = parameters_result.unwrap_or_default();
        parameters.sides = 12;
        parameters.steps = 16;
        assert!(state.update_selected_torus(parameters).is_ok());
        assert!(
            state
                .emboss_text_selected(TextEmbossOptions {
                    text: "A".to_owned(),
                    ..TextEmbossOptions::default()
                })
                .is_ok()
        );
        assert!(state.is_dirty());

        let mut hollow_state = CommandState::default();
        assert!(
            hollow_state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(hollow_state.hollow_selected(1.0).is_ok());
    }

    #[test]
    fn torus_insertion_persists_editable_metadata_and_rejects_other_meshes() {
        let mut torus = CommandState::default();
        assert!(matches!(
            torus.dispatch(CommandId::AddPrimitive(PrimitiveKind::Torus)),
            Ok(CommandOutcome::OpenDialog(DialogKind::Torus))
        ));
        let parameters = torus.selected_torus_parameters();
        assert!(parameters.is_ok());
        let parameters = parameters.unwrap_or_default();
        assert_eq!(parameters.mode, "solid");
        assert_eq!(parameters.radius_mm, 7.5);
        assert_eq!(parameters.tube_mm, 2.5);
        assert_eq!(parameters.sides, 24);
        assert_eq!(parameters.steps, 48);

        let mut cube = CommandState::default();
        assert!(
            cube.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(matches!(
            cube.dispatch(CommandId::EditTorus),
            Err(CommandError::InvalidTorusSelection("Edit Torus"))
        ));
    }

    #[test]
    fn merge_dispatches_boolean_union_and_consumes_the_tool_object() {
        let mut state = CommandState::default();
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(
            state
                .scene
                .set_transform(
                    1,
                    Transform::identity()
                        .translated([1.0, 0.0, 0.0])
                        .unwrap_or_else(|_| Transform::identity())
                )
                .is_ok()
        );
        state.scene.set_selection([0, 1]);
        let before_history = state.scene.history_state_id();
        let result = state.dispatch(CommandId::Merge);
        assert!(matches!(result, Ok(CommandOutcome::Applied(_))));
        assert_eq!(state.scene.objects().len(), 1);
        assert!(state.scene.objects()[0].children().is_empty());
        assert_eq!(state.selected_count(), 1);
        assert_ne!(state.scene.history_state_id(), before_history);
    }

    #[test]
    fn merge_paints_the_result_with_the_top_most_selected_color() {
        let mut state = CommandState::default();
        for _ in 0..2 {
            assert!(
                state
                    .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                    .is_ok()
            );
        }
        let red = [0.9, 0.1, 0.1, 1.0];
        let blue = [0.1, 0.2, 0.9, 1.0];
        assert!(state.scene.set_color(0, red).is_ok());
        assert!(state.scene.set_color(1, blue).is_ok());
        assert!(
            state
                .scene
                .set_transform(
                    1,
                    Transform::identity()
                        .translated([10.0, 0.0, 0.0])
                        .unwrap_or_else(|_| Transform::identity())
                )
                .is_ok()
        );
        // Click the lower list row first: it becomes the boolean target, but
        // the color still comes from the top-most row (Object 1, red).
        state.scene.set_selection([1, 0]);
        assert_eq!(state.scene.ordered_selection(), vec![1, 0]);
        let before = state.scene.history_state_id();
        assert!(matches!(
            state.dispatch(CommandId::Merge),
            Ok(CommandOutcome::Applied(_))
        ));
        assert_eq!(state.scene.objects().len(), 1);
        let merged = &state.scene.objects()[0];
        assert_eq!(merged.color(), red);
        assert!(
            modeler3d_core::boolean_color_provenance(merged.metadata(), merged.mesh().faces.len())
                .is_none(),
            "no per-face source colors remain"
        );
        // One undo entry restores both objects.
        assert!(state.dispatch(CommandId::Undo).is_ok());
        assert_eq!(state.scene.history_state_id(), before);
        assert_eq!(state.scene.objects().len(), 2);
    }

    #[test]
    fn brush_stroke_commits_one_history_entry_and_reports_noop_without_change() {
        let mut state = CommandState::default();
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        let before_history = state.scene.history_state_id();
        let applied = state.apply_brush_stroke(BrushStroke::Blob {
            centers_mm: vec![[20.0, 20.0, 20.0]],
            options: BlobOptions::default(),
        });
        assert!(matches!(applied, Ok(CommandOutcome::Applied(_))));
        let after_history = state.scene.history_state_id();
        assert_ne!(after_history, before_history);
        assert!(matches!(
            state.apply_brush_stroke(BrushStroke::Grab {
                center_mm: [0.0, 0.0, 0.0],
                offset_mm: [0.0, 0.0, 0.0],
                options: ElasticGrabOptions::default(),
            }),
            Ok(CommandOutcome::Noop(_))
        ));
        assert_eq!(state.scene.history_state_id(), after_history);
    }

    #[test]
    fn dirty_marker_tracks_saved_checkpoint_through_undo_redo_and_branches() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let directory = test_temp_dir().join(format!(
            "modeler3d-dirty-savepoint-{}-{unique}",
            std::process::id()
        ));
        assert!(std::fs::create_dir(&directory).is_ok());
        let project = directory.join("savepoint.3dp");
        let mut state = CommandState::default();

        assert!(!state.is_dirty());
        assert!(matches!(
            state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube)),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(state.is_dirty());
        assert!(state.save_to_path(&project).is_ok());
        assert!(!state.is_dirty());
        let saved_state_id = state.scene().history_state_id();

        assert!(matches!(
            state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere)),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(state.is_dirty());
        assert!(matches!(
            state.dispatch(CommandId::Undo),
            Ok(CommandOutcome::Applied(_))
        ));
        assert_eq!(state.scene().history_state_id(), saved_state_id);
        assert!(!state.is_dirty());
        assert!(matches!(
            state.dispatch(CommandId::Redo),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(state.is_dirty());
        assert!(matches!(
            state.dispatch(CommandId::Undo),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(!state.is_dirty());

        assert!(matches!(
            state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cone)),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(state.is_dirty());
        assert_ne!(state.scene().history_state_id(), saved_state_id);
        assert!(matches!(
            state.dispatch(CommandId::Undo),
            Ok(CommandOutcome::Applied(_))
        ));
        assert_eq!(state.scene().history_state_id(), saved_state_id);
        assert!(!state.is_dirty());
        assert!(matches!(
            state.dispatch(CommandId::Redo),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(state.is_dirty());
        assert!(state.save_to_path(&project).is_ok());
        assert!(!state.is_dirty());
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn model_import_is_atomic_selected_and_undoable() {
        let fixture = import_fixture("formats/python_box.stl");
        let mut state = CommandState::default();

        let result = state.import_from_path(&fixture);

        assert!(matches!(result, Ok(ref message) if message == "Imported 1 object(s)"));
        assert_eq!(state.scene().objects().len(), 1);
        assert_eq!(state.selected_count(), 1);
        assert_eq!(state.scene().revision(), 1);
        assert!(state.is_dirty());
        let imported = state.scene().object(0);
        assert!(imported.is_ok());
        if let Ok(object) = imported {
            assert_eq!(object.position(), [0.0, 1.5, 0.0]);
        }

        let undo = state.scene.undo();
        assert!(matches!(undo, Ok(Some(result)) if result.changed));
        assert!(state.scene().objects().is_empty());
    }

    #[test]
    fn missing_import_does_not_change_scene() {
        let mut state = CommandState::default();
        let result = state.import_from_path(Path::new("model.wrl"));

        assert!(result.is_err());
        assert!(state.scene().objects().is_empty());
        assert_eq!(state.scene().revision(), 0);
        assert!(!state.is_dirty());
    }

    fn document_test_directory(label: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let directory =
            test_temp_dir().join(format!("modeler3d-{label}-{}-{unique}", std::process::id()));
        assert!(std::fs::create_dir_all(&directory).is_ok());
        directory
    }

    fn two_cubes_one_hidden() -> CommandState {
        let mut state = CommandState::default();
        for _ in 0..2 {
            assert!(matches!(
                state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube)),
                Ok(CommandOutcome::Applied(_))
            ));
        }
        assert!(state.set_visibility(1, false).is_ok());
        state
    }

    #[test]
    fn save_defaults_to_stl_keeps_hidden_objects_and_resaves_in_place() {
        let directory = document_test_directory("save-stl");
        let mut state = two_cubes_one_hidden();
        assert!(
            state
                .save_document_to_path(&directory.join("scene"))
                .is_ok()
        );
        let saved = directory.join("scene.stl");
        assert_eq!(state.project_path(), Some(saved.as_path()));
        assert!(!state.is_dirty());
        let mesh = modeler3d_core::load_mesh(&saved).expect("saved STL loads");
        assert_eq!(mesh.face_count(), 24, "both cubes, hidden one included");

        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(state.is_dirty());
        assert!(state.dispatch(CommandId::SaveProject).is_ok());
        assert!(!state.is_dirty());
        let mesh = modeler3d_core::load_mesh(&saved).expect("resaved STL loads");
        assert_eq!(mesh.face_count(), 36);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn save_writes_3mf_and_rejects_export_only_formats() {
        let directory = document_test_directory("save-3mf");
        let mut state = two_cubes_one_hidden();
        let target = directory.join("scene.3mf");
        assert!(state.save_document_to_path(&target).is_ok());
        assert!(target.is_file());
        assert_eq!(state.project_path(), Some(target.as_path()));

        let before = state.project_path().map(Path::to_path_buf);
        let obj = state.save_document_to_path(&directory.join("scene.obj"));
        assert!(matches!(obj, Err(CommandError::InvalidPath(_))));
        assert_eq!(state.project_path(), before.as_deref());
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn saving_an_empty_scene_reports_it_and_stays_dirty_free() {
        let directory = document_test_directory("save-empty");
        let mut state = CommandState::default();
        let result = state.save_document_to_path(&directory.join("empty.stl"));
        assert!(matches!(
            result,
            Err(CommandError::Export(ExportError::EmptyScene))
        ));
        assert_eq!(state.project_path(), None);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn exporting_a_project_copy_leaves_the_document_alone() {
        let directory = document_test_directory("export-3dp");
        let mut state = two_cubes_one_hidden();
        let document = directory.join("scene.stl");
        assert!(state.save_document_to_path(&document).is_ok());
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(state.is_dirty());

        let copy = directory.join("copy");
        assert!(state.export_project_copy(&copy).is_ok());
        assert!(directory.join("copy.3dp").is_file());
        assert_eq!(state.project_path(), Some(document.as_path()));
        assert!(state.is_dirty());

        let mut reopened = CommandState::default();
        assert!(reopened.open_from_path(&directory.join("copy.3dp")).is_ok());
        assert_eq!(reopened.scene().objects().len(), 3);
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn project_paths_enforce_format_and_save_as_adds_the_default_extension() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let directory = test_temp_dir().join(format!(
            "modeler3d-project-path-{}-{unique}",
            std::process::id()
        ));
        let path_without_extension = directory.join("scene");
        let expected = directory.join("scene.3dp");
        let mut state = CommandState::default();
        assert!(matches!(
            state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube)),
            Ok(CommandOutcome::Applied(_))
        ));

        assert!(state.save_to_path(&path_without_extension).is_ok());
        assert_eq!(state.project_path(), Some(expected.as_path()));
        assert!(expected.is_file());
        assert!(!state.is_dirty());

        let path_before = state.project_path().map(Path::to_path_buf);
        let history_before = state.scene().history_state_id();
        let invalid = state.open_from_path(&directory.join("not-a-project.obj"));

        assert!(matches!(invalid, Err(CommandError::InvalidPath(_))));
        assert_eq!(state.project_path(), path_before.as_deref());
        assert_eq!(state.scene().history_state_id(), history_before);
        assert!(!state.is_dirty());
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn failed_project_open_and_save_preserve_scene_path_dirty_state_and_history() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let directory = test_temp_dir().join(format!(
            "modeler3d-project-failure-{}-{unique}",
            std::process::id()
        ));
        let project = directory.join("current.3dp");
        let corrupt = directory.join("corrupt.3dp");
        let blocked = directory.join("blocked.3dp");
        let mut state = CommandState::default();
        assert!(matches!(
            state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube)),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(state.save_to_path(&project).is_ok());
        assert!(matches!(
            state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere)),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(std::fs::write(&corrupt, b"not a project archive").is_ok());
        assert!(std::fs::create_dir(&blocked).is_ok());

        let scene_revision = state.scene().revision();
        let history = state.scene().history_state_id();
        let path_before = state.project_path().map(Path::to_path_buf);
        let open_result = state.open_from_path(&corrupt);
        let save_result = state.save_to_path(&blocked);

        assert!(matches!(
            open_result,
            Err(CommandError::Core(CoreError::ProjectFormat(_)))
        ));
        assert!(save_result.is_err());
        assert_eq!(state.scene().revision(), scene_revision);
        assert_eq!(state.scene().history_state_id(), history);
        assert_eq!(state.project_path(), path_before.as_deref());
        assert!(state.is_dirty());
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn export_success_and_failure_preserve_scene_project_dirty_selection_history_and_recents() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/rb03-command-export-tests")
            .join(format!("{}-{unique}", std::process::id()));
        assert!(std::fs::create_dir_all(&directory).is_ok());
        let project = directory.join("current.3dp");
        let mut state = CommandState::default();
        assert!(matches!(
            state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube)),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(state.save_to_path(&project).is_ok());
        assert!(matches!(
            state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere)),
            Ok(CommandOutcome::Applied(_))
        ));
        state.set_selection_indices([0]);

        let objects_before = state
            .scene()
            .objects()
            .iter()
            .map(|object| {
                (
                    object.object_id().to_owned(),
                    object.name().to_owned(),
                    object.mesh().clone(),
                    *object.transform(),
                    object.color(),
                    object.is_visible(),
                )
            })
            .collect::<Vec<_>>();
        let selection_before = state.scene().selection().clone();
        let revision_before = state.scene().revision();
        let history_before = state.scene().history_state_id();
        let project_before = state.project_path().map(Path::to_path_buf);
        let dirty_before = state.is_dirty();
        let recent_before = state.preferences().recent_files().paths().to_vec();
        assert!(dirty_before);

        let prior = directory.join("prior.unsupported");
        assert!(std::fs::write(&prior, b"keep this destination").is_ok());
        let unsupported = state.export_to_path(&prior);
        assert!(matches!(
            unsupported,
            Err(CommandError::Export(ExportError::UnsupportedExtension(_)))
        ));
        assert_eq!(
            std::fs::read(&prior).ok().as_deref(),
            Some(&b"keep this destination"[..])
        );

        let missing_parent = directory.join("missing-parent");
        let failed_write = state.export_to_path(&missing_parent.join("failed.obj"));
        assert!(failed_write.is_err());

        let successful_path = directory.join("successful.ply");
        assert!(state.export_to_path(&successful_path).is_ok());
        assert!(successful_path.is_file());

        let objects_after = state
            .scene()
            .objects()
            .iter()
            .map(|object| {
                (
                    object.object_id().to_owned(),
                    object.name().to_owned(),
                    object.mesh().clone(),
                    *object.transform(),
                    object.color(),
                    object.is_visible(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(objects_after, objects_before);
        assert_eq!(state.scene().selection(), &selection_before);
        assert_eq!(state.scene().revision(), revision_before);
        assert_eq!(state.scene().history_state_id(), history_before);
        assert_eq!(state.project_path(), project_before.as_deref());
        assert_eq!(state.is_dirty(), dirty_before);
        assert_eq!(
            state.preferences().recent_files().paths(),
            recent_before.as_slice()
        );
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn unit_override_scales_import_before_reference_placement() {
        let fixture = import_fixture("formats/python_box.stl");
        let mut state = CommandState::default();

        let result = state.import_from_path_with_unit(&fixture, ImportUnit::Centimetre);

        assert!(result.is_ok());
        assert_eq!(
            state.scene().objects()[0].mesh().extents(),
            [20.0, 30.0, 40.0]
        );
    }

    #[test]
    fn import_unit_choices_match_reference_order_and_scale_factors() {
        let units = ImportUnit::all();
        assert_eq!(
            units.map(ImportUnit::label),
            ["um", "mm", "cm", "in", "ft", "m"]
        );
        assert_eq!(
            units.map(ImportUnit::scale_mm),
            [0.001, 1.0, 10.0, 25.4, 304.8, 1_000.0]
        );
        assert_eq!(
            Preferences::default().default_import_unit(),
            ImportUnit::Millimetre
        );
    }

    #[test]
    fn a_model_opened_into_an_empty_scene_is_saved_back_not_saved_as() {
        let fixture = import_fixture("formats/python_box.stl");
        let directory =
            std::env::temp_dir().join(format!("modeler3d-open-save-{}", std::process::id()));
        assert!(std::fs::create_dir_all(&directory).is_ok());
        let copy = directory.join("opened.stl");
        assert!(std::fs::copy(&fixture, &copy).is_ok());
        let mut state = CommandState::default();
        assert!(state.import_from_path(&copy).is_ok());
        assert_eq!(state.project_path(), Some(copy.as_path()));
        // Save writes straight back to the opened file: no Save As dialog.
        assert!(matches!(
            state.dispatch(CommandId::SaveProject),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(!state.is_dirty());
        // Importing into a scene that already has content does not rebind.
        let mut other = CommandState::default();
        assert!(other.import_from_path(&fixture).is_ok());
        let second = directory.join("second.stl");
        assert!(std::fs::copy(&fixture, &second).is_ok());
        assert!(other.import_from_path(&second).is_ok());
        assert_eq!(other.project_path(), Some(fixture.as_path()));
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn unitless_stl_import_uses_each_reference_scale_once() {
        let fixture = import_fixture("formats/python_box.stl");
        for unit in ImportUnit::all() {
            let mut state = CommandState::default();
            assert!(
                state.import_from_path_with_unit(&fixture, unit).is_ok(),
                "{} import should succeed",
                unit.label()
            );
            assert_eq!(state.scene().objects().len(), 1);
            assert_vec3_close(
                state.scene().objects()[0].mesh().extents(),
                [2.0, 3.0, 4.0].map(|extent| extent * unit.scale_mm()),
            );
        }

        let mut default_state = CommandState::default();
        assert!(default_state.import_from_path(&fixture).is_ok());
        assert_vec3_close(
            default_state.scene().objects()[0].mesh().extents(),
            [2.0, 3.0, 4.0],
        );
    }

    #[test]
    fn python_stl_obj_and_ply_imports_keep_names_geometry_and_vertex_colors() {
        for relative in [
            "formats/python_box.stl",
            "formats/python_box.obj",
            "formats/python_box.ply",
        ] {
            let fixture = import_fixture(relative);
            let mut state = CommandState::default();
            assert!(
                state
                    .import_from_path_with_unit(&fixture, ImportUnit::Millimetre)
                    .is_ok(),
                "{relative} should import"
            );
            assert_eq!(state.scene().objects().len(), 1);
            assert_eq!(
                state.scene().objects()[0].name(),
                fixture
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("")
            );
            assert_vec3_close(state.scene().objects()[0].mesh().extents(), [2.0, 3.0, 4.0]);
        }

        for (relative, expected_color) in [
            ("formats/colored_triangle.obj", [0.2, 0.4, 0.6, 1.0]),
            ("formats/colored_triangle.ply", [0.2, 0.4, 0.6, 0.8]),
        ] {
            let fixture = import_fixture(relative);
            let mut state = CommandState::default();
            assert!(
                state
                    .import_from_path_with_unit(&fixture, ImportUnit::Millimetre)
                    .is_ok(),
                "{relative} should import"
            );
            let object = &state.scene().objects()[0];
            assert_eq!(object.name(), "colored_triangle");
            assert_vec3_close(object.mesh().extents(), [2.0, 3.0, 4.0]);
            assert_color_close(object.color(), expected_color);
        }
    }

    #[test]
    fn authoritative_multimesh_imports_ignore_unit_override_and_undo_as_one_action() {
        let fixtures = [
            "imports/python_multi_object_cm.3mf",
            "imports/python_multi_object.gltf",
            "imports/python_multi_object.glb",
        ];
        for relative in fixtures {
            let fixture = import_fixture(relative);
            let mut state = CommandState::default();
            let history_before = state.scene().history_state_id();

            assert!(
                state
                    .import_from_path_with_unit(&fixture, ImportUnit::Foot)
                    .is_ok(),
                "{relative} should import"
            );
            assert_eq!(state.scene().objects().len(), 2, "{relative}");
            assert_eq!(state.selected_count(), 2, "{relative}");
            assert!(state.is_dirty(), "{relative}");
            // An opened STL or 3MF becomes the document, so Save writes it
            // back; formats this app cannot write (glTF) stay unbound.
            let expected = relative.ends_with(".3mf").then_some(fixture.as_path());
            assert_eq!(state.project_path(), expected, "{relative}");
            assert_eq!(
                state.preferences().recent_files().paths().first(),
                Some(&fixture),
                "{relative}"
            );
            assert_ne!(
                state.scene().history_state_id(),
                history_before,
                "{relative}"
            );

            let objects = state.scene().objects();
            let first = objects
                .iter()
                .find(|object| object.name() == "First part")
                .expect("first imported name");
            let second = objects
                .iter()
                .find(|object| object.name() == "Second part")
                .expect("second imported name");
            assert_vec3_close(first.mesh().extents(), [10.0, 20.0, 30.0]);
            assert_vec3_close(second.mesh().extents(), [2.0, 3.0, 4.0]);
            assert_color_close(first.color(), [0.2, 0.4, 0.8, 128.0 / 255.0]);
            assert_color_close(second.color(), [1.0, 0.0, 0.0, 1.0]);
            assert_vec3_close(first.position(), [1.0, 10.0, 0.0]);
            assert_vec3_close(second.position(), [-5.0, 6.0, -3.0]);

            let mut combined_minimum = [f32::INFINITY; 3];
            for object in objects {
                let bounds = object
                    .transformed_mesh()
                    .expect("world mesh")
                    .bounds()
                    .expect("world bounds");
                for (axis, minimum) in combined_minimum.iter_mut().enumerate() {
                    *minimum = (*minimum).min(bounds.0[axis]);
                }
            }
            assert_vec3_close(combined_minimum, [-6.0, 0.0, -15.0]);

            assert!(matches!(state.scene.undo(), Ok(Some(result)) if result.changed));
            assert!(state.scene().objects().is_empty(), "{relative}");
            assert!(matches!(state.scene.undo(), Ok(None)), "{relative}");
        }
    }

    #[test]
    fn failed_supported_and_unsupported_imports_preserve_live_state() {
        let directory = temporary_import_directory("failure");
        let project = directory.join("active.3dp");
        let malformed = directory.join("malformed.stl");
        let unsupported = directory.join("unsupported.wrl");
        let mut state = CommandState::default();
        assert!(matches!(
            state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube)),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(state.save_to_path(&project).is_ok());
        assert!(matches!(
            state.dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere)),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(std::fs::write(&malformed, b"not an STL mesh").is_ok());

        let objects_before = state
            .scene()
            .objects()
            .iter()
            .map(|object| {
                (
                    object.object_id().to_owned(),
                    object.name().to_owned(),
                    object.position(),
                    object.mesh().extents(),
                    object.color(),
                )
            })
            .collect::<Vec<_>>();
        let selection_before = state.scene().selection().clone();
        let revision_before = state.scene().revision();
        let history_before = state.scene().history_state_id();
        let project_before = state.project_path().map(Path::to_path_buf);
        let dirty_before = state.is_dirty();
        let recent_before = state.preferences().recent_files().paths().to_vec();
        assert!(dirty_before);

        for path in [&malformed, &unsupported] {
            assert!(state.import_from_path(path).is_err(), "{}", path.display());
            let objects_after = state
                .scene()
                .objects()
                .iter()
                .map(|object| {
                    (
                        object.object_id().to_owned(),
                        object.name().to_owned(),
                        object.position(),
                        object.mesh().extents(),
                        object.color(),
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(objects_after, objects_before);
            assert_eq!(state.scene().selection(), &selection_before);
            assert_eq!(state.scene().revision(), revision_before);
            assert_eq!(state.scene().history_state_id(), history_before);
            assert_eq!(state.project_path(), project_before.as_deref());
            assert_eq!(state.is_dirty(), dirty_before);
            assert_eq!(
                state.preferences().recent_files().paths(),
                recent_before.as_slice()
            );
        }
        assert!(std::fs::remove_dir_all(directory).is_ok());
    }

    #[test]
    fn millimetre_import_move_and_project_reopen_preserve_identity_and_geometry() {
        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../modeler3d-core/tests/fixtures/formats/python_box.stl");
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        let project = test_temp_dir().join(format!(
            "modeler3d-rb02-{}-{unique}.3dp",
            std::process::id()
        ));
        let mut state = CommandState::default();

        assert!(
            state
                .import_from_path_with_unit(&fixture, ImportUnit::Millimetre)
                .is_ok()
        );
        let object_id = state.scene().objects()[0].object_id().to_owned();
        let expected_name = state.scene().objects()[0].name().to_owned();
        let expected_mesh = state.scene().objects()[0].mesh().clone();
        let expected_local_bounds = expected_mesh.bounds();
        let expected_color = [0.18, 0.32, 0.84, 1.0];
        assert!(state.set_selected_color(expected_color).is_ok());
        assert!(matches!(
            state.set_object_position_by_id(&object_id, [12.5, 3.0, -4.0]),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(state.save_to_path(&project).is_ok());

        let mut reopened = CommandState::default();
        assert!(reopened.open_from_path(&project).is_ok());
        assert_eq!(reopened.scene().objects().len(), 1);
        let object = &reopened.scene().objects()[0];
        assert_eq!(object.object_id(), object_id);
        assert_eq!(object.name(), expected_name);
        assert_eq!(object.mesh(), &expected_mesh);
        assert_eq!(object.mesh().bounds(), expected_local_bounds);
        assert_eq!(object.color(), expected_color);
        assert_eq!(object.position(), [12.5, 3.0, -4.0]);
        assert!(object.transformed_mesh().is_ok_and(|mesh| {
            mesh.bounds()
                == expected_local_bounds.map(|(minimum, maximum)| {
                    let offset = object.position();
                    let mut world_minimum = minimum;
                    let mut world_maximum = maximum;
                    for axis in 0..3 {
                        world_minimum[axis] += offset[axis];
                        world_maximum[axis] += offset[axis];
                    }
                    (world_minimum, world_maximum)
                })
        }));
        assert!(std::fs::remove_file(project).is_ok());
    }

    #[test]
    fn selected_color_updates_all_selected_objects_in_one_undoable_action() {
        let mut state = CommandState::default();
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .is_ok()
        );
        state.set_selection_indices([0, 1]);
        let old_colors = state
            .scene()
            .objects()
            .iter()
            .map(|object| object.color())
            .collect::<Vec<_>>();
        let history_before = state.scene().history_state_id();
        let color = [0.82, 0.17, 0.34, 0.73];

        assert!(state.set_selected_color(color).is_ok());

        assert_ne!(state.scene().history_state_id(), history_before);
        assert!(
            state
                .scene()
                .objects()
                .iter()
                .all(|object| object.color() == color)
        );
        assert!(state.scene.undo().is_ok());
        assert_eq!(
            state
                .scene()
                .objects()
                .iter()
                .map(|object| object.color())
                .collect::<Vec<_>>(),
            old_colors
        );
        assert!(state.scene.redo().is_ok());
        assert!(
            state
                .scene()
                .objects()
                .iter()
                .all(|object| object.color() == color)
        );
        let history_after_redo = state.scene().history_state_id();
        assert!(state.set_selected_color(color).is_ok());
        assert_eq!(state.scene().history_state_id(), history_after_redo);
    }

    #[test]
    fn image_average_applies_to_all_selected_objects_as_one_action() {
        let mut state = CommandState::default();
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .is_ok()
        );
        state.set_selection_indices([0, 1]);
        let path = test_temp_dir().join(format!(
            "modeler3d-command-image-color-{}.ppm",
            std::process::id()
        ));
        assert!(std::fs::write(&path, b"P3\n2 1\n255\n255 0 0 0 0 255\n").is_ok());
        let history_before = state.scene().history_state_id();

        assert!(state.set_selected_color_from_image(&path, 1).is_ok());
        assert!(
            state
                .last_status()
                .to_ascii_lowercase()
                .contains("average image color")
        );
        assert!(state.last_status().contains("no texture was created"));

        assert_ne!(state.scene().history_state_id(), history_before);
        assert!(
            state
                .scene()
                .objects()
                .iter()
                .all(|object| object.color() == [0.5, 0.0, 0.5, 1.0])
        );
        assert!(state.scene.undo().is_ok());
        assert!(
            state
                .scene()
                .objects()
                .iter()
                .all(|object| object.color() != [0.5, 0.0, 0.5, 1.0])
        );
        assert!(std::fs::remove_file(path).is_ok());
    }

    #[test]
    fn selection_commands_cover_group_cut_and_non_history_selection() {
        let mut state = CommandState::default();
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Sphere))
                .is_ok()
        );
        let revision = state.scene().revision();

        assert!(matches!(
            state.dispatch(CommandId::SelectAll),
            Ok(CommandOutcome::Applied(_))
        ));
        assert_eq!(state.selected_count(), 2);
        assert_eq!(state.scene().revision(), revision);

        assert!(state.dispatch(CommandId::Group).is_ok());
        assert_eq!(state.scene().objects().len(), 1);
        assert_eq!(state.scene().objects()[0].children().len(), 2);

        assert!(state.dispatch(CommandId::Ungroup).is_ok());
        assert_eq!(state.scene().objects().len(), 2);

        assert!(state.dispatch(CommandId::DeselectAll).is_ok());
        assert_eq!(state.selected_count(), 0);
        assert!(state.dispatch(CommandId::InvertSelection).is_ok());
        assert_eq!(state.selected_count(), 2);

        assert!(state.dispatch(CommandId::Cut).is_ok());
        assert!(state.scene().objects().is_empty());
        assert!(state.dispatch(CommandId::Paste).is_ok());
        assert_eq!(state.scene().objects().len(), 2);
    }

    #[test]
    fn selection_modifiers_and_visibility_are_presentation_and_model_seams() {
        let mut state = CommandState::default();
        for kind in [
            PrimitiveKind::Cube,
            PrimitiveKind::Sphere,
            PrimitiveKind::Cylinder,
        ] {
            assert!(state.dispatch(CommandId::AddPrimitive(kind)).is_ok());
        }
        state.select_index_with_modifiers(0, false, None);
        state.select_index_with_modifiers(2, false, Some(0));
        assert_eq!(
            state
                .scene()
                .selection()
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            [0, 1, 2]
        );
        state.select_index_with_modifiers(1, true, None);
        assert_eq!(state.selected_count(), 2);
        let revision = state.scene().revision();
        let dirty_before = state.is_dirty();
        assert!(matches!(
            state.set_visibility(1, false),
            Ok(CommandOutcome::Applied(_))
        ));
        assert!(state.is_dirty() || dirty_before);
        assert!(state.scene().revision() > revision);
        assert!(!state.scene().objects()[1].is_visible());
    }

    #[test]
    fn paste_enablement_follows_clipboard_not_selection() {
        let mut state = CommandState::default();
        assert!(
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .is_ok()
        );
        assert!(state.dispatch(CommandId::Copy).is_ok());
        assert!(state.dispatch(CommandId::DeselectAll).is_ok());
        assert!(state.has_clipboard());
        assert!(state.dispatch(CommandId::Paste).is_ok());
    }
}

#[cfg(test)]
mod rb04_regressions {
    use super::*;
    #[test]
    fn clipboard_offsets_ids_hidden_state_and_one_step_history_match_reference() {
        let mut state = CommandState::default();
        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        state
            .set_selected_position([12., 24., 36.])
            .expect("position");
        state.set_visibility(0, false).expect("hide");
        let original = state.scene.objects()[0].object_id().to_owned();
        let before = state.scene.history_state_id();
        state.dispatch(CommandId::Copy).expect("copy");
        state.dispatch(CommandId::DeselectAll).expect("clear");
        assert_eq!(state.scene.history_state_id(), before);
        state.dispatch(CommandId::Paste).expect("paste");
        let pasted = state.scene.objects()[1].object_id().to_owned();
        assert_ne!(original, pasted);
        // OF-43 replaced the Python offsets: copies sit 10 mm beside the original
        // (40 mm cube + 10 mm gap on X; Python offset 0.5 mm on X and Y).
        assert_eq!(state.scene.objects()[1].position(), [62., 24., 36.]);
        assert!(!state.scene.objects()[1].is_visible());
        state.dispatch(CommandId::Undo).expect("undo");
        assert_eq!(state.scene.objects().len(), 1);
        state.dispatch(CommandId::Redo).expect("redo");
        assert_eq!(state.scene.objects()[1].object_id(), pasted);
        state.dispatch(CommandId::Duplicate).expect("duplicate");
        assert_eq!(state.scene.objects()[2].position(), [112., 24., 36.]);
        state.dispatch(CommandId::Cut).expect("cut");
        state.dispatch(CommandId::Undo).expect("undo cut");
        assert_eq!(state.scene.objects().len(), 3);
    }
    #[test]
    fn paste_after_moving_original_lands_where_it_was_copied_with_one_undo() {
        let mut state = CommandState::default();
        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        state
            .set_selected_position([12., 24., 36.])
            .expect("position");
        state.dispatch(CommandId::Copy).expect("copy");
        state
            .set_selected_position([300., 24., -50.])
            .expect("move original");
        let before = state.scene.history_state_id();
        state.dispatch(CommandId::Paste).expect("paste");
        assert_eq!(state.scene.objects().len(), 2);
        assert_eq!(state.scene.objects()[1].position(), [12., 24., 36.]);
        assert_ne!(state.scene.history_state_id(), before);
        state.dispatch(CommandId::Undo).expect("undo");
        assert_eq!(state.scene.objects().len(), 1);
        assert_eq!(state.scene.history_state_id(), before);
        // Undo then paste lands on the same spot again.
        state.dispatch(CommandId::Paste).expect("paste again");
        assert_eq!(state.scene.objects()[1].position(), [12., 24., 36.]);
    }

    #[test]
    fn settle_after_moving_keeps_x_z_and_is_one_undo_entry() {
        let mut state = CommandState::default();
        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        state
            .set_selected_position([120., 55., -70.])
            .expect("move");
        let before = state.scene.history_state_id();
        state.dispatch(CommandId::Settle).expect("settle");
        let position = state.scene.objects()[0].position();
        assert!((position[0] - 120.).abs() < 1e-3, "{position:?}");
        assert!((position[2] + 70.).abs() < 1e-3, "{position:?}");
        let floor = transforms::world_bounds(&state.scene, &[0])
            .expect("bounds")
            .minimum[1];
        assert!(floor.abs() < 1e-3, "{position:?}");
        state.dispatch(CommandId::Undo).expect("undo");
        assert_eq!(state.scene.history_state_id(), before);
        state.dispatch(CommandId::Redo).expect("redo");
        // Already resting: a second Settle is a no-op with no history entry.
        let settled = state.scene.history_state_id();
        assert!(matches!(
            state.dispatch(CommandId::Settle),
            Ok(CommandOutcome::Noop(_))
        ));
        assert_eq!(state.scene.history_state_id(), settled);
    }

    #[test]
    fn absolute_rotation_is_repeatable_and_resize_preserves_translation() {
        let mut state = CommandState::default();
        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        state
            .set_selected_position([12., 24., 36.])
            .expect("position");
        state.set_selected_size([20., 30., 40.]).expect("size");
        assert_eq!(state.scene.objects()[0].position(), [12., 24., 36.]);
        state
            .set_selected_rotation([15., 30., 60.])
            .expect("rotate");
        let id = state.scene.history_state_id();
        assert!(matches!(
            state.set_selected_rotation([15., 30., 60.]),
            Ok(CommandOutcome::Noop(_))
        ));
        assert_eq!(state.scene.history_state_id(), id);
        let transform = *state.scene.objects()[0].transform();
        assert!(state.set_selected_rotation([f32::NAN, 0., 0.]).is_err());
        assert_eq!(*state.scene.objects()[0].transform(), transform);
    }
    #[test]
    fn placement_uses_user_anchor_order_and_all_axes_without_selection_history() {
        let mut state = CommandState::default();
        for _ in 0..3 {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("cube");
        }
        state
            .scene
            .translate(2, [40., 50., 60.])
            .expect("move anchor");
        state.scene.clear_selection();
        state.set_selection_indices([2, 0]);
        let id = state.scene.history_state_id();
        assert_eq!(state.scene.ordered_selection(), [2, 0]);
        assert_eq!(id, state.scene.history_state_id());
        let anchor = *state.scene.objects()[2].transform();
        state
            .place_selected(CommandId::Align, Axis::Z, Alignment::Maximum, 0.)
            .expect("align");
        assert_eq!(*state.scene.objects()[2].transform(), anchor);
        assert_eq!(state.scene.objects()[0].position()[2], 60.);
        state.dispatch(CommandId::Undo).expect("undo");
        assert_eq!(state.scene.objects()[0].position(), [0., 20., 0.]);
        let history = state.scene.history_state_id();
        assert!(
            state
                .place_selected(CommandId::Arrange, Axis::Y, Alignment::Center, -1.)
                .is_err()
        );
        assert_eq!(state.scene.history_state_id(), history);
    }
}

#[cfg(test)]
mod rb04_names {
    use super::*;
    #[test]
    fn rename_preserves_identity_mesh_and_history_on_invalid_or_unchanged_names() {
        let mut state = CommandState::default();
        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        let id = state.scene.objects()[0].object_id().to_owned();
        let mesh = state.scene.objects()[0].mesh().clone();
        let history = state.scene.history_state_id();
        assert!(state.rename_selected(" ".into()).is_err());
        assert_eq!(state.scene.history_state_id(), history);
        state.rename_selected("Part A".into()).expect("rename");
        let history = state.scene.history_state_id();
        assert!(matches!(
            state.rename_selected("Part A".into()),
            Ok(CommandOutcome::Noop(_))
        ));
        assert_eq!(state.scene.history_state_id(), history);
        state.dispatch(CommandId::Undo).expect("undo");
        assert_ne!(state.scene.objects()[0].name(), "Part A");
        state.dispatch(CommandId::Redo).expect("redo");
        assert_eq!(state.scene.objects()[0].name(), "Part A");
        assert_eq!(state.scene.objects()[0].object_id(), id);
        assert_eq!(state.scene.objects()[0].mesh(), &mesh);
    }
}

#[cfg(test)]
mod rb04_measure_mirror {
    use super::*;
    #[test]
    fn measure_reports_each_objects_width_depth_height_without_history() {
        let mut state = CommandState::default();
        for _ in 0..2 {
            state
                .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
                .expect("cube");
        }
        state.scene.set_selection([0, 1]);
        state
            .scene
            .set_transform(
                0,
                Transform::identity().scaled([2., 3., 4.]).expect("scale"),
            )
            .expect("transform");
        let before = state.scene.history_state_id();
        let CommandOutcome::Applied(text) = state.dispatch(CommandId::Measure).expect("measure")
        else {
            panic!("measurement");
        };
        let size = transforms::world_bounds(&state.scene, &[0])
            .expect("bounds")
            .extents();
        assert!(text.contains(&format!("Width: {:.2} mm", size[0])));
        assert!(text.contains(&format!("Depth: {:.2} mm", size[2])));
        assert!(text.contains(&format!("Height: {:.2} mm", size[1])));
        assert_eq!(text.matches("Width:").count(), 2);
        assert_eq!(state.scene.history_state_id(), before);
    }
    #[test]
    fn mirror_changes_local_column_and_matching_translation_component_after_rotation() {
        let mut state = CommandState::default();
        state
            .dispatch(CommandId::AddPrimitive(PrimitiveKind::Cube))
            .expect("cube");
        state
            .set_selected_position([12., 24., 36.])
            .expect("position");
        state
            .set_selected_rotation([15., 30., 60.])
            .expect("rotate");
        let before = *state.scene.objects()[0].transform();
        let mut expected = before.rows();
        for row in expected.iter_mut().take(3) {
            row[1] *= -1.;
        }
        expected[1][3] *= -1.;
        state
            .place_selected(CommandId::Mirror, Axis::Y, Alignment::Center, 0.)
            .expect("mirror");
        assert_eq!(state.scene.objects()[0].transform().rows(), expected);
        state.dispatch(CommandId::Undo).expect("undo");
        assert_eq!(*state.scene.objects()[0].transform(), before);
    }
}

#[cfg(test)]
mod rb04_scene_order {
    use super::*;
    #[test]
    fn settle_and_measure_use_scene_order_after_reverse_click_selection() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../evidence/rebuild/rb-04-scene-and-precise-transforms/settle-oracle.json"
        ))
        .expect("oracle");
        let case = &cases[1];
        let mut state = CommandState::default();
        for (index, rows) in case["before"]
            .as_array()
            .expect("transforms")
            .iter()
            .enumerate()
        {
            state
                .scene
                .add_object(
                    SceneObject::with_details(
                        modeler3d_core::make_box([20., 30., 40.]).expect("box"),
                        None,
                        Some(format!("part {index}")),
                        None,
                        None,
                    )
                    .expect("object"),
                )
                .expect("add");
            let rows: [[f64; 4]; 4] = serde_json::from_value(rows.clone()).expect("matrix");
            state
                .scene
                .set_transform(index, Transform::from_f64_rows(rows).expect("transform"))
                .expect("set transform");
        }
        state.set_selection_indices([1, 0]);
        assert_eq!(state.scene.ordered_selection(), [1, 0]);
        state.dispatch(CommandId::Settle).expect("settle");
        for (object, expected) in state
            .scene
            .objects()
            .iter()
            .zip(case["after"].as_array().expect("after"))
        {
            let expected: [[f64; 4]; 4] =
                serde_json::from_value(expected.clone()).expect("expected");
            // Settle drops straight down (OWNER-2026-11-21-PASTE-SETTLE); the
            // Python oracle's X/Z recentring is not compared.
            let actual = object.transform().f64_rows();
            for (r, c) in (0..3).flat_map(|r| (0..4).map(move |c| (r, c))) {
                if c == 3 && r != 1 {
                    continue;
                }
                assert!((actual[r][c] - expected[r][c]).abs() < 0.002);
            }
        }
        let history = state.scene.history_state_id();
        let CommandOutcome::Applied(text) = state.dispatch(CommandId::Measure).expect("measure")
        else {
            panic!("measurement")
        };
        assert!(text.find("part 0").expect("first") < text.find("part 1").expect("second"));
        assert_eq!(state.scene.history_state_id(), history);
    }
}

#[cfg(test)]
#[path = "gesture_tests.rs"]
mod rb04_gesture_tests;
