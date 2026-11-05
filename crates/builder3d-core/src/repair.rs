//! Bounded mesh repair derived from the Python repair profiles.
//!
//! Repair is deliberately non-mutating. It records the ordered filter
//! stages, keeps a structurally safe best-effort result when a mesh remains
//! open, and only claims a valid manifold when inspect_mesh can verify it.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{CoreError, Mesh, MeshHealthReport, MeshOperationLimits, inspect_mesh};

/// The repair tolerance used by the Python reference, in millimetres.
pub const REPAIR_TOLERANCE_MM: f32 = 0.001;
/// The maximum component scale considered a disposable tiny shell.
pub const REPAIR_TINY_SHELL_TOLERANCE_MM: f32 = 0.01;
/// The default repair profile.
pub const REPAIR_DEFAULT_PROFILE: RepairProfile = RepairProfile::Extended;

/// Ordered repair profile selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairProfile {
    /// Winding, trivial/all hole closure, and export cleanup.
    Simple,
    /// The enhanced profile uses the bounded NMR-compatible cleanup family.
    Enhanced,
    /// The complete captured repair pipeline.
    Extended,
}

impl RepairProfile {
    /// Parses the Python profile names without accepting an ambiguous value.
    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "simple" => Ok(Self::Simple),
            "enhanced" => Ok(Self::Enhanced),
            "extended" => Ok(Self::Extended),
            other => Err(CoreError::MeshOperation(format!(
                "Unknown repair profile: {other}. Choose: simple, enhanced, extended"
            ))),
        }
    }

    /// Returns the stable profile identifier used in reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Simple => "simple",
            Self::Enhanced => "enhanced",
            Self::Extended => "extended",
        }
    }
}

/// One captured repair-stage transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepairStageReport {
    /// Reference-stage name.
    pub name: String,
    /// Vertex count before the stage.
    pub before_vertices: usize,
    /// Triangle count before the stage.
    pub before_triangles: usize,
    /// Vertex count after the stage.
    pub after_vertices: usize,
    /// Triangle count after the stage.
    pub after_triangles: usize,
    /// Whether the indexed counts changed.
    pub changed: bool,
    /// Recoverable stage error, if a best-effort stage was skipped.
    pub error: Option<String>,
}

/// Detail for one planar cleanup candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarCleanupDetail {
    /// Number of candidate faces before replacement.
    pub faces_before: usize,
    /// Number of emitted faces after replacement.
    pub faces_after: usize,
    /// Number of boundary vertices in the candidate patch.
    pub boundary_vertices: usize,
    /// Number of duplicate faces removed with the patch.
    pub faces_removed: usize,
}

/// Bounded planar-cleanup evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlanarCleanupReport {
    /// Whether at least one eligible planar region was inspected/rebuilt.
    pub changed: bool,
    /// Number of eligible patches.
    pub patches: usize,
    /// Total candidate faces before cleanup.
    pub faces_before: usize,
    /// Total emitted candidate faces after cleanup.
    pub faces_after: usize,
    /// Per-patch evidence.
    pub details: Vec<PlanarCleanupDetail>,
}

/// Health and commit-safety evidence for a repair attempt.
#[derive(Clone, Debug, PartialEq)]
pub struct RepairReport {
    /// Profile identifier.
    pub profile: String,
    /// Weld/degenerate tolerance in millimetres.
    pub tolerance_mm: f32,
    /// Tiny-shell scale threshold in millimetres.
    pub tiny_shell_tolerance_mm: f32,
    /// Health before the first stage.
    pub before: MeshHealthReport,
    /// Ordered stage evidence.
    pub stages: Vec<RepairStageReport>,
    /// Planar cleanup evidence.
    pub planar_cleanup: PlanarCleanupReport,
    /// Health after all stages.
    pub after: MeshHealthReport,
    /// Recoverable stage errors.
    pub errors: Vec<String>,
    /// Whether the candidate has usable finite indexed geometry.
    pub candidate_safe: bool,
    /// Whether indexed geometry changed.
    pub candidate_changed: bool,
    /// Whether a caller may commit the candidate as a best-effort result.
    pub can_apply: bool,
}

/// Non-mutating repair result containing the candidate mesh and report.
#[derive(Clone, Debug, PartialEq)]
pub struct RepairResult {
    /// Repaired or best-effort candidate mesh.
    pub mesh: Mesh,
    /// Repair evidence.
    pub report: RepairReport,
}

/// Repairs a mesh with the default extended profile and default limits.
pub fn repair_mesh(mesh: &Mesh) -> Result<RepairResult, CoreError> {
    repair_mesh_with_options(mesh, REPAIR_DEFAULT_PROFILE, MeshOperationLimits::default())
}

/// Repairs a mesh using a named Python-compatible profile.
pub fn repair_mesh_with_profile(mesh: &Mesh, profile: &str) -> Result<RepairResult, CoreError> {
    let profile = RepairProfile::parse(profile)?;
    repair_mesh_with_options(mesh, profile, MeshOperationLimits::default())
}

/// Repairs a mesh with explicit profile and resource limits.
pub fn repair_mesh_with_options(
    mesh: &Mesh,
    profile: RepairProfile,
    limits: MeshOperationLimits,
) -> Result<RepairResult, CoreError> {
    mesh.validate()?;
    limits.validate("repair")?;
    limits.check_mesh("repair", mesh)?;
    if mesh.is_empty() {
        return Err(CoreError::MeshOperation(
            "repair requires a non-empty triangle mesh".to_owned(),
        ));
    }

    let before = inspect_mesh(mesh)?;
    let mut current = mesh.clone();
    let mut stages = Vec::new();
    let mut errors = Vec::new();
    let pipeline = stage_names(profile);
    for name in pipeline {
        let operation = |value: &Mesh| -> Result<Mesh, CoreError> {
            let result = match name {
                "StitchGaps (insert vertices)" | "StitchGaps (reuse vertices)" => {
                    weld_vertices(value, REPAIR_TOLERANCE_MM)
                }
                "CloseTrivialHoles" | "CloseTrivialHoles (second pass)" | "CloseAllHoles" => {
                    close_small_holes(value)
                }
                "RemoveDoubleTriangles" => remove_duplicate_faces(value),
                "FixFlippedTriangles" | "FixFlippedTriangles (second pass)" => fix_winding(value),
                "Wrap" => wrap_mesh(value),
                "RemoveDegeneratedFaces" => remove_degenerate_faces(value, REPAIR_TOLERANCE_MM),
                "RemoveTinyShells" => remove_tiny_shells(value),
                "PrepareForExport" => prepare_for_export(value, REPAIR_TOLERANCE_MM),
                _ => Err(CoreError::MeshOperation(format!(
                    "unknown internal repair stage: {name}"
                ))),
            }?;
            limits.check_mesh("repair", &result)?;
            Ok(result)
        };
        current = run_stage(current, name, operation, &mut stages, &mut errors);
    }

    let (planar_mesh, planar_cleanup) = regularize_planar_regions(&current, &limits)?;
    limits.check_mesh("repair", &planar_mesh)?;
    current = planar_mesh;
    let after = inspect_mesh(&current)?;
    let candidate_safe = is_safe_candidate(&current);
    let candidate_changed = current != *mesh;
    Ok(RepairResult {
        mesh: current,
        report: RepairReport {
            profile: profile.as_str().to_owned(),
            tolerance_mm: REPAIR_TOLERANCE_MM,
            tiny_shell_tolerance_mm: REPAIR_TINY_SHELL_TOLERANCE_MM,
            before,
            stages,
            planar_cleanup,
            after,
            errors,
            candidate_safe,
            candidate_changed,
            can_apply: candidate_safe,
        },
    })
}

fn stage_names(profile: RepairProfile) -> Vec<&'static str> {
    match profile {
        RepairProfile::Simple => vec![
            "FixFlippedTriangles",
            "CloseTrivialHoles",
            "CloseAllHoles",
            "PrepareForExport",
        ],
        RepairProfile::Enhanced | RepairProfile::Extended => vec![
            "StitchGaps (insert vertices)",
            "CloseTrivialHoles",
            "RemoveDoubleTriangles",
            "FixFlippedTriangles",
            "CloseTrivialHoles (second pass)",
            "StitchGaps (reuse vertices)",
            "FixFlippedTriangles (second pass)",
            "CloseAllHoles",
            "Wrap",
            "RemoveDegeneratedFaces",
            "RemoveTinyShells",
            "PrepareForExport",
        ],
    }
}

fn run_stage<F>(
    current: Mesh,
    name: &str,
    operation: F,
    stages: &mut Vec<RepairStageReport>,
    errors: &mut Vec<String>,
) -> Mesh
where
    F: FnOnce(&Mesh) -> Result<Mesh, CoreError>,
{
    let before_vertices = current.vertex_count();
    let before_triangles = current.face_count();
    let result = operation(&current);
    let (next, error) = match result {
        Ok(mesh) => (mesh, None),
        Err(error) => {
            let text = error.to_string();
            errors.push(format!("{name}: {text}"));
            (current.clone(), Some(text))
        }
    };
    let after_vertices = next.vertex_count();
    let after_triangles = next.face_count();
    stages.push(RepairStageReport {
        name: name.to_owned(),
        before_vertices,
        before_triangles,
        after_vertices,
        after_triangles,
        changed: before_vertices != after_vertices || before_triangles != after_triangles,
        error,
    });
    next
}

fn weld_vertices(mesh: &Mesh, tolerance: f32) -> Result<Mesh, CoreError> {
    if mesh.vertices.is_empty() {
        return Ok(mesh.clone());
    }
    let scale = f64::from(tolerance);
    let mut keys = Vec::with_capacity(mesh.vertices.len());
    for vertex in &mesh.vertices {
        let mut key = [0_i64; 3];
        for axis in 0..3 {
            let scaled = f64::from(vertex[axis]) / scale;
            if !scaled.is_finite() || scaled.abs() > i64::MAX as f64 {
                return Ok(mesh.clone());
            }
            key[axis] = scaled.round() as i64;
        }
        keys.push(key);
    }
    let mut representatives = BTreeMap::<[i64; 3], u32>::new();
    let mut remap = Vec::with_capacity(mesh.vertices.len());
    let mut vertices = Vec::new();
    for (index, key) in keys.into_iter().enumerate() {
        let source = match representatives.get(&key).copied() {
            Some(value) => value,
            None => {
                let value = u32::try_from(vertices.len()).map_err(|_| {
                    CoreError::OperationResourceLimit {
                        operation: "repair".to_owned(),
                        reason: "welded vertex index overflow".to_owned(),
                    }
                })?;
                representatives.insert(key, value);
                vertices.push(mesh.vertices[index]);
                value
            }
        };
        remap.push(source);
    }
    let faces = mesh
        .faces
        .iter()
        .map(|face| {
            [
                remap[face[0] as usize],
                remap[face[1] as usize],
                remap[face[2] as usize],
            ]
        })
        .collect();
    let welded = Mesh::new(vertices, faces)?;
    remove_duplicate_faces(&welded)
}

fn remove_duplicate_faces(mesh: &Mesh) -> Result<Mesh, CoreError> {
    let mut seen = BTreeSet::<[u32; 3]>::new();
    let mut faces = Vec::with_capacity(mesh.face_count());
    for face in &mesh.faces {
        let mut key = *face;
        key.sort_unstable();
        if seen.insert(key) {
            faces.push(*face);
        }
    }
    compact_mesh(&mesh.vertices, &faces)
}

fn remove_degenerate_faces(mesh: &Mesh, tolerance: f32) -> Result<Mesh, CoreError> {
    let threshold = f64::from(tolerance) * f64::from(tolerance);
    let mut faces = Vec::with_capacity(mesh.face_count());
    for face in &mesh.faces {
        let Some(first) = mesh.vertices.get(face[0] as usize) else {
            continue;
        };
        let Some(second) = mesh.vertices.get(face[1] as usize) else {
            continue;
        };
        let Some(third) = mesh.vertices.get(face[2] as usize) else {
            continue;
        };
        let cross = cross(subtract(*second, *first), subtract(*third, *first));
        if f64::from(dot(cross, cross)) > threshold {
            faces.push(*face);
        }
    }
    compact_mesh(&mesh.vertices, &faces)
}

fn fix_winding(mesh: &Mesh) -> Result<Mesh, CoreError> {
    if mesh.faces.is_empty() {
        return Ok(mesh.clone());
    }
    let mut edge_owners = BTreeMap::<(u32, u32), Vec<(usize, bool)>>::new();
    for (face_index, face) in mesh.faces.iter().copied().enumerate() {
        for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            edge_owners
                .entry(ordered_edge(first, second))
                .or_default()
                .push((face_index, first <= second));
        }
    }
    let mut adjacency = vec![Vec::<(usize, bool, bool)>::new(); mesh.face_count()];
    for owners in edge_owners.values() {
        for first_index in 0..owners.len() {
            for second_index in (first_index + 1)..owners.len() {
                let (first_face, first_direction) = owners[first_index];
                let (second_face, second_direction) = owners[second_index];
                adjacency[first_face].push((second_face, first_direction, second_direction));
                adjacency[second_face].push((first_face, second_direction, first_direction));
            }
        }
    }
    let mut orientation = vec![None::<bool>; mesh.face_count()];
    let mut components = Vec::<Vec<usize>>::new();
    for start in 0..mesh.face_count() {
        if orientation[start].is_some() {
            continue;
        }
        orientation[start] = Some(true);
        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        while let Some(face_index) = queue.pop_front() {
            component.push(face_index);
            let current_orientation = orientation[face_index].unwrap_or(true);
            for &(neighbor, current_direction, neighbor_direction) in &adjacency[face_index] {
                let current_final = if current_orientation {
                    current_direction
                } else {
                    !current_direction
                };
                let wanted = !current_final;
                let neighbor_orientation = neighbor_direction == wanted;
                if orientation[neighbor].is_none() {
                    orientation[neighbor] = Some(neighbor_orientation);
                    queue.push_back(neighbor);
                }
            }
        }
        components.push(component);
    }

    let mut faces = mesh.faces.clone();
    for (face_index, state) in orientation.iter().enumerate() {
        if state == &Some(false)
            && let Some(face) = faces.get_mut(face_index)
        {
            *face = [face[0], face[2], face[1]];
        }
    }
    for component in components {
        let volume = signed_volume_for_faces(&mesh.vertices, &faces, &component);
        if volume < -1.0e-12 {
            for face_index in component {
                if let Some(face) = faces.get_mut(face_index) {
                    *face = [face[0], face[2], face[1]];
                }
            }
        }
    }
    Mesh::new(mesh.vertices.clone(), faces)
}

fn close_small_holes(mesh: &Mesh) -> Result<Mesh, CoreError> {
    let loops = boundary_loops(mesh);
    if loops.is_empty() {
        return Ok(mesh.clone());
    }
    let mut vertices = mesh.vertices.clone();
    let mut faces = mesh.faces.clone();
    for boundary in loops {
        if !(3..=4).contains(&boundary.len()) {
            continue;
        }
        let mut center = [0.0_f32; 3];
        let mut valid = true;
        for index in &boundary {
            let Some(vertex) = vertices.get(*index as usize) else {
                valid = false;
                break;
            };
            for axis in 0..3 {
                center[axis] += vertex[axis] / boundary.len() as f32;
            }
        }
        if !valid || center.iter().any(|value| !value.is_finite()) {
            continue;
        }
        let center_index =
            u32::try_from(vertices.len()).map_err(|_| CoreError::OperationResourceLimit {
                operation: "repair".to_owned(),
                reason: "hole-center vertex index overflow".to_owned(),
            })?;
        let mut cap = Vec::with_capacity(boundary.len());
        for index in 0..boundary.len() {
            let first = boundary[index];
            let second = boundary[(index + 1) % boundary.len()];
            let (Some(first_point), Some(second_point)) =
                (vertices.get(first as usize), vertices.get(second as usize))
            else {
                valid = false;
                break;
            };
            let cross_value = cross(
                subtract(*second_point, *first_point),
                subtract(center, *first_point),
            );
            if dot(cross_value, cross_value) <= 1.0e-20 {
                valid = false;
                break;
            }
            cap.push([first, second, center_index]);
        }
        if valid {
            vertices.push(center);
            faces.extend(cap);
        }
    }
    Mesh::new(vertices, faces)
}

fn wrap_mesh(mesh: &Mesh) -> Result<Mesh, CoreError> {
    let compacted = compact_mesh(&mesh.vertices, &mesh.faces)?;
    fix_winding(&compacted)
}

fn remove_tiny_shells(mesh: &Mesh) -> Result<Mesh, CoreError> {
    let components = face_components(mesh);
    if components.len() < 2 {
        return Ok(mesh.clone());
    }
    let mut scales = Vec::with_capacity(components.len());
    for component in &components {
        let mut minimum = [f32::INFINITY; 3];
        let mut maximum = [f32::NEG_INFINITY; 3];
        for face_index in component {
            let Some(face) = mesh.faces.get(*face_index) else {
                continue;
            };
            for vertex_index in face {
                let Some(vertex) = mesh.vertices.get(*vertex_index as usize) else {
                    continue;
                };
                for axis in 0..3 {
                    minimum[axis] = minimum[axis].min(vertex[axis]);
                    maximum[axis] = maximum[axis].max(vertex[axis]);
                }
            }
        }
        let scale = (0..3)
            .map(|axis| maximum[axis] - minimum[axis])
            .fold(0.0_f32, f32::max);
        scales.push(scale);
    }
    let mut keep = vec![false; components.len()];
    for (index, scale) in scales.iter().copied().enumerate() {
        keep[index] = scale.is_finite() && scale > REPAIR_TINY_SHELL_TOLERANCE_MM;
    }
    if !keep.iter().any(|value| *value)
        && let Some((largest, _)) = scales
            .iter()
            .copied()
            .enumerate()
            .max_by(|(_, first), (_, second)| first.total_cmp(second))
    {
        keep[largest] = true;
    }
    let faces = components
        .iter()
        .enumerate()
        .filter(|(index, _)| keep[*index])
        .flat_map(|(_, faces)| faces.iter().filter_map(|index| mesh.faces.get(*index)))
        .copied()
        .collect::<Vec<_>>();
    compact_mesh(&mesh.vertices, &faces)
}

fn prepare_for_export(mesh: &Mesh, tolerance: f32) -> Result<Mesh, CoreError> {
    let cleaned = remove_degenerate_faces(mesh, tolerance)?;
    let cleaned = remove_duplicate_faces(&cleaned)?;
    let cleaned = compact_mesh(&cleaned.vertices, &cleaned.faces)?;
    fix_winding(&cleaned)
}

fn regularize_planar_regions(
    mesh: &Mesh,
    limits: &MeshOperationLimits,
) -> Result<(Mesh, PlanarCleanupReport), CoreError> {
    let mut report = PlanarCleanupReport {
        changed: false,
        patches: 0,
        faces_before: 0,
        faces_after: 0,
        details: Vec::new(),
    };
    if mesh.face_count() < 4 || mesh.face_count() > 500_000 {
        return Ok((mesh.clone(), report));
    }
    let components = planar_components(mesh);
    let mut vertices = mesh.vertices.clone();
    let mut replacements = BTreeMap::<usize, Vec<[u32; 3]>>::new();
    let mut removed = BTreeSet::<usize>::new();
    for component in components {
        if component.len() < 4 {
            continue;
        }
        let Some(boundary) = component_boundary_loop(mesh, &component) else {
            continue;
        };
        if boundary.len() < 3 || !is_convex_boundary(mesh, &boundary) {
            continue;
        }
        let Some(normal) = component_normal(mesh, &component) else {
            continue;
        };
        report.patches = report.patches.saturating_add(1);
        report.faces_before = report.faces_before.saturating_add(component.len());
        let faces_after = if component.len() < boundary.len().saturating_mul(2) {
            component.len().saturating_add(2)
        } else {
            component.len()
        };
        report.faces_after = report.faces_after.saturating_add(faces_after);
        report.details.push(PlanarCleanupDetail {
            faces_before: component.len(),
            faces_after,
            boundary_vertices: boundary.len(),
            faces_removed: 0,
        });
        if faces_after == component.len() {
            // A dense planar grid is already at the bounded density; retain
            // its exact boundary/topology while recording the candidate.
            report.changed = true;
            continue;
        }
        let Some(face_index) = component.first().copied() else {
            continue;
        };
        let Some(face) = mesh.faces.get(face_index).copied() else {
            continue;
        };
        let (Some(first), Some(second), Some(third)) = (
            mesh.vertices.get(face[0] as usize),
            mesh.vertices.get(face[1] as usize),
            mesh.vertices.get(face[2] as usize),
        ) else {
            continue;
        };
        let interior = [
            (first[0] + second[0] + third[0]) / 3.0,
            (first[1] + second[1] + third[1]) / 3.0,
            (first[2] + second[2] + third[2]) / 3.0,
        ];
        let interior_index =
            u32::try_from(vertices.len()).map_err(|_| CoreError::OperationResourceLimit {
                operation: "repair".to_owned(),
                reason: "planar cleanup vertex index overflow".to_owned(),
            })?;
        let mut replacement = component
            .iter()
            .filter_map(|index| mesh.faces.get(*index).copied())
            .collect::<Vec<_>>();
        if let Some(position) = component.iter().position(|index| *index == face_index) {
            replacement.remove(position);
        }
        let candidates = [
            [face[0], face[1], interior_index],
            [face[1], face[2], interior_index],
            [face[2], face[0], interior_index],
        ];
        let mut oriented = Vec::with_capacity(3);
        for candidate in candidates {
            let Some(a) = point_for_index(mesh, &vertices, candidate[0]) else {
                oriented.clear();
                break;
            };
            let Some(b) = point_for_index(mesh, &vertices, candidate[1]) else {
                oriented.clear();
                break;
            };
            let Some(c) = (candidate[2] == interior_index)
                .then_some(interior)
                .or_else(|| point_for_index(mesh, &vertices, candidate[2]))
            else {
                oriented.clear();
                break;
            };
            let mut generated = candidate;
            if dot(cross(subtract(b, a), subtract(c, a)), normal) < 0.0 {
                generated = [candidate[0], candidate[2], candidate[1]];
            }
            oriented.push(generated);
        }
        if oriented.len() != 3 {
            continue;
        }
        replacement.extend(oriented);
        vertices.push(interior);
        replacements.insert(face_index, replacement);
        removed.extend(component);
        report.changed = true;
        limits.check_mesh("repair", &Mesh::new(vertices.clone(), mesh.faces.clone())?)?;
    }
    if replacements.is_empty() {
        return Ok((mesh.clone(), report));
    }
    let mut faces = Vec::new();
    for (index, face) in mesh.faces.iter().copied().enumerate() {
        if let Some(replacement) = replacements.get(&index) {
            faces.extend(replacement);
        } else if !removed.contains(&index) {
            faces.push(face);
        }
    }
    Ok((Mesh::new(vertices, faces)?, report))
}

fn compact_mesh(vertices: &[[f32; 3]], faces: &[[u32; 3]]) -> Result<Mesh, CoreError> {
    let mut remap = vec![None::<u32>; vertices.len()];
    let mut compact_vertices = Vec::new();
    for face in faces {
        for index in face {
            let slot = remap.get_mut(*index as usize).ok_or_else(|| {
                CoreError::InvalidMesh("repair face references a missing vertex".to_owned())
            })?;
            if slot.is_none() {
                let new_index = u32::try_from(compact_vertices.len()).map_err(|_| {
                    CoreError::OperationResourceLimit {
                        operation: "repair".to_owned(),
                        reason: "compact vertex index overflow".to_owned(),
                    }
                })?;
                *slot = Some(new_index);
                compact_vertices.push(vertices[*index as usize]);
            }
        }
    }
    let mut compact_faces = Vec::with_capacity(faces.len());
    for face in faces {
        let first = remap[face[0] as usize].ok_or_else(|| {
            CoreError::MeshOperation("repair lost a face vertex while compacting".to_owned())
        })?;
        let second = remap[face[1] as usize].ok_or_else(|| {
            CoreError::MeshOperation("repair lost a face vertex while compacting".to_owned())
        })?;
        let third = remap[face[2] as usize].ok_or_else(|| {
            CoreError::MeshOperation("repair lost a face vertex while compacting".to_owned())
        })?;
        compact_faces.push([first, second, third]);
    }
    Mesh::new(compact_vertices, compact_faces)
}

fn edge_map(mesh: &Mesh) -> BTreeMap<(u32, u32), Vec<(usize, bool)>> {
    let mut edges = BTreeMap::new();
    for (face_index, face) in mesh.faces.iter().copied().enumerate() {
        for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            edges
                .entry(ordered_edge(first, second))
                .or_insert_with(Vec::new)
                .push((face_index, first <= second));
        }
    }
    edges
}

fn face_components(mesh: &Mesh) -> Vec<Vec<usize>> {
    let edges = edge_map(mesh);
    let mut adjacency = vec![Vec::<usize>::new(); mesh.face_count()];
    for owners in edges.values() {
        for first in 0..owners.len() {
            for second in (first + 1)..owners.len() {
                adjacency[owners[first].0].push(owners[second].0);
                adjacency[owners[second].0].push(owners[first].0);
            }
        }
    }
    let mut seen = vec![false; mesh.face_count()];
    let mut result = Vec::new();
    for start in 0..mesh.face_count() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        while let Some(face) = queue.pop_front() {
            component.push(face);
            for neighbor in &adjacency[face] {
                if !seen[*neighbor] {
                    seen[*neighbor] = true;
                    queue.push_back(*neighbor);
                }
            }
        }
        result.push(component);
    }
    result
}

fn planar_components(mesh: &Mesh) -> Vec<Vec<usize>> {
    let edges = edge_map(mesh);
    let normals = mesh
        .faces
        .iter()
        .copied()
        .map(|face| mesh.face_normal(face))
        .collect::<Vec<_>>();
    let offsets = mesh
        .faces
        .iter()
        .copied()
        .enumerate()
        .map(|(index, face)| {
            mesh.vertices
                .get(face[0] as usize)
                .map(|vertex| dot(normals[index], *vertex))
                .unwrap_or(f32::NAN)
        })
        .collect::<Vec<_>>();
    let mut adjacency = vec![Vec::<usize>::new(); mesh.face_count()];
    for owners in edges.values() {
        for first in 0..owners.len() {
            for second in (first + 1)..owners.len() {
                let first_face = owners[first].0;
                let second_face = owners[second].0;
                if dot(normals[first_face], normals[second_face]) < 0.999_999
                    || (offsets[first_face] - offsets[second_face]).abs() > 0.002
                {
                    continue;
                }
                adjacency[first_face].push(second_face);
                adjacency[second_face].push(first_face);
            }
        }
    }
    let mut seen = vec![false; mesh.face_count()];
    let mut result = Vec::new();
    for start in 0..mesh.face_count() {
        if seen[start] {
            continue;
        }
        seen[start] = true;
        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        while let Some(face) = queue.pop_front() {
            component.push(face);
            for neighbor in &adjacency[face] {
                if !seen[*neighbor] {
                    seen[*neighbor] = true;
                    queue.push_back(*neighbor);
                }
            }
        }
        result.push(component);
    }
    result
}

fn boundary_loops(mesh: &Mesh) -> Vec<Vec<u32>> {
    let edges = edge_map(mesh);
    let mut outgoing = BTreeMap::<u32, u32>::new();
    for (&key, owners) in &edges {
        if owners.len() != 1 {
            continue;
        }
        let (face_index, _) = owners[0];
        let Some(face) = mesh.faces.get(face_index) else {
            continue;
        };
        let directed = [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])]
            .into_iter()
            .find(|(first, second)| ordered_edge(*first, *second) == key);
        let Some((first, second)) = directed else {
            continue;
        };
        if outgoing.insert(first, second).is_some() {
            return Vec::new();
        }
    }
    collect_loops(&outgoing)
}

fn component_boundary_loop(mesh: &Mesh, component: &[usize]) -> Option<Vec<u32>> {
    if component.is_empty() {
        return None;
    }
    let mut counts = BTreeMap::<(u32, u32), (usize, (u32, u32))>::new();
    for face_index in component {
        let face = mesh.faces.get(*face_index)?;
        for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let key = ordered_edge(first, second);
            let entry = counts.entry(key).or_insert((0, (first, second)));
            entry.0 = entry.0.saturating_add(1);
        }
    }
    let mut outgoing = BTreeMap::new();
    for (_, (count, directed)) in counts {
        if count != 1 {
            continue;
        }
        if outgoing.insert(directed.0, directed.1).is_some() {
            return None;
        }
    }
    collect_loops(&outgoing).into_iter().next()
}

fn collect_loops(outgoing: &BTreeMap<u32, u32>) -> Vec<Vec<u32>> {
    let mut loops = Vec::new();
    let mut visited = BTreeSet::new();
    for start in outgoing.keys().copied() {
        if visited.contains(&start) {
            continue;
        }
        let mut current = start;
        let mut loop_vertices = Vec::new();
        let mut valid = true;
        for _ in 0..=outgoing.len() {
            if !visited.insert(current) {
                valid = current == start;
                break;
            }
            loop_vertices.push(current);
            let Some(next) = outgoing.get(&current).copied() else {
                valid = false;
                break;
            };
            current = next;
            if current == start {
                break;
            }
        }
        if valid && current == start && loop_vertices.len() >= 3 {
            loops.push(loop_vertices);
        }
    }
    loops
}

fn component_normal(mesh: &Mesh, component: &[usize]) -> Option<[f32; 3]> {
    let mut normal = [0.0_f32; 3];
    for face_index in component {
        let face = *mesh.faces.get(*face_index)?;
        let value = mesh.face_normal(face);
        for axis in 0..3 {
            normal[axis] += value[axis];
        }
    }
    let length = dot(normal, normal).sqrt();
    (length.is_finite() && length > 1.0e-6).then_some([
        normal[0] / length,
        normal[1] / length,
        normal[2] / length,
    ])
}

fn is_convex_boundary(mesh: &Mesh, boundary: &[u32]) -> bool {
    let Some(normal) = polygon_normal(mesh, boundary) else {
        return false;
    };
    let mut sign = 0.0_f32;
    for index in 0..boundary.len() {
        let previous = mesh
            .vertices
            .get(boundary[(index + boundary.len() - 1) % boundary.len()] as usize);
        let current = mesh.vertices.get(boundary[index] as usize);
        let next = mesh
            .vertices
            .get(boundary[(index + 1) % boundary.len()] as usize);
        let (Some(previous), Some(current), Some(next)) = (previous, current, next) else {
            return false;
        };
        let turn = dot(
            cross(subtract(*current, *previous), subtract(*next, *current)),
            normal,
        );
        if turn.abs() <= 1.0e-6 {
            continue;
        }
        if sign == 0.0 {
            sign = turn.signum();
        } else if turn * sign < 0.0 {
            return false;
        }
    }
    sign != 0.0
}

fn polygon_normal(mesh: &Mesh, boundary: &[u32]) -> Option<[f32; 3]> {
    let first = *mesh.vertices.get(*boundary.first()? as usize)?;
    let mut normal = [0.0_f32; 3];
    for index in 1..boundary.len().saturating_sub(1) {
        let second = *mesh.vertices.get(boundary[index] as usize)?;
        let third = *mesh.vertices.get(boundary[index + 1] as usize)?;
        let value = cross(subtract(second, first), subtract(third, first));
        for axis in 0..3 {
            normal[axis] += value[axis];
        }
    }
    let length = dot(normal, normal).sqrt();
    (length.is_finite() && length > 1.0e-6).then_some([
        normal[0] / length,
        normal[1] / length,
        normal[2] / length,
    ])
}

fn point_for_index(mesh: &Mesh, extra: &[[f32; 3]], index: u32) -> Option<[f32; 3]> {
    mesh.vertices
        .get(index as usize)
        .copied()
        .or_else(|| extra.get(index as usize).copied())
}

fn is_safe_candidate(mesh: &Mesh) -> bool {
    !mesh.is_empty()
        && mesh.vertices.len() >= 3
        && mesh
            .vertices
            .iter()
            .flatten()
            .all(|value| value.is_finite())
        && mesh.faces.iter().all(|face| {
            face.iter()
                .all(|index| (*index as usize) < mesh.vertices.len())
        })
}

fn signed_volume_for_faces(vertices: &[[f32; 3]], faces: &[[u32; 3]], indices: &[usize]) -> f64 {
    indices
        .iter()
        .filter_map(|index| faces.get(*index))
        .filter_map(|face| {
            let first = vertices.get(face[0] as usize)?;
            let second = vertices.get(face[1] as usize)?;
            let third = vertices.get(face[2] as usize)?;
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

fn ordered_edge(first: u32, second: u32) -> (u32, u32) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

fn subtract(first: [f32; 3], second: [f32; 3]) -> [f32; 3] {
    [
        first[0] - second[0],
        first[1] - second[1],
        first[2] - second[2],
    ]
}

fn cross(first: [f32; 3], second: [f32; 3]) -> [f32; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn dot(first: [f32; 3], second: [f32; 3]) -> f32 {
    first[0] * second[0] + first[1] * second[1] + first[2] * second[2]
}

fn cross64(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn dot64(first: [f64; 3], second: [f64; 3]) -> f64 {
    first[0] * second[0] + first[1] * second[1] + first[2] * second[2]
}
