use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::commands::{Command, HistoryEntry};
use crate::{
    BooleanColorProvenance, CoreError, Mesh, Transform, with_boolean_color_provenance,
    without_boolean_color_provenance,
};

/// Durable object ID used by scene, command, and project boundaries.
pub type ObjectId = String;

/// The Builder3D default green RGBA color for imported or generic meshes.
pub const DEFAULT_MESH_COLOR: [f32; 4] = [33.0 / 255.0, 187.0 / 255.0, 76.0 / 255.0, 1.0];

static NEXT_OBJECT_NUMBER: AtomicU64 = AtomicU64::new(1);

/// A nested group child persisted inside a scene object.
#[derive(Clone, Debug)]
pub struct SceneChild {
    mesh: Mesh,
    object_id: ObjectId,
    name: String,
    transform: Transform,
    color: [f32; 4],
    visible: bool,
    metadata: Value,
    children: Vec<SceneChild>,
}

impl SceneChild {
    /// Returns the child ID.
    #[must_use]
    pub fn object_id(&self) -> &str {
        &self.object_id
    }

    /// Returns the child name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the authoritative child mesh.
    #[must_use]
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    /// Returns the child transform.
    #[must_use]
    pub const fn transform(&self) -> &Transform {
        &self.transform
    }

    /// Returns the child RGBA color.
    #[must_use]
    pub const fn color(&self) -> [f32; 4] {
        self.color
    }

    /// Returns whether the child is visible.
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    /// Returns object metadata.
    #[must_use]
    pub const fn metadata(&self) -> &Value {
        &self.metadata
    }

    /// Returns nested children in archive order.
    #[must_use]
    pub fn children(&self) -> &[SceneChild] {
        &self.children
    }

    pub(crate) fn from_object(object: &SceneObject) -> Self {
        Self {
            mesh: object.mesh.clone(),
            object_id: object.object_id.clone(),
            name: object.name.clone(),
            transform: object.transform,
            color: object.color,
            visible: object.visible,
            metadata: object.metadata.clone(),
            children: object.children.clone(),
        }
    }

    fn duplicate_with_new_ids(&self) -> Self {
        Self {
            mesh: self.mesh.clone(),
            object_id: new_object_id(),
            name: self.name.clone(),
            transform: self.transform,
            color: self.color,
            visible: self.visible,
            metadata: self.metadata.clone(),
            children: self
                .children
                .iter()
                .map(SceneChild::duplicate_with_new_ids)
                .collect(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        mesh: Mesh,
        object_id: ObjectId,
        name: String,
        transform: Transform,
        color: [f32; 4],
        visible: bool,
        metadata: Value,
        children: Vec<SceneChild>,
    ) -> Result<Self, CoreError> {
        validate_object_fields(&object_id, &name, color, &metadata)?;
        mesh.validate_non_empty()?;
        Ok(Self {
            mesh,
            object_id,
            name,
            transform,
            color,
            visible,
            metadata,
            children,
        })
    }

    pub(crate) fn all_object_ids(&self, output: &mut Vec<ObjectId>) {
        output.push(self.object_id.clone());
        for child in &self.children {
            child.all_object_ids(output);
        }
    }
}

/// One top-level scene object and its optional group children.
#[derive(Clone, Debug)]
pub struct SceneObject {
    mesh: Mesh,
    object_id: ObjectId,
    name: String,
    transform: Transform,
    color: [f32; 4],
    visible: bool,
    metadata: Value,
    children: Vec<SceneChild>,
}

impl SceneObject {
    /// Creates a generic object with a durable ID, identity transform, and
    /// the reference application's default green color.
    #[must_use]
    pub fn new(mesh: Mesh) -> Self {
        let number = NEXT_OBJECT_NUMBER.fetch_add(1, Ordering::Relaxed);
        Self {
            mesh,
            object_id: new_object_id(),
            name: format!("Object {number}"),
            transform: Transform::identity(),
            color: DEFAULT_MESH_COLOR,
            visible: true,
            metadata: empty_metadata(),
            children: Vec::new(),
        }
    }

    /// Creates an object with explicit persisted fields.
    pub fn with_details(
        mesh: Mesh,
        object_id: Option<ObjectId>,
        name: Option<String>,
        color: Option<[f32; 4]>,
        metadata: Option<Value>,
    ) -> Result<Self, CoreError> {
        let object_id = object_id.unwrap_or_else(new_object_id);
        let name = name.unwrap_or_else(|| {
            let number = NEXT_OBJECT_NUMBER.fetch_add(1, Ordering::Relaxed);
            format!("Object {number}")
        });
        let color = color.unwrap_or(DEFAULT_MESH_COLOR);
        let metadata = metadata.unwrap_or_else(empty_metadata);
        Self::from_parts(
            mesh,
            object_id,
            name,
            Transform::identity(),
            color,
            true,
            metadata,
            Vec::new(),
        )
    }

    /// Returns the durable object ID.
    #[must_use]
    pub fn object_id(&self) -> &str {
        &self.object_id
    }

    /// Returns the display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the authoritative mesh.
    #[must_use]
    pub const fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    /// Returns the row-major object transform.
    #[must_use]
    pub const fn transform(&self) -> &Transform {
        &self.transform
    }

    /// Returns the object translation in millimetres.
    #[must_use]
    pub const fn position(&self) -> [f32; 3] {
        self.transform.position()
    }

    /// Returns the object RGBA color.
    #[must_use]
    pub const fn color(&self) -> [f32; 4] {
        self.color
    }

    /// Returns whether the object is visible to adapters.
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    /// Returns object metadata.
    #[must_use]
    pub const fn metadata(&self) -> &Value {
        &self.metadata
    }

    /// Returns nested group children in persisted order.
    #[must_use]
    pub fn children(&self) -> &[SceneChild] {
        &self.children
    }

    /// Returns a transformed copy of the authoritative mesh.
    pub fn transformed_mesh(&self) -> Result<Mesh, CoreError> {
        self.mesh.transformed(&self.transform)
    }

    /// Computes the next transform after a world-independent translation.
    pub fn translated_transform(&self, offset_mm: [f32; 3]) -> Result<Transform, CoreError> {
        self.transform.translated(offset_mm)
    }

    /// Computes the next transform after an axis-angle rotation.
    pub fn rotated_transform(
        &self,
        angle_degrees: f32,
        axis: [f32; 3],
    ) -> Result<Transform, CoreError> {
        self.transform.rotated(angle_degrees, axis)
    }

    /// Computes the next transform after a scale.
    pub fn scaled_transform(&self, scale: [f32; 3]) -> Result<Transform, CoreError> {
        self.transform.scaled(scale)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        mesh: Mesh,
        object_id: ObjectId,
        name: String,
        transform: Transform,
        color: [f32; 4],
        visible: bool,
        metadata: Value,
        children: Vec<SceneChild>,
    ) -> Result<Self, CoreError> {
        validate_object_fields(&object_id, &name, color, &metadata)?;
        mesh.validate_non_empty()?;
        Ok(Self {
            mesh,
            object_id,
            name,
            transform,
            color,
            visible,
            metadata,
            children,
        })
    }

    pub(crate) fn set_transform_raw(&mut self, transform: Transform) {
        self.transform = transform;
    }

    pub(crate) fn set_color_raw(&mut self, color: [f32; 4]) {
        self.color = color;
    }

    pub(crate) fn set_mesh_raw(&mut self, mesh: Mesh) {
        self.mesh = mesh;
    }

    pub(crate) fn set_metadata_raw(&mut self, metadata: Value) {
        self.metadata = metadata;
    }

    pub(crate) fn set_visible_raw(&mut self, visible: bool) {
        self.visible = visible;
    }

    pub(crate) fn set_name_raw(&mut self, name: String) -> Result<(), CoreError> {
        validate_object_fields(&self.object_id, &name, self.color, &self.metadata)?;
        self.name = name;
        Ok(())
    }

    fn duplicate_with_new_id(&self, name: String) -> Result<Self, CoreError> {
        Self::from_parts(
            self.mesh.clone(),
            new_object_id(),
            name,
            self.transform,
            self.color,
            self.visible,
            self.metadata.clone(),
            self.children
                .iter()
                .map(SceneChild::duplicate_with_new_ids)
                .collect(),
        )
    }

    fn from_child(child: &SceneChild, parent_transform: &Transform) -> Result<Self, CoreError> {
        let transform = parent_transform.multiply(&child.transform)?;
        Self::from_parts(
            child.mesh.clone(),
            child.object_id.clone(),
            child.name.clone(),
            transform,
            child.color,
            child.visible,
            child.metadata.clone(),
            child.children.clone(),
        )
    }

    pub(crate) fn all_object_ids(&self, output: &mut Vec<ObjectId>) {
        output.push(self.object_id.clone());
        for child in &self.children {
            child.all_object_ids(output);
        }
    }
}

/// A cloneable scene state used by isolated operations and grouped commits.
#[derive(Clone, Debug)]
pub struct SceneSnapshot {
    /// Project identity retained across an isolated operation.
    pub project_id: ObjectId,
    /// Scene revision at snapshot time.
    pub revision: u64,
    /// Top-level objects in order.
    pub objects: Vec<SceneObject>,
    /// Selected top-level indices.
    pub selection: BTreeSet<usize>,
    /// Selection order used by ordered multi-object operations.
    pub selection_order: Vec<usize>,
}

/// Result of applying, undoing, or redoing one semantic command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandResult {
    /// Whether the scene changed and a history entry was created.
    pub changed: bool,
    /// Monotonic scene revision after the transition.
    pub revision: u64,
    /// Command name registered at the semantic boundary.
    pub command_name: String,
    /// Durable IDs affected by the command.
    pub affected_object_ids: Vec<ObjectId>,
    /// Stable history-state identity used by conflict-aware adapters.
    pub history_state_id: u64,
}

/// The portable authoritative scene and its undo/redo history.
pub struct Scene {
    objects: Vec<SceneObject>,
    project_id: ObjectId,
    revision: u64,
    selection: BTreeSet<usize>,
    selection_order: Vec<usize>,
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    history_state_id: u64,
    next_history_state_id: u64,
    source_archive: Option<Vec<u8>>,
    dirty: bool,
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    /// Creates an empty scene with a new project ID.
    #[must_use]
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            project_id: new_object_id(),
            revision: 0,
            selection: BTreeSet::new(),
            selection_order: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            history_state_id: 0,
            next_history_state_id: 1,
            source_archive: None,
            dirty: false,
        }
    }

    /// Returns the project ID persisted in `.3dp`.
    #[must_use]
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    /// Returns the current semantic scene revision.
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Returns the stable history-state identity.
    #[must_use]
    pub const fn history_state_id(&self) -> u64 {
        self.history_state_id
    }

    /// Returns top-level objects in scene order.
    #[must_use]
    pub fn objects(&self) -> &[SceneObject] {
        &self.objects
    }

    /// Returns one top-level object.
    pub fn object(&self, index: usize) -> Result<&SceneObject, CoreError> {
        self.objects.get(index).ok_or(CoreError::ObjectIndex(index))
    }

    /// Returns the selected top-level indices in ascending order.
    #[must_use]
    pub fn selection(&self) -> &BTreeSet<usize> {
        &self.selection
    }

    /// Selects valid top-level indices without creating a model-history entry.
    pub fn set_selection<I>(&mut self, indices: I)
    where
        I: IntoIterator<Item = usize>,
    {
        let indices = indices.into_iter().collect::<Vec<_>>();
        self.selection = indices
            .iter()
            .copied()
            .filter(|index| *index < self.objects.len())
            .collect();
        self.selection_order
            .retain(|index| self.selection.contains(index));
        let mut ordered = self
            .selection_order
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        for index in indices {
            if self.selection.contains(&index) && ordered.insert(index) {
                self.selection_order.push(index);
            }
        }
    }

    /// Returns selection in user order for placement anchors and operands.
    #[must_use]
    pub fn ordered_selection(&self) -> Vec<usize> {
        let seen = self
            .selection_order
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let mut ordered = self
            .selection_order
            .iter()
            .copied()
            .filter(|index| self.selection.contains(index))
            .collect::<Vec<_>>();
        ordered.extend(
            self.selection
                .iter()
                .copied()
                .filter(|index| !seen.contains(index)),
        );
        ordered
    }

    /// Clears the selection without changing scene revision.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
        self.selection_order.clear();
    }

    /// Finds a top-level object index by durable ID.
    pub fn object_index(&self, object_id: &str) -> Result<usize, CoreError> {
        self.objects
            .iter()
            .position(|object| object.object_id() == object_id)
            .ok_or_else(|| CoreError::ObjectId(format!("object {object_id:?} was not found")))
    }

    /// Applies a command and records one history entry only when it changes
    /// the model.  Desktop and MCP adapters use this same boundary.
    pub fn execute<C>(&mut self, command: C) -> Result<CommandResult, CoreError>
    where
        C: Command + 'static,
    {
        self.execute_boxed(Box::new(command))
    }

    /// Applies an already boxed command for dynamic registries.
    pub fn execute_boxed(
        &mut self,
        mut command: Box<dyn Command>,
    ) -> Result<CommandResult, CoreError> {
        if self.revision == u64::MAX || self.next_history_state_id == u64::MAX {
            return Err(CoreError::CommandFailed {
                command: command.name().to_owned(),
                reason: "scene history is exhausted".to_owned(),
            });
        }
        let command_name = command.name().to_owned();
        let before_selection_order = self.ordered_selection();
        let changed = command.execute(self)?;
        let affected_object_ids = command.affected_object_ids();
        if !changed {
            return Ok(CommandResult {
                changed: false,
                revision: self.revision,
                command_name,
                affected_object_ids,
                history_state_id: self.history_state_id,
            });
        }
        let before_state_id = self.history_state_id;
        let after_state_id = self.next_history_state_id;
        self.next_history_state_id =
            self.next_history_state_id
                .checked_add(1)
                .ok_or_else(|| CoreError::CommandFailed {
                    command: command_name.clone(),
                    reason: "history state ID overflow".to_owned(),
                })?;
        self.revision = self.revision.saturating_add(1);
        self.history_state_id = after_state_id;
        let after_selection_order = self.ordered_selection();
        self.undo_stack.push(HistoryEntry {
            command,
            before_state_id,
            after_state_id,
            before_selection_order,
            after_selection_order,
        });
        self.redo_stack.clear();
        self.mark_dirty();
        Ok(CommandResult {
            changed: true,
            revision: self.revision,
            command_name,
            affected_object_ids,
            history_state_id: self.history_state_id,
        })
    }

    /// Undoes the most recent changed command, if one exists.
    pub fn undo(&mut self) -> Result<Option<CommandResult>, CoreError> {
        let Some(mut entry) = self.undo_stack.pop() else {
            return Ok(None);
        };
        let command_name = entry.command.name().to_owned();
        if let Err(error) = entry.command.undo(self) {
            self.undo_stack.push(entry);
            return Err(error);
        }
        let affected_object_ids = entry.command.affected_object_ids();
        self.selection_order = entry.before_selection_order.clone();
        self.history_state_id = entry.before_state_id;
        self.revision = self.revision.saturating_add(1);
        self.redo_stack.push(entry);
        self.mark_dirty();
        Ok(Some(CommandResult {
            changed: true,
            revision: self.revision,
            command_name,
            affected_object_ids,
            history_state_id: self.history_state_id,
        }))
    }

    /// Redoes the most recently undone command, if one exists.
    pub fn redo(&mut self) -> Result<Option<CommandResult>, CoreError> {
        let Some(mut entry) = self.redo_stack.pop() else {
            return Ok(None);
        };
        let command_name = entry.command.name().to_owned();
        if let Err(error) = entry.command.execute(self) {
            self.redo_stack.push(entry);
            return Err(error);
        }
        let affected_object_ids = entry.command.affected_object_ids();
        self.history_state_id = entry.after_state_id;
        self.selection_order = entry.after_selection_order.clone();
        self.revision = self.revision.saturating_add(1);
        self.undo_stack.push(entry);
        self.mark_dirty();
        Ok(Some(CommandResult {
            changed: true,
            revision: self.revision,
            command_name,
            affected_object_ids,
            history_state_id: self.history_state_id,
        }))
    }

    /// Returns a cloneable state for an isolated operation.
    #[must_use]
    pub fn snapshot(&self) -> SceneSnapshot {
        SceneSnapshot {
            project_id: self.project_id.clone(),
            revision: self.revision,
            objects: self.objects.clone(),
            selection: self.selection.clone(),
            selection_order: self.selection_order.clone(),
        }
    }

    /// Clones the visible document for isolated work without transferring the
    /// live undo/redo stacks. A clean loaded archive remains available for
    /// byte-preserving saves in that isolated copy.
    #[must_use]
    pub fn isolated_copy(&self) -> Self {
        Self {
            objects: self.objects.clone(),
            project_id: self.project_id.clone(),
            revision: self.revision,
            selection: self.selection.clone(),
            selection_order: self.selection_order.clone(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            history_state_id: self.history_state_id,
            next_history_state_id: self.next_history_state_id,
            source_archive: self.source_archive.clone(),
            dirty: self.dirty,
        }
    }

    /// Returns the retained source archive length for copy-budget estimates.
    #[must_use]
    pub fn source_archive_bytes_len(&self) -> usize {
        self.source_archive.as_ref().map_or(0, Vec::len)
    }

    /// Consumes the scene and moves its visible document data into a snapshot.
    /// The isolated adapter's history stack is intentionally not transferred.
    #[must_use]
    pub fn into_snapshot(self) -> SceneSnapshot {
        let Self {
            objects,
            project_id,
            revision,
            selection,
            selection_order,
            ..
        } = self;
        SceneSnapshot {
            project_id,
            revision,
            objects,
            selection,
            selection_order,
        }
    }

    /// Commits one isolated operation result through the ordinary scene
    /// history boundary. The caller must check document generation before
    /// calling this method; this method independently checks project identity
    /// and the source revision so a stale worker cannot overwrite a newer
    /// document. The result is one undoable command regardless of how many
    /// objects the isolated operation changed.
    pub fn commit_snapshot_from_operation(
        &mut self,
        expected_project_id: &str,
        expected_revision: u64,
        snapshot: SceneSnapshot,
    ) -> Result<CommandResult, CoreError> {
        if self.project_id != expected_project_id {
            return Err(CoreError::CommandFailed {
                command: "commit isolated operation".to_owned(),
                reason: "the active project changed while the operation was running".to_owned(),
            });
        }
        if self.revision != expected_revision {
            return Err(CoreError::CommandFailed {
                command: "commit isolated operation".to_owned(),
                reason: "the active scene changed while the operation was running".to_owned(),
            });
        }
        if snapshot.project_id != expected_project_id {
            return Err(CoreError::CommandFailed {
                command: "commit isolated operation".to_owned(),
                reason: "the isolated result belongs to a different project".to_owned(),
            });
        }
        let expected_result_revision =
            expected_revision
                .checked_add(1)
                .ok_or_else(|| CoreError::CommandFailed {
                    command: "commit isolated operation".to_owned(),
                    reason: "scene revision is exhausted".to_owned(),
                })?;
        if snapshot.revision != expected_result_revision {
            return Err(CoreError::CommandFailed {
                command: "commit isolated operation".to_owned(),
                reason: format!(
                    "expected one isolated scene commit at revision {expected_result_revision}, got {}",
                    snapshot.revision
                ),
            });
        }
        validate_unique_object_ids(&snapshot.objects)?;
        self.execute(ReplaceSceneContentsCommand::new(snapshot))
    }

    /// Replaces the live model with an isolated snapshot and clears history.
    pub fn restore_snapshot(&mut self, snapshot: SceneSnapshot) -> Result<(), CoreError> {
        validate_unique_object_ids(&snapshot.objects)?;
        self.project_id = snapshot.project_id;
        self.revision = snapshot.revision;
        self.objects = snapshot.objects;
        self.selection = snapshot
            .selection
            .into_iter()
            .filter(|index| *index < self.objects.len())
            .collect();
        self.selection_order.clear();
        let mut ordered = BTreeSet::new();
        for index in snapshot.selection_order {
            if self.selection.contains(&index) && ordered.insert(index) {
                self.selection_order.push(index);
            }
        }
        for index in &self.selection {
            if ordered.insert(*index) {
                self.selection_order.push(*index);
            }
        }
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.mark_dirty();
        Ok(())
    }

    /// Inserts one validated object through the command layer.
    pub fn add_object(&mut self, object: SceneObject) -> Result<CommandResult, CoreError> {
        self.execute(crate::commands::AddObjectCommand::new(object))
    }

    /// Creates and inserts a catalog primitive centered over the Y=0 build bed.
    pub fn add_primitive(
        &mut self,
        kind: crate::PrimitiveKind,
        dimensions_mm: Option<[f32; 3]>,
        name: Option<String>,
    ) -> Result<(ObjectId, CommandResult), CoreError> {
        let (mesh, metadata) = if kind == crate::PrimitiveKind::Torus && dimensions_mm.is_none() {
            let (mesh, parameters) =
                crate::build_parametric_torus(crate::TorusParameters::default())?;
            (mesh, Some(crate::torus_metadata(parameters, None)?))
        } else {
            (crate::make_primitive(kind, dimensions_mm)?, None)
        };
        let Some((minimum, maximum)) = mesh.bounds() else {
            return Err(CoreError::InvalidMesh(
                "primitive has no vertex bounds".to_owned(),
            ));
        };
        let center_x = (f64::from(minimum[0]) + f64::from(maximum[0])) * 0.5;
        let center_z = (f64::from(minimum[2]) + f64::from(maximum[2])) * 0.5;
        let placement = Transform::identity().translated([
            -(center_x as f32),
            -minimum[1],
            -(center_z as f32),
        ])?;
        let mut object = SceneObject::with_details(
            mesh,
            None,
            name,
            Some(crate::primitive_default_color(kind)),
            metadata,
        )?;
        object.set_transform_raw(placement);
        let object_id = object.object_id().to_owned();
        let result = self.add_object(object)?;
        Ok((object_id, result))
    }

    /// Removes one object through an undoable command.
    pub fn remove_object(&mut self, index: usize) -> Result<CommandResult, CoreError> {
        self.execute(crate::commands::RemoveObjectCommand::new(index))
    }

    /// Removes all selected objects as one grouped command.
    pub fn remove_selected(&mut self) -> Result<CommandResult, CoreError> {
        let commands = self
            .selection
            .iter()
            .rev()
            .copied()
            .map(|index| {
                Box::new(crate::commands::RemoveObjectCommand::new(index)) as Box<dyn Command>
            })
            .collect();
        self.execute(crate::commands::GroupedCommand::new(commands))
    }

    /// Duplicates selected objects, placing the copies beside the originals
    /// ([`beside_offset`], OF-43) and selecting them, so duplicating again
    /// steps on to the side.
    pub fn duplicate_selected(&mut self) -> Result<CommandResult, CoreError> {
        let indices: Vec<usize> = self.selection.iter().copied().collect();
        let sources = indices
            .iter()
            .map(|index| self.object(*index))
            .collect::<Result<Vec<_>, _>>()?;
        let step = beside_offset(sources.iter().copied());
        let start = self.objects.len();
        let mut commands = Vec::with_capacity(indices.len());
        for (offset, index) in indices.iter().copied().enumerate() {
            let source = self.object(index)?;
            let mut duplicate =
                source.duplicate_with_new_id(format!("{} (copy)", source.name()))?;
            duplicate.set_transform_raw(source.translated_transform(step)?);
            commands.push(Box::new(crate::commands::AddObjectCommand::at_index(
                duplicate,
                start + offset,
            )) as Box<dyn Command>);
        }
        let after_selection = (start..start.saturating_add(commands.len())).collect();
        self.execute(crate::commands::GroupedCommand::with_after_selection(
            commands,
            after_selection,
        ))
    }

    /// Copies selected top-level objects without changing scene history.
    pub fn copy_selected(&self) -> Result<Vec<SceneObject>, CoreError> {
        let indices: Vec<usize> = self.selection.iter().copied().collect();
        if indices.is_empty() {
            return Err(CoreError::CommandFailed {
                command: "copy_selected".to_owned(),
                reason: "a selection is required".to_owned(),
            });
        }
        indices
            .into_iter()
            .map(|index| self.object(index).cloned())
            .collect()
    }

    /// Pastes copied objects as one undoable command with fresh durable IDs,
    /// beside where the clipboard objects stand ([`beside_offset`], OF-43).
    /// The clipboard then moves along with the pasted copies, so pasting
    /// again places the next copy beside the previous one. The desktop and
    /// MCP both paste through this.
    pub fn paste_beside(
        &mut self,
        clipboard: &mut [SceneObject],
    ) -> Result<CommandResult, CoreError> {
        let step = beside_offset(clipboard.iter());
        let result = self.paste_objects_offset(clipboard, step)?;
        if result.changed {
            for object in clipboard.iter_mut() {
                let moved = object.translated_transform(step)?;
                object.set_transform_raw(moved);
            }
        }
        Ok(result)
    }

    /// Pastes copied objects, unmoved, as one undoable command with fresh
    /// durable IDs. Adapters use [`Self::paste_beside`].
    pub fn paste_objects(&mut self, objects: &[SceneObject]) -> Result<CommandResult, CoreError> {
        self.paste_objects_offset(objects, [0.0; 3])
    }

    fn paste_objects_offset(
        &mut self,
        objects: &[SceneObject],
        step: [f32; 3],
    ) -> Result<CommandResult, CoreError> {
        if objects.is_empty() {
            return Err(CoreError::CommandFailed {
                command: "paste".to_owned(),
                reason: "the clipboard is empty".to_owned(),
            });
        }
        let start = self.objects.len();
        let mut commands = Vec::with_capacity(objects.len());
        for (offset, source) in objects.iter().enumerate() {
            let mut pasted = source.duplicate_with_new_id(format!("{} (copy)", source.name()))?;
            pasted.set_transform_raw(source.translated_transform(step)?);
            commands.push(Box::new(crate::commands::AddObjectCommand::at_index(
                pasted,
                start + offset,
            )) as Box<dyn Command>);
        }
        let after_selection = (start..start.saturating_add(commands.len())).collect();
        self.execute(crate::commands::GroupedCommand::with_after_selection(
            commands,
            after_selection,
        ))
    }

    /// Groups selected objects into one parent with preserved child state.
    pub fn group_selected(&mut self) -> Result<CommandResult, CoreError> {
        let indices: Vec<usize> = self.selection.iter().copied().collect();
        if indices.len() < 2 {
            return self.execute(crate::commands::GroupedCommand::new(Vec::new()));
        }
        // A newly grouped object must remain within the existing .3dp depth
        // contract. Validate iteratively before any recursive child cloning.
        for index in &indices {
            let mut pending = vec![(1usize, self.object(*index)?.children())];
            while let Some((depth, children)) = pending.pop() {
                if depth > crate::MAX_PROJECT_GROUP_DEPTH {
                    return Err(CoreError::OperationResourceLimit {
                        operation: "group".into(),
                        reason: format!(
                            "maximum group nesting depth is {}",
                            crate::MAX_PROJECT_GROUP_DEPTH
                        ),
                    });
                }
                for child in children {
                    pending.push((depth + 1, child.children()));
                }
            }
        }
        let mut children = Vec::with_capacity(indices.len());
        let mut meshes = Vec::with_capacity(indices.len());
        let mut group_visible = false;
        let first_color = self.object(indices[0])?.color();
        let mut same_color = true;
        for index in indices.iter().copied() {
            let object = self.object(index)?;
            children.push(SceneChild::from_object(object));
            meshes.push(object.transformed_mesh()?);
            group_visible |= object.is_visible();
            same_color &= object.color() == first_color;
        }
        let merged_mesh = Mesh::concatenate(&meshes)?;
        let group = SceneObject::from_parts(
            merged_mesh,
            new_object_id(),
            "Group".to_owned(),
            Transform::identity(),
            if same_color {
                first_color
            } else {
                DEFAULT_MESH_COLOR
            },
            group_visible,
            empty_metadata(),
            children,
        )?;
        let group_index = self.objects.len() - indices.len();
        let mut commands: Vec<Box<dyn Command>> = indices
            .iter()
            .rev()
            .copied()
            .map(|index| {
                Box::new(crate::commands::RemoveObjectCommand::new(index)) as Box<dyn Command>
            })
            .collect();
        commands.push(Box::new(crate::commands::AddObjectCommand::at_index(
            group,
            group_index,
        )));
        self.execute(crate::commands::GroupedCommand::with_after_selection(
            commands,
            [group_index].into_iter().collect(),
        ))
    }

    /// Whether the selection contains a group, the only thing Ungroup can
    /// act on (OF-37). A merged object is one mesh, not a group. The desktop
    /// enables Ungroup and the MCP `ungroup_selected` tool accepts a call
    /// only when this is true.
    #[must_use]
    pub fn selection_contains_group(&self) -> bool {
        self.selection.iter().any(|index| {
            self.objects
                .get(*index)
                .is_some_and(|object| !object.children().is_empty())
        })
    }

    /// Replaces selected groups with their children, composing parent and
    /// child transforms as the Python scene graph does.
    pub fn ungroup_selected(&mut self) -> Result<CommandResult, CoreError> {
        let indices: Vec<usize> = self.selection.iter().rev().copied().collect();
        let mut commands: Vec<Box<dyn Command>> = Vec::new();
        for index in indices {
            let object = self.object(index)?;
            if object.children().is_empty() {
                continue;
            }
            commands.push(Box::new(crate::commands::RemoveObjectCommand::new(index)));
            for (child_index, child) in object.children().iter().enumerate() {
                commands.push(Box::new(crate::commands::AddObjectCommand::at_index(
                    SceneObject::from_child(child, object.transform())?,
                    index + child_index,
                )));
            }
        }
        self.execute(crate::commands::GroupedCommand::with_after_selection(
            commands,
            BTreeSet::new(),
        ))
    }

    /// Replaces one authoritative mesh through an undoable command.
    pub fn replace_mesh(&mut self, index: usize, mesh: Mesh) -> Result<CommandResult, CoreError> {
        self.execute(crate::commands::ReplaceMeshCommand::new(index, mesh))
    }

    /// Simplifies one object through an isolated, undoable mesh replacement.
    /// The object identity, transform, display state, and metadata remain
    /// owned by the existing scene object.
    pub fn apply_simplify(
        &mut self,
        index: usize,
        face_count: Option<usize>,
    ) -> Result<(CommandResult, crate::SimplifyReport), CoreError> {
        let source = self.object(index)?.mesh().clone();
        let result = crate::simplify_mesh(&source, face_count)?;
        let report = result.report.clone();
        let command_result = self.replace_mesh(index, result.mesh)?;
        Ok((command_result, report))
    }

    /// Subdivides one object through an isolated, undoable mesh replacement.
    pub fn apply_subdivide(
        &mut self,
        index: usize,
    ) -> Result<(CommandResult, crate::SubdivideReport), CoreError> {
        let source = self.object(index)?.mesh().clone();
        let result = crate::subdivide_mesh(&source)?;
        let report = result.report.clone();
        let command_result = self.replace_mesh(index, result.mesh)?;
        Ok((command_result, report))
    }

    /// Smooths one object through an isolated, undoable mesh replacement.
    pub fn apply_smooth(
        &mut self,
        index: usize,
        iterations: usize,
    ) -> Result<(CommandResult, crate::SmoothReport), CoreError> {
        let source = self.object(index)?.mesh().clone();
        let result = crate::smooth_mesh(&source, iterations)?;
        let report = result.report.clone();
        let command_result = self.replace_mesh(index, result.mesh)?;
        Ok((command_result, report))
    }

    /// Applies Elastic Grab to one object through one undoable replacement.
    pub fn apply_elastic_grab(
        &mut self,
        index: usize,
        center_mm: [f32; 3],
        offset_mm: [f32; 3],
        options: crate::ElasticGrabOptions,
    ) -> Result<(CommandResult, crate::BrushReport), CoreError> {
        let source = self.object(index)?.mesh().clone();
        let result = crate::elastic_grab(&source, center_mm, offset_mm, options)?;
        let report = result.report.clone();
        let command_result = self.replace_mesh(index, result.mesh)?;
        Ok((command_result, report))
    }

    /// Applies Crease Polish to one object through one undoable replacement.
    pub fn apply_crease_polish(
        &mut self,
        index: usize,
        centers_mm: &[[f32; 3]],
        options: crate::CreasePolishOptions,
    ) -> Result<(CommandResult, crate::BrushReport), CoreError> {
        let source = self.object(index)?.mesh().clone();
        let result = crate::crease_polish(&source, centers_mm, options)?;
        let report = result.report.clone();
        let command_result = self.replace_mesh(index, result.mesh)?;
        Ok((command_result, report))
    }

    /// Applies Smooth Brush to one object through one undoable replacement.
    pub fn apply_smooth_brush(
        &mut self,
        index: usize,
        centers_mm: &[[f32; 3]],
        options: crate::SmoothBrushOptions,
    ) -> Result<(CommandResult, crate::BrushReport), CoreError> {
        let source = self.object(index)?.mesh().clone();
        let result = crate::smooth_brush(&source, centers_mm, options)?;
        let report = result.report.clone();
        let command_result = self.replace_mesh(index, result.mesh)?;
        Ok((command_result, report))
    }

    /// Applies Blob to one object through one undoable replacement.
    pub fn apply_blob(
        &mut self,
        index: usize,
        centers_mm: &[[f32; 3]],
        options: crate::BlobOptions,
    ) -> Result<(CommandResult, crate::BrushReport), CoreError> {
        let source = self.object(index)?.mesh().clone();
        let result = crate::blob(&source, centers_mm, options)?;
        let report = result.report.clone();
        let command_result = self.replace_mesh(index, result.mesh)?;
        Ok((command_result, report))
    }

    /// Applies text embossing through one undoable mesh replacement.
    pub fn apply_text_emboss(
        &mut self,
        index: usize,
        options: crate::TextEmbossOptions,
    ) -> Result<(CommandResult, crate::TextEmbossReport), CoreError> {
        let source = self.object(index)?.mesh().clone();
        let result = crate::text_emboss(&source, options)?;
        let report = result.report.clone();
        let command_result = self.replace_mesh(index, result.mesh)?;
        Ok((command_result, report))
    }

    /// Applies constant-thickness hollowing through one undoable mesh
    /// replacement.
    pub fn apply_hollow(
        &mut self,
        index: usize,
        options: crate::HollowOptions,
    ) -> Result<(CommandResult, crate::HollowReport), CoreError> {
        let source = self.object(index)?.mesh().clone();
        let result = crate::hollow_mesh(&source, options)?;
        let report = result.report.clone();
        let command_result = self.replace_mesh(index, result.mesh)?;
        Ok((command_result, report))
    }

    /// Normalizes one object's selected face density through one undoable replacement.
    pub fn apply_normalize_face_density(
        &mut self,
        index: usize,
        face_indices: Option<&[usize]>,
        centers_mm: Option<&[[f32; 3]]>,
        options: crate::NormalizeDensityOptions,
    ) -> Result<(CommandResult, crate::NormalizeDensityReport), CoreError> {
        let source = self.object(index)?.mesh().clone();
        let result = crate::normalize_face_density(&source, face_indices, centers_mm, options)?;
        let report = result.report.clone();
        let command_result = self.replace_mesh(index, result.mesh)?;
        Ok((command_result, report))
    }

    /// Computes a boolean from isolated world-space snapshots and commits it
    /// as one grouped history entry.  The target ID, name, color, metadata,
    /// visibility, and transform survive; the tool is removed.  A failure
    /// occurs before the grouped command, leaving the scene untouched.
    pub fn apply_boolean(
        &mut self,
        target_index: usize,
        tool_index: usize,
        operation: crate::BooleanOperation,
        options: crate::BooleanOptions,
    ) -> Result<(usize, CommandResult, crate::BooleanReport), CoreError> {
        self.apply_boolean_with_color(target_index, tool_index, operation, options, None)
    }

    /// Applies the product's boolean rules, which the desktop and MCP both
    /// call so the two adapters produce the same objects:
    ///
    /// - A Merge result takes one color, that of whichever operand sits
    ///   higher in the Items list (the lower scene index), regardless of
    ///   which one is the target (OF-9). Subtract and Intersect keep their
    ///   per-face source colors.
    /// - A Merge of objects that are not touching, so that the result would
    ///   still be separate pieces, groups the two objects instead
    ///   (decision D8, OF-38); adapters tell the user with
    ///   [`MERGE_NOT_TOUCHING_MESSAGE`].
    pub fn apply_product_boolean(
        &mut self,
        target_index: usize,
        tool_index: usize,
        operation: crate::BooleanOperation,
        options: crate::BooleanOptions,
    ) -> Result<ProductBooleanOutcome, CoreError> {
        let merge = operation == crate::BooleanOperation::Union;
        let uniform_color = if merge {
            Some(self.object(target_index.min(tool_index))?.color())
        } else {
            None
        };
        self.apply_boolean_inner(
            target_index,
            tool_index,
            operation,
            options,
            uniform_color,
            merge,
        )
    }

    /// [`Self::apply_boolean`] with an optional single result color. With
    /// `Some(color)` the result drops the per-face source colors and is
    /// painted `color` in the same grouped history entry; with `None` the
    /// faces keep their source colors. Adapters normally go through
    /// [`Self::apply_product_boolean`] instead.
    pub fn apply_boolean_with_color(
        &mut self,
        target_index: usize,
        tool_index: usize,
        operation: crate::BooleanOperation,
        options: crate::BooleanOptions,
        uniform_color: Option<[f32; 4]>,
    ) -> Result<(usize, CommandResult, crate::BooleanReport), CoreError> {
        match self.apply_boolean_inner(
            target_index,
            tool_index,
            operation,
            options,
            uniform_color,
            false,
        )? {
            ProductBooleanOutcome::Applied {
                index,
                result,
                report,
            }
            | ProductBooleanOutcome::GroupedApart {
                index,
                result,
                report,
            } => Ok((index, result, report)),
        }
    }

    fn apply_boolean_inner(
        &mut self,
        target_index: usize,
        tool_index: usize,
        operation: crate::BooleanOperation,
        options: crate::BooleanOptions,
        uniform_color: Option<[f32; 4]>,
        group_if_apart: bool,
    ) -> Result<ProductBooleanOutcome, CoreError> {
        if target_index == tool_index {
            return Err(CoreError::MeshOperation(
                "boolean target and tool must be different objects".to_owned(),
            ));
        }
        let target = self.object(target_index)?;
        let tool = self.object(tool_index)?;
        let target_transform = *target.transform();
        let source_colors = [target.color(), tool.color()];
        let operation_key = match operation {
            crate::BooleanOperation::Union => "merge",
            crate::BooleanOperation::Difference => "subtract",
            crate::BooleanOperation::Intersection => "intersect",
        };
        let target_metadata = target.metadata().clone();
        let target_world = target.transformed_mesh()?;
        let tool_world = tool.transformed_mesh()?;
        let result = crate::boolean_operation(&target_world, &tool_world, operation, options)?;
        if group_if_apart && merge_left_pieces_apart(&result.report) {
            self.set_selection([target_index, tool_index]);
            let command_result = self.group_selected()?;
            let index =
                self.selection
                    .iter()
                    .next()
                    .copied()
                    .ok_or_else(|| CoreError::CommandFailed {
                        command: "merge".to_owned(),
                        reason: "grouping the objects left nothing selected".to_owned(),
                    })?;
            return Ok(ProductBooleanOutcome::GroupedApart {
                index,
                result: command_result,
                report: result.report,
            });
        }
        let face_sources = result
            .face_sources
            .iter()
            .map(|source| match source {
                crate::BooleanFaceSource::Tool => 1,
                crate::BooleanFaceSource::Target | crate::BooleanFaceSource::Unknown => 0,
            })
            .collect::<Vec<_>>();
        let tool_face_count = face_sources.iter().filter(|source| **source == 1).count();
        let provenance = BooleanColorProvenance {
            operation: operation_key.to_owned(),
            source_colors,
            face_sources,
            tool_face_count,
        };
        let metadata = if uniform_color.is_some() {
            without_boolean_color_provenance(&target_metadata)
        } else {
            with_boolean_color_provenance(&target_metadata, &provenance)
        };
        let target_local = result.mesh.transformed(&target_transform.inverse()?)?;
        let new_target_index = if target_index < tool_index {
            target_index
        } else {
            target_index.saturating_sub(1)
        };
        let after_selection = [new_target_index].into_iter().collect();
        let mut commands: Vec<Box<dyn crate::Command>> = vec![
            Box::new(crate::ReplaceMeshCommand::new(target_index, target_local)),
            Box::new(crate::SetMetadataCommand::new(target_index, metadata)),
        ];
        if let Some(color) = uniform_color {
            commands.push(Box::new(crate::SetColorCommand::new(target_index, color)));
        }
        commands.push(Box::new(crate::RemoveObjectCommand::new(tool_index)));
        let command_result = self.execute(crate::GroupedCommand::with_after_selection(
            commands,
            after_selection,
        ))?;
        Ok(ProductBooleanOutcome::Applied {
            index: new_target_index,
            result: command_result,
            report: result.report,
        })
    }

    /// Sets one object transform through an undoable command.
    pub fn set_transform(
        &mut self,
        index: usize,
        transform: Transform,
    ) -> Result<CommandResult, CoreError> {
        self.execute(crate::commands::TransformObjectCommand::new(
            index, transform,
        ))
    }

    /// Translates one object through an undoable command.
    pub fn translate(
        &mut self,
        index: usize,
        offset_mm: [f32; 3],
    ) -> Result<CommandResult, CoreError> {
        let transform = self.object(index)?.translated_transform(offset_mm)?;
        self.set_transform(index, transform)
    }

    /// Rotates one object through an undoable command.
    pub fn rotate(
        &mut self,
        index: usize,
        angle_degrees: f32,
        axis: [f32; 3],
    ) -> Result<CommandResult, CoreError> {
        let transform = self.object(index)?.rotated_transform(angle_degrees, axis)?;
        self.set_transform(index, transform)
    }

    /// Scales one object through an undoable command.
    pub fn scale(&mut self, index: usize, scale: [f32; 3]) -> Result<CommandResult, CoreError> {
        let transform = self.object(index)?.scaled_transform(scale)?;
        self.set_transform(index, transform)
    }

    /// Sets one object color through an undoable command.
    pub fn set_color(&mut self, index: usize, color: [f32; 4]) -> Result<CommandResult, CoreError> {
        self.execute(crate::commands::SetColorCommand::new(index, color))
    }

    /// Paints a whole object one colour as one undo entry (Paint at full
    /// coverage). Region paint and boolean source colours are cleared.
    pub fn paint_object(
        &mut self,
        index: usize,
        color: [f32; 4],
    ) -> Result<CommandResult, CoreError> {
        self.execute(crate::PaintFacesCommand::whole_object(index, color))
    }

    /// Paints the listed faces as one undo entry.
    pub fn paint_faces(
        &mut self,
        index: usize,
        faces: Vec<usize>,
        color: [f32; 4],
    ) -> Result<CommandResult, CoreError> {
        self.execute(crate::PaintFacesCommand::faces(index, faces, color))
    }

    /// One Paint click: paints the region around `face` that `coverage`
    /// selects (see [`crate::paint_region_faces`]) as one undo entry.
    pub fn paint_region(
        &mut self,
        index: usize,
        face: usize,
        coverage: f32,
        color: [f32; 4],
    ) -> Result<CommandResult, CoreError> {
        let faces = crate::paint_region_faces(self.object(index)?.mesh(), face, coverage)?;
        if faces.len() == self.object(index)?.mesh().face_count() {
            return self.paint_object(index, color);
        }
        self.paint_faces(index, faces, color)
    }

    /// Sets one object visibility flag through an undoable command.
    pub fn set_visible(&mut self, index: usize, visible: bool) -> Result<CommandResult, CoreError> {
        self.execute(crate::commands::SetVisibilityCommand::new(index, visible))
    }

    pub(crate) fn objects_mut(&mut self) -> &mut Vec<SceneObject> {
        &mut self.objects
    }

    pub(crate) fn insert_selection_index(&mut self, index: usize) {
        for selected in &mut self.selection_order {
            if *selected >= index {
                *selected += 1;
            }
        }
        self.selection = self
            .selection
            .iter()
            .map(|selected| {
                if *selected >= index {
                    *selected + 1
                } else {
                    *selected
                }
            })
            .collect();
    }

    pub(crate) fn remove_selection_index(&mut self, index: usize) {
        self.selection_order.retain(|selected| *selected != index);
        for selected in &mut self.selection_order {
            if *selected > index {
                *selected -= 1;
            }
        }
        self.selection = self
            .selection
            .iter()
            .filter_map(|selected| {
                if *selected == index {
                    None
                } else if *selected > index {
                    Some(*selected - 1)
                } else {
                    Some(*selected)
                }
            })
            .collect();
    }

    pub(crate) fn set_selection_raw(&mut self, selection: BTreeSet<usize>) {
        self.set_selection(selection);
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.source_archive = None;
        self.dirty = true;
    }

    pub(crate) fn set_loaded_archive(&mut self, archive: Vec<u8>) {
        self.source_archive = Some(archive);
        self.dirty = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    pub(crate) fn source_archive(&self) -> Option<&[u8]> {
        if self.dirty {
            None
        } else {
            self.source_archive.as_deref()
        }
    }

    pub(crate) fn set_project_fields(&mut self, project_id: ObjectId, revision: u64) {
        self.project_id = project_id;
        self.revision = revision;
    }
}

/// One undoable replacement produced by a completed isolated operation.
/// Object vectors are swapped between the scene and history entry so the
/// history stores the prior and result states without another mesh clone.
struct ReplaceSceneContentsCommand {
    before_objects: Option<Vec<SceneObject>>,
    after_objects: Option<Vec<SceneObject>>,
    before_selection: Option<BTreeSet<usize>>,
    after_selection: BTreeSet<usize>,
    before_selection_order: Option<Vec<usize>>,
    after_selection_order: Vec<usize>,
    affected_object_ids: Vec<ObjectId>,
}

impl ReplaceSceneContentsCommand {
    fn new(snapshot: SceneSnapshot) -> Self {
        let mut affected = BTreeSet::new();
        affected.extend(
            snapshot
                .objects
                .iter()
                .map(|object| object.object_id().to_owned()),
        );
        let selection = snapshot
            .selection
            .into_iter()
            .filter(|index| *index < snapshot.objects.len())
            .collect::<BTreeSet<_>>();
        let mut selection_order = Vec::new();
        let mut ordered = BTreeSet::new();
        for index in snapshot.selection_order {
            if selection.contains(&index) && ordered.insert(index) {
                selection_order.push(index);
            }
        }
        for index in &selection {
            if ordered.insert(*index) {
                selection_order.push(*index);
            }
        }
        Self {
            before_objects: None,
            after_objects: Some(snapshot.objects),
            before_selection: None,
            after_selection: selection,
            before_selection_order: None,
            after_selection_order: selection_order,
            affected_object_ids: affected.into_iter().collect(),
        }
    }
}

impl Command for ReplaceSceneContentsCommand {
    fn name(&self) -> &str {
        "Commit isolated operation"
    }

    fn execute(&mut self, scene: &mut Scene) -> Result<bool, CoreError> {
        let Some(after_objects) = self.after_objects.take() else {
            return Err(CoreError::CommandFailed {
                command: self.name().to_owned(),
                reason: "the isolated result is no longer available".to_owned(),
            });
        };
        let before_objects = std::mem::replace(&mut scene.objects, after_objects);
        let mut affected = self
            .affected_object_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        affected.extend(
            before_objects
                .iter()
                .map(|object| object.object_id().to_owned()),
        );
        self.affected_object_ids = affected.into_iter().collect();
        self.before_objects = Some(before_objects);
        self.before_selection = Some(std::mem::replace(
            &mut scene.selection,
            self.after_selection.clone(),
        ));
        self.before_selection_order = Some(std::mem::replace(
            &mut scene.selection_order,
            self.after_selection_order.clone(),
        ));
        scene.mark_dirty();
        Ok(true)
    }

    fn undo(&mut self, scene: &mut Scene) -> Result<(), CoreError> {
        if self.before_objects.is_none()
            || self.before_selection.is_none()
            || self.before_selection_order.is_none()
        {
            return Err(CoreError::CommandFailed {
                command: self.name().to_owned(),
                reason: "the prior scene state is incomplete".to_owned(),
            });
        }
        let before_objects =
            self.before_objects
                .take()
                .ok_or_else(|| CoreError::CommandFailed {
                    command: self.name().to_owned(),
                    reason: "the prior scene objects are no longer available".to_owned(),
                })?;
        let before_selection =
            self.before_selection
                .take()
                .ok_or_else(|| CoreError::CommandFailed {
                    command: self.name().to_owned(),
                    reason: "the prior selection is no longer available".to_owned(),
                })?;
        let before_selection_order =
            self.before_selection_order
                .take()
                .ok_or_else(|| CoreError::CommandFailed {
                    command: self.name().to_owned(),
                    reason: "the prior selection order is no longer available".to_owned(),
                })?;
        self.after_objects = Some(std::mem::replace(&mut scene.objects, before_objects));
        self.after_selection = std::mem::replace(&mut scene.selection, before_selection);
        self.after_selection_order =
            std::mem::replace(&mut scene.selection_order, before_selection_order);
        scene.mark_dirty();
        Ok(())
    }

    fn affected_object_ids(&self) -> Vec<ObjectId> {
        self.affected_object_ids.clone()
    }
}

fn new_object_id() -> ObjectId {
    Uuid::new_v4().simple().to_string()
}

fn empty_metadata() -> Value {
    Value::Object(Map::new())
}

fn validate_object_fields(
    object_id: &str,
    name: &str,
    color: [f32; 4],
    metadata: &Value,
) -> Result<(), CoreError> {
    if object_id.trim().is_empty() {
        return Err(CoreError::ObjectId(
            "object ID must not be empty".to_owned(),
        ));
    }
    if name.trim().is_empty() {
        return Err(CoreError::InvalidObject(
            "object name must not be empty".to_owned(),
        ));
    }
    if color
        .iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(CoreError::InvalidColor(
            "RGBA values must be finite and between 0 and 1".to_owned(),
        ));
    }
    if !metadata.is_object() {
        return Err(CoreError::InvalidObject(
            "metadata must be a JSON object".to_owned(),
        ));
    }
    Ok(())
}

fn validate_unique_object_ids(objects: &[SceneObject]) -> Result<(), CoreError> {
    let mut ids = BTreeSet::new();
    for object in objects {
        let mut object_ids = Vec::new();
        object.all_object_ids(&mut object_ids);
        for object_id in object_ids {
            if !ids.insert(object_id.clone()) {
                return Err(CoreError::ObjectId(format!(
                    "duplicate object ID {object_id:?}"
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_scene_objects(objects: &[SceneObject]) -> Result<(), CoreError> {
    validate_unique_object_ids(objects)
}

/// Gap between objects and their copies from Paste or Duplicate, in
/// millimetres (OF-43).
pub const COPY_GAP_MM: f32 = 10.0;

/// Where copies of `objects` go relative to them: along +X, the axis that
/// points screen-right in the opening view, by the objects' combined width
/// plus [`COPY_GAP_MM`], so the copies sit beside the originals with a
/// 10 mm gap. Y and Z are unchanged. The width is the X extent of each
/// object's world-space box (its local box corners, transformed).
fn beside_offset<'a>(objects: impl IntoIterator<Item = &'a SceneObject>) -> [f32; 3] {
    let mut minimum = f32::INFINITY;
    let mut maximum = f32::NEG_INFINITY;
    for object in objects {
        let Some((low, high)) = object.mesh().bounds() else {
            continue;
        };
        for corner in 0..8 {
            let point = [
                if corner & 1 == 0 { low[0] } else { high[0] },
                if corner & 2 == 0 { low[1] } else { high[1] },
                if corner & 4 == 0 { low[2] } else { high[2] },
            ];
            let x = object.transform().transform_point(point)[0];
            if x.is_finite() {
                minimum = minimum.min(x);
                maximum = maximum.max(x);
            }
        }
    }
    let width = if maximum >= minimum {
        maximum - minimum
    } else {
        0.0
    };
    [width + COPY_GAP_MM, 0.0, 0.0]
}

/// Fewest objects Align and Arrange work on (OF-42): both place objects
/// relative to each other, so one object alone has nothing to line up with.
pub const PLACEMENT_MIN_OBJECTS: usize = 2;

/// What the user is told when Align or Arrange has fewer than
/// [`PLACEMENT_MIN_OBJECTS`] objects. The desktop shows it on the disabled
/// buttons; MCP `align` and `arrange` return it as their error.
pub const PLACEMENT_NEEDS_TWO_MESSAGE: &str =
    "Select at least two objects to align or arrange them.";

/// What the user is told when Merge groups objects that are not touching
/// (decision D8, OF-38). The desktop shows it for about four seconds; MCP
/// returns it as the result's `message`.
pub const MERGE_NOT_TOUCHING_MESSAGE: &str =
    "These objects weren't touching, so they were grouped instead of merged.";

/// Result of [`Scene::apply_product_boolean`].
#[derive(Debug)]
#[must_use]
pub enum ProductBooleanOutcome {
    /// The boolean was applied; `index` is the resulting object.
    Applied {
        /// Scene index of the resulting object.
        index: usize,
        /// The history entry.
        result: CommandResult,
        /// Kernel report of the operation.
        report: crate::BooleanReport,
    },
    /// A Merge of objects that were not touching: they were grouped instead
    /// and `index` is the new group.
    GroupedApart {
        /// Scene index of the new group.
        index: usize,
        /// The history entry (the grouping).
        result: CommandResult,
        /// Kernel report of the Merge that was not applied.
        report: crate::BooleanReport,
    },
}

/// Whether a Merge result is still as many separate pieces as its two
/// inputs together, i.e. the objects did not overlap (touching at a point
/// or along a line does not join them either).
fn merge_left_pieces_apart(report: &crate::BooleanReport) -> bool {
    let inputs = report
        .target_before
        .body_count
        .max(1)
        .saturating_add(report.tool_before.body_count.max(1));
    report.result_after.body_count >= inputs
}

#[cfg(test)]
mod rb04_group_limits {
    use super::*;
    #[test]
    fn grouping_beyond_the_project_depth_limit_preserves_scene_and_history() {
        let mut scene = Scene::new();
        scene
            .add_primitive(crate::PrimitiveKind::Cube, None, None)
            .expect("cube");
        for _ in 0..crate::MAX_PROJECT_GROUP_DEPTH {
            scene
                .add_primitive(crate::PrimitiveKind::Cube, None, None)
                .expect("cube");
            scene.set_selection(0..scene.objects().len());
            scene.group_selected().expect("bounded group");
        }
        scene
            .add_primitive(crate::PrimitiveKind::Cube, None, None)
            .expect("cube");
        scene.set_selection([0, 1]);
        let before = scene.history_state_id();
        let ids = scene
            .objects()
            .iter()
            .map(|o| o.object_id().to_owned())
            .collect::<Vec<_>>();
        assert!(matches!(
            scene.group_selected(),
            Err(CoreError::OperationResourceLimit { .. })
        ));
        assert_eq!(scene.history_state_id(), before);
        assert_eq!(
            scene
                .objects()
                .iter()
                .map(|o| o.object_id().to_owned())
                .collect::<Vec<_>>(),
            ids
        );
    }
}

#[cfg(test)]
mod rb04_selection_order {
    use super::*;
    #[test]
    fn removing_an_earlier_unselected_row_preserves_anchor_identity_and_undo_order() {
        let mut scene = Scene::new();
        for _ in 0..4 {
            scene
                .add_object(SceneObject::new(crate::make_box([2.; 3]).expect("box")))
                .expect("add");
        }
        scene.clear_selection();
        scene.set_selection([3, 1, 2]);
        let ids = scene
            .ordered_selection()
            .iter()
            .map(|i| scene.objects()[*i].object_id().to_owned())
            .collect::<Vec<_>>();
        scene.remove_object(0).expect("remove");
        assert_eq!(scene.ordered_selection(), [2, 0, 1]);
        assert_eq!(
            scene
                .ordered_selection()
                .iter()
                .map(|i| scene.objects()[*i].object_id().to_owned())
                .collect::<Vec<_>>(),
            ids
        );
        scene.undo().expect("undo");
        assert_eq!(scene.ordered_selection(), [3, 1, 2]);
        scene.redo().expect("redo");
        assert_eq!(scene.ordered_selection(), [2, 0, 1]);
    }
}
