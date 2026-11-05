//! Deterministic bounded triangle-mesh simplification.
//!
//! The Python reference delegates to trimesh's quadric decimator. The
//! portable Rust core uses a conservative shortest-edge collapse policy:
//! interior edges are preferred, each collapse is validated, and the result
//! never exceeds the requested face target. This keeps the topology-changing
//! operation deterministic and resource-accountable on every adapter.

use std::collections::{BTreeMap, BTreeSet};

use crate::{CoreError, Mesh, MeshOperationLimits};

/// The default simplification target policy from the Python reference.
pub const DEFAULT_SIMPLIFY_MIN_FACES: usize = 100;

/// Evidence from one simplification attempt.
#[derive(Clone, Debug, PartialEq)]
pub struct SimplifyReport {
    /// Input face count.
    pub before_faces: usize,
    /// Input vertex count.
    pub before_vertices: usize,
    /// Requested face target after the minimum clamp.
    pub target_faces: usize,
    /// Result face count.
    pub after_faces: usize,
    /// Result vertex count.
    pub after_vertices: usize,
    /// Whether any collapse changed the mesh.
    pub changed: bool,
    /// Number of accepted edge collapses.
    pub collapsed_edges: usize,
    /// Number of rebuild iterations.
    pub iterations: usize,
    /// Counted work units.
    pub work_units: u64,
    /// Largest half-edge length used as a conservative geometric deviation.
    pub geometric_deviation_mm: f32,
    /// Absolute input volume estimate.
    pub before_volume_mm3: f64,
    /// Absolute output volume estimate.
    pub after_volume_mm3: f64,
    /// Non-fatal notes.
    pub warnings: Vec<String>,
}

/// Result mesh and simplification evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct SimplifyResult {
    /// Simplified mesh, or an unchanged clone on a no-op.
    pub mesh: Mesh,
    /// Operation evidence.
    pub report: SimplifyReport,
}

/// Simplifies toward the optional face target using default limits.
pub fn simplify_mesh(mesh: &Mesh, face_count: Option<usize>) -> Result<SimplifyResult, CoreError> {
    simplify_mesh_with_options(mesh, face_count, MeshOperationLimits::default())
}

/// Simplifies toward the optional face target with explicit limits.
pub fn simplify_mesh_with_options(
    mesh: &Mesh,
    face_count: Option<usize>,
    limits: MeshOperationLimits,
) -> Result<SimplifyResult, CoreError> {
    mesh.validate_non_empty()?;
    limits.validate("simplify")?;
    limits.check_mesh("simplify", mesh)?;
    let target = face_count.unwrap_or_else(|| mesh.face_count() / 4).max(4);
    let before_faces = mesh.face_count();
    let before_vertices = mesh.vertex_count();
    let before_volume_mm3 = signed_volume(mesh).abs();
    if before_faces <= target {
        return Ok(SimplifyResult {
            mesh: mesh.clone(),
            report: SimplifyReport {
                before_faces,
                before_vertices,
                target_faces: target,
                after_faces: before_faces,
                after_vertices: before_vertices,
                changed: false,
                collapsed_edges: 0,
                iterations: 0,
                work_units: 0,
                geometric_deviation_mm: 0.0,
                before_volume_mm3,
                after_volume_mm3: before_volume_mm3,
                warnings: Vec::new(),
            },
        });
    }

    let mut current = mesh.clone();
    let mut collapsed_edges = 0_usize;
    let mut iterations = 0_usize;
    let mut work_units = 0_u64;
    let mut maximum_deviation = 0.0_f32;
    let mut warnings = Vec::new();
    while current.face_count() > target {
        iterations = iterations.saturating_add(1);
        let iteration_work = u64::try_from(current.face_count())
            .ok()
            .and_then(|faces| faces.checked_mul(6))
            .ok_or_else(|| CoreError::MeshOperation("simplify work count overflow".to_owned()))?;
        work_units = work_units
            .checked_add(iteration_work)
            .ok_or_else(|| CoreError::MeshOperation("simplify work count overflow".to_owned()))?;
        limits.check_work("simplify", work_units)?;
        let Some((first, second, length)) = shortest_edge(&current) else {
            warnings.push("no collapsible edges remain before the target".to_owned());
            break;
        };
        if !length.is_finite() {
            return Err(CoreError::InvalidMesh(
                "simplify encountered a non-finite edge length".to_owned(),
            ));
        }
        let next = collapse_edge(&current, first, second)?;
        if next.face_count() >= current.face_count() || next.is_empty() {
            warnings.push("shortest edge could not reduce the mesh further".to_owned());
            break;
        }
        maximum_deviation = maximum_deviation.max(length * 0.5);
        limits.check_mesh("simplify", &next)?;
        current = next;
        collapsed_edges = collapsed_edges.saturating_add(1);
    }
    if current.face_count() > target {
        warnings.push(format!(
            "simplifier stopped at {} faces above target {target}",
            current.face_count()
        ));
    }
    let after_volume_mm3 = signed_volume(&current).abs();
    Ok(SimplifyResult {
        mesh: current.clone(),
        report: SimplifyReport {
            before_faces,
            before_vertices,
            target_faces: target,
            after_faces: current.face_count(),
            after_vertices: current.vertex_count(),
            changed: current != *mesh,
            collapsed_edges,
            iterations,
            work_units,
            geometric_deviation_mm: maximum_deviation,
            before_volume_mm3,
            after_volume_mm3,
            warnings,
        },
    })
}

fn shortest_edge(mesh: &Mesh) -> Option<(u32, u32, f32)> {
    let mut owners = BTreeMap::<(u32, u32), usize>::new();
    for face in &mesh.faces {
        for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            *owners.entry(ordered_edge(first, second)).or_default() += 1;
        }
    }
    let mut best: Option<(u8, f32, (u32, u32))> = None;
    for (edge, count) in owners {
        let Some(first) = mesh.vertices.get(edge.0 as usize) else {
            continue;
        };
        let Some(second) = mesh.vertices.get(edge.1 as usize) else {
            continue;
        };
        let delta = [
            first[0] - second[0],
            first[1] - second[1],
            first[2] - second[2],
        ];
        let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
        if !length.is_finite() || length <= f32::EPSILON {
            continue;
        }
        let priority = if count == 2 { 0 } else { 1 };
        let candidate = (priority, length, edge);
        let replace = match best {
            None => true,
            Some(current) => {
                candidate.0 < current.0
                    || (candidate.0 == current.0
                        && (candidate.1 < current.1
                            || (candidate.1 == current.1 && candidate.2 < current.2)))
            }
        };
        if replace {
            best = Some(candidate);
        }
    }
    best.map(|(_, length, edge)| (edge.0, edge.1, length))
}

fn collapse_edge(mesh: &Mesh, first: u32, second: u32) -> Result<Mesh, CoreError> {
    let first_point = *mesh.vertices.get(first as usize).ok_or_else(|| {
        CoreError::InvalidMesh("simplify edge references no first vertex".to_owned())
    })?;
    let second_point = *mesh.vertices.get(second as usize).ok_or_else(|| {
        CoreError::InvalidMesh("simplify edge references no second vertex".to_owned())
    })?;
    let midpoint = [
        (first_point[0] + second_point[0]) * 0.5,
        (first_point[1] + second_point[1]) * 0.5,
        (first_point[2] + second_point[2]) * 0.5,
    ];
    let mut faces = Vec::with_capacity(mesh.face_count());
    let mut seen = BTreeSet::<[u32; 3]>::new();
    for face in &mesh.faces {
        let remap = |index: u32| if index == second { first } else { index };
        let candidate = [remap(face[0]), remap(face[1]), remap(face[2])];
        if candidate[0] == candidate[1]
            || candidate[1] == candidate[2]
            || candidate[2] == candidate[0]
        {
            continue;
        }
        let Some(a) = mesh.vertices.get(candidate[0] as usize) else {
            continue;
        };
        let Some(b) = mesh.vertices.get(candidate[1] as usize) else {
            continue;
        };
        let Some(c) = mesh.vertices.get(candidate[2] as usize) else {
            continue;
        };
        let cross = [
            (b[1] - a[1]) * (c[2] - a[2]) - (b[2] - a[2]) * (c[1] - a[1]),
            (b[2] - a[2]) * (c[0] - a[0]) - (b[0] - a[0]) * (c[2] - a[2]),
            (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]),
        ];
        if cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2] <= 1.0e-20 {
            continue;
        }
        let mut canonical = candidate;
        canonical.sort_unstable();
        if seen.insert(canonical) {
            faces.push(candidate);
        }
    }
    let mut vertices = mesh.vertices.clone();
    vertices[first as usize] = midpoint;
    compact_mesh(&vertices, &faces)
}

fn compact_mesh(vertices: &[[f32; 3]], faces: &[[u32; 3]]) -> Result<Mesh, CoreError> {
    let mut remap = vec![None::<u32>; vertices.len()];
    let mut compact_vertices = Vec::new();
    for face in faces {
        for index in face {
            let slot = remap.get_mut(*index as usize).ok_or_else(|| {
                CoreError::InvalidMesh("simplify face references a missing vertex".to_owned())
            })?;
            if slot.is_none() {
                let value = u32::try_from(compact_vertices.len()).map_err(|_| {
                    CoreError::OperationResourceLimit {
                        operation: "simplify".to_owned(),
                        reason: "compact vertex index overflow".to_owned(),
                    }
                })?;
                *slot = Some(value);
                compact_vertices.push(vertices[*index as usize]);
            }
        }
    }
    let mut compact_faces = Vec::with_capacity(faces.len());
    for face in faces {
        let first = remap[face[0] as usize]
            .ok_or_else(|| CoreError::MeshOperation("simplify lost a face vertex".to_owned()))?;
        let second = remap[face[1] as usize]
            .ok_or_else(|| CoreError::MeshOperation("simplify lost a face vertex".to_owned()))?;
        let third = remap[face[2] as usize]
            .ok_or_else(|| CoreError::MeshOperation("simplify lost a face vertex".to_owned()))?;
        compact_faces.push([first, second, third]);
    }
    Mesh::new(compact_vertices, compact_faces)
}

fn signed_volume(mesh: &Mesh) -> f64 {
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
            Some(
                (a[0] * (b[1] * c[2] - b[2] * c[1])
                    + a[1] * (b[2] * c[0] - b[0] * c[2])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0,
            )
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
