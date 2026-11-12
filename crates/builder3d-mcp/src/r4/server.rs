//! R4 Builder3D MCP server.
//!
//! This module is the MCP adapter only.  It owns protocol negotiation,
//! profiles, schemas, filesystem policy, replay, and operation lifecycle.  It
//! calls the portable `builder3d_core` scene/mesh boundary for model behavior;
//! no UI or platform code is imported here.

use std::{
    collections::BTreeSet,
    fs,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use builder3d_core::{
    BlobOptions, BooleanOperation, BooleanOptions, BrushDirection, BrushFalloff, BrushSymmetry,
    CreasePolishOptions, ElasticGrabOptions, EmbossFont, GroupedCommand, HollowOptions, Mesh,
    NormalizeDensityOptions, PrimitiveKind, Scene, SceneChild, SceneObject, SetColorCommand,
    SetMetadataCommand, SmoothBrushOptions, TextEmbossOptions, Transform, TransformObjectCommand,
    build_parametric_torus, inspect_mesh, load_mesh_asset, load_project,
    normalize_face_density_with_hooks, repair_mesh_with_profile, save_mesh, save_project,
    simplify_mesh, smooth_mesh, subdivide_mesh,
};
use builder3d_platform::{self, SlicerError};
use rmcp::{
    ErrorData as McpError, RoleServer, ServerHandler,
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ErrorCode, Implementation,
        ListToolsResult, MetaObject, PaginatedRequestParams, ProtocolVersion, ServerCapabilities,
        ServerConfig, Tool, ToolAnnotations,
    },
    service::RequestContext,
};
use serde_json::{Map, Value, json};
use thiserror::Error;
use uuid::Uuid;

use super::{
    catalog::{self, Profile, ToolSpec},
    schema,
    security::{ApprovedRoots, SecurityError},
    state::{self, OperationPhase, OperationRecord, R4State, ViewState},
};

/// A document temporarily attached to the R4 semantic adapter.
///
/// The desktop moves its live scene into the adapter only while the UI thread
/// services one synchronous request. Long operations instead receive a scene
/// restored from a bounded snapshot and return their result for a checked
/// owner-thread commit.
pub struct R4Document {
    /// The scene and its undo/redo history.
    pub scene: Scene,
    /// Current approved project path, if any.
    pub project_path: Option<PathBuf>,
    /// Desktop dirty flag synchronized with successful save/open operations.
    pub dirty: bool,
    /// Camera and display state exposed through the MCP view tools.
    pub view: ViewState,
    /// Object landmark/floor snapping preference.
    pub snapping: bool,
    /// Bounded object clipboard shared with the desktop command state.
    pub clipboard: Option<Vec<SceneObject>>,
}

impl R4Document {
    /// Creates a document attachment with the adapter's default presentation
    /// state and an empty MCP clipboard.
    #[must_use]
    pub fn new(scene: Scene) -> Self {
        Self {
            scene,
            project_path: None,
            dirty: false,
            view: ViewState::default(),
            snapping: true,
            clipboard: None,
        }
    }
}

/// Stable launch configuration for an R4 server instance.
#[derive(Clone, Debug)]
pub struct R4Config {
    /// Published tool profile.
    pub profile: Profile,
    /// Canonical approved filesystem roots.
    pub approved_roots: ApprovedRoots,
}

impl R4Config {
    /// Loads profile and approved roots from the product environment.
    pub fn from_env() -> Result<Self, AdapterError> {
        let profile = match std::env::var("BUILDER3D_MCP_PROFILE") {
            Ok(value) => Profile::parse(&value).ok_or_else(|| {
                AdapterError::Validation(format!(
                    "unknown MCP profile {value:?}; expected observe, model, files, or full"
                ))
            })?,
            Err(std::env::VarError::NotPresent) => Profile::default(),
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(AdapterError::Validation(
                    "BUILDER3D_MCP_PROFILE must be valid UTF-8".to_owned(),
                ));
            }
        };
        Ok(Self {
            profile,
            approved_roots: ApprovedRoots::from_env()?,
        })
    }

    /// Builds a test/embedder configuration without reading environment state.
    pub fn for_profile(profile: Profile, approved_roots: ApprovedRoots) -> Self {
        Self {
            profile,
            approved_roots,
        }
    }
}

/// Typed adapter failures.  Tool failures are serialized as structured MCP
/// results; protocol routing failures remain JSON-RPC errors.
#[derive(Debug, Error)]
pub enum AdapterError {
    /// Caller arguments or a schema value is invalid.
    #[error("invalid input: {0}")]
    Validation(String),
    /// The caller's expected revision/project identity no longer matches.
    #[error("revision conflict: {0}")]
    Conflict(String),
    /// Another bounded long-running operation is active.
    #[error("another operation is already active")]
    Busy,
    /// An operation or object was not found.
    #[error("not found: {0}")]
    NotFound(String),
    /// The requested semantic operation has no safe portable implementation.
    #[error("unsupported capability: {0}")]
    Unsupported(String),
    /// A replay key was reused with different arguments.
    #[error("idempotency key conflicts with a prior request")]
    ReplayConflict,
    /// The approved-root policy rejected a file operation.
    #[error(transparent)]
    Security(#[from] SecurityError),
    /// The constrained known-slicer platform handoff failed.
    #[error(transparent)]
    Slicer(#[from] SlicerError),
    /// The portable core rejected a model or file operation.
    #[error(transparent)]
    Core(#[from] builder3d_core::CoreError),
    /// The adapter state could not be locked or serialized safely.
    #[error("adapter state failure: {0}")]
    Internal(String),
    /// The live desktop bridge is not running or did not answer.
    #[error("desktop unavailable: {0}")]
    Unavailable(String),
}

impl AdapterError {
    fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "invalid_input",
            Self::Conflict(_) => "revision_conflict",
            Self::Busy => "busy",
            Self::NotFound(_) => "not_found",
            Self::Unsupported(_) => "unsupported_capability",
            Self::ReplayConflict => "idempotency_conflict",
            Self::Security(_) => "path_policy",
            Self::Slicer(_) => "slicer_error",
            Self::Core(error) => match error {
                builder3d_core::CoreError::OperationResourceLimit { .. } => "resource_limit",
                builder3d_core::CoreError::OperationCancelled(_) => "cancelled",
                _ => "core_error",
            },
            Self::Internal(_) => "internal_error",
            Self::Unavailable(_) => "desktop_unavailable",
        }
    }

    fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Busy
                | Self::Conflict(_)
                | Self::Core(builder3d_core::CoreError::OperationResourceLimit { .. })
                | Self::Unavailable(_)
        )
    }

    /// Returns the stable structured tool-error envelope.
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({
            "code": self.code(),
            "message": self.to_string(),
            "retryable": self.retryable(),
            "details": {},
        })
    }
}

impl From<String> for AdapterError {
    fn from(message: String) -> Self {
        Self::Validation(message)
    }
}

/// One live MCP server instance.
#[derive(Clone)]
pub struct R4Server {
    state: Arc<Mutex<R4State>>,
    call_gate: Arc<Mutex<()>>,
    config: R4Config,
}

impl std::fmt::Debug for R4Server {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("R4Server")
            .field("profile", &self.config.profile)
            .field("approved_roots", &self.config.approved_roots.roots())
            .finish_non_exhaustive()
    }
}

impl R4Server {
    /// Creates a server over a fresh empty core scene.
    #[must_use]
    pub fn new(config: R4Config) -> Self {
        Self {
            state: Arc::new(Mutex::new(R4State::default())),
            call_gate: Arc::new(Mutex::new(())),
            config,
        }
    }

    /// Attaches one document while retaining this server's replay, operation,
    /// and staging records. Active long operations exclude document swapping.
    pub fn replace_document(&self, document: &mut Option<R4Document>) -> Result<(), AdapterError> {
        let mut state = self.lock_state()?;
        if state.has_active_operation() {
            return Err(AdapterError::Busy);
        }
        let Some(document) = document.take() else {
            return Err(AdapterError::Internal(
                "no desktop document was supplied".to_owned(),
            ));
        };
        state.scene = document.scene;
        state.project_path = document.project_path;
        state.dirty = document.dirty;
        state.view = document.view;
        state.snapping = document.snapping;
        state.clipboard = document.clipboard;
        Ok(())
    }

    /// Takes ownership of the attached document while retaining bounded
    /// adapter records. The caller must be the exclusive document owner.
    pub fn take_document(&self) -> Result<R4Document, AdapterError> {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
        if state.has_active_operation() {
            return Err(AdapterError::Busy);
        }
        Ok(R4Document {
            scene: std::mem::replace(&mut state.scene, Scene::new()),
            project_path: state.project_path.take(),
            dirty: state.dirty,
            view: std::mem::take(&mut state.view),
            snapping: state.snapping,
            clipboard: state.clipboard.take(),
        })
    }

    /// Returns whether a published tool uses the asynchronous operation path.
    #[must_use]
    pub fn is_long_running_tool(&self, name: &str) -> Option<bool> {
        catalog::find_tool(name)
            .filter(|spec| spec.visible_in(self.config.profile))
            .map(|spec| spec.long_running)
    }

    /// Returns whether a published tool is declared read-only.
    #[must_use]
    pub fn is_read_only_tool(&self, name: &str) -> Option<bool> {
        catalog::find_tool(name)
            .filter(|spec| spec.visible_in(self.config.profile))
            .map(|spec| spec.read_only)
    }

    /// Loads the production environment configuration and creates a server.
    pub fn from_env() -> Result<Self, AdapterError> {
        Ok(Self::new(R4Config::from_env()?))
    }

    /// Returns the configured profile.
    #[must_use]
    pub const fn profile(&self) -> Profile {
        self.config.profile
    }

    /// Returns the deterministic visible tool catalog.
    #[must_use]
    pub fn tools(&self) -> Vec<Tool> {
        catalog::TOOL_CATALOG
            .iter()
            .copied()
            .filter(|spec| spec.visible_in(self.config.profile))
            .map(|spec| self.tool(spec))
            .collect()
    }

    /// Calls one tool without going through JSON-RPC.  Integration tests use
    /// this same path as the stdio handler to avoid a second semantic adapter.
    pub fn call_value(&self, name: &str, arguments: Value) -> Result<Value, AdapterError> {
        let _call_guard = self
            .call_gate
            .lock()
            .map_err(|_| AdapterError::Internal("call mutex is poisoned".to_owned()))?;
        let spec = catalog::find_tool(name)
            .ok_or_else(|| AdapterError::NotFound(format!("unknown MCP tool: {name}")))?;
        if !spec.visible_in(self.config.profile) {
            return Err(AdapterError::NotFound(format!(
                "tool {name} is not available in profile {}",
                self.config.profile
            )));
        }
        let argument_bytes = serde_json::to_vec(&arguments)
            .map_err(|error| AdapterError::Validation(error.to_string()))?;
        if argument_bytes.len() > state::MAX_ARGUMENT_BYTES {
            return Err(AdapterError::Validation(format!(
                "argument payload exceeds {} bytes",
                state::MAX_ARGUMENT_BYTES
            )));
        }
        let arguments =
            schema::normalize_arguments(name, &arguments).map_err(AdapterError::Validation)?;
        let canonical_arguments = without_idempotency_key(&arguments);
        let replay_key = arguments
            .as_object()
            .and_then(|object| object.get("idempotency_key"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let replay_fingerprint = if replay_key.is_some() {
            Some(
                state::fingerprint(name, &canonical_arguments)
                    .map_err(|error| AdapterError::Internal(error.to_string()))?,
            )
        } else {
            None
        };
        if let (Some(key), Some(fingerprint)) = (&replay_key, &replay_fingerprint) {
            let mut state = self.lock_state()?;
            if let Some(result) = state.replay.find(key, fingerprint) {
                return result.map_err(|()| AdapterError::ReplayConflict);
            }
        }

        let result = self.dispatch(spec, arguments.clone())?;
        let result = self.shape_result(name, &arguments, result)?;
        schema::validate_result(name, &result).map_err(|message| {
            AdapterError::Internal(format!(
                "tool {name} returned a value outside its declared output schema: {message}"
            ))
        })?;
        let result_bytes = serde_json::to_vec(&result)
            .map_err(|error| AdapterError::Internal(error.to_string()))?;
        if result_bytes.len() > state::MAX_RESULT_BYTES {
            return Err(AdapterError::Internal(format!(
                "structured result exceeds {} bytes",
                state::MAX_RESULT_BYTES
            )));
        }
        if let (Some(key), Some(fingerprint)) = (replay_key, replay_fingerprint) {
            let mut state = self.lock_state()?;
            state.replay.insert(key, fingerprint, result.clone());
        }
        Ok(result)
    }

    /// Checks the adapter's bounded idempotency cache without dispatching a
    /// tool. Desktop bridges use this before returning the single-operation
    /// busy result so a legitimate retry can still replay its original value.
    pub fn replay_value(
        &self,
        name: &str,
        arguments: &Value,
    ) -> Result<Option<Value>, AdapterError> {
        let spec = catalog::find_tool(name)
            .ok_or_else(|| AdapterError::NotFound(format!("unknown MCP tool: {name}")))?;
        if !spec.visible_in(self.config.profile) {
            return Err(AdapterError::NotFound(format!(
                "tool {name} is not available in profile {}",
                self.config.profile
            )));
        }
        let argument_bytes = serde_json::to_vec(arguments)
            .map_err(|error| AdapterError::Validation(error.to_string()))?;
        if argument_bytes.len() > state::MAX_ARGUMENT_BYTES {
            return Err(AdapterError::Validation(format!(
                "argument payload exceeds {} bytes",
                state::MAX_ARGUMENT_BYTES
            )));
        }
        let arguments =
            schema::normalize_arguments(name, arguments).map_err(AdapterError::Validation)?;
        let Some(key) = arguments
            .as_object()
            .and_then(|object| object.get("idempotency_key"))
            .and_then(Value::as_str)
        else {
            return Ok(None);
        };
        let canonical_arguments = without_idempotency_key(&arguments);
        let fingerprint = state::fingerprint(name, &canonical_arguments)
            .map_err(|error| AdapterError::Internal(error.to_string()))?;
        let mut state = self.lock_state()?;
        state
            .replay
            .find(key, &fingerprint)
            .map(|result| result.map_err(|()| AdapterError::ReplayConflict))
            .transpose()
    }

    fn tool(&self, spec: ToolSpec) -> Tool {
        Self::tool_for_profile(self.config.profile, spec)
    }

    pub(super) fn tools_for_profile(profile: Profile) -> Vec<Tool> {
        catalog::TOOL_CATALOG
            .iter()
            .copied()
            .filter(|spec| spec.visible_in(profile))
            .map(|spec| Self::tool_for_profile(profile, spec))
            .collect()
    }

    pub(super) fn tool_for_profile(profile: Profile, spec: ToolSpec) -> Tool {
        let mut metadata = Map::new();
        metadata.insert(
            "com.builder3d/catalogVersion".to_owned(),
            json!(catalog::CATALOG_VERSION),
        );
        metadata.insert("com.builder3d/category".to_owned(), json!(spec.category));
        metadata.insert(
            "com.builder3d/profile".to_owned(),
            json!(spec.profile.as_str()),
        );
        metadata.insert(
            "com.builder3d/launchProfile".to_owned(),
            json!(profile.as_str()),
        );
        metadata.insert(
            "com.builder3d/longRunning".to_owned(),
            json!(spec.long_running),
        );
        Tool::new(
            spec.name,
            format!("Builder3D {} command", spec.name),
            Arc::new(schema::input_schema(spec.name)),
        )
        .with_title(spec.name.replace('_', " "))
        .with_raw_output_schema(Arc::new(schema::output_schema(spec.name)))
        .with_annotations(
            ToolAnnotations::new()
                .read_only(spec.read_only)
                .destructive(spec.destructive)
                .idempotent(spec.idempotent)
                .open_world(spec.open_world),
        )
        .with_meta(MetaObject::from(metadata))
    }

    fn lock_state(&self) -> Result<std::sync::MutexGuard<'_, R4State>, AdapterError> {
        self.state
            .lock()
            .map_err(|_| AdapterError::Internal("state mutex is poisoned".to_owned()))
    }

    fn dispatch(&self, spec: ToolSpec, arguments: Value) -> Result<Value, AdapterError> {
        if spec.long_running {
            self.start_operation(spec.name, arguments)
        } else {
            self.dispatch_sync(spec.name, arguments)
        }
    }

    fn shape_result(&self, name: &str, args: &Value, raw: Value) -> Result<Value, AdapterError> {
        let arguments = args
            .as_object()
            .ok_or_else(|| AdapterError::Validation("arguments must be an object".to_owned()))?;
        let result = match name {
            "translate"
            | "set_transform"
            | "set_color"
            | "transform_selection"
            | "apply_actions" => self.wrapped_detail_result(name, arguments, &raw)?,
            "scene_summary" => self.scene_summary(arguments.get("fields"))?,
            "select" | "settle" | "mirror" | "align" | "arrange" | "undo" | "redo"
            | "delete_selected" | "cut_selected" | "paste" | "duplicate_selected"
            | "group_selected" | "ungroup_selected" | "sample_image_color" => {
                self.scene_summary_with_changes(None, changed_object_ids(&raw))?
            }
            "snap_translate" => self.snap_translation_result(arguments, &raw)?,
            "view_state" | "set_view" | "frame_view" | "orbit_view" | "pan_view" | "zoom_view"
            | "view_preset" => self.view_state_result()?,
            "new_project" => self.project_result()?,
            "set_snapping" => self.snapping_result()?,
            "add_primitive"
            | "set_torus_parameters"
            | "elastic_grab"
            | "crease_polish"
            | "smooth_brush"
            | "blob" => self.object_mutation_result(name, arguments, &raw)?,
            _ => raw,
        };
        self.retain_declared_output_fields(name, result)
    }

    fn retain_declared_output_fields(
        &self,
        name: &str,
        result: Value,
    ) -> Result<Value, AdapterError> {
        let properties = schema::output_schema(name)
            .remove("properties")
            .and_then(|value| value.as_object().cloned())
            .ok_or_else(|| {
                AdapterError::Internal(format!("the output schema for {name} has no properties"))
            })?;
        let mut result = result.as_object().cloned().ok_or_else(|| {
            AdapterError::Internal(format!("tool {name} returned a non-object output"))
        })?;
        if !result.contains_key("changed_object_ids")
            && let Some(affected) = result.remove("affected_object_ids")
            && properties.contains_key("changed_object_ids")
        {
            result.insert("changed_object_ids".to_owned(), affected);
        }
        result.retain(|key, _| properties.contains_key(key));
        Ok(Value::Object(result))
    }

    fn wrapped_detail_result(
        &self,
        name: &str,
        args: &Map<String, Value>,
        raw: &Value,
    ) -> Result<Value, AdapterError> {
        let detail = schema::string_or(args, "return_detail", "summary")?;
        let result = match detail.as_str() {
            "none" => Value::Null,
            "changes" => self.change_result(raw)?,
            "summary" if name == "transform_selection" || name == "apply_actions" => {
                self.scene_summary_with_changes(None, changed_object_ids(raw))?
            }
            "summary" => self.object_mutation_result(name, args, raw)?,
            _ => {
                return Err(AdapterError::Validation(
                    "return_detail must be none, changes, or summary".to_owned(),
                ));
            }
        };
        Ok(json!({"result":result}))
    }

    fn change_result(&self, raw: &Value) -> Result<Value, AdapterError> {
        let state = self.lock_state()?;
        let revision = raw
            .get("scene_revision")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| state.scene.revision());
        let project_id = state.scene.project_id();
        let changed_ids = changed_object_ids(raw).unwrap_or_default();
        let changes = raw
            .get("changes")
            .filter(|value| value.is_object())
            .cloned()
            .unwrap_or_else(|| json!({"changes":[],"reset_required":false}));
        Ok(json!({
            "project_id": project_id,
            "scene_revision": revision,
            "changed_object_ids": changed_ids,
            "changes": changes,
        }))
    }

    fn scene_summary_with_changes(
        &self,
        fields: Option<&Value>,
        changed_ids: Option<Vec<String>>,
    ) -> Result<Value, AdapterError> {
        let mut summary = self.scene_summary(fields)?;
        if let Some(mut changed_ids) = changed_ids {
            changed_ids.sort();
            changed_ids.dedup();
            summary["changed_object_ids"] = json!(changed_ids);
        }
        Ok(summary)
    }

    fn object_mutation_result(
        &self,
        name: &str,
        args: &Map<String, Value>,
        raw: &Value,
    ) -> Result<Value, AdapterError> {
        let raw_object = raw.as_object().ok_or_else(|| {
            AdapterError::Internal(format!("tool {name} returned a non-object mutation result"))
        })?;
        let index = raw_object
            .get("index")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| {
                AdapterError::Internal(format!("tool {name} omitted its object index"))
            })?;
        let state = self.lock_state()?;
        let object = state.scene.object(index)?;
        if raw_object
            .get("object_id")
            .and_then(Value::as_str)
            .is_some_and(|object_id| object_id != object.object_id())
        {
            return Err(AdapterError::Internal(format!(
                "tool {name} returned an object ID that does not match index {index}"
            )));
        }
        let mut output = object_summary(index, object, None)?
            .as_object()
            .cloned()
            .ok_or_else(|| AdapterError::Internal("object summary was not an object".to_owned()))?;
        let mut changed_ids = changed_object_ids(raw).unwrap_or_default();
        if changed_ids.is_empty()
            && raw_object.get("changed").and_then(Value::as_bool) == Some(true)
        {
            changed_ids.push(object.object_id().to_owned());
        }
        if !changed_ids.is_empty() {
            changed_ids.sort();
            changed_ids.dedup();
            output.insert("changed_object_ids".to_owned(), json!(changed_ids));
        }
        output.insert("project_id".to_owned(), json!(state.scene.project_id()));
        output.insert("scene_revision".to_owned(), json!(state.scene.revision()));
        match name {
            "set_torus_parameters" => {
                if let Some(parameters) = raw_object.get("torus") {
                    output.insert("torus_parameters".to_owned(), parameters.clone());
                }
            }
            "elastic_grab" => {
                output.insert(
                    "elastic_grab".to_owned(),
                    json!({
                        "radius_mm":schema::number_or(args, "radius_mm", 10.0)?,
                        "strength":schema::number_or(args, "strength", 0.75)?,
                        "falloff":schema::string_or(args, "falloff", "smooth")?,
                        "symmetry":schema::string_or(args, "symmetry", "none")?,
                    }),
                );
            }
            "crease_polish" => {
                output.insert(
                    "crease_polish".to_owned(),
                    json!({
                        "radius_mm":schema::number_or(args, "radius_mm", 5.0)?,
                        "strength":schema::number_or(args, "strength", 0.5)?,
                        "direction":schema::string_or(args, "direction", "inward")?,
                        "falloff":schema::string_or(args, "falloff", "smooth")?,
                    }),
                );
            }
            "smooth_brush" => {
                output.insert(
                    "smooth_brush".to_owned(),
                    json!({
                        "radius_mm":schema::number_or(args, "radius_mm", 8.0)?,
                        "strength":schema::number_or(args, "strength", 0.5)?,
                        "shrinkage":schema::number_or(args, "shrinkage", 0.25)?,
                        "falloff":schema::string_or(args, "falloff", "smooth")?,
                    }),
                );
            }
            "blob" => {
                output.insert(
                    "blob".to_owned(),
                    json!({
                        "radius_mm":schema::number_or(args, "radius_mm", 8.0)?,
                        "strength":schema::number_or(args, "strength", 0.5)?,
                        "direction":schema::string_or(args, "direction", "outward")?,
                        "falloff":schema::string_or(args, "falloff", "smooth")?,
                    }),
                );
            }
            _ => {}
        }
        Ok(Value::Object(output))
    }

    fn snap_translation_result(
        &self,
        args: &Map<String, Value>,
        raw: &Value,
    ) -> Result<Value, AdapterError> {
        let details = raw
            .get("snap_translation")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                AdapterError::Internal("snap_translate omitted its details".to_owned())
            })?;
        let axis = details
            .get("axis")
            .cloned()
            .ok_or_else(|| AdapterError::Internal("snap_translate omitted its axis".to_owned()))?;
        let requested = details.get("proposed_delta_mm").cloned().ok_or_else(|| {
            AdapterError::Internal("snap_translate omitted its requested delta".to_owned())
        })?;
        let applied = details.get("applied_delta_mm").cloned().ok_or_else(|| {
            AdapterError::Internal("snap_translate omitted its applied delta".to_owned())
        })?;
        let snapped = details.get("snapped").cloned().ok_or_else(|| {
            AdapterError::Internal("snap_translate omitted its snapped flag".to_owned())
        })?;
        let tolerance = details.get("tolerance_mm").cloned().ok_or_else(|| {
            AdapterError::Internal("snap_translate omitted its tolerance".to_owned())
        })?;
        let tolerance_mode = schema::string_or(args, "tolerance_mode", "fixed_mm")?;
        let snap_translation = json!({
            "axis":axis,
            "requested_delta_mm":requested,
            "applied_delta_mm":applied,
            "snapped":snapped,
            "tolerance_mm":tolerance,
            "tolerance_mode":tolerance_mode,
        });
        let mut output = self
            .scene_summary_with_changes(None, changed_object_ids(raw))?
            .as_object()
            .cloned()
            .ok_or_else(|| AdapterError::Internal("scene summary was not an object".to_owned()))?;
        output.insert("snap_translation".to_owned(), snap_translation);
        Ok(Value::Object(output))
    }

    fn view_state_result(&self) -> Result<Value, AdapterError> {
        let state = self.lock_state()?;
        let mut output = state
            .view
            .to_value()
            .as_object()
            .cloned()
            .ok_or_else(|| AdapterError::Internal("view state was not an object".to_owned()))?;
        output.insert("project_id".to_owned(), json!(state.scene.project_id()));
        output.insert("scene_revision".to_owned(), json!(state.scene.revision()));
        Ok(Value::Object(output))
    }

    fn project_result(&self) -> Result<Value, AdapterError> {
        let state = self.lock_state()?;
        Ok(json!({
            "project_id": state.scene.project_id(),
            "scene_revision": state.scene.revision(),
            "dirty": state.dirty,
            "object_count": state.scene.objects().len(),
            "path": state.project_path.as_ref().map(|path| path.display().to_string()),
        }))
    }

    fn snapping_result(&self) -> Result<Value, AdapterError> {
        let state = self.lock_state()?;
        Ok(json!({
            "object_snapping_enabled": state.snapping,
            "tolerance_mm": 4.0,
            "tolerance_mode": "view_scaled",
            "movement_only": true,
            "shift_bypass": true,
            "project_id": state.scene.project_id(),
            "scene_revision": state.scene.revision(),
        }))
    }

    fn dispatch_sync(&self, name: &str, arguments: Value) -> Result<Value, AdapterError> {
        let args = arguments
            .as_object()
            .ok_or_else(|| AdapterError::Validation("arguments must be an object".to_owned()))?;
        match name {
            "get_status" => self.get_status(),
            "operation_status" => self.operation_status(args),
            "cancel_operation" => self.cancel_operation(args),
            "scene_summary" => self.scene_summary(args.get("fields")),
            "print_check" => self.print_check(args),
            "scene_changes" => self.scene_changes(args),
            "apply_actions" => self.apply_actions(args),
            "add_primitive" => self.add_primitive(args),
            "set_torus_parameters" => self.set_torus_parameters(args),
            "elastic_grab" => self.elastic_grab(args),
            "crease_polish" => self.crease_polish(args),
            "smooth_brush" => self.smooth_brush(args),
            "blob" => self.blob(args),
            "select" => self.select(args),
            "set_snapping" => self.set_snapping(args),
            "translate" => self.translate(args),
            "set_transform" => self.set_transform(args),
            "transform_selection" => self.transform_selection(args),
            "snap_translate" => self.snap_translate(args),
            "settle" => self.settle(args),
            "mirror" => self.mirror(args),
            "align" => self.align(args),
            "arrange" => self.arrange(args),
            "set_color" => self.set_color(args),
            "mesh_health" => self.mesh_health(args),
            "undo" => self.history(false, args),
            "redo" => self.history(true, args),
            "delete_selected" => {
                self.simple_scene_command("delete_selected", args, |scene| scene.remove_selected())
            }
            "copy_selected" => self.copy_selected(args),
            "cut_selected" => self.cut_selected(args),
            "paste" => self.paste(args),
            "duplicate_selected" => {
                self.simple_scene_command("duplicate_selected", args, Scene::duplicate_selected)
            }
            "group_selected" => {
                self.simple_scene_command("group_selected", args, Scene::group_selected)
            }
            "ungroup_selected" => {
                self.simple_scene_command("ungroup_selected", args, Scene::ungroup_selected)
            }
            "view_state" => self.view_state(),
            "set_view" => self.set_view(args),
            "frame_view" => self.frame_view(args),
            "orbit_view" => self.orbit_view(args),
            "pan_view" => self.pan_view(args),
            "zoom_view" => self.zoom_view(args),
            "view_preset" => self.view_preset(args),
            "new_project" => self.new_project(args),
            "sample_image_color" => self.sample_image_color(args),
            "discover_slicers" => self.discover_slicers(),
            "emboss_text" | "hollow" => Err(AdapterError::Internal(
                "shell operations must be dispatched as long operations".to_owned(),
            )),
            "launch_slicer" => Err(AdapterError::Internal(
                "launch_slicer must be dispatched as a long operation".to_owned(),
            )),
            _ => Err(AdapterError::NotFound(format!(
                "tool {name} is not implemented"
            ))),
        }
    }

    fn get_status(&self) -> Result<Value, AdapterError> {
        let state = self.lock_state()?;
        let active = state
            .operations
            .iter()
            .find(|record| !record.phase.is_terminal());
        Ok(json!({
            "app_state": if active.is_some() { "busy" } else { "ready" },
            "busy": active.is_some(),
            "active_operation": active.map(OperationRecord::status_value),
            "project_id": state.scene.project_id(),
            "scene_revision": state.scene.revision(),
            "dirty": state.dirty,
            "project_path": state.project_path.as_ref().map(|path| path.display().to_string()),
            "view": state.view.to_value(),
        }))
    }

    fn operation_status(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let id = schema::required_string(args, "operation_id")?;
        let mut state = self.lock_state()?;
        state.prune_operations();
        state
            .operation(&id)
            .map(OperationRecord::status_value)
            .ok_or_else(|| AdapterError::NotFound(format!("operation {id} was not found")))
    }

    fn cancel_operation(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let id = schema::required_string(args, "operation_id")?;
        let mut state = self.lock_state()?;
        state.prune_operations();
        let record = state
            .operation_mut(&id)
            .ok_or_else(|| AdapterError::NotFound(format!("operation {id} was not found")))?;
        if record.phase.is_terminal() {
            return Ok(record.status_value());
        }
        record.cancel.store(true, Ordering::Release);
        record.set_phase(
            OperationPhase::Cancelling,
            record.progress,
            "cancellation requested",
        );
        Ok(record.status_value())
    }

    fn scene_summary(&self, fields: Option<&Value>) -> Result<Value, AdapterError> {
        let fields = parse_summary_fields(fields)?;
        let state = self.lock_state()?;
        let objects = state
            .scene
            .objects()
            .iter()
            .enumerate()
            .map(|(index, object)| object_summary(index, object, fields.as_ref()))
            .collect::<Result<Vec<_>, _>>()?;
        let selected_indices = state.scene.selection().iter().copied().collect::<Vec<_>>();
        let selected_object_ids = selected_indices
            .iter()
            .filter_map(|index| state.scene.object(*index).ok())
            .map(|object| object.object_id().to_owned())
            .collect::<Vec<_>>();
        Ok(json!({
            "project_id": state.scene.project_id(),
            "scene_revision": state.scene.revision(),
            "objects": objects,
            "selected_indices": selected_indices,
            "selected_object_ids": selected_object_ids,
        }))
    }

    fn print_check(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let bed = bed_size(args)?;
        let state = self.lock_state()?;
        let mut issues = Vec::new();
        for (index, object) in state.scene.objects().iter().enumerate() {
            let mesh = object.transformed_mesh()?;
            let Some((minimum, maximum)) = mesh.bounds() else {
                continue;
            };
            let fits = (0..3)
                .all(|axis| minimum[axis] >= -bed[axis] * 0.5 && maximum[axis] <= bed[axis] * 0.5);
            if !fits {
                issues.push(format!("object {index} exceeds the print bed"));
            }
        }
        Ok(json!({
            "ok": issues.is_empty(),
            "errors": issues,
            "warnings": [],
            "project_id": state.scene.project_id(),
            "scene_revision": state.scene.revision(),
        }))
    }

    fn scene_changes(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let since = schema::integer_or(args, "since_revision", 0)?;
        let state = self.lock_state()?;
        let current = i64::try_from(state.scene.revision()).unwrap_or(i64::MAX);
        let reset_required = since != current;
        let fields = parse_summary_fields(args.get("fields"))?;
        let objects = if reset_required {
            state
                .scene
                .objects()
                .iter()
                .enumerate()
                .map(|(index, object)| object_summary(index, object, fields.as_ref()))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            Vec::new()
        };
        let changed_object_ids = if reset_required {
            state
                .scene
                .objects()
                .iter()
                .map(|object| object.object_id().to_owned())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        Ok(json!({
            "project_id": state.scene.project_id(),
            "scene_revision": current,
            "from_scene_revision": since,
            "reset_required": reset_required,
            "changes": [],
            "objects": objects,
            "changed_object_ids": changed_object_ids,
        }))
    }

    fn apply_actions(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let actions = args
            .get("actions")
            .and_then(Value::as_array)
            .ok_or_else(|| AdapterError::Validation("actions must be an array".to_owned()))?;
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let mut commands: Vec<Box<dyn builder3d_core::Command>> = Vec::with_capacity(actions.len());
        let mut pending_transforms = std::collections::BTreeMap::new();
        for action_entry in actions {
            let action_entry = action_entry.as_object().ok_or_else(|| {
                AdapterError::Validation("each action must be an object".to_owned())
            })?;
            let name = action_entry
                .get("action")
                .and_then(Value::as_str)
                .ok_or_else(|| AdapterError::Validation("action.action is required".to_owned()))?;
            let action_args = action_entry
                .get("args")
                .and_then(Value::as_object)
                .ok_or_else(|| AdapterError::Validation("action.args is required".to_owned()))?;
            match name {
                "translate" => {
                    let offset = schema::vec3_or(action_args, "offset_mm", [0.0; 3])?;
                    let index = target_index(&state.scene, action_args)?;
                    let current = pending_transforms
                        .get(&index)
                        .copied()
                        .unwrap_or(*state.scene.object(index)?.transform());
                    let transform = current.translated(offset)?;
                    pending_transforms.insert(index, transform);
                    commands.push(Box::new(TransformObjectCommand::new(index, transform)));
                }
                "set_color" => {
                    let color = color_from(action_args.get("color"), "color")?;
                    let index = target_index(&state.scene, action_args)?;
                    commands.push(Box::new(SetColorCommand::new(index, color)));
                }
                other => {
                    return Err(AdapterError::Validation(format!(
                        "unsupported action variant: {other}"
                    )));
                }
            }
        }
        let result = state.scene.execute(GroupedCommand::new(commands))?;
        state.dirty |= result.changed;
        Ok(command_output(
            "apply_actions",
            &result,
            None,
            Some(json!({"results": actions.len()})),
        ))
    }

    fn add_primitive(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let primitive_name = schema::required_string(args, "primitive")?;
        let primitive = PrimitiveKind::parse(&primitive_name)?;
        let dimensions = match args.get("dimensions_mm") {
            None | Some(Value::Null) => None,
            Some(_) => Some(schema::vec3_or(args, "dimensions_mm", [0.0; 3])?),
        };
        let name = schema::string_or(args, "name", "")?;
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let (object_id, result) =
            state
                .scene
                .add_primitive(primitive, dimensions, nonempty_name(&name))?;
        state.dirty = true;
        Ok(command_output(
            "add_primitive",
            &result,
            Some((object_id, result_index(&state.scene, &result))),
            None,
        ))
    }

    fn set_torus_parameters(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let index = target_index(&state.scene, args)?;
        let old = state.scene.object(index)?.metadata().clone();
        let current = builder3d_core::torus_parameters_from_metadata(&old)?.unwrap_or_default();
        let mut parameters = current;
        if let Some(value) = args.get("mode").and_then(Value::as_str) {
            parameters.mode = value.to_owned();
        }
        if args.contains_key("radius_mm") {
            parameters.radius_mm = schema::number_or(args, "radius_mm", parameters.radius_mm)?;
        }
        if args.contains_key("tube_mm") {
            parameters.tube_mm = schema::number_or(args, "tube_mm", parameters.tube_mm)?;
        }
        if args.contains_key("sides") {
            parameters.sides =
                usize::try_from(schema::integer_or(args, "sides", parameters.sides as i64)?)
                    .map_err(|_| AdapterError::Validation("sides is too large".to_owned()))?;
        }
        if args.contains_key("steps") {
            parameters.steps =
                usize::try_from(schema::integer_or(args, "steps", parameters.steps as i64)?)
                    .map_err(|_| AdapterError::Validation("steps is too large".to_owned()))?;
        }
        let (mesh, normalized) = build_parametric_torus(parameters)?;
        let metadata = builder3d_core::torus_metadata(normalized.clone(), Some(old))?;
        let mesh_result = state.scene.replace_mesh(index, mesh)?;
        let metadata_result = state
            .scene
            .execute(SetMetadataCommand::new(index, metadata))?;
        state.dirty |= mesh_result.changed || metadata_result.changed;
        let object_id = state.scene.object(index)?.object_id().to_owned();
        Ok(json!({
            "object_id":object_id,
            "index":index,
            "torus": {"mode":normalized.mode,"radius_mm":normalized.radius_mm,"tube_mm":normalized.tube_mm,"sides":normalized.sides,"steps":normalized.steps},
            "scene_revision":state.scene.revision(),
            "changed":mesh_result.changed || metadata_result.changed,
        }))
    }

    fn elastic_grab(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let index = target_index(&state.scene, args)?;
        let center = schema::vec3_or(args, "center_mm", [0.0; 3])?;
        let offset = schema::vec3_or(args, "offset_mm", [0.0; 3])?;
        let options = ElasticGrabOptions {
            radius_mm: schema::number_or(args, "radius_mm", 10.0)?,
            strength: schema::number_or(args, "strength", 0.75)?,
            falloff: BrushFalloff::parse(&schema::string_or(args, "falloff", "smooth")?)?,
            symmetry: BrushSymmetry::parse(&schema::string_or(args, "symmetry", "none")?)?,
            ..ElasticGrabOptions::default()
        };
        let (result, report) = state
            .scene
            .apply_elastic_grab(index, center, offset, options)?;
        state.dirty |= result.changed;
        let object_id = state.scene.object(index)?.object_id().to_owned();
        Ok(
            json!({"object_id":object_id,"index":index,"scene_revision":state.scene.revision(),"changed":result.changed,"elastic_grab":brush_report(&report)}),
        )
    }

    fn crease_polish(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let centers = centers(args)?;
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let index = target_index(&state.scene, args)?;
        let options = CreasePolishOptions {
            radius_mm: schema::number_or(args, "radius_mm", 5.0)?,
            strength: schema::number_or(args, "strength", 0.5)?,
            direction: BrushDirection::parse(&schema::string_or(args, "direction", "inward")?)?,
            falloff: BrushFalloff::parse(&schema::string_or(args, "falloff", "smooth")?)?,
            ..CreasePolishOptions::default()
        };
        let (result, report) = state.scene.apply_crease_polish(index, &centers, options)?;
        state.dirty |= result.changed;
        let object_id = state.scene.object(index)?.object_id().to_owned();
        Ok(
            json!({"object_id":object_id,"index":index,"scene_revision":state.scene.revision(),"changed":result.changed,"crease_polish":brush_report(&report)}),
        )
    }

    fn smooth_brush(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let centers = centers(args)?;
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let index = target_index(&state.scene, args)?;
        let options = SmoothBrushOptions {
            radius_mm: schema::number_or(args, "radius_mm", 8.0)?,
            strength: schema::number_or(args, "strength", 0.5)?,
            shrinkage: schema::number_or(args, "shrinkage", 0.25)?,
            falloff: BrushFalloff::parse(&schema::string_or(args, "falloff", "smooth")?)?,
            ..SmoothBrushOptions::default()
        };
        let (result, report) = state.scene.apply_smooth_brush(index, &centers, options)?;
        state.dirty |= result.changed;
        let object_id = state.scene.object(index)?.object_id().to_owned();
        Ok(
            json!({"object_id":object_id,"index":index,"scene_revision":state.scene.revision(),"changed":result.changed,"smooth_brush":brush_report(&report)}),
        )
    }

    fn blob(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let centers = centers(args)?;
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let index = target_index(&state.scene, args)?;
        let options = BlobOptions {
            radius_mm: schema::number_or(args, "radius_mm", 8.0)?,
            strength: schema::number_or(args, "strength", 0.5)?,
            direction: BrushDirection::parse(&schema::string_or(args, "direction", "outward")?)?,
            falloff: BrushFalloff::parse(&schema::string_or(args, "falloff", "smooth")?)?,
            ..BlobOptions::default()
        };
        let (result, report) = state.scene.apply_blob(index, &centers, options)?;
        state.dirty |= result.changed;
        let object_id = state.scene.object(index)?.object_id().to_owned();
        Ok(
            json!({"object_id":object_id,"index":index,"scene_revision":state.scene.revision(),"changed":result.changed,"blob":brush_report(&report)}),
        )
    }

    fn select(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let indices = schema::indices(args)?;
        let ids = schema::object_ids(args)?;
        if indices.is_none() && ids.is_none() {
            return Err(AdapterError::Validation(
                "select requires indices or object_ids".to_owned(),
            ));
        }
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let selected = resolve_indices(&state.scene, indices.as_deref(), ids.as_deref())?;
        state.scene.set_selection(selected.iter().copied());
        Ok(
            json!({"action":"select","changed":true,"scene_revision":state.scene.revision(),"selection":selected}),
        )
    }

    fn set_snapping(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let enabled = args
            .get("enabled")
            .and_then(Value::as_bool)
            .ok_or_else(|| AdapterError::Validation("enabled must be boolean".to_owned()))?;
        let mut state = self.lock_state()?;
        state.snapping = enabled;
        Ok(json!({"object_snapping_enabled":enabled}))
    }

    fn translate(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let offset = schema::vec3_or(args, "offset_mm", [0.0; 3])?;
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let index = target_index(&state.scene, args)?;
        let result = state.scene.translate(index, offset)?;
        state.dirty |= result.changed;
        Ok(command_output(
            "translate",
            &result,
            Some((state.scene.object(index)?.object_id().to_owned(), index)),
            None,
        ))
    }

    fn set_transform(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let index = target_index(&state.scene, args)?;
        let object = state.scene.object(index)?;
        let object_id = object.object_id().to_owned();
        let object_mesh_extents = object.mesh().extents();
        let object_world_extents = object.transformed_mesh()?.extents();
        let mut transform = *object.transform();
        if args
            .get("position_mm")
            .is_some_and(|value| !value.is_null())
        {
            let position = schema::vec3_or(args, "position_mm", transform.position())?;
            let mut rows = transform.rows();
            rows[0][3] = position[0];
            rows[1][3] = position[1];
            rows[2][3] = position[2];
            transform = Transform::from_rows(rows)?;
        }
        if args
            .get("rotation_deg")
            .is_some_and(|value| !value.is_null())
        {
            let rotation = schema::vec3_or(args, "rotation_deg", [0.0; 3])?;
            for (angle, axis) in [
                (rotation[0], [1.0, 0.0, 0.0]),
                (rotation[1], [0.0, 1.0, 0.0]),
                (rotation[2], [0.0, 0.0, 1.0]),
            ] {
                if angle != 0.0 {
                    transform = transform.rotated(angle, axis)?;
                }
            }
        }
        if args.get("size_mm").is_some_and(|value| !value.is_null()) {
            let desired = schema::vec3_or(args, "size_mm", object_mesh_extents)?;
            let current = object_world_extents;
            let scale = [
                safe_scale(desired[0], current[0])?,
                safe_scale(desired[1], current[1])?,
                safe_scale(desired[2], current[2])?,
            ];
            transform = transform.scaled(scale)?;
        }
        let result = state.scene.set_transform(index, transform)?;
        state.dirty |= result.changed;
        Ok(command_output(
            "set_transform",
            &result,
            Some((object_id, index)),
            None,
        ))
    }

    fn transform_selection(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let indices = selected_or_targets(&state.scene, args)?;
        let translation = schema::vec3_or(args, "translation_mm", [0.0; 3])?;
        let rotation = schema::vec3_or(args, "rotation_delta_deg", [0.0; 3])?;
        let scale = match args.get("scale_factor") {
            None | Some(Value::Null) => [1.0; 3],
            Some(_) => schema::vec3_or(args, "scale_factor", [1.0; 3])?,
        };
        let mut affected = Vec::new();
        let mut changed = false;
        for index in indices {
            let object_id = state.scene.object(index)?.object_id().to_owned();
            let mut transform = *state.scene.object(index)?.transform();
            if translation != [0.0; 3] {
                transform = transform.translated(translation)?;
            }
            for (angle, axis) in [
                (rotation[0], [1.0, 0.0, 0.0]),
                (rotation[1], [0.0, 1.0, 0.0]),
                (rotation[2], [0.0, 0.0, 1.0]),
            ] {
                if angle != 0.0 {
                    transform = transform.rotated(angle, axis)?;
                }
            }
            if scale != [1.0; 3] {
                transform = transform.scaled(scale)?;
            }
            let result = state.scene.set_transform(index, transform)?;
            changed |= result.changed;
            if result.changed {
                affected.push(object_id);
            }
        }
        state.dirty |= changed;
        Ok(
            json!({"action":"transform_selection","changed":changed,"scene_revision":state.scene.revision(),"affected_object_ids":affected}),
        )
    }

    fn snap_translate(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let axis = schema::required_string(args, "axis")?;
        let axis_index = axis_index(&axis)?;
        let proposed = schema::number_or(args, "proposed_delta_mm", 0.0)?;
        let tolerance = schema::number_or(args, "tolerance_mm", 4.0)?;
        let targets = schema::string_or(args, "targets", "objects_and_floor")?;
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let indices = selected_or_targets(&state.scene, args)?;
        let snapped = if !state.snapping {
            proposed
        } else if targets == "floor" {
            proposed.round()
        } else {
            // Object/floor snapping is a geometric operation in the desktop
            // adapter.  The portable MCP fallback uses a stable 10 mm grid
            // when the proposed delta is within the requested tolerance.
            let grid = 10.0_f32;
            let candidate = (proposed / grid).round() * grid;
            if (candidate - proposed).abs() <= tolerance.max(0.001) {
                candidate
            } else {
                proposed
            }
        };
        let mut affected = Vec::new();
        let mut changed = false;
        for index in indices {
            let mut offset = [0.0; 3];
            offset[axis_index] = snapped;
            let result = state.scene.translate(index, offset)?;
            changed |= result.changed;
            affected.extend(result.affected_object_ids);
        }
        state.dirty |= changed;
        Ok(json!({
            "action":"snap_translate",
            "changed":changed,
            "scene_revision":state.scene.revision(),
            "affected_object_ids":affected,
            "snap_translation":{"snapped":(snapped - proposed).abs() > f32::EPSILON,"axis":axis,"proposed_delta_mm":proposed,"applied_delta_mm":snapped,"tolerance_mm":tolerance},
        }))
    }

    fn settle(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let indices = if args
            .get("object_id")
            .and_then(Value::as_str)
            .is_some_and(|id| !id.is_empty())
            || args.get("index").is_some()
        {
            vec![target_index(&state.scene, args)?]
        } else {
            selected_or_targets(&state.scene, args)?
        };
        let mut affected = Vec::new();
        let mut changed = false;
        for index in indices {
            let object = state.scene.object(index)?;
            let Some((minimum, _)) = object.transformed_mesh()?.bounds() else {
                continue;
            };
            let offset = [0.0, 0.0, -minimum[2]];
            let result = state.scene.translate(index, offset)?;
            changed |= result.changed;
            affected.extend(result.affected_object_ids);
        }
        state.dirty |= changed;
        Ok(
            json!({"action":"settle","changed":changed,"scene_revision":state.scene.revision(),"affected_object_ids":affected}),
        )
    }

    fn mirror(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let axis = schema::required_string(args, "axis")?;
        let axis_index = axis_index(&axis)?;
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let index = target_index(&state.scene, args)?;
        let mut scale = [1.0; 3];
        scale[axis_index] = -1.0;
        let result = state.scene.scale(index, scale)?;
        state.dirty |= result.changed;
        Ok(command_output(
            "mirror",
            &result,
            Some((state.scene.object(index)?.object_id().to_owned(), index)),
            None,
        ))
    }

    fn align(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let axis = schema::required_string(args, "axis")?;
        let axis_index = axis_index(&axis)?;
        let mode = schema::string_or(args, "mode", "center")?;
        if !matches!(mode.as_str(), "min" | "center" | "max") {
            return Err(AdapterError::Validation(
                "mode must be min, center, or max".to_owned(),
            ));
        }
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let indices = selected_or_targets(&state.scene, args)?;
        if indices.is_empty() {
            return Err(AdapterError::Validation(
                "align requires at least one object".to_owned(),
            ));
        }
        let mut bounds = Vec::new();
        for index in &indices {
            bounds.push((
                *index,
                state
                    .scene
                    .object(*index)?
                    .transformed_mesh()?
                    .bounds()
                    .ok_or_else(|| AdapterError::Validation("object has no bounds".to_owned()))?,
            ));
        }
        let reference = match mode.as_str() {
            "min" => bounds
                .iter()
                .map(|(_, (minimum, _))| minimum[axis_index])
                .fold(f32::INFINITY, f32::min),
            "max" => bounds
                .iter()
                .map(|(_, (_, maximum))| maximum[axis_index])
                .fold(f32::NEG_INFINITY, f32::max),
            _ => {
                bounds
                    .iter()
                    .map(|(_, (minimum, maximum))| {
                        (minimum[axis_index] + maximum[axis_index]) * 0.5
                    })
                    .sum::<f32>()
                    / bounds.len() as f32
            }
        };
        let mut affected = Vec::new();
        for (index, (minimum, maximum)) in bounds {
            let current = match mode.as_str() {
                "min" => minimum[axis_index],
                "max" => maximum[axis_index],
                _ => (minimum[axis_index] + maximum[axis_index]) * 0.5,
            };
            let mut offset = [0.0; 3];
            offset[axis_index] = reference - current;
            let result = state.scene.translate(index, offset)?;
            if result.changed {
                affected.extend(result.affected_object_ids);
            }
        }
        let changed = !affected.is_empty();
        state.dirty |= changed;
        Ok(
            json!({"action":"align","changed":changed,"scene_revision":state.scene.revision(),"affected_object_ids":affected,"axis":axis,"mode":mode}),
        )
    }

    fn arrange(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let axis = schema::required_string(args, "axis")?;
        let axis_index = axis_index(&axis)?;
        let gap = schema::number_or(args, "gap_mm", 0.0)?;
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let indices = selected_or_targets(&state.scene, args)?;
        let mut entries = Vec::new();
        for index in indices {
            let object = state.scene.object(index)?;
            let bounds = object
                .transformed_mesh()?
                .bounds()
                .ok_or_else(|| AdapterError::Validation("object has no bounds".to_owned()))?;
            entries.push((index, bounds.0[axis_index], bounds.1[axis_index]));
        }
        entries.sort_by(|left, right| left.1.total_cmp(&right.1));
        let mut cursor = entries.first().map(|entry| entry.1).unwrap_or(0.0);
        let mut affected = Vec::new();
        for (index, minimum, maximum) in entries {
            let offset_value = cursor - minimum;
            let mut offset = [0.0; 3];
            offset[axis_index] = offset_value;
            let result = state.scene.translate(index, offset)?;
            if result.changed {
                affected.extend(result.affected_object_ids);
            }
            cursor = maximum + offset_value + gap;
        }
        let changed = !affected.is_empty();
        state.dirty |= changed;
        Ok(
            json!({"action":"arrange","changed":changed,"scene_revision":state.scene.revision(),"affected_object_ids":affected,"axis":axis,"gap_mm":gap}),
        )
    }

    fn set_color(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let color = color_from(args.get("color"), "color")?;
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let index = target_index(&state.scene, args)?;
        let object_id = state.scene.object(index)?.object_id().to_owned();
        let result = state.scene.set_color(index, color)?;
        state.dirty |= result.changed;
        Ok(command_output(
            "set_color",
            &result,
            Some((object_id, index)),
            None,
        ))
    }

    fn mesh_health(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let state = self.lock_state()?;
        let index = match schema::integer_or(args, "index", -1)? {
            value if value >= 0 => usize::try_from(value)
                .map_err(|_| AdapterError::Validation("index is too large".to_owned()))?,
            _ => target_index_or_first(&state.scene)?,
        };
        let object = state.scene.object(index)?;
        let health = inspect_mesh(object.mesh())?;
        Ok(json!({
            "index": index,
            "object_id": object.object_id(),
            "name": object.name(),
            "health": health_value(&health),
            "project_id": state.scene.project_id(),
            "scene_revision": state.scene.revision(),
        }))
    }

    fn history(&self, redo: bool, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let result = if redo {
            state.scene.redo()?
        } else {
            state.scene.undo()?
        };
        let Some(result) = result else {
            return Ok(
                json!({"action":if redo { "redo" } else { "undo" },"changed":false,"scene_revision":state.scene.revision(),"affected_object_ids":[]}),
            );
        };
        state.dirty = true;
        Ok(command_output(
            if redo { "redo" } else { "undo" },
            &result,
            None,
            None,
        ))
    }

    fn simple_scene_command<F>(
        &self,
        name: &str,
        args: &Map<String, Value>,
        command: F,
    ) -> Result<Value, AdapterError>
    where
        F: FnOnce(&mut Scene) -> Result<builder3d_core::CommandResult, builder3d_core::CoreError>,
    {
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let result = command(&mut state.scene)?;
        state.dirty |= result.changed;
        Ok(command_output(name, &result, None, None))
    }

    fn copy_selected(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let selection = state.scene.selection().iter().copied().collect::<Vec<_>>();
        if selection.is_empty() {
            return Err(AdapterError::Validation(
                "copy_selected requires a selection".to_owned(),
            ));
        }
        let clipboard = state.scene.copy_selected()?;
        validate_clipboard(&clipboard)?;
        state.clipboard = Some(clipboard);
        Ok(json!({
            "copied": selection.len(),
            "project_id": state.scene.project_id(),
            "scene_revision": state.scene.revision(),
        }))
    }

    fn cut_selected(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        if state.scene.selection().is_empty() {
            return Err(AdapterError::Validation(
                "cut_selected requires a selection".to_owned(),
            ));
        }
        let clipboard = state.scene.copy_selected()?;
        validate_clipboard(&clipboard)?;
        let result = state.scene.remove_selected()?;
        state.clipboard = Some(clipboard);
        state.dirty |= result.changed;
        Ok(command_output("cut_selected", &result, None, None))
    }

    fn paste(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let clipboard = state.clipboard.clone().ok_or_else(|| {
            AdapterError::Validation(
                "paste requires a prior copy_selected or cut_selected".to_owned(),
            )
        })?;
        let result = state.scene.paste_objects(&clipboard)?;
        state.dirty |= result.changed;
        Ok(command_output("paste", &result, None, None))
    }

    fn view_state(&self) -> Result<Value, AdapterError> {
        let state = self.lock_state()?;
        Ok(state.view.to_value())
    }

    fn set_view(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected_project(&state, args)?;
        if args.get("yaw_deg").is_some_and(|value| !value.is_null()) {
            state.view.yaw_deg = schema::number_or(args, "yaw_deg", state.view.yaw_deg)?;
        }
        if args.get("pitch_deg").is_some_and(|value| !value.is_null()) {
            state.view.pitch_deg =
                schema::number_or(args, "pitch_deg", state.view.pitch_deg)?.clamp(-89.0, 89.0);
        }
        if args
            .get("distance_mm")
            .is_some_and(|value| !value.is_null())
        {
            state.view.distance_mm =
                schema::number_or(args, "distance_mm", state.view.distance_mm)?.max(0.5);
        }
        if args.get("center_mm").is_some_and(|value| !value.is_null()) {
            state.view.center_mm = schema::vec3_or(args, "center_mm", state.view.center_mm)?;
        }
        if let Some(value) = args.get("show_grid").and_then(Value::as_bool) {
            state.view.show_grid = value;
        }
        if let Some(value) = args.get("wireframe").and_then(Value::as_bool) {
            state.view.wireframe = value;
        }
        if let Some(value) = args.get("xray").and_then(Value::as_bool) {
            state.view.xray = value;
        }
        Ok(
            json!({"action":"set_view","view":state.view.to_value(),"scene_revision":state.scene.revision()}),
        )
    }

    fn frame_view(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let target = schema::string_or(args, "target", "all")?;
        let mut state = self.lock_state()?;
        check_expected_project(&state, args)?;
        let indices = if target == "selected" {
            state.scene.selection().iter().copied().collect::<Vec<_>>()
        } else {
            (0..state.scene.objects().len()).collect::<Vec<_>>()
        };
        let mut points = Vec::new();
        for index in indices {
            if let Some((minimum, maximum)) =
                state.scene.object(index)?.transformed_mesh()?.bounds()
            {
                points.push((minimum, maximum));
            }
        }
        if !points.is_empty() {
            let minimum = points.iter().fold([f32::INFINITY; 3], |mut acc, (lo, _)| {
                for axis in 0..3 {
                    acc[axis] = acc[axis].min(lo[axis]);
                }
                acc
            });
            let maximum = points
                .iter()
                .fold([f32::NEG_INFINITY; 3], |mut acc, (_, hi)| {
                    for axis in 0..3 {
                        acc[axis] = acc[axis].max(hi[axis]);
                    }
                    acc
                });
            state.view.center_mm = [
                (minimum[0] + maximum[0]) * 0.5,
                (minimum[1] + maximum[1]) * 0.5,
                (minimum[2] + maximum[2]) * 0.5,
            ];
            let extents = [
                maximum[0] - minimum[0],
                maximum[1] - minimum[1],
                maximum[2] - minimum[2],
            ];
            state.view.distance_mm = extents.into_iter().fold(1.0, f32::max) * 2.0;
        }
        Ok(
            json!({"action":"frame_view","view":state.view.to_value(),"scene_revision":state.scene.revision()}),
        )
    }

    fn orbit_view(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected_project(&state, args)?;
        state.view.yaw_deg += schema::number_or(args, "yaw_delta_deg", 0.0)?;
        state.view.pitch_deg = (state.view.pitch_deg
            + schema::number_or(args, "pitch_delta_deg", 0.0)?)
        .clamp(-89.0, 89.0);
        Ok(
            json!({"action":"orbit_view","view":state.view.to_value(),"scene_revision":state.scene.revision()}),
        )
    }

    fn pan_view(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let offset = schema::vec3_or(args, "offset_mm", [0.0; 3])?;
        let mut state = self.lock_state()?;
        check_expected_project(&state, args)?;
        for (center, delta) in state.view.center_mm.iter_mut().zip(offset) {
            *center += delta;
        }
        Ok(
            json!({"action":"pan_view","view":state.view.to_value(),"scene_revision":state.scene.revision()}),
        )
    }

    fn zoom_view(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let factor = schema::number_or(args, "factor", 1.0)?;
        let mut state = self.lock_state()?;
        check_expected_project(&state, args)?;
        state.view.distance_mm = (state.view.distance_mm / factor).clamp(0.5, 5000.0);
        Ok(
            json!({"action":"zoom_view","view":state.view.to_value(),"scene_revision":state.scene.revision()}),
        )
    }

    fn view_preset(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let preset = schema::required_string(args, "preset")?;
        let mut state = self.lock_state()?;
        check_expected_project(&state, args)?;
        match preset.as_str() {
            "reset" => state.view = ViewState::default(),
            "top" => {
                state.view.yaw_deg = 0.0;
                state.view.pitch_deg = -89.0;
            }
            "front" => {
                state.view.yaw_deg = 0.0;
                state.view.pitch_deg = 0.0;
            }
            "side" => {
                state.view.yaw_deg = 90.0;
                state.view.pitch_deg = 0.0;
            }
            _ => {
                return Err(AdapterError::Validation(
                    "preset must be reset, top, front, or side".to_owned(),
                ));
            }
        }
        Ok(
            json!({"action":"view_preset","view":state.view.to_value(),"scene_revision":state.scene.revision()}),
        )
    }

    fn new_project(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        if state.dirty {
            return Err(AdapterError::Conflict(
                "the current project has unsaved changes; save it before creating or opening a project"
                    .to_owned(),
            ));
        }
        state.scene = Scene::new();
        state.project_path = None;
        state.dirty = false;
        state.view = ViewState::default();
        state.clipboard = None;
        Ok(
            json!({"action":"new_project","changed":true,"project_id":state.scene.project_id(),"scene_revision":state.scene.revision()}),
        )
    }

    fn sample_image_color(&self, args: &Map<String, Value>) -> Result<Value, AdapterError> {
        let path = schema::required_string(args, "path")?;
        let path = self.config.approved_roots.read_path(path)?;
        let bytes = fs::read(&path)
            .map_err(|error| AdapterError::Internal(format!("image read failed: {error}")))?;
        if bytes.len() > state::MAX_ARGUMENT_BYTES {
            return Err(AdapterError::Validation(
                "image exceeds the bounded sample size".to_owned(),
            ));
        }
        let color = parse_ppm_average(&bytes).ok_or_else(|| {
            AdapterError::Unsupported(
                "only bounded PPM image sampling is available in the portable R4 adapter"
                    .to_owned(),
            )
        })?;
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let indices = state.scene.selection().iter().copied().collect::<Vec<_>>();
        if indices.is_empty() {
            return Err(AdapterError::Validation(
                "sample_image_color requires a selection".to_owned(),
            ));
        }
        let commands = indices
            .iter()
            .filter_map(|index| {
                state
                    .scene
                    .object(*index)
                    .ok()
                    .filter(|object| object.color() != color)
                    .map(|_| {
                        Box::new(SetColorCommand::new(*index, color))
                            as Box<dyn builder3d_core::Command>
                    })
            })
            .collect::<Vec<_>>();
        let result = state.scene.execute(GroupedCommand::new(commands))?;
        state.dirty |= result.changed;
        drop(state);
        self.scene_summary(None)
    }

    fn discover_slicers(&self) -> Result<Value, AdapterError> {
        let slicers = builder3d_platform::discover_slicers()
            .into_iter()
            .map(|slicer| {
                json!({
                    "key": slicer.key,
                    "label": slicer.label,
                    "path": slicer.path.display().to_string(),
                    "stl_fallback": slicer.stl_fallback(),
                })
            })
            .collect::<Vec<_>>();
        Ok(json!({"slicers": slicers}))
    }

    fn start_operation(&self, action: &str, arguments: Value) -> Result<Value, AdapterError> {
        if let Some(args) = arguments.as_object() {
            let state = self.lock_state()?;
            check_expected(&state, args)?;
            if action == "open_project" && state.dirty {
                return Err(AdapterError::Conflict(
                    "the current project has unsaved changes; save it before creating or opening a project"
                        .to_owned(),
                ));
            }
        }
        let (operation_id, cancel) = {
            let mut state = self.lock_state()?;
            state.prune_operations();
            if state.has_active_operation() {
                return Err(AdapterError::Busy);
            }
            let record = OperationRecord::new(action);
            let id = record.id.clone();
            let cancel = Arc::clone(&record.cancel);
            state.operations.push_back(record);
            (id, cancel)
        };
        let server = self.clone();
        let action_name = action.to_owned();
        let operation_arguments = arguments;
        let thread_name = format!("builder3d-r4-{action_name}");
        let worker_operation_id = operation_id.clone();
        let spawn_result = thread::Builder::new().name(thread_name).spawn(move || {
            server.update_operation(&worker_operation_id, |record| {
                record.set_phase(OperationPhase::Running, 5, "running")
            });
            let result = server.execute_long_operation(
                &worker_operation_id,
                &action_name,
                operation_arguments,
                &cancel,
            );
            match result {
                Ok(value) => server.update_operation(&worker_operation_id, |record| {
                    record.result = Some(value);
                    record.set_phase(OperationPhase::Completed, 100, "completed");
                }),
                Err(error) => server.update_operation(&worker_operation_id, |record| {
                    if matches!(
                        error,
                        AdapterError::Core(builder3d_core::CoreError::OperationCancelled(_))
                    ) || cancel.load(Ordering::Acquire)
                    {
                        record.error = Some(
                            AdapterError::Core(builder3d_core::CoreError::OperationCancelled(
                                action_name.clone(),
                            ))
                            .to_value(),
                        );
                        record.set_phase(OperationPhase::Cancelled, record.progress, "cancelled");
                    } else {
                        record.error = Some(error.to_value());
                        record.set_phase(OperationPhase::Failed, record.progress, "failed");
                    }
                }),
            }
        });
        if let Err(error) = spawn_result {
            let failure =
                AdapterError::Internal(format!("operation worker could not start: {error}"));
            self.update_operation(&operation_id, |record| {
                record.error = Some(failure.to_value());
                record.set_phase(OperationPhase::Failed, 0, "worker start failed");
            });
            return Err(failure);
        }
        let state = self.lock_state()?;
        state
            .operation(&operation_id)
            .map(OperationRecord::status_value)
            .ok_or_else(|| {
                AdapterError::Internal("operation disappeared after creation".to_owned())
            })
    }

    fn update_operation<F>(&self, id: &str, update: F)
    where
        F: FnOnce(&mut OperationRecord),
    {
        if let Ok(mut state) = self.state.lock()
            && let Some(record) = state.operation_mut(id)
        {
            update(record);
        }
    }

    fn execute_long_operation(
        &self,
        id: &str,
        action: &str,
        arguments: Value,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Value, AdapterError> {
        if cancel.load(Ordering::Acquire) {
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled(action.to_owned()),
            ));
        }
        match action {
            "normalize_face_density" | "repair" | "simplify" | "subdivide" | "smooth" => {
                self.execute_mesh_operation(id, action, &arguments, cancel)
            }
            "boolean_merge" | "boolean_subtract" | "boolean_intersect" => {
                self.execute_boolean_operation(action, &arguments, cancel)
            }
            "import_file" => self.execute_import(&arguments, cancel),
            "export_file" => self.execute_export(&arguments, cancel),
            "open_project" => self.execute_open_project(&arguments, cancel),
            "save_project" | "save_project_as" => {
                self.execute_save_project(action, &arguments, cancel)
            }
            "emboss_text" | "hollow" => {
                self.execute_shell_operation(id, action, &arguments, cancel)
            }
            "launch_slicer" => self.execute_launch_slicer(id, &arguments, cancel),
            _ => Err(AdapterError::Internal(format!(
                "no long-operation handler for {action}"
            ))),
        }
    }

    fn execute_mesh_operation(
        &self,
        id: &str,
        action: &str,
        arguments: &Value,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Value, AdapterError> {
        let args = arguments
            .as_object()
            .ok_or_else(|| AdapterError::Validation("arguments must be an object".to_owned()))?;
        let (index, object_id, revision, source) = {
            let state = self.lock_state()?;
            check_expected(&state, args)?;
            let index = target_index(&state.scene, args)?;
            let object = state.scene.object(index)?;
            (
                index,
                object.object_id().to_owned(),
                state.scene.revision(),
                object.mesh().clone(),
            )
        };
        if cancel.load(Ordering::Acquire) {
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled(action.to_owned()),
            ));
        }
        let output = match action {
            "normalize_face_density" => {
                let face_indices = optional_usize_list(args, "face_indices")?;
                let centers = optional_centers(args)?;
                let options = NormalizeDensityOptions {
                    radius_mm: schema::number_or(args, "radius_mm", 8.0)?,
                    target_edge_length_mm: optional_number(args, "target_edge_length_mm")?,
                    max_iterations: usize::try_from(schema::integer_or(args, "max_iterations", 4)?)
                        .map_err(|_| {
                            AdapterError::Validation("max_iterations is too large".to_owned())
                        })?,
                    ..NormalizeDensityOptions::default()
                };
                let mut cancel_check = || {
                    if cancel.load(Ordering::Acquire) {
                        Err(builder3d_core::CoreError::OperationCancelled(
                            action.to_owned(),
                        ))
                    } else {
                        Ok(())
                    }
                };
                let mut progress = |value: u8, message: &str| {
                    self.update_operation(id, |record| {
                        record.set_phase(OperationPhase::Running, value.min(95), message)
                    });
                };
                let result = normalize_face_density_with_hooks(
                    &source,
                    face_indices.as_deref(),
                    centers.as_deref(),
                    options,
                    Some(&mut cancel_check),
                    Some(&mut progress),
                )?;
                let report = normalize_report(&result.report);
                (
                    result.mesh,
                    json!({"action":action,"operation_id":id,"index":index,"object_id":object_id,"report":report}),
                )
            }
            "repair" => {
                let profile = schema::string_or(args, "profile", "extended")?;
                let result = repair_mesh_with_profile(&source, &profile)?;
                (
                    result.mesh,
                    json!({"action":action,"operation_id":id,"index":index,"object_id":object_id,"report":repair_report(&result.report)}),
                )
            }
            "simplify" => {
                let face_count = optional_usize(args, "face_count")?;
                let result = simplify_mesh(&source, face_count)?;
                (
                    result.mesh,
                    json!({"action":action,"operation_id":id,"index":index,"object_id":object_id,"report":simplify_report(&result.report)}),
                )
            }
            "subdivide" => {
                let result = subdivide_mesh(&source)?;
                (
                    result.mesh,
                    json!({"action":action,"operation_id":id,"index":index,"object_id":object_id,"report":subdivide_report(&result.report)}),
                )
            }
            "smooth" => {
                let iterations = usize::try_from(schema::integer_or(args, "iterations", 1)?)
                    .map_err(|_| AdapterError::Validation("iterations is too large".to_owned()))?;
                let result = smooth_mesh(&source, iterations)?;
                (
                    result.mesh,
                    json!({"action":action,"operation_id":id,"index":index,"object_id":object_id,"report":smooth_report(&result.report)}),
                )
            }
            _ => {
                return Err(AdapterError::Internal(format!(
                    "unknown mesh operation {action}"
                )));
            }
        };
        if cancel.load(Ordering::Acquire) {
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled(action.to_owned()),
            ));
        }
        self.update_operation(id, |record| {
            record.set_phase(OperationPhase::Committing, 95, "committing")
        });
        let (mesh, mut result) = output;
        let mut state = self.lock_state()?;
        if state.scene.revision() != revision {
            return Err(AdapterError::Conflict(
                "scene changed while the operation was running".to_owned(),
            ));
        }
        let command = state.scene.replace_mesh(index, mesh)?;
        state.dirty |= command.changed;
        if let Some(object) = result.as_object_mut() {
            object.insert("changed".to_owned(), json!(command.changed));
            object.insert("scene_revision".to_owned(), json!(state.scene.revision()));
        }
        Ok(result)
    }

    fn execute_shell_operation(
        &self,
        id: &str,
        action: &str,
        arguments: &Value,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Value, AdapterError> {
        let args = arguments
            .as_object()
            .ok_or_else(|| AdapterError::Validation("arguments must be an object".to_owned()))?;
        let (index, object_id, revision, source) = {
            let state = self.lock_state()?;
            check_expected(&state, args)?;
            let index = target_index(&state.scene, args)?;
            let object = state.scene.object(index)?;
            (
                index,
                object.object_id().to_owned(),
                state.scene.revision(),
                object.mesh().clone(),
            )
        };
        if cancel.load(Ordering::Acquire) {
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled(action.to_owned()),
            ));
        }
        let mut cancel_check = || {
            if cancel.load(Ordering::Acquire) {
                Err(builder3d_core::CoreError::OperationCancelled(
                    action.to_owned(),
                ))
            } else {
                Ok(())
            }
        };
        let (mesh, mut result) = match action {
            "emboss_text" => {
                let font_size_value = schema::integer_or(args, "font_size", 10)?;
                let font_size = if font_size_value < 0 {
                    return Err(AdapterError::Validation(
                        "font_size must be a positive integer".to_owned(),
                    ));
                } else {
                    u32::try_from(font_size_value).map_err(|_| {
                        AdapterError::Validation("font_size is too large".to_owned())
                    })?
                };
                let font_name = schema::string_or(args, "font_name", "default")?;
                let font = EmbossFont::parse(&font_name).ok_or_else(|| {
                    AdapterError::Validation(
                        "font_name must be default, sans, serif, or mono".to_owned(),
                    )
                })?;
                let options = TextEmbossOptions {
                    text: schema::required_string(args, "text")?,
                    font_size,
                    depth_mm: schema::number_or(args, "depth_mm", 1.0)?,
                    raised: args.get("raised").and_then(Value::as_bool).unwrap_or(true),
                    font,
                    offset_x_mm: schema::number_or(args, "offset_x_mm", 0.0)?,
                    offset_z_mm: schema::number_or(args, "offset_z_mm", 0.0)?,
                    limits: Default::default(),
                };
                let result = builder3d_core::text_emboss_with_hooks(
                    &source,
                    options,
                    Some(&mut cancel_check),
                )?;
                (
                    result.mesh,
                    json!({
                        "action": action,
                        "operation_id": id,
                        "index": index,
                        "object_id": object_id,
                        "report": text_emboss_report(&result.report)
                    }),
                )
            }
            "hollow" => {
                let options = HollowOptions {
                    wall_thickness_mm: schema::number_or(args, "wall_thickness_mm", 1.0)?,
                    limits: Default::default(),
                };
                let result = builder3d_core::hollow_mesh_with_hooks(
                    &source,
                    options,
                    Some(&mut cancel_check),
                )?;
                (
                    result.mesh,
                    json!({
                        "action": action,
                        "operation_id": id,
                        "index": index,
                        "object_id": object_id,
                        "report": hollow_report(&result.report)
                    }),
                )
            }
            _ => {
                return Err(AdapterError::Internal(format!(
                    "unknown shell operation {action}"
                )));
            }
        };
        if cancel.load(Ordering::Acquire) {
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled(action.to_owned()),
            ));
        }
        self.update_operation(id, |record| {
            record.set_phase(OperationPhase::Committing, 95, "committing")
        });
        let mut state = self.lock_state()?;
        if state.scene.revision() != revision {
            return Err(AdapterError::Conflict(
                "scene changed while the operation was running".to_owned(),
            ));
        }
        let command = state.scene.replace_mesh(index, mesh)?;
        state.dirty |= command.changed;
        if let Some(object) = result.as_object_mut() {
            object.insert("changed".to_owned(), json!(command.changed));
            object.insert("scene_revision".to_owned(), json!(state.scene.revision()));
        }
        Ok(result)
    }

    fn execute_boolean_operation(
        &self,
        action: &str,
        arguments: &Value,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Value, AdapterError> {
        let args = arguments
            .as_object()
            .ok_or_else(|| AdapterError::Validation("arguments must be an object".to_owned()))?;
        let operation = match action {
            "boolean_merge" => BooleanOperation::Union,
            "boolean_subtract" => BooleanOperation::Difference,
            "boolean_intersect" => BooleanOperation::Intersection,
            _ => return Err(AdapterError::Internal("invalid boolean action".to_owned())),
        };
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        if cancel.load(Ordering::Acquire) {
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled(action.to_owned()),
            ));
        }
        let target_id = schema::required_string(args, "target_object_id")?;
        let tool_id = schema::required_string(args, "tool_object_id")?;
        let target_index = state.scene.object_index(&target_id)?;
        let tool_index = state.scene.object_index(&tool_id)?;
        let (new_index, result, report) = state.scene.apply_boolean(
            target_index,
            tool_index,
            operation,
            BooleanOptions::default(),
        )?;
        state.dirty |= result.changed;
        Ok(
            json!({"action":action,"changed":result.changed,"scene_revision":state.scene.revision(),"index":new_index,"object_id":state.scene.object(new_index)?.object_id(),"report":boolean_report(&report)}),
        )
    }

    fn execute_import(
        &self,
        arguments: &Value,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Value, AdapterError> {
        let args = arguments
            .as_object()
            .ok_or_else(|| AdapterError::Validation("arguments must be an object".to_owned()))?;
        let raw_path = schema::required_string(args, "path")?;
        let path = self.config.approved_roots.read_path(raw_path)?;
        let asset = load_mesh_asset(&path)?;
        if cancel.load(Ordering::Acquire) {
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled("import_file".to_owned()),
            ));
        }
        let factor = unit_factor(&schema::string_or(args, "unit", "mm")?)?;
        let mesh = scale_mesh(asset.mesh(), factor)?;
        let color = asset.display_color();
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        let object = SceneObject::with_details(
            mesh,
            None,
            Some(
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Imported mesh")
                    .to_owned(),
            ),
            color,
            None,
        )?;
        let object_id = object.object_id().to_owned();
        let result = state.scene.add_object(object)?;
        state.dirty = true;
        Ok(
            json!({"action":"import_file","changed":result.changed,"scene_revision":state.scene.revision(),"object_id":object_id,"index":state.scene.object_index(&object_id)?}),
        )
    }

    fn execute_export(
        &self,
        arguments: &Value,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Value, AdapterError> {
        let args = arguments
            .as_object()
            .ok_or_else(|| AdapterError::Validation("arguments must be an object".to_owned()))?;
        let raw_path = schema::required_string(args, "path")?;
        let overwrite = args
            .get("overwrite")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let path = self.config.approved_roots.write_path(raw_path, overwrite)?;
        let mesh = {
            let state = self.lock_state()?;
            check_expected(&state, args)?;
            let indices = selected_or_targets(&state.scene, args)
                .unwrap_or_else(|_| (0..state.scene.objects().len()).collect());
            let meshes = indices
                .iter()
                .filter_map(|index| state.scene.object(*index).ok())
                .map(|object| object.transformed_mesh())
                .collect::<Result<Vec<_>, _>>()?;
            Mesh::concatenate(&meshes)?
        };
        if cancel.load(Ordering::Acquire) {
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled("export_file".to_owned()),
            ));
        }
        save_mesh(&mesh, &path)?;
        Ok(
            json!({"action":"export_file","path":path.display().to_string(),"bytes":fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or(0),"changed":false}),
        )
    }

    fn execute_launch_slicer(
        &self,
        operation_id: &str,
        arguments: &Value,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Value, AdapterError> {
        let args = arguments
            .as_object()
            .ok_or_else(|| AdapterError::Validation("arguments must be an object".to_owned()))?;
        let slicer_key = schema::required_string(args, "slicer_key")?;
        let slicer = builder3d_platform::discover_slicers()
            .into_iter()
            .find(|slicer| slicer.key == slicer_key || slicer.path.to_string_lossy() == slicer_key)
            .ok_or_else(|| SlicerError::UnknownSlicer(slicer_key.clone()))?;
        const MAX_SLICER_OBJECTS: usize = 64;

        self.update_operation(operation_id, |record| {
            record.set_phase(OperationPhase::Running, 15, "capturing visible scene")
        });
        let (project_id, revision, meshes) = {
            let state = self.lock_state()?;
            check_expected(&state, args)?;
            let visible = state
                .scene
                .objects()
                .iter()
                .filter(|object| object.is_visible())
                .collect::<Vec<_>>();
            if visible.is_empty() {
                return Err(AdapterError::Validation(
                    "there are no visible objects to send to a slicer".to_owned(),
                ));
            }
            if visible.len() > MAX_SLICER_OBJECTS {
                return Err(AdapterError::Core(
                    builder3d_core::CoreError::OperationResourceLimit {
                        operation: "launch_slicer".to_owned(),
                        reason: format!(
                            "visible object count {} exceeds the {} object handoff limit",
                            visible.len(),
                            MAX_SLICER_OBJECTS
                        ),
                    },
                ));
            }
            let meshes = visible
                .iter()
                .map(|object| object.transformed_mesh())
                .collect::<Result<Vec<_>, _>>()?;
            (
                state.scene.project_id().to_owned(),
                state.scene.revision(),
                meshes,
            )
        };
        if cancel.load(Ordering::Acquire) {
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled("launch_slicer".to_owned()),
            ));
        }

        let staging = create_staging_directory()?;
        let mut paths = Vec::new();
        let format = if slicer.stl_fallback() { "stl" } else { "3mf" };
        let staging_result = (|| -> Result<(), AdapterError> {
            if slicer.stl_fallback() {
                for (index, mesh) in meshes.iter().enumerate() {
                    if cancel.load(Ordering::Acquire) {
                        return Err(AdapterError::Core(
                            builder3d_core::CoreError::OperationCancelled(
                                "launch_slicer".to_owned(),
                            ),
                        ));
                    }
                    let path = staging.join(format!("Builder3D_Print_{:03}.stl", index + 1));
                    save_mesh(mesh, &path)?;
                    paths.push(path);
                    let progress =
                        20_u8.saturating_add(((index + 1).saturating_mul(50) / meshes.len()) as u8);
                    self.update_operation(operation_id, |record| {
                        record.set_phase(OperationPhase::Running, progress, "staging STL models")
                    });
                }
            } else {
                let mesh = Mesh::concatenate(&meshes)?;
                let path = staging.join("Builder3D_Print.3mf");
                save_mesh(&mesh, &path)?;
                paths.push(path);
                self.update_operation(operation_id, |record| {
                    record.set_phase(OperationPhase::Running, 70, "staging 3MF model")
                });
            }
            Ok(())
        })();
        if let Err(error) = staging_result {
            let _ = fs::remove_dir_all(&staging);
            return Err(error);
        }
        if cancel.load(Ordering::Acquire) {
            let _ = fs::remove_dir_all(&staging);
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled("launch_slicer".to_owned()),
            ));
        }

        self.update_operation(operation_id, |record| {
            record.set_phase(OperationPhase::Committing, 85, "starting discovered slicer")
        });
        let launch = {
            let mut state = self.lock_state()?;
            if let Err(error) = check_expected(&state, args) {
                let _ = fs::remove_dir_all(&staging);
                return Err(error);
            }
            if state.scene.project_id() != project_id || state.scene.revision() != revision {
                let _ = fs::remove_dir_all(&staging);
                return Err(AdapterError::Conflict(
                    "scene changed while the slicer handoff was staged".to_owned(),
                ));
            }
            let launch = match builder3d_platform::launch_slicer(&slicer, &paths) {
                Ok(launch) => launch,
                Err(error) => {
                    let _ = fs::remove_dir_all(&staging);
                    return Err(error.into());
                }
            };
            state.retain_staging_directory(staging.clone());
            launch
        };
        Ok(json!({
            "action": "launch_slicer",
            "slicer": {
                "key": launch.slicer.key,
                "label": launch.slicer.label,
                "path": launch.slicer.path.display().to_string(),
                "stl_fallback": launch.slicer.stl_fallback(),
            },
            "paths": paths.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
            "objects_exported": meshes.len(),
            "format": format,
            "process_id": launch.process_id,
        }))
    }

    fn execute_open_project(
        &self,
        arguments: &Value,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Value, AdapterError> {
        let args = arguments
            .as_object()
            .ok_or_else(|| AdapterError::Validation("arguments must be an object".to_owned()))?;
        let raw_path = schema::required_string(args, "path")?;
        let path = self.config.approved_roots.read_path(raw_path)?;
        let loaded = load_project(&path)?;
        if cancel.load(Ordering::Acquire) {
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled("open_project".to_owned()),
            ));
        }
        let mut state = self.lock_state()?;
        check_expected(&state, args)?;
        if state.dirty {
            return Err(AdapterError::Conflict(
                "the current project has unsaved changes; save it before creating or opening a project"
                    .to_owned(),
            ));
        }
        state.scene = loaded;
        state.project_path = Some(path.clone());
        state.dirty = false;
        Ok(
            json!({"action":"open_project","changed":true,"path":path.display().to_string(),"project_id":state.scene.project_id(),"scene_revision":state.scene.revision()}),
        )
    }

    fn execute_save_project(
        &self,
        action: &str,
        arguments: &Value,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Value, AdapterError> {
        let args = arguments
            .as_object()
            .ok_or_else(|| AdapterError::Validation("arguments must be an object".to_owned()))?;
        let overwrite = args
            .get("overwrite")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let raw_path = args
            .get("path")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let mut state = self.lock_state()?;
        check_expected_project(&state, args)?;
        let path = if let Some(raw_path) = raw_path {
            self.config.approved_roots.write_path(raw_path, overwrite)?
        } else if let Some(path) = &state.project_path {
            self.config.approved_roots.write_path(path, true)?
        } else {
            return Err(AdapterError::Validation(
                "save_project requires path when no project is open".to_owned(),
            ));
        };
        if cancel.load(Ordering::Acquire) {
            return Err(AdapterError::Core(
                builder3d_core::CoreError::OperationCancelled(action.to_owned()),
            ));
        }
        save_project(&state.scene, &path)?;
        state.project_path = Some(path.clone());
        state.dirty = false;
        Ok(
            json!({"action":action,"changed":false,"path":path.display().to_string(),"project_id":state.scene.project_id(),"scene_revision":state.scene.revision()}),
        )
    }
}

fn create_staging_directory() -> Result<PathBuf, AdapterError> {
    let root = std::env::temp_dir().join("Builder3D-R4");
    fs::create_dir_all(&root).map_err(|error| {
        AdapterError::Internal(format!("could not create staging root: {error}"))
    })?;
    let staging = root.join(format!("slicer-{}", Uuid::new_v4().simple()));
    fs::create_dir(&staging).map_err(|error| {
        AdapterError::Internal(format!(
            "could not create slicer staging directory: {error}"
        ))
    })?;
    Ok(staging)
}

impl ServerHandler for R4Server {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_protocol_version(ProtocolVersion::V_2026_07_28)
        .with_server_info(Implementation::new("builder3d-rust-mcp", env!("CARGO_PKG_VERSION")))
        .with_instructions("Builder3D R4 exposes the fixed catalog through the portable core. Use operation_status/cancel_operation for bounded long operations; file tools require configured approved roots.")
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, McpError>>
    + rmcp::service::MaybeSendFuture
    + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(self.tools())))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        catalog::find_tool(name)
            .filter(|spec| spec.visible_in(self.config.profile))
            .map(|spec| self.tool(spec))
    }

    fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, McpError>>
    + rmcp::service::MaybeSendFuture
    + '_ {
        let name = request.name.to_string();
        let arguments = Value::Object(request.arguments.unwrap_or_default());
        let server = self.clone();
        async move {
            let Some(spec) = catalog::find_tool(&name) else {
                return Err(McpError::new(
                    ErrorCode::METHOD_NOT_FOUND,
                    format!("unknown MCP tool: {name}"),
                    None,
                ));
            };
            if !spec.visible_in(server.config.profile) {
                return Err(McpError::new(
                    ErrorCode::METHOD_NOT_FOUND,
                    format!(
                        "tool {name} is not available in profile {}",
                        server.config.profile
                    ),
                    None,
                ));
            }
            match server.call_value(&name, arguments) {
                Ok(value) => Ok(CallToolResult::structured(value).into()),
                Err(AdapterError::NotFound(message)) => {
                    Err(McpError::new(ErrorCode::METHOD_NOT_FOUND, message, None))
                }
                Err(error) => Ok(CallToolResult::structured_error(error.to_value()).into()),
            }
        }
    }
}

fn without_idempotency_key(arguments: &Value) -> Value {
    let Some(object) = arguments.as_object() else {
        return arguments.clone();
    };
    let mut object = object.clone();
    object.remove("idempotency_key");
    Value::Object(object)
}

fn validate_clipboard(objects: &[SceneObject]) -> Result<(), AdapterError> {
    if objects.len() > state::MAX_CLIPBOARD_OBJECTS {
        return Err(AdapterError::Validation(format!(
            "clipboard is limited to {} objects",
            state::MAX_CLIPBOARD_OBJECTS
        )));
    }
    let mut estimated_bytes = 0_usize;
    for object in objects {
        add_clipboard_bytes(
            object.mesh(),
            object.name(),
            object.metadata(),
            object.children(),
            &mut estimated_bytes,
        )?;
    }
    if estimated_bytes > state::MAX_CLIPBOARD_BYTES {
        return Err(AdapterError::Validation(format!(
            "clipboard exceeds {} estimated bytes",
            state::MAX_CLIPBOARD_BYTES
        )));
    }
    Ok(())
}

fn add_clipboard_bytes(
    mesh: &Mesh,
    name: &str,
    metadata: &Value,
    children: &[SceneChild],
    total: &mut usize,
) -> Result<(), AdapterError> {
    let mesh_bytes = mesh
        .vertex_count()
        .checked_mul(std::mem::size_of::<[f32; 3]>())
        .and_then(|bytes| {
            mesh.face_count()
                .checked_mul(std::mem::size_of::<[u32; 3]>())
                .and_then(|face_bytes| bytes.checked_add(face_bytes))
        })
        .ok_or_else(|| AdapterError::Validation("clipboard size overflow".to_owned()))?;
    let metadata_bytes = serde_json::to_vec(metadata)
        .map_err(|error| {
            AdapterError::Internal(format!(
                "clipboard metadata could not be serialized: {error}"
            ))
        })?
        .len();
    *total = total
        .checked_add(mesh_bytes)
        .and_then(|bytes| bytes.checked_add(metadata_bytes))
        .and_then(|bytes| bytes.checked_add(name.len()))
        .ok_or_else(|| AdapterError::Validation("clipboard size overflow".to_owned()))?;
    for child in children {
        add_clipboard_bytes(
            child.mesh(),
            child.name(),
            child.metadata(),
            child.children(),
            total,
        )?;
    }
    Ok(())
}

fn check_expected(state: &R4State, args: &Map<String, Value>) -> Result<(), AdapterError> {
    if let Some(expected) = args.get("expected_scene_revision").and_then(Value::as_u64)
        && expected != state.scene.revision()
    {
        return Err(AdapterError::Conflict(format!(
            "expected scene revision {expected}, current revision is {}",
            state.scene.revision()
        )));
    }
    check_expected_project(state, args)
}

fn check_expected_project(state: &R4State, args: &Map<String, Value>) -> Result<(), AdapterError> {
    if let Some(expected) = args.get("expected_project_id").and_then(Value::as_str)
        && expected != state.scene.project_id()
    {
        return Err(AdapterError::Conflict(format!(
            "expected project {expected}, current project is {}",
            state.scene.project_id()
        )));
    }
    Ok(())
}

fn target_index(scene: &Scene, args: &Map<String, Value>) -> Result<usize, AdapterError> {
    if let Some(object_id) = args.get("object_id").and_then(Value::as_str)
        && !object_id.is_empty()
    {
        return scene.object_index(object_id).map_err(AdapterError::from);
    }
    if let Some(index) = args.get("index").and_then(Value::as_u64) {
        return usize::try_from(index)
            .map_err(|_| AdapterError::Validation("index is too large".to_owned()));
    }
    target_index_or_first(scene)
}

fn target_index_or_first(scene: &Scene) -> Result<usize, AdapterError> {
    if let Some(index) = scene.selection().iter().next().copied() {
        return Ok(index);
    }
    if scene.objects().len() == 1 {
        return Ok(0);
    }
    Err(AdapterError::Validation(
        "an object_id, index, or single selected object is required".to_owned(),
    ))
}

fn selected_or_targets(
    scene: &Scene,
    args: &Map<String, Value>,
) -> Result<Vec<usize>, AdapterError> {
    let indices = schema::indices(args)?;
    let ids = schema::object_ids(args)?;
    if indices.is_some() || ids.is_some() {
        return resolve_indices(scene, indices.as_deref(), ids.as_deref());
    }
    if !scene.selection().is_empty() {
        return Ok(scene.selection().iter().copied().collect());
    }
    if scene.objects().len() == 1 {
        return Ok(vec![0]);
    }
    Err(AdapterError::Validation(
        "indices, object_ids, or a selection is required".to_owned(),
    ))
}

fn resolve_indices(
    scene: &Scene,
    indices: Option<&[usize]>,
    ids: Option<&[String]>,
) -> Result<Vec<usize>, AdapterError> {
    let mut result = BTreeSet::new();
    if let Some(indices) = indices {
        for index in indices {
            if *index >= scene.objects().len() {
                return Err(AdapterError::NotFound(format!(
                    "object index {index} is out of range"
                )));
            }
            result.insert(*index);
        }
    }
    if let Some(ids) = ids {
        for id in ids {
            result.insert(scene.object_index(id)?);
        }
    }
    Ok(result.into_iter().collect())
}

fn parse_summary_fields(fields: Option<&Value>) -> Result<Option<BTreeSet<String>>, AdapterError> {
    let Some(fields) = fields.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let values = fields.as_array().ok_or_else(|| {
        AdapterError::Validation("fields must be a list of field names".to_owned())
    })?;
    let fields = values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .ok_or_else(|| AdapterError::Validation("fields must contain strings".to_owned()))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let allowed = [
        "index",
        "object_id",
        "name",
        "visible",
        "vertices",
        "triangles",
        "position_mm",
        "rotation_deg",
        "size_mm",
        "color",
        "children",
        "torus_parameters",
        "mesh_health",
    ];
    let unknown = fields
        .iter()
        .filter(|field| !allowed.contains(&field.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !unknown.is_empty() {
        return Err(AdapterError::Validation(format!(
            "unknown summary fields: {}",
            unknown.join(", ")
        )));
    }
    Ok(Some(fields))
}

fn object_summary(
    index: usize,
    object: &SceneObject,
    fields: Option<&BTreeSet<String>>,
) -> Result<Value, AdapterError> {
    let mesh = object.transformed_mesh()?;
    let include = |field: &str| fields.is_none_or(|fields| fields.contains(field));
    let mut summary = Map::new();
    summary.insert("index".to_owned(), json!(index));
    summary.insert("object_id".to_owned(), json!(object.object_id()));
    if include("name") {
        summary.insert("name".to_owned(), json!(object.name()));
    }
    if include("visible") {
        summary.insert("visible".to_owned(), json!(object.is_visible()));
    }
    if include("vertices") {
        summary.insert("vertices".to_owned(), json!(object.mesh().vertex_count()));
    }
    if include("triangles") {
        summary.insert("triangles".to_owned(), json!(object.mesh().face_count()));
    }
    if include("position_mm") {
        summary.insert("position_mm".to_owned(), json!(object.position()));
    }
    if include("rotation_deg") {
        summary.insert(
            "rotation_deg".to_owned(),
            json!(rotation_degrees(object.transform())),
        );
    }
    if include("size_mm") {
        summary.insert("size_mm".to_owned(), json!(mesh.extents()));
    }
    if include("color") {
        summary.insert("color".to_owned(), json!(object.color()));
    }
    if include("mesh_health") {
        summary.insert(
            "mesh_health".to_owned(),
            health_value(&inspect_mesh(object.mesh())?),
        );
    }
    if include("children") && !object.children().is_empty() {
        summary.insert(
            "children".to_owned(),
            Value::Array(object.children().iter().map(child_summary).collect()),
        );
    }
    if include("torus_parameters")
        && let Some(parameters) = builder3d_core::torus_parameters_from_metadata(object.metadata())?
    {
        summary.insert(
            "torus_parameters".to_owned(),
            json!({
                "mode":parameters.mode,
                "radius_mm":parameters.radius_mm,
                "tube_mm":parameters.tube_mm,
                "sides":parameters.sides,
                "steps":parameters.steps,
            }),
        );
    }
    Ok(Value::Object(summary))
}

fn child_summary(child: &SceneChild) -> Value {
    let mut summary = json!({
        "object_id": child.object_id(),
        "name": child.name(),
        "visible": child.is_visible(),
        "color": child.color(),
    });
    if !child.children().is_empty() {
        summary["children"] = Value::Array(child.children().iter().map(child_summary).collect());
    }
    summary
}

fn rotation_degrees(transform: &Transform) -> [f32; 3] {
    let rows = transform.rows();
    let scales = std::array::from_fn::<_, 3, _>(|column| {
        (rows[0][column].powi(2) + rows[1][column].powi(2) + rows[2][column].powi(2)).sqrt()
    });
    let mut rotation = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            let scale = scales[column];
            rotation[row][column] = if scale > f32::EPSILON {
                rows[row][column] / scale
            } else {
                0.0
            };
        }
    }
    let pitch = rotation[0][2].clamp(-1.0, 1.0).asin();
    let cosine_pitch = pitch.cos();
    let (roll, yaw) = if cosine_pitch.abs() > 1.0e-6 {
        (
            (-rotation[1][2]).atan2(rotation[2][2]),
            (-rotation[0][1]).atan2(rotation[0][0]),
        )
    } else {
        (rotation[1][0].atan2(rotation[1][1]), 0.0)
    };
    [roll, pitch, yaw].map(|value| {
        let degrees = value.to_degrees();
        (degrees * 10_000.0).round() / 10_000.0
    })
}

fn changed_object_ids(value: &Value) -> Option<Vec<String>> {
    value
        .get("changed_object_ids")
        .or_else(|| value.get("affected_object_ids"))
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
}

fn command_output(
    action: &str,
    result: &builder3d_core::CommandResult,
    object: Option<(String, usize)>,
    extra: Option<Value>,
) -> Value {
    let mut output = json!({
        "action":action,
        "changed":result.changed,
        "scene_revision":result.revision,
        "history_state_id":result.history_state_id,
        "affected_object_ids":result.affected_object_ids,
    });
    if let Some((object_id, index)) = object
        && let Some(map) = output.as_object_mut()
    {
        map.insert("object_id".to_owned(), json!(object_id));
        map.insert("index".to_owned(), json!(index));
    }
    if let Some(extra) = extra
        && let (Some(output), Some(extra)) = (output.as_object_mut(), extra.as_object())
    {
        for (key, value) in extra {
            output.insert(key.clone(), value.clone());
        }
    }
    output
}

fn result_index(scene: &Scene, result: &builder3d_core::CommandResult) -> usize {
    result
        .affected_object_ids
        .first()
        .and_then(|id| scene.object_index(id).ok())
        .unwrap_or_else(|| scene.objects().len().saturating_sub(1))
}

fn nonempty_name(name: &str) -> Option<String> {
    (!name.trim().is_empty()).then(|| name.to_owned())
}

fn centers(args: &Map<String, Value>) -> Result<Vec<[f32; 3]>, AdapterError> {
    let values = args
        .get("centers_mm")
        .and_then(Value::as_array)
        .ok_or_else(|| AdapterError::Validation("centers_mm must be an array".to_owned()))?;
    values
        .iter()
        .map(|value| {
            let mut temporary = Map::new();
            temporary.insert("value".to_owned(), value.clone());
            schema::vec3_or(&temporary, "value", [0.0; 3]).map_err(AdapterError::Validation)
        })
        .collect()
}

fn optional_centers(args: &Map<String, Value>) -> Result<Option<Vec<[f32; 3]>>, AdapterError> {
    match args.get("centers_mm") {
        None | Some(Value::Null) => Ok(None),
        Some(_) => centers(args).map(Some),
    }
}

fn optional_usize_list(
    args: &Map<String, Value>,
    key: &str,
) -> Result<Option<Vec<usize>>, AdapterError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| {
                value
                    .as_u64()
                    .and_then(|number| usize::try_from(number).ok())
                    .ok_or_else(|| {
                        AdapterError::Validation(format!(
                            "{key} must contain non-negative integers"
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err(AdapterError::Validation(format!("{key} must be an array"))),
    }
}

fn optional_usize(args: &Map<String, Value>, key: &str) -> Result<Option<usize>, AdapterError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|number| usize::try_from(number).ok())
            .map(Some)
            .ok_or_else(|| {
                AdapterError::Validation(format!("{key} must be a non-negative integer"))
            }),
    }
}

fn optional_number(args: &Map<String, Value>, key: &str) -> Result<Option<f32>, AdapterError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => schema::number_or(args, key, 0.0)
            .map(Some)
            .map_err(AdapterError::Validation),
    }
}

fn color_from(value: Option<&Value>, key: &str) -> Result<[f32; 4], AdapterError> {
    let values = value.and_then(Value::as_array).ok_or_else(|| {
        AdapterError::Validation(format!("{key} must be a 3- or 4-component array"))
    })?;
    if !(3..=4).contains(&values.len()) {
        return Err(AdapterError::Validation(format!(
            "{key} must contain 3 or 4 channels"
        )));
    }
    let mut color = [1.0; 4];
    for (index, value) in values.iter().enumerate() {
        color[index] = value
            .as_f64()
            .filter(|number| number.is_finite())
            .map(|number| number as f32)
            .ok_or_else(|| AdapterError::Validation(format!("{key}[{index}] must be finite")))?;
    }
    if color[..values.len()]
        .iter()
        .copied()
        .fold(0.0_f32, f32::max)
        > 1.0
    {
        for channel in &mut color[..values.len()] {
            *channel /= 255.0;
        }
    }
    if color[..values.len()]
        .iter()
        .any(|channel| !(0.0..=1.0).contains(channel))
    {
        return Err(AdapterError::Validation(format!(
            "{key} values must be between 0 and 1, or between 0 and 255"
        )));
    }
    Ok(color)
}

fn axis_index(axis: &str) -> Result<usize, AdapterError> {
    match axis {
        "x" => Ok(0),
        "y" => Ok(1),
        "z" => Ok(2),
        _ => Err(AdapterError::Validation(
            "axis must be x, y, or z".to_owned(),
        )),
    }
}

fn safe_scale(desired: f32, current: f32) -> Result<f32, AdapterError> {
    if current.abs() <= f32::EPSILON {
        return Err(AdapterError::Validation(
            "cannot scale an object with a zero extent".to_owned(),
        ));
    }
    let scale = desired / current;
    if !scale.is_finite() || scale <= 0.0 {
        return Err(AdapterError::Validation(
            "size_mm produces an invalid scale".to_owned(),
        ));
    }
    Ok(scale)
}

fn brush_report(report: &builder3d_core::BrushReport) -> Value {
    json!({
        "operation":report.operation,
        "changed":report.changed,
        "affected_vertices":report.affected_vertices,
        "maximum_displacement_mm":report.maximum_displacement_mm,
        "warnings":report.warnings
    })
}

fn health_value(report: &builder3d_core::MeshHealthReport) -> Value {
    json!({
        "valid":report.valid,
        "vertices":report.vertices,
        "triangles":report.triangles,
        "watertight":report.watertight,
        "winding_consistent":report.winding_consistent,
        "body_count":report.body_count,
        "is_volume":report.is_volume,
        "degenerate_faces":report.degenerate_faces,
        "duplicate_faces":report.duplicate_faces,
        "non_manifold_edges":report.non_manifold_edges,
        "boundary_edges":report.boundary_edges,
        "boundary_edge_loops":report.boundary_edge_loops,
        "tiny_shells":report.tiny_shells,
        "self_intersections":report.self_intersections,
        "self_intersection_status":report.self_intersection_status,
        "issues":report.issues
    })
}

fn normalize_report(report: &builder3d_core::NormalizeDensityReport) -> Value {
    json!({
        "changed":report.changed,
        "face_count_before":report.face_count_before,
        "face_count_after":report.face_count_after,
        "selected_faces_before":report.selected_faces_before,
        "selected_faces_after":report.selected_faces_after,
        "target_edge_length_mm":report.target_edge_length_mm,
        "boundary_edge_length_mm":report.boundary_edge_length_mm,
        "characteristic_edge_length_before_mm":report.characteristic_edge_length_before_mm,
        "characteristic_edge_length_after_mm":report.characteristic_edge_length_after_mm,
        "subdivided_faces":report.subdivided_faces,
        "collapsed_edges":report.collapsed_edges,
        "geometric_deviation_mm":report.geometric_deviation_mm,
        "boundary_preserved":report.boundary_preserved,
        "selection_boundary_preserved":report.selection_boundary_preserved,
        "iterations":report.iterations,
        "resource_work_units":report.resource_work_units,
        "topology_passes":report.topology_passes,
        "resource_budget_units":report.resource_budget_units,
        "issues":report.issues
    })
}

fn repair_report(report: &builder3d_core::RepairReport) -> Value {
    json!({
        "profile":report.profile,
        "tolerance_mm":report.tolerance_mm,
        "tiny_shell_tolerance_mm":report.tiny_shell_tolerance_mm,
        "before":health_value(&report.before),
        "after":health_value(&report.after),
        "candidate_safe":report.candidate_safe,
        "candidate_changed":report.candidate_changed,
        "can_apply":report.can_apply,
        "errors":report.errors
    })
}

fn simplify_report(report: &builder3d_core::SimplifyReport) -> Value {
    json!({
        "before_faces":report.before_faces,
        "before_vertices":report.before_vertices,
        "target_faces":report.target_faces,
        "after_faces":report.after_faces,
        "after_vertices":report.after_vertices,
        "changed":report.changed,
        "collapsed_edges":report.collapsed_edges,
        "iterations":report.iterations,
        "work_units":report.work_units,
        "geometric_deviation_mm":report.geometric_deviation_mm,
        "before_volume_mm3":report.before_volume_mm3,
        "after_volume_mm3":report.after_volume_mm3,
        "warnings":report.warnings
    })
}

fn subdivide_report(report: &builder3d_core::SubdivideReport) -> Value {
    json!({
        "before_faces":report.before_faces,
        "before_vertices":report.before_vertices,
        "after_faces":report.after_faces,
        "after_vertices":report.after_vertices,
        "selected_interior_edges":report.selected_interior_edges,
        "selected_boundary_edges":report.selected_boundary_edges,
        "changed":report.changed,
        "before_volume_mm3":report.before_volume_mm3,
        "after_volume_mm3":report.after_volume_mm3,
        "before_extents_mm":report.before_extents_mm,
        "after_extents_mm":report.after_extents_mm
    })
}

fn smooth_report(report: &builder3d_core::SmoothReport) -> Value {
    json!({
        "before_vertices":report.before_vertices,
        "before_faces":report.before_faces,
        "iterations":report.iterations,
        "applied_iterations":report.applied_iterations,
        "changed":report.changed,
        "maximum_displacement_mm":report.maximum_displacement_mm,
        "before_volume_mm3":report.before_volume_mm3,
        "after_volume_mm3":report.after_volume_mm3
    })
}

fn boolean_report(report: &builder3d_core::BooleanReport) -> Value {
    json!({
        "operation":report.operation,
        "target_before":health_value(&report.target_before),
        "tool_before":health_value(&report.tool_before),
        "result_after":health_value(&report.result_after),
        "target_volume_mm3":report.target_volume_mm3,
        "tool_volume_mm3":report.tool_volume_mm3,
        "result_volume_mm3":report.result_volume_mm3,
        "estimated_peak_bytes":report.estimated_peak_bytes,
        "work_units":report.work_units,
        "regularized_cap_patches":report.regularized_cap_patches
    })
}

fn text_emboss_report(report: &builder3d_core::TextEmbossReport) -> Value {
    json!({
        "source_vertices": report.source_vertices,
        "source_faces": report.source_faces,
        "result_vertices": report.result_vertices,
        "result_faces": report.result_faces,
        "source_volume_mm3": report.source_volume_mm3,
        "result_volume_mm3": report.result_volume_mm3,
        "font": report.font,
        "raster_width": report.raster_width,
        "raster_height": report.raster_height,
        "occupied_cells": report.occupied_cells,
        "depth_mm": report.depth_mm,
        "raised": report.raised
    })
}

fn hollow_report(report: &builder3d_core::HollowReport) -> Value {
    json!({
        "source_vertices": report.source_vertices,
        "source_faces": report.source_faces,
        "result_vertices": report.result_vertices,
        "result_faces": report.result_faces,
        "source_volume_mm3": report.source_volume_mm3,
        "result_volume_mm3": report.result_volume_mm3,
        "branch": report.branch,
        "wall_thickness_mm": report.wall_thickness_mm,
        "inner_volume_mm3": report.inner_volume_mm3
    })
}

fn unit_factor(unit: &str) -> Result<f32, AdapterError> {
    match unit {
        "um" => Ok(0.001),
        "mm" => Ok(1.0),
        "cm" => Ok(10.0),
        "in" => Ok(25.4),
        "ft" => Ok(304.8),
        "m" => Ok(1000.0),
        _ => Err(AdapterError::Validation(
            "unit must be um, mm, cm, in, ft, or m".to_owned(),
        )),
    }
}

fn scale_mesh(mesh: &Mesh, factor: f32) -> Result<Mesh, AdapterError> {
    if !factor.is_finite() || factor <= 0.0 {
        return Err(AdapterError::Validation(
            "unit scale must be positive and finite".to_owned(),
        ));
    }
    if (factor - 1.0).abs() <= f32::EPSILON {
        return Ok(mesh.clone());
    }
    let vertices = mesh
        .vertices
        .iter()
        .map(|vertex| [vertex[0] * factor, vertex[1] * factor, vertex[2] * factor])
        .collect();
    Ok(Mesh::new(vertices, mesh.faces.clone())?)
}

fn bed_size(args: &Map<String, Value>) -> Result<[f32; 3], AdapterError> {
    match args.get("bed_size_mm") {
        None | Some(Value::Null) => Ok([200.0; 3]),
        Some(Value::Number(_)) => {
            let size = schema::number_or(args, "bed_size_mm", 200.0)?;
            Ok([size; 3])
        }
        Some(Value::Array(_)) => {
            schema::vec3_or(args, "bed_size_mm", [200.0; 3]).map_err(AdapterError::Validation)
        }
        Some(_) => Err(AdapterError::Validation(
            "bed_size_mm must be a number or vector".to_owned(),
        )),
    }
}

fn parse_ppm_average(bytes: &[u8]) -> Option<[f32; 4]> {
    const MAX_PPM_PIXELS: usize = 4_000_000;
    let text = std::str::from_utf8(bytes).ok()?;
    let mut tokens = text.split_whitespace();
    if tokens.next()? != "P3" {
        return None;
    }
    let width = tokens.next()?.parse::<usize>().ok()?;
    let height = tokens.next()?.parse::<usize>().ok()?;
    let maximum = tokens.next()?.parse::<f32>().ok()?;
    let count = width.checked_mul(height)?;
    if width == 0
        || height == 0
        || count > MAX_PPM_PIXELS
        || !maximum.is_finite()
        || !(0.0..=65_535.0).contains(&maximum)
        || maximum <= 0.0
    {
        return None;
    }
    let mut totals = [0.0_f32; 3];
    for _ in 0..count {
        for channel in &mut totals {
            let sample = tokens.next()?.parse::<f32>().ok()?;
            if !sample.is_finite() || !(0.0..=maximum).contains(&sample) {
                return None;
            }
            *channel += sample / maximum;
        }
    }
    Some([
        totals[0] / count as f32,
        totals[1] / count as f32,
        totals[2] / count as f32,
        1.0,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env,
        sync::{Arc, Barrier},
        thread,
        time::Duration,
    };

    fn server(profile: Profile) -> R4Server {
        let root = env::temp_dir().join(format!("builder3d-r4-server-{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        let roots = ApprovedRoots::from_paths([root]).expect("approved root");
        R4Server::new(R4Config::for_profile(profile, roots))
    }

    #[test]
    fn attached_desktop_scene_keeps_the_same_core_undo_history() {
        let server = server(Profile::Full);
        let mut scene = Scene::new();
        let (object_id, _) = scene
            .add_primitive(PrimitiveKind::Cube, Some([10.0; 3]), None)
            .expect("add cube");
        let initial_history = scene.history_state_id();
        let mut document = Some(R4Document::new(scene));
        server
            .replace_document(&mut document)
            .expect("attach desktop document");

        server
            .call_value(
                "translate",
                json!({"object_id":object_id,"offset_mm":[2,0,0]}),
            )
            .expect("MCP translation");
        let mut document = server.take_document().expect("return desktop document");
        assert_eq!(document.scene.history_state_id(), initial_history + 1);
        assert_eq!(document.scene.objects()[0].object_id(), object_id);
        assert!(document.scene.undo().expect("shared undo").is_some());
        assert_eq!(document.scene.history_state_id(), initial_history);
    }

    #[test]
    fn project_replacement_tools_reject_an_unsaved_document() {
        let server = server(Profile::Full);
        server
            .call_value("add_primitive", json!({"primitive":"cube"}))
            .expect("make the document dirty");
        let before = server.call_value("get_status", json!({})).expect("status");

        assert!(matches!(
            server.call_value("new_project", json!({})),
            Err(AdapterError::Conflict(message)) if message.contains("unsaved changes")
        ));
        assert!(matches!(
            server.call_value("open_project", json!({"path":"C:/outside/project.3dp"})),
            Err(AdapterError::Conflict(message)) if message.contains("unsaved changes")
        ));

        let after = server.call_value("get_status", json!({})).expect("status");
        assert_eq!(after["project_id"], before["project_id"]);
        assert_eq!(after["scene_revision"], before["scene_revision"]);
        assert_eq!(after["dirty"], true);
        assert_eq!(after["busy"], false);
    }

    #[test]
    fn full_profile_publishes_the_fixed_catalog_with_strict_metadata() {
        let server = server(Profile::Full);
        let tools = server.tools();
        assert_eq!(tools.len(), 60);
        assert_eq!(
            tools.first().map(|tool| tool.name.as_ref()),
            Some("get_status")
        );
        assert_eq!(
            tools.last().map(|tool| tool.name.as_ref()),
            Some("launch_slicer")
        );
        let launch = tools.iter().find(|tool| tool.name == "launch_slicer");
        assert_eq!(
            launch
                .and_then(|tool| tool.meta.as_ref())
                .and_then(|meta| meta.0.get("com.builder3d/profile")),
            Some(&json!("full"))
        );
        assert_eq!(
            launch
                .and_then(|tool| tool.meta.as_ref())
                .and_then(|meta| meta.0.get("com.builder3d/longRunning")),
            Some(&json!(true))
        );
        assert_eq!(
            tools
                .iter()
                .find(|tool| tool.name == "scene_summary")
                .and_then(|tool| tool.annotations.as_ref())
                .and_then(|annotations| annotations.read_only_hint),
            Some(true)
        );
        assert_eq!(
            tools
                .iter()
                .find(|tool| tool.name == "get_status")
                .map(|tool| tool.input_schema["type"].clone()),
            Some(json!("object"))
        );
        assert!(
            tools
                .iter()
                .find(|tool| tool.name == "get_status")
                .is_some_and(|tool| !tool.input_schema.contains_key("additionalProperties"))
        );
        assert_eq!(
            tools
                .iter()
                .find(|tool| tool.name == "emboss_text")
                .and_then(|tool| tool.annotations.as_ref())
                .and_then(|annotations| annotations.destructive_hint),
            Some(false)
        );
        assert_eq!(
            launch
                .and_then(|tool| tool.annotations.as_ref())
                .and_then(|annotations| annotations.open_world_hint),
            Some(false)
        );
    }

    #[test]
    fn core_calls_are_structured_and_replay_is_idempotent() {
        let server = server(Profile::Full);
        let added = server
            .call_value(
                "add_primitive",
                json!({"primitive":"cube","dimensions_mm":[10,10,10]}),
            )
            .expect("add primitive");
        let object_id = added["object_id"].as_str().expect("object ID").to_owned();
        let request = json!({"object_id":object_id,"offset_mm":[2,0,0],"idempotency_key":"move-1"});
        let first = server
            .call_value("translate", request.clone())
            .expect("translate");
        let second = server.call_value("translate", request).expect("replay");
        assert_eq!(first, second);
        let conflict = server.call_value(
            "translate",
            json!({"object_id":object_id,"offset_mm":[3,0,0],"idempotency_key":"move-1"}),
        );
        assert!(matches!(conflict, Err(AdapterError::ReplayConflict)));
        let summary = server
            .call_value("scene_summary", json!({}))
            .expect("summary");
        assert_eq!(summary["objects"].as_array().map(Vec::len), Some(1));
        // A 10 mm cube rests on the floor at Y=5. Owner-approved deviation
        // (2026-11-12): the Python MCP leaves primitives at identity (Y=0).
        assert_eq!(summary["objects"][0]["position_mm"], json!([2.0, 5.0, 0.0]));
    }

    #[test]
    fn concurrent_same_key_requests_replay_one_mutation() {
        let server = server(Profile::Full);
        let added = server
            .call_value("add_primitive", json!({"primitive":"cube"}))
            .expect("add primitive");
        let object_id = added["object_id"].as_str().expect("object ID");
        let revision =
            server.call_value("get_status", json!({})).expect("status")["scene_revision"]
                .as_u64()
                .expect("scene revision");
        let request = json!({
            "object_id":object_id,
            "offset_mm":[1,0,0],
            "idempotency_key":"same-race-key"
        });
        let barrier = Arc::new(Barrier::new(3));
        let first = thread::spawn({
            let server = server.clone();
            let barrier = Arc::clone(&barrier);
            let request = request.clone();
            move || {
                barrier.wait();
                server.call_value("translate", request)
            }
        });
        let second = thread::spawn({
            let server = server.clone();
            let barrier = Arc::clone(&barrier);
            move || {
                barrier.wait();
                server.call_value("translate", request)
            }
        });
        barrier.wait();
        let first = first.join().expect("first client").expect("first result");
        let second = second
            .join()
            .expect("second client")
            .expect("second result");

        assert_eq!(first, second);
        let status = server.call_value("get_status", json!({})).expect("status");
        assert_eq!(status["scene_revision"], json!(revision + 1));
        server.call_value("undo", json!({})).expect("undo one edit");
        let summary = server
            .call_value("scene_summary", json!({}))
            .expect("summary after undo");
        assert_eq!(
            summary["objects"][0]["position_mm"],
            json!([0.0, 20.0, 0.0])
        );
    }

    #[test]
    fn concurrent_expected_revision_race_commits_once() {
        let server = server(Profile::Full);
        let added = server
            .call_value("add_primitive", json!({"primitive":"cube"}))
            .expect("add primitive");
        let object_id = added["object_id"].as_str().expect("object ID").to_owned();
        let status = server.call_value("get_status", json!({})).expect("status");
        let project_id = status["project_id"]
            .as_str()
            .expect("project ID")
            .to_owned();
        let revision = status["scene_revision"].as_u64().expect("scene revision");
        let barrier = Arc::new(Barrier::new(3));
        let first = thread::spawn({
            let server = server.clone();
            let barrier = Arc::clone(&barrier);
            let object_id = object_id.clone();
            let project_id = project_id.clone();
            move || {
                barrier.wait();
                server.call_value(
                    "translate",
                    json!({
                        "object_id":object_id,
                        "offset_mm":[1,0,0],
                        "expected_scene_revision":revision,
                        "expected_project_id":project_id
                    }),
                )
            }
        });
        let second = thread::spawn({
            let server = server.clone();
            let barrier = Arc::clone(&barrier);
            move || {
                barrier.wait();
                server.call_value(
                    "translate",
                    json!({
                        "object_id":object_id,
                        "offset_mm":[2,0,0],
                        "expected_scene_revision":revision,
                        "expected_project_id":project_id
                    }),
                )
            }
        });
        barrier.wait();
        let results = [
            first.join().expect("first client"),
            second.join().expect("second client"),
        ];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| matches!(result, Err(AdapterError::Conflict(_))))
                .count(),
            1
        );
        let status = server.call_value("get_status", json!({})).expect("status");
        assert_eq!(status["scene_revision"], json!(revision + 1));
    }

    #[test]
    fn reference_mesh_tools_and_profile_filter_are_live() {
        let full = server(Profile::Full);
        let _ = full
            .call_value(
                "add_primitive",
                json!({"primitive":"cube","dimensions_mm":[10,10,10]}),
            )
            .expect("add primitive");
        let health = full
            .call_value("mesh_health", json!({"index":0}))
            .expect("mesh health");
        assert_eq!(health["health"]["valid"], json!(true));
        let brush = full.call_value("elastic_grab", json!({"index":0,"center_mm":[0,0,0],"offset_mm":[1,0,0],"radius_mm":8,"strength":0.5,"falloff":"smooth","symmetry":"x"})).expect("elastic grab");
        assert_eq!(brush["elastic_grab"]["symmetry"], json!("x"));
        let invalid = full.call_value("mirror", json!({"axis":"bad","index":0}));
        assert!(matches!(invalid, Err(AdapterError::Validation(_))));
        let observe = server(Profile::Observe);
        assert_eq!(observe.tools().len(), 7);
        assert!(
            observe
                .call_value("translate", json!({"index":0,"offset_mm":[1,0,0]}))
                .is_err()
        );
    }

    #[test]
    fn color_inputs_match_reference_rgb_255_normalization() {
        let server = server(Profile::Full);
        server
            .call_value("add_primitive", json!({"primitive":"cube"}))
            .expect("add primitive");
        let output = server
            .call_value("set_color", json!({"index":0,"color":[255,128,0]}))
            .expect("set RGB color using 0-to-255 channels");
        let color = output["result"]["color"]
            .as_array()
            .expect("color output array");
        assert_eq!(color.len(), 4);
        assert_eq!(color[0].as_f64(), Some(1.0));
        assert!((color[1].as_f64().expect("green channel") - 128.0 / 255.0).abs() < 1.0e-6);
        assert_eq!(color[2].as_f64(), Some(0.0));
        assert_eq!(color[3].as_f64(), Some(1.0));
        assert!(
            server
                .call_value("set_color", json!({"index":0,"color":[256,0,0]}))
                .is_err()
        );
    }

    #[test]
    fn long_operation_reports_bounded_status_and_commits_once() {
        let server = server(Profile::Full);
        let _ = server
            .call_value("add_primitive", json!({"primitive":"cube"}))
            .expect("add primitive");
        let started = server
            .call_value("smooth", json!({"index":0,"iterations":1}))
            .expect("start smooth");
        let operation_id = started["operation_id"]
            .as_str()
            .expect("operation ID")
            .to_owned();
        let mut terminal = None;
        for _ in 0..100 {
            let status = server
                .call_value("operation_status", json!({"operation_id":operation_id}))
                .expect("operation status");
            if matches!(
                status["state"].as_str(),
                Some("completed" | "failed" | "cancelled")
            ) {
                terminal = Some(status);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let status = terminal.expect("operation must reach terminal state");
        assert_eq!(status["state"], json!("completed"));
        assert_eq!(status["result"]["action"], json!("smooth"));
        let summary = server
            .call_value("scene_summary", json!({}))
            .expect("summary");
        assert_eq!(summary["scene_revision"], json!(2));
    }

    #[test]
    fn shell_long_operations_commit_reports_and_are_undoable() {
        let emboss_server = server(Profile::Full);
        let added = emboss_server
            .call_value(
                "add_primitive",
                json!({"primitive":"cube","dimensions_mm":[40,40,40]}),
            )
            .expect("add primitive");
        let object_id = added["object_id"].as_str().expect("object ID").to_owned();

        let emboss_started = emboss_server
            .call_value(
                "emboss_text",
                json!({"object_id":object_id,"text":"A","font_size":10,"depth_mm":1}),
            )
            .expect("start emboss");
        let emboss_id = emboss_started["operation_id"]
            .as_str()
            .expect("emboss operation ID")
            .to_owned();
        let mut emboss_status = None;
        for _ in 0..200 {
            let status = emboss_server
                .call_value("operation_status", json!({"operation_id":emboss_id}))
                .expect("emboss status");
            if matches!(
                status["state"].as_str(),
                Some("completed" | "failed" | "cancelled")
            ) {
                emboss_status = Some(status);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let emboss_status = emboss_status.expect("emboss must reach terminal state");
        assert_eq!(emboss_status["state"], json!("completed"));
        assert_eq!(emboss_status["result"]["action"], json!("emboss_text"));
        assert!(
            emboss_status["result"]["report"]["occupied_cells"]
                .as_u64()
                .is_some_and(|value| value > 0)
        );
        assert_eq!(emboss_status["result"]["object_id"], json!(object_id));

        let stale = emboss_server.call_value(
            "hollow",
            json!({
                "object_id":object_id,
                "wall_thickness_mm":1,
                "expected_scene_revision":1
            }),
        );
        assert!(matches!(stale, Err(AdapterError::Conflict(_))));

        let hollow_server = server(Profile::Full);
        let hollow_added = hollow_server
            .call_value(
                "add_primitive",
                json!({"primitive":"cube","dimensions_mm":[40,40,40]}),
            )
            .expect("add hollow primitive");
        let hollow_object_id = hollow_added["object_id"]
            .as_str()
            .expect("hollow object ID")
            .to_owned();
        let hollow_started = hollow_server
            .call_value(
                "hollow",
                json!({"object_id":hollow_object_id,"wall_thickness_mm":1}),
            )
            .expect("start hollow");
        let hollow_id = hollow_started["operation_id"]
            .as_str()
            .expect("hollow operation ID")
            .to_owned();
        let mut hollow_status = None;
        for _ in 0..200 {
            let status = hollow_server
                .call_value("operation_status", json!({"operation_id":hollow_id}))
                .expect("hollow status");
            if matches!(
                status["state"].as_str(),
                Some("completed" | "failed" | "cancelled")
            ) {
                hollow_status = Some(status);
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        let hollow_status = hollow_status.expect("hollow must reach terminal state");
        assert_eq!(hollow_status["state"], json!("completed"));
        assert_eq!(hollow_status["result"]["action"], json!("hollow"));
        assert_eq!(
            hollow_status["result"]["report"]["branch"],
            json!("convex-exact")
        );

        let undone = hollow_server
            .call_value("undo", json!({}))
            .expect("undo hollow");
        assert!(!undone["changed_object_ids"].as_array().unwrap().is_empty());
        let summary = hollow_server
            .call_value("scene_summary", json!({}))
            .expect("summary");
        assert_eq!(summary["scene_revision"], json!(3));
    }

    #[test]
    fn grouped_actions_clipboard_and_discovery_are_live() {
        let server = server(Profile::Full);
        let added = server
            .call_value("add_primitive", json!({"primitive":"cube"}))
            .expect("add primitive");
        let object_id = added["object_id"].as_str().expect("object ID");
        let revision = added["scene_revision"].as_u64().expect("revision");
        let batch = server
            .call_value(
                "apply_actions",
                json!({
                    "actions":[
                        {"action":"translate","args":{"object_id":object_id,"offset_mm":[2,0,0]}},
                        {"action":"set_color","args":{"object_id":object_id,"color":[0.2,0.4,0.6,1.0]}}
                    ]
                }),
            )
            .expect("grouped actions");
        assert_eq!(batch["result"]["scene_revision"], json!(revision + 1));
        server
            .call_value("select", json!({"indices":[0]}))
            .expect("select object");

        let copied = server
            .call_value("copy_selected", json!({}))
            .expect("copy selected");
        assert_eq!(copied["copied"], json!(1));
        server
            .call_value("cut_selected", json!({}))
            .expect("cut selected");
        assert_eq!(
            server
                .call_value("scene_summary", json!({}))
                .expect("summary")["objects"]
                .as_array()
                .map(Vec::len),
            Some(0)
        );
        server.call_value("paste", json!({})).expect("paste");
        assert_eq!(
            server
                .call_value("scene_summary", json!({}))
                .expect("summary")["objects"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
        let slicers = server
            .call_value("discover_slicers", json!({}))
            .expect("slicers")["slicers"]
            .as_array()
            .cloned()
            .expect("slicer array");
        for slicer in slicers {
            assert!(slicer["key"].is_string());
            assert!(slicer["label"].is_string());
            assert!(slicer["path"].is_string());
            assert!(slicer["stl_fallback"].is_boolean());
        }
        let stale = server.call_value(
            "translate",
            json!({"index":0,"offset_mm":[1,0,0],"expected_scene_revision":revision}),
        );
        assert!(matches!(stale, Err(AdapterError::Conflict(_))));
    }

    #[test]
    fn sample_image_color_applies_a_bounded_ppm_to_selection() {
        let root = env::temp_dir().join(format!("builder3d-r4-ppm-{}", std::process::id()));
        let _ = fs::create_dir_all(&root);
        let image = root.join("sample.ppm");
        fs::write(&image, b"P3\n2 1\n255\n255 0 0 0 0 255\n").expect("ppm");
        let roots = ApprovedRoots::from_paths([root]).expect("approved root");
        let server = R4Server::new(R4Config::for_profile(Profile::Full, roots));
        server
            .call_value("add_primitive", json!({"primitive":"cube"}))
            .expect("add primitive");
        server
            .call_value("select", json!({"indices":[0]}))
            .expect("select");
        let summary = server
            .call_value(
                "sample_image_color",
                json!({"path":image.display().to_string()}),
            )
            .expect("sample image color");
        assert_eq!(summary["objects"][0]["color"], json!([0.5, 0.0, 0.5, 1.0]));
    }
}
