//! Bounded, adapter-neutral mesh operations.
//!
//! The Python reference uses `manifold3d` for booleans and trimesh for the
//! remaining mesh filters.  Rust uses the `manifold-csg` bindings for the
//! boolean kernel and keeps the other operations in this crate so the same
//! validated mesh contract is shared by desktop, MCP, and future background
//! workers.  All coordinates remain millimetres.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use manifold_csg::{Manifold, MeshGL64, MeshGL64Options};

use crate::{CoreError, Mesh};

/// A hard upper bound used by the default mesh-operation policy.
pub const DEFAULT_MAX_OPERATION_VERTICES: usize = 2_000_000;
/// A hard upper bound used by the default mesh-operation policy.
pub const DEFAULT_MAX_OPERATION_FACES: usize = 4_000_000;
/// Maximum estimated working memory for one isolated mesh operation.
pub const DEFAULT_MAX_OPERATION_MEMORY_BYTES: u64 = 512 * 1024 * 1024;
/// Maximum counted topology/geometry work units for one operation.
pub const DEFAULT_MAX_OPERATION_WORK_UNITS: u64 = 20_000_000;
/// Maximum accepted input face count for a boolean pair under the default
/// complexity policy.
pub const DEFAULT_MAX_BOOLEAN_INPUT_FACES: usize = 1_000_000;

/// Limits applied before and after an isolated mesh operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeshOperationLimits {
    /// Maximum authoritative vertex count in an operation result.
    pub max_vertices: usize,
    /// Maximum authoritative triangle count in an operation result.
    pub max_faces: usize,
    /// Maximum estimated simultaneous working memory.
    pub max_memory_bytes: u64,
    /// Maximum counted topology/geometry work units.
    pub max_work_units: u64,
    /// Maximum combined input triangles for a boolean pair.
    pub max_boolean_input_faces: usize,
}

impl Default for MeshOperationLimits {
    fn default() -> Self {
        Self {
            max_vertices: DEFAULT_MAX_OPERATION_VERTICES,
            max_faces: DEFAULT_MAX_OPERATION_FACES,
            max_memory_bytes: DEFAULT_MAX_OPERATION_MEMORY_BYTES,
            max_work_units: DEFAULT_MAX_OPERATION_WORK_UNITS,
            max_boolean_input_faces: DEFAULT_MAX_BOOLEAN_INPUT_FACES,
        }
    }
}

impl MeshOperationLimits {
    pub(crate) fn validate(&self, operation: &str) -> Result<(), CoreError> {
        if self.max_vertices == 0
            || self.max_faces == 0
            || self.max_memory_bytes == 0
            || self.max_work_units == 0
            || self.max_boolean_input_faces == 0
        {
            return Err(CoreError::OperationResourceLimit {
                operation: operation.to_owned(),
                reason: "operation limits must all be positive".to_owned(),
            });
        }
        Ok(())
    }

    pub(crate) fn check_mesh(&self, operation: &str, mesh: &Mesh) -> Result<u64, CoreError> {
        if mesh.vertex_count() > self.max_vertices {
            return Err(CoreError::OperationResourceLimit {
                operation: operation.to_owned(),
                reason: format!(
                    "result has {} vertices, above the {} vertex limit",
                    mesh.vertex_count(),
                    self.max_vertices
                ),
            });
        }
        if mesh.face_count() > self.max_faces {
            return Err(CoreError::OperationResourceLimit {
                operation: operation.to_owned(),
                reason: format!(
                    "result has {} faces, above the {} face limit",
                    mesh.face_count(),
                    self.max_faces
                ),
            });
        }
        let bytes = estimated_mesh_bytes(mesh.vertex_count(), mesh.face_count())?;
        if bytes > self.max_memory_bytes {
            return Err(CoreError::OperationResourceLimit {
                operation: operation.to_owned(),
                reason: format!(
                    "mesh requires {bytes} estimated bytes, above the {} byte limit",
                    self.max_memory_bytes
                ),
            });
        }
        Ok(bytes)
    }

    pub(crate) fn check_work(&self, operation: &str, work_units: u64) -> Result<(), CoreError> {
        if work_units > self.max_work_units {
            return Err(CoreError::OperationResourceLimit {
                operation: operation.to_owned(),
                reason: format!(
                    "operation requires {work_units} work units, above the {} unit limit",
                    self.max_work_units
                ),
            });
        }
        Ok(())
    }
}

/// Counts and topology facts used in operation reports and repair decisions.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshHealthReport {
    /// Whether the mesh has no detected health issues.
    pub valid: bool,
    /// Number of indexed vertices.
    pub vertices: usize,
    /// Number of indexed triangles.
    pub triangles: usize,
    /// Whether every undirected edge has exactly two incident faces.
    pub watertight: bool,
    /// Whether shared edges have opposite winding directions.
    pub winding_consistent: bool,
    /// Number of connected face components, or -1 when there are no faces.
    pub body_count: i32,
    /// Shells that face inward, enclosing a hollow cavity inside another
    /// shell (Hollow's inner surface, OF-54). They are counted in
    /// `body_count` but are not reported as disconnected pieces.
    pub cavity_shells: i32,
    /// Whether the mesh encloses a non-zero signed volume.
    pub is_volume: bool,
    /// Number of zero-area or non-finite triangles.
    pub degenerate_faces: usize,
    /// Number of duplicate triangles ignoring winding.
    pub duplicate_faces: usize,
    /// Number of undirected edges with more than two incident faces.
    pub non_manifold_edges: usize,
    /// Number of boundary edges.
    pub boundary_edges: usize,
    /// Number of connected boundary loops.
    pub boundary_edge_loops: usize,
    /// Number of disconnected components below the repair tiny-shell scale.
    pub tiny_shells: usize,
    /// Self-intersection testing is deliberately explicit because no optional
    /// accelerator is part of the portable core.
    pub self_intersections: Option<usize>,
    /// Explains why self-intersections were not reported.
    pub self_intersection_status: String,
    /// Human-readable issues in the reference report order.
    pub issues: Vec<String>,
}

impl MeshHealthReport {
    /// Returns whether this report describes a closed, consistently wound
    /// triangle mesh without local topology defects.
    #[must_use]
    pub const fn is_closed_manifold(&self) -> bool {
        self.valid
            && self.watertight
            && self.winding_consistent
            && self.degenerate_faces == 0
            && self.non_manifold_edges == 0
            && self.duplicate_faces == 0
    }
}

/// Boolean operation supported by the manifold3d-compatible kernel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanOperation {
    /// Keep the volume of both operands.
    Union,
    /// Remove the tool volume from the target volume.
    Difference,
    /// Keep only the overlapping volume.
    Intersection,
}

impl BooleanOperation {
    fn name(self) -> &'static str {
        match self {
            Self::Union => "boolean_union",
            Self::Difference => "boolean_difference",
            Self::Intersection => "boolean_intersection",
        }
    }
}

/// Optional behavior for a boolean operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BooleanOptions {
    /// Re-triangulate eligible flat tool-origin cap patches while preserving
    /// their boundary edges, matching the Python cut-cap regularization.
    pub regularize_cut_caps: bool,
    /// Resource and complexity policy for this isolated computation.
    pub limits: MeshOperationLimits,
}

impl Default for BooleanOptions {
    fn default() -> Self {
        Self {
            regularize_cut_caps: true,
            limits: MeshOperationLimits::default(),
        }
    }
}

/// Source label for one result triangle produced by a boolean.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BooleanFaceSource {
    /// The face came from the target operand.
    Target,
    /// The face came from the tool operand.
    Tool,
    /// The kernel did not provide a source label for this face.
    Unknown,
}

/// Measured result of a boolean operation.
#[derive(Clone, Debug, PartialEq)]
pub struct BooleanResult {
    /// The new mesh in the coordinate system of the supplied operands.
    pub mesh: Mesh,
    /// One source label per result triangle.
    pub face_sources: Vec<BooleanFaceSource>,
    /// Health and resource evidence for the operation.
    pub report: BooleanReport,
}

/// Boolean operation evidence suitable for desktop/MCP diagnostics.
#[derive(Clone, Debug, PartialEq)]
pub struct BooleanReport {
    /// Operation identifier.
    pub operation: String,
    /// Target health before the operation.
    pub target_before: MeshHealthReport,
    /// Tool health before the operation.
    pub tool_before: MeshHealthReport,
    /// Result health after the operation.
    pub result_after: MeshHealthReport,
    /// Signed target volume in cubic millimetres before the operation.
    pub target_volume_mm3: f64,
    /// Signed tool volume in cubic millimetres before the operation.
    pub tool_volume_mm3: f64,
    /// Signed result volume in cubic millimetres after the operation.
    pub result_volume_mm3: f64,
    /// Estimated peak mesh bytes charged to the operation.
    pub estimated_peak_bytes: u64,
    /// Counted work units charged before the operation.
    pub work_units: u64,
    /// Number of cap patches regularized after the kernel result.
    pub regularized_cap_patches: usize,
}

type CapReplacement = (Vec<[u32; 3]>, Vec<BooleanFaceSource>, [f32; 3], u32);

/// Validates a mesh and returns the portable health report used by R3.
pub fn inspect_mesh(mesh: &Mesh) -> Result<MeshHealthReport, CoreError> {
    mesh.validate_non_empty()?;
    let vertices = &mesh.vertices;
    let faces = &mesh.faces;
    let mut issues = Vec::new();
    let mut degenerate_faces = 0_usize;
    let mut canonical_faces: BTreeMap<[u32; 3], usize> = BTreeMap::new();
    let mut edges: BTreeMap<(u32, u32), Vec<(usize, bool)>> = BTreeMap::new();

    for (face_index, face) in faces.iter().copied().enumerate() {
        let Some(first) = vertices.get(face[0] as usize) else {
            return Err(CoreError::InvalidMesh(format!(
                "face {face_index} references a missing vertex"
            )));
        };
        let Some(second) = vertices.get(face[1] as usize) else {
            return Err(CoreError::InvalidMesh(format!(
                "face {face_index} references a missing vertex"
            )));
        };
        let Some(third) = vertices.get(face[2] as usize) else {
            return Err(CoreError::InvalidMesh(format!(
                "face {face_index} references a missing vertex"
            )));
        };
        let cross = cross_product(subtract(*second, *first), subtract(*third, *first));
        let area_measure = dot(cross, cross);
        if !area_measure.is_finite() || area_measure <= 1.0e-24 {
            degenerate_faces = degenerate_faces.saturating_add(1);
        }
        let mut canonical = face;
        canonical.sort_unstable();
        *canonical_faces.entry(canonical).or_default() += 1;
        for (first_index, second_index) in
            [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])]
        {
            let key = ordered_edge(first_index, second_index);
            let forward = first_index <= second_index;
            edges.entry(key).or_default().push((face_index, forward));
        }
    }

    let duplicate_faces = canonical_faces
        .values()
        .map(|count| count.saturating_sub(1))
        .sum();
    if duplicate_faces > 0 {
        issues.push(format!("{duplicate_faces} duplicate faces"));
    }
    if degenerate_faces > 0 {
        issues.push(format!("{degenerate_faces} degenerate faces"));
    }

    let mut boundary_edges = 0_usize;
    let mut non_manifold_edges = 0_usize;
    let mut winding_consistent = true;
    let mut boundary_adjacency: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (&(first, second), owners) in &edges {
        match owners.len() {
            1 => {
                boundary_edges = boundary_edges.saturating_add(1);
                boundary_adjacency.entry(first).or_default().insert(second);
                boundary_adjacency.entry(second).or_default().insert(first);
            }
            2 => {
                if owners[0].1 == owners[1].1 {
                    winding_consistent = false;
                }
            }
            _ => {
                non_manifold_edges = non_manifold_edges.saturating_add(1);
                winding_consistent = false;
            }
        }
    }
    if boundary_edges > 0 {
        let loops = count_boundary_loops(&boundary_adjacency);
        issues.push(format!("{loops} boundary edge loops"));
    }
    if non_manifold_edges > 0 {
        issues.push(format!("{non_manifold_edges} non-manifold edges"));
    }
    if !winding_consistent {
        issues.push("winding inconsistent".to_owned());
    }

    let body_count = connected_face_components(faces, &edges) as i32;
    let tiny_shells = count_tiny_components(mesh, faces, &edges, 0.01);
    let cavity_shells = if body_count > 1 && winding_consistent {
        count_cavity_shells(mesh, faces, &edges)
    } else {
        0
    };
    let pieces = body_count - cavity_shells;
    if pieces > 1 {
        issues.push(format!("{pieces} disconnected shells"));
    }
    if tiny_shells > 0 {
        issues.push(format!("{tiny_shells} tiny shells"));
    }
    let signed_volume = signed_volume_mm3(mesh);
    let watertight = boundary_edges == 0 && non_manifold_edges == 0 && !faces.is_empty();
    let is_volume = watertight && winding_consistent && signed_volume.abs() > 1.0e-12;
    if !watertight {
        issues.push("not watertight".to_owned());
    }
    if !is_volume {
        issues.push("does not enclose volume".to_owned());
    }
    if !mesh
        .vertices
        .iter()
        .flatten()
        .all(|value| value.is_finite())
    {
        issues.push("non-finite vertex coordinates".to_owned());
    }

    Ok(MeshHealthReport {
        valid: issues.is_empty(),
        vertices: mesh.vertex_count(),
        triangles: mesh.face_count(),
        watertight,
        winding_consistent,
        body_count,
        cavity_shells,
        is_volume,
        degenerate_faces,
        duplicate_faces,
        non_manifold_edges,
        boundary_edges,
        boundary_edge_loops: if boundary_edges == 0 {
            0
        } else {
            count_boundary_loops(&boundary_adjacency)
        },
        tiny_shells,
        self_intersections: None,
        self_intersection_status: "not checked: optional self-intersection accelerator unavailable"
            .to_owned(),
        issues,
    })
}

/// Performs one bounded boolean and returns source-labelled result faces.
pub fn boolean_operation(
    target: &Mesh,
    tool: &Mesh,
    operation: BooleanOperation,
    options: BooleanOptions,
) -> Result<BooleanResult, CoreError> {
    let operation_name = operation.name();
    options.limits.validate(operation_name)?;
    target.validate_non_empty()?;
    tool.validate_non_empty()?;
    options
        .limits
        .check_mesh(operation_name, target)
        .map(|_| ())?;
    options
        .limits
        .check_mesh(operation_name, tool)
        .map(|_| ())?;
    let input_faces = target
        .face_count()
        .checked_add(tool.face_count())
        .ok_or_else(|| CoreError::MeshOperation("boolean input face count overflow".to_owned()))?;
    if input_faces > options.limits.max_boolean_input_faces {
        return Err(CoreError::OperationResourceLimit {
            operation: operation_name.to_owned(),
            reason: format!(
                "combined input has {input_faces} faces, above the {} face limit",
                options.limits.max_boolean_input_faces
            ),
        });
    }
    let target_health = inspect_mesh(target)?;
    let tool_health = inspect_mesh(tool)?;
    let target_operand =
        prepare_boolean_operand(target, &target_health, "target", &options.limits)?;
    let tool_operand = prepare_boolean_operand(tool, &tool_health, "tool", &options.limits)?;
    options
        .limits
        .check_mesh(operation_name, &target_operand)
        .map(|_| ())?;
    options
        .limits
        .check_mesh(operation_name, &tool_operand)
        .map(|_| ())?;
    let input_faces = target_operand
        .face_count()
        .checked_add(tool_operand.face_count())
        .ok_or_else(|| CoreError::MeshOperation("boolean input face count overflow".to_owned()))?;
    if input_faces > options.limits.max_boolean_input_faces {
        return Err(CoreError::OperationResourceLimit {
            operation: operation_name.to_owned(),
            reason: format!(
                "combined repaired input has {input_faces} faces, above the {} face limit",
                options.limits.max_boolean_input_faces
            ),
        });
    }
    let work_units = (input_faces as u64)
        .checked_mul(8)
        .ok_or_else(|| CoreError::MeshOperation("boolean work count overflow".to_owned()))?;
    options.limits.check_work(operation_name, work_units)?;
    let input_bytes =
        estimated_mesh_bytes(target_operand.vertex_count(), target_operand.face_count())?
            .checked_add(estimated_mesh_bytes(
                tool_operand.vertex_count(),
                tool_operand.face_count(),
            )?)
            .ok_or_else(|| {
                CoreError::MeshOperation("boolean memory estimate overflow".to_owned())
            })?;
    if input_bytes > options.limits.max_memory_bytes {
        return Err(CoreError::OperationResourceLimit {
            operation: operation_name.to_owned(),
            reason: format!(
                "input meshes require {input_bytes} estimated bytes, above the {} byte limit",
                options.limits.max_memory_bytes
            ),
        });
    }

    let target_manifold = manifold_from_mesh(&target_operand, 0)?;
    let tool_manifold = manifold_from_mesh(&tool_operand, 1)?;
    let result_manifold = match operation {
        BooleanOperation::Union => target_manifold.union(&tool_manifold),
        BooleanOperation::Difference => target_manifold.difference(&tool_manifold),
        BooleanOperation::Intersection => target_manifold.intersection(&tool_manifold),
    };
    result_manifold.status().map_err(|error| {
        CoreError::MeshOperation(format!("manifold kernel rejected result: {error}"))
    })?;
    if result_manifold.is_empty() {
        return Err(CoreError::MeshOperation(
            "boolean result is empty; check operand order and overlap".to_owned(),
        ));
    }
    let (mut result_mesh, mut face_sources) = mesh_from_manifold(&result_manifold)?;
    let mut regularized_cap_patches = 0_usize;
    if options.regularize_cut_caps && operation == BooleanOperation::Difference {
        let regularized = regularize_tool_caps(&result_mesh, &face_sources)?;
        result_mesh = regularized.0;
        face_sources = regularized.1;
        regularized_cap_patches = regularized.2;
    }
    let output_bytes = options.limits.check_mesh(operation_name, &result_mesh)?;
    let peak_bytes = input_bytes.checked_add(output_bytes).ok_or_else(|| {
        CoreError::MeshOperation("boolean peak memory estimate overflow".to_owned())
    })?;
    if peak_bytes > options.limits.max_memory_bytes {
        return Err(CoreError::OperationResourceLimit {
            operation: operation_name.to_owned(),
            reason: format!(
                "input plus result requires {peak_bytes} estimated bytes, above the {} byte limit",
                options.limits.max_memory_bytes
            ),
        });
    }
    let result_after = inspect_mesh(&result_mesh)?;
    let result_volume_mm3 = signed_volume_mm3(&result_mesh);
    Ok(BooleanResult {
        mesh: result_mesh,
        face_sources,
        report: BooleanReport {
            operation: operation_name.to_owned(),
            target_volume_mm3: signed_volume_mm3(&target_operand),
            tool_volume_mm3: signed_volume_mm3(&tool_operand),
            result_volume_mm3,
            target_before: target_health,
            tool_before: tool_health,
            result_after,
            estimated_peak_bytes: peak_bytes,
            work_units,
            regularized_cap_patches,
        },
    })
}

/// Boolean union using the default bounded policy.
pub fn boolean_union(target: &Mesh, tool: &Mesh) -> Result<Mesh, CoreError> {
    Ok(boolean_operation(
        target,
        tool,
        BooleanOperation::Union,
        BooleanOptions::default(),
    )?
    .mesh)
}

/// Boolean difference using the default bounded policy.
pub fn boolean_difference(target: &Mesh, tool: &Mesh) -> Result<Mesh, CoreError> {
    Ok(boolean_operation(
        target,
        tool,
        BooleanOperation::Difference,
        BooleanOptions::default(),
    )?
    .mesh)
}

/// Boolean intersection using the default bounded policy.
pub fn boolean_intersection(target: &Mesh, tool: &Mesh) -> Result<Mesh, CoreError> {
    Ok(boolean_operation(
        target,
        tool,
        BooleanOperation::Intersection,
        BooleanOptions::default(),
    )?
    .mesh)
}

fn prepare_boolean_operand(
    mesh: &Mesh,
    health: &MeshHealthReport,
    label: &str,
    limits: &MeshOperationLimits,
) -> Result<Mesh, CoreError> {
    if health.is_closed_manifold() {
        return Ok(outward_facing(mesh.clone()));
    }
    let repaired = crate::repair_mesh_with_options(mesh, crate::RepairProfile::Extended, *limits)?;
    if repaired.report.after.is_closed_manifold() {
        return Ok(outward_facing(repaired.mesh));
    }
    Err(CoreError::MeshOperation(format!(
        "{label} operand must be watertight with consistent winding after repair: {}",
        repaired.report.after.issues.join("; ")
    )))
}

/// Turns a closed, consistently wound operand whose faces point inwards
/// (negative enclosed volume) the right way out. The kernel reads an
/// inside-out operand as empty space, so a Merge with it would keep every
/// face of both objects and remove nothing.
fn outward_facing(mut mesh: Mesh) -> Mesh {
    if signed_volume_mm3(&mesh) < 0.0 {
        for face in &mut mesh.faces {
            face.swap(1, 2);
        }
    }
    mesh
}

pub(crate) fn manifold_from_mesh(mesh: &Mesh, source: u32) -> Result<Manifold, CoreError> {
    let mut vertices = Vec::with_capacity(mesh.vertex_count().saturating_mul(3));
    for vertex in &mesh.vertices {
        for value in vertex {
            if !value.is_finite() {
                return Err(CoreError::InvalidMesh(
                    "boolean input contains a non-finite coordinate".to_owned(),
                ));
            }
            vertices.push(f64::from(*value));
        }
    }
    let mut indices = Vec::with_capacity(mesh.face_count().saturating_mul(3));
    for face in &mesh.faces {
        indices.extend(face.iter().map(|index| u64::from(*index)));
    }
    let end = u64::try_from(indices.len())
        .map_err(|_| CoreError::MeshOperation("boolean index count overflow".to_owned()))?;
    let run_indices = [0_u64, end];
    let source_ids = [source];
    let meshgl = MeshGL64::new_with_options(
        &vertices,
        3,
        &indices,
        MeshGL64Options::new().runs(&run_indices, &source_ids),
    )
    .map_err(|error| CoreError::MeshOperation(format!("invalid manifold input: {error}")))?;
    Manifold::from_meshgl64(&meshgl)
        .map_err(|error| CoreError::MeshOperation(format!("manifold construction failed: {error}")))
}

pub(crate) fn mesh_from_manifold(
    manifold: &Manifold,
) -> Result<(Mesh, Vec<BooleanFaceSource>), CoreError> {
    let (properties, property_count, indices) = manifold.to_mesh_f64();
    if property_count < 3 || properties.len() % property_count != 0 || indices.len() % 3 != 0 {
        return Err(CoreError::MeshOperation(
            "manifold returned malformed mesh buffers".to_owned(),
        ));
    }
    let vertex_count = properties.len() / property_count;
    let mut vertices = Vec::with_capacity(vertex_count);
    for chunk in properties.chunks_exact(property_count) {
        let values = [chunk[0], chunk[1], chunk[2]];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(CoreError::MeshOperation(
                "manifold returned non-finite vertex coordinates".to_owned(),
            ));
        }
        let mut vertex = [0.0_f32; 3];
        for (slot, value) in vertex.iter_mut().zip(values) {
            if value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
                return Err(CoreError::MeshOperation(
                    "manifold result exceeds the mesh float32 coordinate range".to_owned(),
                ));
            }
            *slot = value as f32;
        }
        vertices.push(vertex);
    }
    let mut faces = Vec::with_capacity(indices.len() / 3);
    for chunk in indices.as_chunks::<3>().0 {
        faces.push([
            u32::try_from(chunk[0]).map_err(|_| {
                CoreError::MeshOperation("manifold result has too many vertices".to_owned())
            })?,
            u32::try_from(chunk[1]).map_err(|_| {
                CoreError::MeshOperation("manifold result has too many vertices".to_owned())
            })?,
            u32::try_from(chunk[2]).map_err(|_| {
                CoreError::MeshOperation("manifold result has too many vertices".to_owned())
            })?,
        ]);
    }
    let mesh = Mesh::new(vertices, faces)?;
    let meshgl = manifold.to_meshgl64();
    let run_indices = meshgl.run_index();
    let run_sources = meshgl.run_original_id();
    let mut sources = vec![BooleanFaceSource::Unknown; mesh.face_count()];
    if !run_sources.is_empty()
        && (run_indices.len() == run_sources.len()
            || run_indices.len() == run_sources.len().saturating_add(1))
    {
        for (run_index, source_id) in run_sources.iter().copied().enumerate() {
            let Some(start_flat) = run_indices.get(run_index).copied() else {
                continue;
            };
            let end_flat = run_indices
                .get(run_index.saturating_add(1))
                .copied()
                .unwrap_or((mesh.face_count().saturating_mul(3)) as u64);
            let start = usize::try_from(start_flat / 3).unwrap_or(mesh.face_count());
            let end = usize::try_from(end_flat / 3)
                .unwrap_or(mesh.face_count())
                .min(mesh.face_count());
            let source = match source_id {
                0 => BooleanFaceSource::Target,
                1 => BooleanFaceSource::Tool,
                _ => BooleanFaceSource::Unknown,
            };
            for slot in sources.iter_mut().take(end).skip(start) {
                *slot = source;
            }
        }
    }
    if sources
        .iter()
        .all(|source| *source == BooleanFaceSource::Unknown)
    {
        let face_ids = meshgl.face_id();
        if face_ids.len() == mesh.face_count() && face_ids.iter().all(|id| *id <= 1) {
            for (slot, id) in sources.iter_mut().zip(face_ids) {
                *slot = if id == 0 {
                    BooleanFaceSource::Target
                } else {
                    BooleanFaceSource::Tool
                };
            }
        }
    }
    Ok((mesh, sources))
}

fn regularize_tool_caps(
    mesh: &Mesh,
    sources: &[BooleanFaceSource],
) -> Result<(Mesh, Vec<BooleanFaceSource>, usize), CoreError> {
    if sources.len() != mesh.face_count() {
        return Err(CoreError::MeshOperation(
            "boolean face-source count does not match result faces".to_owned(),
        ));
    }
    // Source labels are filled by `boolean_operation` from run metadata when
    // the kernel exposes them.  Without labels there is no safe way to tell a
    // cutter cap from a target surface, so leave the result unchanged.
    if !sources.contains(&BooleanFaceSource::Tool) {
        return Ok((mesh.clone(), sources.to_vec(), 0));
    }
    let mut edge_owners: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
    let mut edge_directions: BTreeMap<(u32, u32), (u32, u32)> = BTreeMap::new();
    for (face_index, face) in mesh.faces.iter().copied().enumerate() {
        for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let key = ordered_edge(first, second);
            edge_owners.entry(key).or_default().push(face_index);
            edge_directions.entry(key).or_insert((first, second));
        }
    }
    let normals: Vec<[f32; 3]> = mesh
        .faces
        .iter()
        .copied()
        .map(|face| mesh.face_normal(face))
        .collect();
    let mut plane_offsets = vec![0.0_f32; mesh.face_count()];
    for (face_index, face) in mesh.faces.iter().copied().enumerate() {
        if let Some(vertex) = mesh.vertices.get(face[0] as usize) {
            plane_offsets[face_index] = dot(normals[face_index], *vertex);
        }
    }
    let mut face_neighbors = vec![Vec::<usize>::new(); mesh.face_count()];
    for owners in edge_owners.values() {
        for first_index in 0..owners.len() {
            for second_index in (first_index + 1)..owners.len() {
                let first = owners[first_index];
                let second = owners[second_index];
                if sources.get(first) != Some(&BooleanFaceSource::Tool)
                    || sources.get(second) != Some(&BooleanFaceSource::Tool)
                    || dot(normals[first], normals[second]) < 0.999_999
                    || (plane_offsets[first] - plane_offsets[second]).abs() > 0.002
                {
                    continue;
                }
                if let Some(neighbors) = face_neighbors.get_mut(first) {
                    neighbors.push(second);
                }
                if let Some(neighbors) = face_neighbors.get_mut(second) {
                    neighbors.push(first);
                }
            }
        }
    }
    let mut visited = vec![false; mesh.face_count()];
    let mut replacements: BTreeMap<usize, CapReplacement> = BTreeMap::new();
    let mut removed = BTreeSet::new();
    for start in 0..mesh.face_count() {
        if visited[start] || sources[start] != BooleanFaceSource::Tool {
            continue;
        }
        let mut component = Vec::new();
        visited[start] = true;
        let mut queue = VecDeque::from([start]);
        while let Some(face_index) = queue.pop_front() {
            component.push(face_index);
            if let Some(neighbors) = face_neighbors.get(face_index) {
                for neighbor in neighbors {
                    if let Some(was_visited) = visited.get_mut(*neighbor)
                        && !*was_visited
                    {
                        *was_visited = true;
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
        if component.len() < 2 {
            continue;
        }
        let Some(&anchor) = component.first() else {
            continue;
        };
        let normal = normals[anchor];
        let component_set: BTreeSet<usize> = component.iter().copied().collect();
        let mut boundary_counts: BTreeMap<(u32, u32), usize> = BTreeMap::new();
        let mut boundary_directions: BTreeMap<(u32, u32), (u32, u32)> = BTreeMap::new();
        for face_index in &component {
            let Some(face) = mesh.faces.get(*face_index).copied() else {
                continue;
            };
            for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
                let key = ordered_edge(first, second);
                *boundary_counts.entry(key).or_default() += 1;
                boundary_directions.entry(key).or_insert((first, second));
            }
        }
        let boundary_edges: Vec<(u32, u32)> = boundary_counts
            .iter()
            .filter_map(|(edge, count)| (*count == 1).then_some(*edge))
            .collect();
        if boundary_edges.len() < 3 {
            continue;
        }
        if boundary_edges.iter().any(|edge| {
            edge_owners.get(edge).is_some_and(|owners| {
                owners
                    .iter()
                    .filter(|owner| component_set.contains(owner))
                    .count()
                    != 1
            })
        }) {
            continue;
        }
        let mut outgoing = BTreeMap::<u32, u32>::new();
        for edge in &boundary_edges {
            let Some(&(first, second)) = boundary_directions.get(edge) else {
                continue;
            };
            if outgoing.insert(first, second).is_some() {
                outgoing.clear();
                break;
            }
        }
        if outgoing.len() != boundary_edges.len() {
            continue;
        }
        let Some(&begin) = outgoing.keys().next() else {
            continue;
        };
        let mut boundary = Vec::with_capacity(outgoing.len());
        let mut current = begin;
        let mut valid_loop = true;
        for _ in 0..outgoing.len() {
            if boundary.contains(&current) {
                valid_loop = false;
                break;
            }
            boundary.push(current);
            let Some(next) = outgoing.get(&current).copied() else {
                valid_loop = false;
                break;
            };
            current = next;
        }
        if !valid_loop || current != begin || boundary.len() != boundary_edges.len() {
            continue;
        }
        let mut center = [0.0_f32; 3];
        for vertex_index in &boundary {
            let Some(vertex) = mesh.vertices.get(*vertex_index as usize) else {
                valid_loop = false;
                break;
            };
            for axis in 0..3 {
                center[axis] += vertex[axis] / boundary.len() as f32;
            }
        }
        if !valid_loop || center.iter().any(|value| !value.is_finite()) {
            continue;
        }
        let mut turn_sign = 0.0_f32;
        for index in 0..boundary.len() {
            let previous = boundary[(index + boundary.len() - 1) % boundary.len()];
            let current_vertex = boundary[index];
            let next = boundary[(index + 1) % boundary.len()];
            let (Some(previous), Some(current_vertex), Some(next)) = (
                mesh.vertices.get(previous as usize),
                mesh.vertices.get(current_vertex as usize),
                mesh.vertices.get(next as usize),
            ) else {
                valid_loop = false;
                break;
            };
            let turn = dot(
                cross_product(
                    subtract(*current_vertex, *previous),
                    subtract(*next, *current_vertex),
                ),
                normal,
            );
            if turn.abs() <= 1.0e-6 {
                continue;
            }
            if turn_sign == 0.0 {
                turn_sign = turn.signum();
            } else if turn * turn_sign < 0.0 {
                valid_loop = false;
                break;
            }
        }
        if !valid_loop || turn_sign == 0.0 {
            continue;
        }
        let placeholder =
            u32::try_from(mesh.vertices.len() + replacements.len()).map_err(|_| {
                CoreError::MeshOperation("boolean cap vertex count overflow".to_owned())
            })?;
        let mut patch_faces = Vec::with_capacity(boundary.len());
        for index in 0..boundary.len() {
            let first = boundary[index];
            let second = boundary[(index + 1) % boundary.len()];
            let (Some(first_point), Some(second_point)) = (
                mesh.vertices.get(first as usize),
                mesh.vertices.get(second as usize),
            ) else {
                valid_loop = false;
                break;
            };
            let cross = cross_product(
                subtract(*second_point, *first_point),
                subtract(center, *first_point),
            );
            if dot(cross, cross) <= 1.0e-20 {
                valid_loop = false;
                break;
            }
            let face = if dot(cross, normal) < 0.0 {
                [second, first, placeholder]
            } else {
                [first, second, placeholder]
            };
            patch_faces.push(face);
        }
        if !valid_loop || patch_faces.len() != boundary.len() {
            continue;
        }
        for face_index in component {
            removed.insert(face_index);
        }
        replacements.insert(
            anchor,
            (
                patch_faces,
                vec![BooleanFaceSource::Tool; boundary.len()],
                center,
                placeholder,
            ),
        );
    }
    if replacements.is_empty() {
        return Ok((mesh.clone(), sources.to_vec(), 0));
    }
    let original_vertex_count = mesh.vertices.len() as u32;
    let mut vertices = mesh.vertices.clone();
    let mut faces = Vec::new();
    let mut output_sources = Vec::new();
    for (face_index, face) in mesh.faces.iter().copied().enumerate() {
        if let Some((replacement, replacement_sources, center, placeholder)) =
            replacements.get(&face_index)
        {
            let center_index = u32::try_from(vertices.len()).map_err(|_| {
                CoreError::MeshOperation("boolean cap vertex count overflow".to_owned())
            })?;
            vertices.push(*center);
            for (patch_face, source) in replacement.iter().zip(replacement_sources) {
                let mut patch_face = *patch_face;
                for index in &mut patch_face {
                    if *index == *placeholder || *index >= original_vertex_count {
                        *index = center_index;
                    }
                }
                faces.push(patch_face);
                output_sources.push(*source);
            }
            continue;
        }
        if removed.contains(&face_index) {
            continue;
        }
        faces.push(face);
        output_sources.push(sources[face_index]);
    }
    Ok((
        Mesh::new(vertices, faces)?,
        output_sources,
        replacements.len(),
    ))
}

fn estimated_mesh_bytes(vertices: usize, faces: usize) -> Result<u64, CoreError> {
    let vertex_bytes = u64::try_from(vertices)
        .ok()
        .and_then(|count| count.checked_mul(12))
        .ok_or_else(|| CoreError::MeshOperation("vertex memory estimate overflow".to_owned()))?;
    let face_bytes = u64::try_from(faces)
        .ok()
        .and_then(|count| count.checked_mul(12))
        .ok_or_else(|| CoreError::MeshOperation("face memory estimate overflow".to_owned()))?;
    vertex_bytes
        .checked_add(face_bytes)
        .ok_or_else(|| CoreError::MeshOperation("mesh memory estimate overflow".to_owned()))
}

fn ordered_edge(first: u32, second: u32) -> (u32, u32) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

fn count_boundary_loops(adjacency: &BTreeMap<u32, BTreeSet<u32>>) -> usize {
    let mut unseen: BTreeSet<u32> = adjacency.keys().copied().collect();
    let mut components = 0_usize;
    while let Some(start) = unseen.pop_first() {
        components = components.saturating_add(1);
        let mut queue = VecDeque::from([start]);
        while let Some(vertex) = queue.pop_front() {
            if let Some(neighbors) = adjacency.get(&vertex) {
                for neighbor in neighbors {
                    if unseen.remove(neighbor) {
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
    }
    components
}

fn connected_face_components(
    faces: &[[u32; 3]],
    edges: &BTreeMap<(u32, u32), Vec<(usize, bool)>>,
) -> usize {
    if faces.is_empty() {
        return 0;
    }
    let mut adjacency = vec![Vec::<usize>::new(); faces.len()];
    for owners in edges.values() {
        for (first_index, &(first_face, _)) in owners.iter().enumerate() {
            for &(second_face, _) in owners.iter().skip(first_index + 1) {
                adjacency[first_face].push(second_face);
                adjacency[second_face].push(first_face);
            }
        }
    }
    let mut seen = vec![false; faces.len()];
    let mut components = 0_usize;
    for start in 0..faces.len() {
        if seen[start] {
            continue;
        }
        components = components.saturating_add(1);
        seen[start] = true;
        let mut queue = VecDeque::from([start]);
        while let Some(face) = queue.pop_front() {
            if let Some(neighbors) = adjacency.get(face) {
                for neighbor in neighbors {
                    if let Some(was_seen) = seen.get_mut(*neighbor)
                        && !*was_seen
                    {
                        *was_seen = true;
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
    }
    components
}

/// Number of closed shells whose signed volume is negative (they face
/// inward), when at least one shell faces outward.
fn count_cavity_shells(
    mesh: &Mesh,
    faces: &[[u32; 3]],
    edges: &BTreeMap<(u32, u32), Vec<(usize, bool)>>,
) -> i32 {
    let mut label = vec![usize::MAX; faces.len()];
    let mut adjacency = vec![Vec::<usize>::new(); faces.len()];
    for owners in edges.values() {
        for (first_index, &(first_face, _)) in owners.iter().enumerate() {
            for &(second_face, _) in owners.iter().skip(first_index + 1) {
                adjacency[first_face].push(second_face);
                adjacency[second_face].push(first_face);
            }
        }
    }
    let mut volumes = Vec::<f64>::new();
    for start in 0..faces.len() {
        if label[start] != usize::MAX {
            continue;
        }
        let component = volumes.len();
        volumes.push(0.0);
        label[start] = component;
        let mut queue = VecDeque::from([start]);
        while let Some(face) = queue.pop_front() {
            let corners = faces[face].map(|index| mesh.vertices.get(index as usize).copied());
            if let [Some(a), Some(b), Some(c)] = corners {
                let [a, b, c] = [a, b, c].map(|v| v.map(f64::from));
                volumes[component] += (a[0] * (b[1] * c[2] - b[2] * c[1])
                    + a[1] * (b[2] * c[0] - b[0] * c[2])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0;
            }
            for neighbor in &adjacency[face] {
                if label[*neighbor] == usize::MAX {
                    label[*neighbor] = component;
                    queue.push_back(*neighbor);
                }
            }
        }
    }
    let outward = volumes.iter().filter(|volume| **volume > 0.0).count();
    if outward == 0 {
        return 0;
    }
    i32::try_from(volumes.iter().filter(|volume| **volume < 0.0).count()).unwrap_or(0)
}

fn count_tiny_components(
    mesh: &Mesh,
    faces: &[[u32; 3]],
    edges: &BTreeMap<(u32, u32), Vec<(usize, bool)>>,
    threshold: f32,
) -> usize {
    if faces.is_empty() {
        return 0;
    }
    let mut adjacency = vec![Vec::<usize>::new(); faces.len()];
    for owners in edges.values() {
        for first_index in 0..owners.len() {
            for second_index in (first_index + 1)..owners.len() {
                let first = owners[first_index].0;
                let second = owners[second_index].0;
                if let Some(neighbors) = adjacency.get_mut(first) {
                    neighbors.push(second);
                }
                if let Some(neighbors) = adjacency.get_mut(second) {
                    neighbors.push(first);
                }
            }
        }
    }
    let mut seen = vec![false; faces.len()];
    let mut tiny = 0_usize;
    for start in 0..faces.len() {
        if seen[start] {
            continue;
        }
        let mut component_faces = Vec::new();
        seen[start] = true;
        let mut queue = VecDeque::from([start]);
        while let Some(face_index) = queue.pop_front() {
            component_faces.push(face_index);
            if let Some(neighbors) = adjacency.get(face_index) {
                for neighbor in neighbors {
                    if let Some(was_seen) = seen.get_mut(*neighbor)
                        && !*was_seen
                    {
                        *was_seen = true;
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
        let mut minimum = [f32::INFINITY; 3];
        let mut maximum = [f32::NEG_INFINITY; 3];
        for face_index in component_faces {
            if let Some(face) = faces.get(face_index) {
                for index in face {
                    if let Some(vertex) = mesh.vertices.get(*index as usize) {
                        for axis in 0..3 {
                            minimum[axis] = minimum[axis].min(vertex[axis]);
                            maximum[axis] = maximum[axis].max(vertex[axis]);
                        }
                    }
                }
            }
        }
        let scale = (0..3)
            .map(|axis| maximum[axis] - minimum[axis])
            .fold(0.0_f32, f32::max);
        if scale.is_finite() && scale <= threshold {
            tiny = tiny.saturating_add(1);
        }
    }
    tiny
}

fn signed_volume_mm3(mesh: &Mesh) -> f64 {
    mesh.faces
        .iter()
        .filter_map(|face| {
            let first = mesh.vertices.get(face[0] as usize)?;
            let second = mesh.vertices.get(face[1] as usize)?;
            let third = mesh.vertices.get(face[2] as usize)?;
            let a = [
                f64::from(first[0]),
                f64::from(first[1]),
                f64::from(first[2]),
            ];
            let b = [
                f64::from(second[0]),
                f64::from(second[1]),
                f64::from(second[2]),
            ];
            let c = [
                f64::from(third[0]),
                f64::from(third[1]),
                f64::from(third[2]),
            ];
            Some(dot64(a, cross64(b, c)) / 6.0)
        })
        .sum()
}

fn subtract(first: [f32; 3], second: [f32; 3]) -> [f32; 3] {
    [
        first[0] - second[0],
        first[1] - second[1],
        first[2] - second[2],
    ]
}

fn cross_product(first: [f32; 3], second: [f32; 3]) -> [f32; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn dot(vector: [f32; 3], other: [f32; 3]) -> f32 {
    vector[0] * other[0] + vector[1] * other[1] + vector[2] * other[2]
}

fn cross64(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn dot64(vector: [f64; 3], other: [f64; 3]) -> f64 {
    vector[0] * other[0] + vector[1] * other[1] + vector[2] * other[2]
}
