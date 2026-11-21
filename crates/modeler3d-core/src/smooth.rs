//! Bounded Laplacian mesh smoothing that keeps sharp features (H1 / D9).

use crate::brushes::feature_neighbors;
use crate::{CoreError, Mesh, MeshOperationLimits};

/// Maximum smoothing iterations accepted by the portable core.
pub const DEFAULT_SMOOTH_MAX_ITERATIONS: usize = 10;

/// Evidence from one smoothing operation.
#[derive(Clone, Debug, PartialEq)]
pub struct SmoothReport {
    /// Input vertex count.
    pub before_vertices: usize,
    /// Input face count.
    pub before_faces: usize,
    /// Requested iterations.
    pub iterations: usize,
    /// Iterations actually applied.
    pub applied_iterations: usize,
    /// Whether positions changed.
    pub changed: bool,
    /// Maximum displacement from the source positions.
    pub maximum_displacement_mm: f32,
    /// Absolute input volume estimate.
    pub before_volume_mm3: f64,
    /// Absolute output volume estimate.
    pub after_volume_mm3: f64,
}

/// Smoothed mesh and evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct SmoothResult {
    /// Smoothed mesh, or an unchanged clone for zero iterations.
    pub mesh: Mesh,
    /// Operation evidence.
    pub report: SmoothReport,
}

/// Smooths a mesh with the default operation limits.
pub fn smooth_mesh(mesh: &Mesh, iterations: usize) -> Result<SmoothResult, CoreError> {
    smooth_mesh_with_options(mesh, iterations, MeshOperationLimits::default())
}

/// Smooths a mesh with explicit limits.
pub fn smooth_mesh_with_options(
    mesh: &Mesh,
    iterations: usize,
    limits: MeshOperationLimits,
) -> Result<SmoothResult, CoreError> {
    mesh.validate_non_empty()?;
    limits.validate("smooth")?;
    limits.check_mesh("smooth", mesh)?;
    if iterations > DEFAULT_SMOOTH_MAX_ITERATIONS {
        return Err(CoreError::OperationResourceLimit {
            operation: "smooth".to_owned(),
            reason: format!(
                "iterations {iterations} exceed the {} iteration limit",
                DEFAULT_SMOOTH_MAX_ITERATIONS
            ),
        });
    }
    let before_volume_mm3 = signed_volume(mesh).abs();
    if iterations == 0 {
        return Ok(SmoothResult {
            mesh: mesh.clone(),
            report: SmoothReport {
                before_vertices: mesh.vertex_count(),
                before_faces: mesh.face_count(),
                iterations,
                applied_iterations: 0,
                changed: false,
                maximum_displacement_mm: 0.0,
                before_volume_mm3,
                after_volume_mm3: before_volume_mm3,
            },
        });
    }
    let work_units = u64::try_from(mesh.vertex_count())
        .ok()
        .and_then(|vertices| vertices.checked_mul(iterations as u64))
        .and_then(|units| units.checked_mul(4))
        .ok_or_else(|| CoreError::MeshOperation("smooth work count overflow".to_owned()))?;
    limits.check_work("smooth", work_units)?;
    // H1 / D9: Blender's sculpt Smooth with the Smooth Brush's hard-edge
    // rule, so sparse stock shapes keep their form instead of melting: a
    // vertex on a sharp edge only slides along that edge, and a corner stays
    // put. Sharp edges are found once on the source mesh.
    let adjacency = feature_neighbors(mesh);
    let source_vertices = mesh.vertices.clone();
    let mut vertices = source_vertices.clone();
    for _ in 0..iterations {
        let mut next = vertices.clone();
        for (index, neighbors) in adjacency.iter().enumerate() {
            if neighbors.is_empty() {
                continue;
            }
            let mut average = [0.0_f32; 3];
            for neighbor in neighbors {
                let Some(vertex) = vertices.get(*neighbor) else {
                    return Err(CoreError::InvalidMesh(
                        "smooth adjacency references a missing vertex".to_owned(),
                    ));
                };
                for axis in 0..3 {
                    average[axis] += vertex[axis] / neighbors.len() as f32;
                }
            }
            let Some(current) = vertices.get(index) else {
                return Err(CoreError::InvalidMesh(
                    "smooth vertex index disappeared".to_owned(),
                ));
            };
            // The reference filter uses a conservative Laplacian lambda; a
            // half-step avoids collapsing coarse authored geometry immediately.
            let value = [
                current[0] + (average[0] - current[0]) * 0.5,
                current[1] + (average[1] - current[1]) * 0.5,
                current[2] + (average[2] - current[2]) * 0.5,
            ];
            if value.iter().any(|coordinate| !coordinate.is_finite()) {
                return Err(CoreError::MeshOperation(
                    "smooth produced non-finite vertex positions".to_owned(),
                ));
            }
            next[index] = value;
        }
        vertices = next;
    }
    let result = Mesh::new(vertices, mesh.faces.clone())?;
    limits.check_mesh("smooth", &result)?;
    let maximum_displacement_mm = source_vertices
        .iter()
        .zip(&result.vertices)
        .map(|(first, second)| {
            let delta = [
                second[0] - first[0],
                second[1] - first[1],
                second[2] - first[2],
            ];
            (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt()
        })
        .fold(0.0_f32, f32::max);
    let after_volume_mm3 = signed_volume(&result).abs();
    Ok(SmoothResult {
        report: SmoothReport {
            before_vertices: mesh.vertex_count(),
            before_faces: mesh.face_count(),
            iterations,
            applied_iterations: iterations,
            changed: result != *mesh,
            maximum_displacement_mm,
            before_volume_mm3,
            after_volume_mm3,
        },
        mesh: result,
    })
}

fn signed_volume(mesh: &Mesh) -> f64 {
    mesh.faces
        .iter()
        .filter_map(|face| {
            let a = mesh.vertices.get(face[0] as usize)?;
            let b = mesh.vertices.get(face[1] as usize)?;
            let c = mesh.vertices.get(face[2] as usize)?;
            Some(
                (f64::from(a[0])
                    * (f64::from(b[1]) * f64::from(c[2]) - f64::from(b[2]) * f64::from(c[1]))
                    + f64::from(a[1])
                        * (f64::from(b[2]) * f64::from(c[0]) - f64::from(b[0]) * f64::from(c[2]))
                    + f64::from(a[2])
                        * (f64::from(b[0]) * f64::from(c[1]) - f64::from(b[1]) * f64::from(c[0])))
                    / 6.0,
            )
        })
        .sum()
}
