use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{
    BooleanOperation, BooleanOptions, CoreError, Mesh, ObjectId, PrimitiveKind, Scene, SceneObject,
    Transform, with_boolean_target_color, without_boolean_color_provenance,
};

/// One semantic mutation that can be applied, undone, and redone.
pub trait Command: Send {
    /// Stable registry name used by desktop and MCP adapters.
    fn name(&self) -> &str;
    /// Applies the command and reports whether it changed the scene.
    fn execute(&mut self, scene: &mut Scene) -> Result<bool, CoreError>;
    /// Reverts a previously changed command.
    fn undo(&mut self, scene: &mut Scene) -> Result<(), CoreError>;
    /// Returns durable IDs touched by this command.
    fn affected_object_ids(&self) -> Vec<ObjectId>;
}

/// An internal history item retaining the state IDs around a command.
pub(crate) struct HistoryEntry {
    pub(crate) command: Box<dyn Command>,
    pub(crate) before_state_id: u64,
    pub(crate) after_state_id: u64,
    pub(crate) before_selection_order: Vec<usize>,
    pub(crate) after_selection_order: Vec<usize>,
}

/// Inserts one object at a stable scene index.
pub struct AddObjectCommand {
    object: SceneObject,
    index: Option<usize>,
    before_selection: Option<BTreeSet<usize>>,
}

impl AddObjectCommand {
    /// Creates an add command; `index` defaults to the end of the scene.
    #[must_use]
    pub fn new(object: SceneObject) -> Self {
        Self {
            object,
            index: None,
            before_selection: None,
        }
    }

    /// Creates an add command with an explicit insertion index.
    #[must_use]
    pub fn at_index(object: SceneObject, index: usize) -> Self {
        Self {
            object,
            index: Some(index),
            before_selection: None,
        }
    }
}

impl Command for AddObjectCommand {
    fn name(&self) -> &str {
        "add_object"
    }

    fn execute(&mut self, scene: &mut Scene) -> Result<bool, CoreError> {
        if self.before_selection.is_none() {
            self.before_selection = Some(scene.selection().clone());
        }
        self.object.mesh().validate_non_empty()?;
        if scene_contains_id(scene, self.object.object_id()) {
            return Err(CoreError::ObjectId(format!(
                "object ID {:?} already exists",
                self.object.object_id()
            )));
        }
        let insert_at = self
            .index
            .unwrap_or(scene.objects().len())
            .min(scene.objects().len());
        self.index = Some(insert_at);
        scene.insert_selection_index(insert_at);
        scene.objects_mut().insert(insert_at, self.object.clone());
        Ok(true)
    }

    fn undo(&mut self, scene: &mut Scene) -> Result<(), CoreError> {
        let index = scene.object_index(self.object.object_id())?;
        scene.objects_mut().remove(index);
        scene.remove_selection_index(index);
        if let Some(selection) = &self.before_selection {
            scene.set_selection_raw(selection.clone());
        }
        Ok(())
    }

    fn affected_object_ids(&self) -> Vec<ObjectId> {
        object_ids(&self.object)
    }
}

/// Removes one top-level object and retains it for undo.
pub struct RemoveObjectCommand {
    index: usize,
    removed: Option<SceneObject>,
    before_selection: Option<BTreeSet<usize>>,
}

impl RemoveObjectCommand {
    /// Creates a remove command for one top-level index.
    #[must_use]
    pub const fn new(index: usize) -> Self {
        Self {
            index,
            removed: None,
            before_selection: None,
        }
    }
}

impl Command for RemoveObjectCommand {
    fn name(&self) -> &str {
        "remove_object"
    }

    fn execute(&mut self, scene: &mut Scene) -> Result<bool, CoreError> {
        if self.before_selection.is_none() {
            self.before_selection = Some(scene.selection().clone());
        }
        let object = scene
            .objects()
            .get(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        if let Some(removed) = &self.removed {
            if object.object_id() != removed.object_id() {
                return Err(CoreError::CommandFailed {
                    command: self.name().to_owned(),
                    reason: "scene order no longer matches the command".to_owned(),
                });
            }
        } else {
            self.removed = Some(object.clone());
        }
        scene.objects_mut().remove(self.index);
        scene.remove_selection_index(self.index);
        Ok(true)
    }

    fn undo(&mut self, scene: &mut Scene) -> Result<(), CoreError> {
        let object = self
            .removed
            .as_ref()
            .ok_or_else(|| CoreError::CommandFailed {
                command: self.name().to_owned(),
                reason: "remove command has not executed".to_owned(),
            })?;
        if scene_contains_id(scene, object.object_id()) {
            return Err(CoreError::ObjectId(format!(
                "object ID {:?} already exists",
                object.object_id()
            )));
        }
        let insert_at = self.index.min(scene.objects().len());
        scene.insert_selection_index(insert_at);
        scene.objects_mut().insert(insert_at, object.clone());
        if let Some(selection) = &self.before_selection {
            scene.set_selection_raw(selection.clone());
        }
        Ok(())
    }

    fn affected_object_ids(&self) -> Vec<ObjectId> {
        self.removed.as_ref().map(object_ids).unwrap_or_default()
    }
}

/// Replaces one authoritative mesh while retaining the old mesh for undo.
pub struct ReplaceMeshCommand {
    index: usize,
    new_mesh: Mesh,
    old_mesh: Option<Mesh>,
    old_metadata: Option<Value>,
    target_id: Option<ObjectId>,
}

impl ReplaceMeshCommand {
    /// Creates a replacement command.
    #[must_use]
    pub fn new(index: usize, new_mesh: Mesh) -> Self {
        Self {
            index,
            new_mesh,
            old_mesh: None,
            old_metadata: None,
            target_id: None,
        }
    }
}

impl Command for ReplaceMeshCommand {
    fn name(&self) -> &str {
        "replace_mesh"
    }

    fn execute(&mut self, scene: &mut Scene) -> Result<bool, CoreError> {
        self.new_mesh.validate_non_empty()?;
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        if self.target_id.is_none() {
            self.target_id = Some(object.object_id().to_owned());
        }
        if object.mesh() == &self.new_mesh {
            return Ok(false);
        }
        if self.old_mesh.is_none() {
            self.old_mesh = Some(object.mesh().clone());
            self.old_metadata = Some(object.metadata().clone());
        }
        object.set_mesh_raw(self.new_mesh.clone());
        let metadata = without_boolean_color_provenance(object.metadata());
        object.set_metadata_raw(metadata);
        Ok(true)
    }

    fn undo(&mut self, scene: &mut Scene) -> Result<(), CoreError> {
        let old_mesh = self
            .old_mesh
            .as_ref()
            .ok_or_else(|| CoreError::CommandFailed {
                command: self.name().to_owned(),
                reason: "replace command has not executed".to_owned(),
            })?;
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        object.set_mesh_raw(old_mesh.clone());
        if let Some(old_metadata) = self.old_metadata.as_ref() {
            object.set_metadata_raw(old_metadata.clone());
        }
        Ok(())
    }

    fn affected_object_ids(&self) -> Vec<ObjectId> {
        self.target_id.iter().cloned().collect()
    }
}

/// Sets one object transform.
pub struct TransformObjectCommand {
    index: usize,
    new_transform: Transform,
    old_transform: Option<Transform>,
    target_id: Option<ObjectId>,
}

impl TransformObjectCommand {
    /// Creates a transform command after validating the target index at apply time.
    #[must_use]
    pub const fn new(index: usize, new_transform: Transform) -> Self {
        Self {
            index,
            new_transform,
            old_transform: None,
            target_id: None,
        }
    }
}

impl Command for TransformObjectCommand {
    fn name(&self) -> &str {
        "set_transform"
    }

    fn execute(&mut self, scene: &mut Scene) -> Result<bool, CoreError> {
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        if self.target_id.is_none() {
            self.target_id = Some(object.object_id().to_owned());
        }
        if object.transform() == &self.new_transform {
            return Ok(false);
        }
        if self.old_transform.is_none() {
            self.old_transform = Some(*object.transform());
        }
        object.set_transform_raw(self.new_transform);
        Ok(true)
    }

    fn undo(&mut self, scene: &mut Scene) -> Result<(), CoreError> {
        let old_transform = self.old_transform.ok_or_else(|| CoreError::CommandFailed {
            command: self.name().to_owned(),
            reason: "transform command has not executed".to_owned(),
        })?;
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        object.set_transform_raw(old_transform);
        Ok(())
    }

    fn affected_object_ids(&self) -> Vec<ObjectId> {
        self.target_id.iter().cloned().collect()
    }
}

/// Sets one object's RGBA color.
pub struct SetColorCommand {
    index: usize,
    new_color: [f32; 4],
    old_color: Option<[f32; 4]>,
    old_metadata: Option<Value>,
    target_id: Option<ObjectId>,
}

impl SetColorCommand {
    /// Creates a color command; the range is checked at apply time.
    #[must_use]
    pub const fn new(index: usize, new_color: [f32; 4]) -> Self {
        Self {
            index,
            new_color,
            old_color: None,
            old_metadata: None,
            target_id: None,
        }
    }
}

impl Command for SetColorCommand {
    fn name(&self) -> &str {
        "set_color"
    }

    fn execute(&mut self, scene: &mut Scene) -> Result<bool, CoreError> {
        if self
            .new_color
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(CoreError::InvalidColor(
                "RGBA values must be finite and between 0 and 1".to_owned(),
            ));
        }
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        if self.target_id.is_none() {
            self.target_id = Some(object.object_id().to_owned());
        }
        if object.color() == self.new_color {
            return Ok(false);
        }
        if self.old_color.is_none() {
            self.old_color = Some(object.color());
            self.old_metadata = Some(object.metadata().clone());
        }
        let metadata = with_boolean_target_color(object.metadata(), self.new_color);
        object.set_color_raw(self.new_color);
        object.set_metadata_raw(metadata);
        Ok(true)
    }

    fn undo(&mut self, scene: &mut Scene) -> Result<(), CoreError> {
        let old_color = self.old_color.ok_or_else(|| CoreError::CommandFailed {
            command: self.name().to_owned(),
            reason: "color command has not executed".to_owned(),
        })?;
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        object.set_color_raw(old_color);
        if let Some(old_metadata) = self.old_metadata.as_ref() {
            object.set_metadata_raw(old_metadata.clone());
        }
        Ok(())
    }

    fn affected_object_ids(&self) -> Vec<ObjectId> {
        self.target_id.iter().cloned().collect()
    }
}

/// Sets object metadata without allowing non-object JSON values.
pub struct SetMetadataCommand {
    index: usize,
    new_metadata: Value,
    old_metadata: Option<Value>,
    target_id: Option<ObjectId>,
}

impl SetMetadataCommand {
    /// Creates a metadata command.
    #[must_use]
    pub fn new(index: usize, new_metadata: Value) -> Self {
        Self {
            index,
            new_metadata,
            old_metadata: None,
            target_id: None,
        }
    }
}

impl Command for SetMetadataCommand {
    fn name(&self) -> &str {
        "set_metadata"
    }

    fn execute(&mut self, scene: &mut Scene) -> Result<bool, CoreError> {
        if !self.new_metadata.is_object() {
            return Err(CoreError::InvalidObject(
                "metadata must be a JSON object".to_owned(),
            ));
        }
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        if self.target_id.is_none() {
            self.target_id = Some(object.object_id().to_owned());
        }
        if object.metadata() == &self.new_metadata {
            return Ok(false);
        }
        if self.old_metadata.is_none() {
            self.old_metadata = Some(object.metadata().clone());
        }
        object.set_metadata_raw(self.new_metadata.clone());
        Ok(true)
    }

    fn undo(&mut self, scene: &mut Scene) -> Result<(), CoreError> {
        let old_metadata = self
            .old_metadata
            .as_ref()
            .ok_or_else(|| CoreError::CommandFailed {
                command: self.name().to_owned(),
                reason: "metadata command has not executed".to_owned(),
            })?;
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        object.set_metadata_raw(old_metadata.clone());
        Ok(())
    }

    fn affected_object_ids(&self) -> Vec<ObjectId> {
        self.target_id.iter().cloned().collect()
    }
}

/// Sets one object's visibility flag.
pub struct SetVisibilityCommand {
    index: usize,
    new_visible: bool,
    old_visible: Option<bool>,
    target_id: Option<ObjectId>,
}

impl SetVisibilityCommand {
    /// Creates a visibility command.
    #[must_use]
    pub const fn new(index: usize, new_visible: bool) -> Self {
        Self {
            index,
            new_visible,
            old_visible: None,
            target_id: None,
        }
    }
}

impl Command for SetVisibilityCommand {
    fn name(&self) -> &str {
        "set_visibility"
    }

    fn execute(&mut self, scene: &mut Scene) -> Result<bool, CoreError> {
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        if self.target_id.is_none() {
            self.target_id = Some(object.object_id().to_owned());
        }
        if object.is_visible() == self.new_visible {
            return Ok(false);
        }
        if self.old_visible.is_none() {
            self.old_visible = Some(object.is_visible());
        }
        object.set_visible_raw(self.new_visible);
        Ok(true)
    }

    fn undo(&mut self, scene: &mut Scene) -> Result<(), CoreError> {
        let old_visible = self.old_visible.ok_or_else(|| CoreError::CommandFailed {
            command: self.name().to_owned(),
            reason: "visibility command has not executed".to_owned(),
        })?;
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        object.set_visible_raw(old_visible);
        Ok(())
    }

    fn affected_object_ids(&self) -> Vec<ObjectId> {
        self.target_id.iter().cloned().collect()
    }
}

/// Renames one object without replacing its durable identity or mesh.
pub struct RenameObjectCommand {
    index: usize,
    name: String,
    before: Option<String>,
    id: Option<ObjectId>,
}

impl RenameObjectCommand {
    /// Creates one undoable name change without replacing identity or geometry.
    #[must_use]
    pub fn new(index: usize, name: String) -> Self {
        Self {
            index,
            name,
            before: None,
            id: None,
        }
    }
}

impl Command for RenameObjectCommand {
    fn name(&self) -> &str {
        "rename_object"
    }
    fn execute(&mut self, scene: &mut Scene) -> Result<bool, CoreError> {
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        if object.name() == self.name {
            return Ok(false);
        }
        let before = object.name().to_owned();
        object.set_name_raw(self.name.clone())?;
        self.before.get_or_insert(before);
        self.id = Some(object.object_id().to_owned());
        Ok(true)
    }
    fn undo(&mut self, scene: &mut Scene) -> Result<(), CoreError> {
        let before = self
            .before
            .clone()
            .ok_or_else(|| CoreError::CommandFailed {
                command: "rename_object".into(),
                reason: "rename has not executed".into(),
            })?;
        scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?
            .set_name_raw(before)
    }
    fn affected_object_ids(&self) -> Vec<ObjectId> {
        self.id.iter().cloned().collect()
    }
}

/// Applies a sequence of commands as one undo/redo history entry.
pub struct GroupedCommand {
    commands: Vec<Box<dyn Command>>,
    changed: Vec<bool>,
    before_selection: Option<BTreeSet<usize>>,
    after_selection: Option<BTreeSet<usize>>,
}

impl GroupedCommand {
    /// Creates a grouped command. Empty groups are valid no-ops.
    #[must_use]
    pub fn new(commands: Vec<Box<dyn Command>>) -> Self {
        Self {
            commands,
            changed: Vec::new(),
            before_selection: None,
            after_selection: None,
        }
    }

    /// Creates a grouped command and applies a replacement selection after it
    /// changes the scene. The previous selection is restored by undo.
    #[must_use]
    pub fn with_after_selection(
        commands: Vec<Box<dyn Command>>,
        after_selection: BTreeSet<usize>,
    ) -> Self {
        Self {
            commands,
            changed: Vec::new(),
            before_selection: None,
            after_selection: Some(after_selection),
        }
    }
}

impl Command for GroupedCommand {
    fn name(&self) -> &str {
        "grouped"
    }

    fn execute(&mut self, scene: &mut Scene) -> Result<bool, CoreError> {
        if self.before_selection.is_none() {
            self.before_selection = Some(scene.selection().clone());
        }
        self.changed.clear();
        let group_name = self.name().to_owned();
        for index in 0..self.commands.len() {
            let command = self
                .commands
                .get_mut(index)
                .ok_or_else(|| CoreError::CommandFailed {
                    command: group_name.clone(),
                    reason: "group command index disappeared".to_owned(),
                })?;
            match command.execute(scene) {
                Ok(changed) => self.changed.push(changed),
                Err(error) => {
                    for rollback_index in (0..self.changed.len()).rev() {
                        if self.changed[rollback_index]
                            && let Some(rollback) = self.commands.get_mut(rollback_index)
                        {
                            let _ = rollback.undo(scene);
                        }
                    }
                    if let Some(selection) = &self.before_selection {
                        scene.set_selection_raw(selection.clone());
                    }
                    self.changed.clear();
                    return Err(error);
                }
            }
        }
        let changed = self.changed.iter().any(|changed| *changed);
        if changed && let Some(selection) = &self.after_selection {
            scene.set_selection_raw(selection.clone());
        }
        Ok(changed)
    }

    fn undo(&mut self, scene: &mut Scene) -> Result<(), CoreError> {
        let group_name = self.name().to_owned();
        for index in (0..self.changed.len()).rev() {
            if self.changed[index] {
                let command =
                    self.commands
                        .get_mut(index)
                        .ok_or_else(|| CoreError::CommandFailed {
                            command: group_name.clone(),
                            reason: "group command index disappeared".to_owned(),
                        })?;
                command.undo(scene)?;
            }
        }
        if let Some(selection) = &self.before_selection {
            scene.set_selection_raw(selection.clone());
        }
        Ok(())
    }

    fn affected_object_ids(&self) -> Vec<ObjectId> {
        let mut ids = BTreeSet::new();
        for command in &self.commands {
            ids.extend(command.affected_object_ids());
        }
        ids.into_iter().collect()
    }
}

/// The typed request set accepted by the core command registry.
pub enum CommandRequest {
    /// Insert an already constructed object.
    AddObject(SceneObject),
    /// Insert one catalog primitive.
    AddPrimitive {
        /// Primitive catalog kind.
        kind: PrimitiveKind,
        /// Optional width/depth/height dimensions in millimetres.
        dimensions_mm: Option<[f32; 3]>,
        /// Optional display name.
        name: Option<String>,
    },
    /// Remove one top-level object.
    RemoveObject { index: usize },
    /// Replace one top-level mesh.
    ReplaceMesh { index: usize, mesh: Mesh },
    /// Set one object transform.
    SetTransform { index: usize, transform: Transform },
    /// Set one object RGBA color.
    SetColor { index: usize, color: [f32; 4] },
    /// Set one object metadata object.
    SetMetadata { index: usize, metadata: Value },
    /// Set one object visibility flag.
    SetVisibility { index: usize, visible: bool },
    /// Duplicate the current top-level selection.
    DuplicateSelected,
    /// Group the current top-level selection.
    GroupSelected,
    /// Ungroup the current top-level selection.
    UngroupSelected,
    /// Apply one world-space boolean and remove its tool object.
    Boolean {
        /// Target object index whose identity and transform survive.
        target_index: usize,
        /// Tool object index removed after the result is committed.
        tool_index: usize,
        /// Boolean kernel operation.
        operation: BooleanOperation,
        /// Boolean resource and cap policy.
        options: BooleanOptions,
    },
    /// Simplify one authoritative mesh toward a bounded face target.
    Simplify {
        /// Target object index.
        index: usize,
        /// Optional target face count.
        face_count: Option<usize>,
    },
    /// Subdivide one authoritative mesh to approximately twice its faces.
    Subdivide {
        /// Target object index.
        index: usize,
    },
    /// Smooth one authoritative mesh with bounded Laplacian iterations.
    Smooth {
        /// Target object index.
        index: usize,
        /// Number of smoothing iterations.
        iterations: usize,
    },
    /// Apply Elastic Grab to one local-space center/offset.
    ElasticGrab {
        /// Target object index.
        index: usize,
        /// Local brush center.
        center_mm: [f32; 3],
        /// Local displacement offset.
        offset_mm: [f32; 3],
        /// Brush parameters.
        options: crate::ElasticGrabOptions,
    },
    /// Apply Crease Polish to one local-space path.
    CreasePolish {
        /// Target object index.
        index: usize,
        /// Local path centers.
        centers_mm: Vec<[f32; 3]>,
        /// Brush parameters.
        options: crate::CreasePolishOptions,
    },
    /// Apply Smooth Brush to one local-space path.
    SmoothBrush {
        /// Target object index.
        index: usize,
        /// Local path centers.
        centers_mm: Vec<[f32; 3]>,
        /// Brush parameters.
        options: crate::SmoothBrushOptions,
    },
    /// Apply Blob to one local-space path.
    Blob {
        /// Target object index.
        index: usize,
        /// Local path centers.
        centers_mm: Vec<[f32; 3]>,
        /// Brush parameters.
        options: crate::BlobOptions,
    },
    /// Normalize local face density around an optional face region or center path.
    NormalizeDensity {
        /// Target object index.
        index: usize,
        /// Optional authoritative face selection.
        face_indices: Option<Vec<usize>>,
        /// Optional local-space selection centers.
        centers_mm: Option<Vec<[f32; 3]>>,
        /// Density normalization parameters.
        options: crate::NormalizeDensityOptions,
    },
}

/// Describes a registered semantic command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandSpec {
    /// Stable command ID.
    pub id: String,
    /// Short human-readable description.
    pub description: String,
}

/// Registry of the core semantic commands shared by adapters.
#[derive(Clone, Debug, Default)]
pub struct CommandRegistry {
    specs: BTreeMap<String, CommandSpec>,
}

impl CommandRegistry {
    /// Creates the built-in R1 command catalog.
    #[must_use]
    pub fn core() -> Self {
        let mut registry = Self::default();
        let entries = [
            ("add_object", "Insert one scene object"),
            ("add_primitive", "Insert one catalog primitive"),
            ("remove_object", "Remove one scene object"),
            ("replace_mesh", "Replace authoritative mesh data"),
            ("set_transform", "Set an object transform"),
            ("set_color", "Set an object RGBA color"),
            ("set_metadata", "Set an object metadata object"),
            ("set_visibility", "Set an object visibility flag"),
            ("duplicate_selected", "Duplicate selected scene objects"),
            ("group_selected", "Group selected scene objects"),
            ("ungroup_selected", "Ungroup selected scene objects"),
            ("boolean_union", "Union two solid scene objects"),
            (
                "boolean_difference",
                "Subtract one solid scene object from another",
            ),
            (
                "boolean_intersection",
                "Keep the overlap of two solid scene objects",
            ),
            ("simplify", "Reduce one mesh toward a bounded face target"),
            (
                "subdivide",
                "Refine one mesh to approximately twice its faces",
            ),
            (
                "smooth",
                "Smooth one mesh with bounded Laplacian iterations",
            ),
            ("elastic_grab", "Apply a bounded local Elastic Grab stroke"),
            (
                "crease_polish",
                "Apply a bounded local Crease Polish stroke",
            ),
            ("smooth_brush", "Apply a bounded local Smooth Brush stroke"),
            ("blob", "Apply a bounded local Blob stroke"),
            (
                "normalize_face_density",
                "Normalize bounded local face density",
            ),
        ];
        for (id, description) in entries {
            // These literals are validated by construction and cannot fail.
            registry.specs.insert(
                id.to_owned(),
                CommandSpec {
                    id: id.to_owned(),
                    description: description.to_owned(),
                },
            );
        }
        registry
    }

    /// Registers one additional semantic command descriptor.
    pub fn register(&mut self, id: String, description: String) -> Result<(), CoreError> {
        if id.trim().is_empty() || description.trim().is_empty() {
            return Err(CoreError::CommandFailed {
                command: "register".to_owned(),
                reason: "command ID and description must not be empty".to_owned(),
            });
        }
        if self.specs.contains_key(&id) {
            return Err(CoreError::CommandFailed {
                command: "register".to_owned(),
                reason: format!("command {id:?} is already registered"),
            });
        }
        self.specs
            .insert(id.clone(), CommandSpec { id, description });
        Ok(())
    }

    /// Returns whether a command ID is registered.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.specs.contains_key(id)
    }

    /// Returns a descriptor by ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&CommandSpec> {
        self.specs.get(id)
    }

    /// Returns registered command IDs in deterministic order.
    #[must_use]
    pub fn ids(&self) -> Vec<&str> {
        self.specs.keys().map(String::as_str).collect()
    }

    /// Dispatches one typed request through the scene history boundary.
    pub fn dispatch(
        &self,
        scene: &mut Scene,
        request: CommandRequest,
    ) -> Result<crate::CommandResult, CoreError> {
        match request {
            CommandRequest::AddObject(object) => scene.execute(AddObjectCommand::new(object)),
            CommandRequest::AddPrimitive {
                kind,
                dimensions_mm,
                name,
            } => scene
                .add_primitive(kind, dimensions_mm, name)
                .map(|(_, result)| result),
            CommandRequest::RemoveObject { index } => scene.remove_object(index),
            CommandRequest::ReplaceMesh { index, mesh } => scene.replace_mesh(index, mesh),
            CommandRequest::SetTransform { index, transform } => {
                scene.set_transform(index, transform)
            }
            CommandRequest::SetColor { index, color } => scene.set_color(index, color),
            CommandRequest::SetMetadata { index, metadata } => {
                scene.execute(SetMetadataCommand::new(index, metadata))
            }
            CommandRequest::SetVisibility { index, visible } => scene.set_visible(index, visible),
            CommandRequest::DuplicateSelected => scene.duplicate_selected(),
            CommandRequest::GroupSelected => scene.group_selected(),
            CommandRequest::UngroupSelected => scene.ungroup_selected(),
            CommandRequest::Boolean {
                target_index,
                tool_index,
                operation,
                options,
            } => scene
                .apply_boolean(target_index, tool_index, operation, options)
                .map(|(_, result, _)| result),
            CommandRequest::Simplify { index, face_count } => scene
                .apply_simplify(index, face_count)
                .map(|(result, _)| result),
            CommandRequest::Subdivide { index } => {
                scene.apply_subdivide(index).map(|(result, _)| result)
            }
            CommandRequest::Smooth { index, iterations } => scene
                .apply_smooth(index, iterations)
                .map(|(result, _)| result),
            CommandRequest::ElasticGrab {
                index,
                center_mm,
                offset_mm,
                options,
            } => scene
                .apply_elastic_grab(index, center_mm, offset_mm, options)
                .map(|(result, _)| result),
            CommandRequest::CreasePolish {
                index,
                centers_mm,
                options,
            } => scene
                .apply_crease_polish(index, &centers_mm, options)
                .map(|(result, _)| result),
            CommandRequest::SmoothBrush {
                index,
                centers_mm,
                options,
            } => scene
                .apply_smooth_brush(index, &centers_mm, options)
                .map(|(result, _)| result),
            CommandRequest::Blob {
                index,
                centers_mm,
                options,
            } => scene
                .apply_blob(index, &centers_mm, options)
                .map(|(result, _)| result),
            CommandRequest::NormalizeDensity {
                index,
                face_indices,
                centers_mm,
                options,
            } => scene
                .apply_normalize_face_density(
                    index,
                    face_indices.as_deref(),
                    centers_mm.as_deref(),
                    options,
                )
                .map(|(result, _)| result),
        }
    }
}

fn object_ids(object: &SceneObject) -> Vec<ObjectId> {
    let mut ids = Vec::new();
    object.all_object_ids(&mut ids);
    ids
}

fn scene_contains_id(scene: &Scene, object_id: &str) -> bool {
    scene.objects().iter().any(|object| {
        let mut ids = Vec::new();
        object.all_object_ids(&mut ids);
        ids.iter().any(|id| id == object_id)
    })
}
