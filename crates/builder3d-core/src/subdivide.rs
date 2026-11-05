//! Global conforming triangle subdivision.
//!
//! The Python reference selects manifold interior edges first, then boundary
//! edges when an odd face count requires one-sided splits. Midpoint indices
//! are global, so neighboring triangles share the same new edge vertices.

use std::collections::{BTreeMap, BTreeSet};

use crate::{CoreError, Mesh, MeshOperationLimits};

/// Evidence from one subdivision.
#[derive(Clone, Debug, PartialEq)]
pub struct SubdivideReport {
    /// Input face count.
    pub before_faces: usize,
    /// Input vertex count.
    pub before_vertices: usize,
    /// Result face count.
    pub after_faces: usize,
    /// Result vertex count.
    pub after_vertices: usize,
    /// Number of selected interior edges.
    pub selected_interior_edges: usize,
    /// Number of selected boundary edges.
    pub selected_boundary_edges: usize,
    /// Whether the operation added faces.
    pub changed: bool,
    /// Input absolute signed volume.
    pub before_volume_mm3: f64,
    /// Output absolute signed volume.
    pub after_volume_mm3: f64,
    /// Input extents.
    pub before_extents_mm: [f32; 3],
    /// Output extents.
    pub after_extents_mm: [f32; 3],
}

/// Subdivision result and evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct SubdivideResult {
    /// Refined mesh.
    pub mesh: Mesh,
    /// Operation evidence.
    pub report: SubdivideReport,
}

/// Subdivides a mesh toward twice its face count using default limits.
pub fn subdivide_mesh(mesh: &Mesh) -> Result<SubdivideResult, CoreError> {
    subdivide_mesh_with_options(mesh, MeshOperationLimits::default())
}

/// Subdivides a mesh with explicit resource limits.
pub fn subdivide_mesh_with_options(
    mesh: &Mesh,
    limits: MeshOperationLimits,
) -> Result<SubdivideResult, CoreError> {
    mesh.validate_non_empty()?;
    limits.validate("subdivide")?;
    limits.check_mesh("subdivide", mesh)?;
    let source_faces = mesh.face_count();
    let source_vertices = mesh.vertex_count();
    let expected_faces =
        source_faces
            .checked_mul(2)
            .ok_or_else(|| CoreError::OperationResourceLimit {
                operation: "subdivide".to_owned(),
                reason: "subdivision face count overflow".to_owned(),
            })?;
    if expected_faces > limits.max_faces {
        return Err(CoreError::OperationResourceLimit {
            operation: "subdivide".to_owned(),
            reason: format!(
                "subdivision result needs {expected_faces} faces, above the {} face limit",
                limits.max_faces
            ),
        });
    }
    let mut edge_faces = BTreeMap::<(u32, u32), Vec<usize>>::new();
    let mut face_edges = Vec::<[(u32, u32); 3]>::with_capacity(source_faces);
    for (face_index, face) in mesh.faces.iter().copied().enumerate() {
        let keys = [
            ordered_edge(face[0], face[1]),
            ordered_edge(face[1], face[2]),
            ordered_edge(face[2], face[0]),
        ];
        for key in keys {
            edge_faces.entry(key).or_default().push(face_index);
        }
        face_edges.push(keys);
    }
    let interior = edge_faces
        .iter()
        .filter_map(|(key, owners)| (owners.len() == 2).then_some(*key))
        .collect::<Vec<_>>();
    let boundary = edge_faces
        .iter()
        .filter_map(|(key, owners)| (owners.len() == 1).then_some(*key))
        .collect::<Vec<_>>();
    let mut selected = BTreeSet::new();
    let mut remaining = source_faces;
    for key in interior {
        if remaining < 2 {
            break;
        }
        selected.insert(key);
        remaining -= 2;
    }
    for key in boundary {
        if remaining < 1 {
            break;
        }
        selected.insert(key);
        remaining -= 1;
    }
    if remaining != 0 {
        return Err(CoreError::MeshOperation(
            "subdivide could not double this mesh without unsupported edge topology".to_owned(),
        ));
    }
    let midpoint_count = selected.len();
    let result_vertices = source_vertices.checked_add(midpoint_count).ok_or_else(|| {
        CoreError::OperationResourceLimit {
            operation: "subdivide".to_owned(),
            reason: "subdivision vertex count overflow".to_owned(),
        }
    })?;
    if result_vertices > limits.max_vertices {
        return Err(CoreError::OperationResourceLimit {
            operation: "subdivide".to_owned(),
            reason: format!(
                "subdivision result needs {result_vertices} vertices, above the {} vertex limit",
                limits.max_vertices
            ),
        });
    }
    let work_units = u64::try_from(source_faces)
        .ok()
        .and_then(|faces| faces.checked_mul(8))
        .and_then(|units| units.checked_add(u64::try_from(selected.len()).ok()?))
        .ok_or_else(|| CoreError::MeshOperation("subdivision work count overflow".to_owned()))?;
    limits.check_work("subdivide", work_units)?;

    let mut vertices = mesh.vertices.clone();
    let mut midpoint_indices = BTreeMap::<(u32, u32), u32>::new();
    for key in &selected {
        let Some(first) = mesh.vertices.get(key.0 as usize) else {
            return Err(CoreError::InvalidMesh(
                "subdivision edge references a missing first vertex".to_owned(),
            ));
        };
        let Some(second) = mesh.vertices.get(key.1 as usize) else {
            return Err(CoreError::InvalidMesh(
                "subdivision edge references a missing second vertex".to_owned(),
            ));
        };
        let index =
            u32::try_from(vertices.len()).map_err(|_| CoreError::OperationResourceLimit {
                operation: "subdivide".to_owned(),
                reason: "subdivision midpoint index overflow".to_owned(),
            })?;
        let midpoint = [
            (first[0] + second[0]) * 0.5,
            (first[1] + second[1]) * 0.5,
            (first[2] + second[2]) * 0.5,
        ];
        vertices.push(midpoint);
        midpoint_indices.insert(*key, index);
    }

    let mut faces = Vec::with_capacity(expected_faces);
    for (face, keys) in mesh.faces.iter().copied().zip(face_edges) {
        let mab = midpoint_indices.get(&keys[0]).copied();
        let mbc = midpoint_indices.get(&keys[1]).copied();
        let mca = midpoint_indices.get(&keys[2]).copied();
        let flags = (mab.is_some(), mbc.is_some(), mca.is_some());
        match flags {
            (false, false, false) => faces.push(face),
            (true, false, false) => {
                faces.push([face[0], mab.unwrap_or(face[0]), face[2]]);
                faces.push([mab.unwrap_or(face[0]), face[1], face[2]]);
            }
            (false, true, false) => {
                faces.push([face[1], mbc.unwrap_or(face[1]), face[0]]);
                faces.push([mbc.unwrap_or(face[1]), face[2], face[0]]);
            }
            (false, false, true) => {
                faces.push([face[2], mca.unwrap_or(face[2]), face[1]]);
                faces.push([mca.unwrap_or(face[2]), face[0], face[1]]);
            }
            (true, true, false) => {
                let (Some(mab), Some(mbc)) = (mab, mbc) else {
                    continue;
                };
                faces.extend([
                    [face[0], mab, mbc],
                    [mab, face[1], mbc],
                    [face[0], mbc, face[2]],
                ]);
            }
            (false, true, true) => {
                let (Some(mbc), Some(mca)) = (mbc, mca) else {
                    continue;
                };
                faces.extend([
                    [face[0], face[1], mbc],
                    [face[0], mbc, mca],
                    [face[2], mca, mbc],
                ]);
            }
            (true, false, true) => {
                let (Some(mab), Some(mca)) = (mab, mca) else {
                    continue;
                };
                faces.extend([
                    [face[0], mab, mca],
                    [mab, face[1], face[2]],
                    [mab, face[2], mca],
                ]);
            }
            (true, true, true) => {
                let (Some(mab), Some(mbc), Some(mca)) = (mab, mbc, mca) else {
                    continue;
                };
                faces.extend([
                    [face[0], mab, mca],
                    [mab, face[1], mbc],
                    [mca, mbc, face[2]],
                    [mab, mbc, mca],
                ]);
            }
        }
    }
    if faces.len() <= source_faces {
        return Err(CoreError::MeshOperation(
            "subdivision produced no additional faces".to_owned(),
        ));
    }
    let result = Mesh::new(vertices, faces)?;
    limits.check_mesh("subdivide", &result)?;
    Ok(SubdivideResult {
        report: SubdivideReport {
            before_faces: source_faces,
            before_vertices: source_vertices,
            after_faces: result.face_count(),
            after_vertices: result.vertex_count(),
            selected_interior_edges: selected
                .iter()
                .filter(|key| edge_faces.get(key).is_some_and(|owners| owners.len() == 2))
                .count(),
            selected_boundary_edges: selected
                .iter()
                .filter(|key| edge_faces.get(key).is_some_and(|owners| owners.len() == 1))
                .count(),
            changed: true,
            before_volume_mm3: signed_volume(mesh).abs(),
            after_volume_mm3: signed_volume(&result).abs(),
            before_extents_mm: mesh.extents(),
            after_extents_mm: result.extents(),
        },
        mesh: result,
    })
}

fn ordered_edge(first: u32, second: u32) -> (u32, u32) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
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
