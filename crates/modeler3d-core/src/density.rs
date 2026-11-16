//! Bounded local face-density normalization.
//!
//! Selected/unselected edges are treated as a hard boundary. Long internal
//! selected edges are conformingly split; short internal selected edges are
//! conservatively collapsed only when both incident faces remain selected.
//! The operation is non-mutating and can be cancelled between topology passes.

use std::collections::{BTreeMap, BTreeSet};

use crate::{CoreError, Mesh, MeshOperationLimits};

/// Default brush/selection radius in millimetres.
pub const NORMALIZE_DENSITY_DEFAULT_RADIUS_MM: f32 = 8.0;
/// Minimum accepted selection radius.
pub const NORMALIZE_DENSITY_RADIUS_MIN_MM: f32 = 0.1;
/// Maximum accepted selection radius.
pub const NORMALIZE_DENSITY_RADIUS_MAX_MM: f32 = 1000.0;
/// Minimum explicit target edge length.
pub const NORMALIZE_DENSITY_TARGET_MIN_MM: f32 = 0.01;
/// Maximum explicit target edge length.
pub const NORMALIZE_DENSITY_TARGET_MAX_MM: f32 = 1000.0;
/// Maximum topology iterations.
pub const NORMALIZE_DENSITY_MAX_ITERATIONS: usize = 4;
/// Maximum selected faces accepted by the public contract.
pub const NORMALIZE_DENSITY_MAX_REGION_FACES: usize = 250_000;
/// Maximum selected faces for bounded working topology.
pub const NORMALIZE_DENSITY_MAX_WORKING_REGION_FACES: usize = 50_000;
/// Maximum source faces before topology tables are allocated.
pub const NORMALIZE_DENSITY_MAX_SOURCE_FACES: usize = 250_000;
/// Maximum density-specific work units.
pub const NORMALIZE_DENSITY_MAX_WORK_UNITS: u64 = 2_000_000;
/// Maximum edge-table rebuilds.
pub const NORMALIZE_DENSITY_MAX_EDGE_REBUILDS: usize = 128;
/// Maximum collapses per iteration.
pub const NORMALIZE_DENSITY_MAX_COLLAPSES_PER_ITERATION: usize = 32;
/// Maximum resulting faces.
pub const NORMALIZE_DENSITY_MAX_FACES: usize = 1_000_000;

/// Parameters for density normalization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NormalizeDensityOptions {
    /// Selection radius in millimetres.
    pub radius_mm: f32,
    /// Explicit target edge length, or boundary/region median when absent.
    pub target_edge_length_mm: Option<f32>,
    /// Number of bounded split/collapse passes.
    pub max_iterations: usize,
    /// Shared operation resource policy.
    pub limits: MeshOperationLimits,
}

impl Default for NormalizeDensityOptions {
    fn default() -> Self {
        Self {
            radius_mm: NORMALIZE_DENSITY_DEFAULT_RADIUS_MM,
            target_edge_length_mm: None,
            max_iterations: NORMALIZE_DENSITY_MAX_ITERATIONS,
            limits: MeshOperationLimits::default(),
        }
    }
}

/// Plain evidence from one density-normalization attempt.
#[derive(Clone, Debug, PartialEq)]
pub struct NormalizeDensityReport {
    /// Whether indexed geometry changed.
    pub changed: bool,
    /// Input face count.
    pub face_count_before: usize,
    /// Output face count.
    pub face_count_after: usize,
    /// Selected faces before topology work.
    pub selected_faces_before: usize,
    /// Selected faces after topology work.
    pub selected_faces_after: usize,
    /// Effective target edge length.
    pub target_edge_length_mm: f32,
    /// Median selected-boundary edge length, if a boundary exists.
    pub boundary_edge_length_mm: Option<f32>,
    /// Median selected-region edge length before the operation.
    pub characteristic_edge_length_before_mm: f32,
    /// Median selected-region edge length after the operation.
    pub characteristic_edge_length_after_mm: f32,
    /// Faces added by conforming splits.
    pub subdivided_faces: usize,
    /// Accepted internal edge collapses.
    pub collapsed_edges: usize,
    /// Largest half-edge collapse displacement.
    pub geometric_deviation_mm: f32,
    /// Whether the selected/unselected boundary vertices remain present.
    pub boundary_preserved: bool,
    /// Alias kept explicit for adapter contracts.
    pub selection_boundary_preserved: bool,
    /// Requested iteration count.
    pub iterations: usize,
    /// Charged topology work.
    pub resource_work_units: u64,
    /// Edge-table rebuild count.
    pub topology_passes: usize,
    /// Density-specific budget.
    pub resource_budget_units: u64,
    /// Recoverable boundary/operation issues.
    pub issues: Vec<String>,
}

/// Result mesh and density report.
#[derive(Clone, Debug, PartialEq)]
pub struct NormalizeDensityResult {
    /// Normalized mesh.
    pub mesh: Mesh,
    /// Operation evidence.
    pub report: NormalizeDensityReport,
}

/// Optional cancellation callback used by density normalization.
pub type NormalizeDensityCancel<'a> = Option<&'a mut dyn FnMut() -> Result<(), CoreError>>;

/// Optional progress callback used by density normalization.
pub type NormalizeDensityProgress<'a> = Option<&'a mut dyn FnMut(u8, &str)>;

type DensityMeshBuffers = (Vec<[f32; 3]>, Vec<[u32; 3]>, Vec<bool>);
type SubdivideOutput = (Vec<[f32; 3]>, Vec<[u32; 3]>, Vec<bool>, usize);
type CollapseOutput = (Vec<[f32; 3]>, Vec<[u32; 3]>, Vec<bool>, f32, bool);

/// Selects faces near one or more local-space centers.
pub fn select_face_region(
    mesh: &Mesh,
    centers_mm: &[[f32; 3]],
    radius_mm: f32,
) -> Result<Vec<usize>, CoreError> {
    mesh.validate_non_empty()?;
    validate_radius(radius_mm)?;
    if centers_mm.is_empty() || centers_mm.len() > 256 {
        return Err(CoreError::MeshOperation(
            "centers_mm must contain between 1 and 256 points".to_owned(),
        ));
    }
    if centers_mm
        .iter()
        .any(|center| center.iter().any(|value| !value.is_finite()))
    {
        return Err(CoreError::InvalidMesh(
            "centers_mm must contain finite points".to_owned(),
        ));
    }
    let mut selected = vec![false; mesh.face_count()];
    let mut nearest = vec![usize::MAX; centers_mm.len()];
    let mut nearest_distance = vec![f32::INFINITY; centers_mm.len()];
    for (face_index, face) in mesh.faces.iter().copied().enumerate() {
        let Some(first) = mesh.vertices.get(face[0] as usize) else {
            continue;
        };
        let Some(second) = mesh.vertices.get(face[1] as usize) else {
            continue;
        };
        let Some(third) = mesh.vertices.get(face[2] as usize) else {
            continue;
        };
        let center = [
            (first[0] + second[0] + third[0]) / 3.0,
            (first[1] + second[1] + third[1]) / 3.0,
            (first[2] + second[2] + third[2]) / 3.0,
        ];
        let face_radius = [
            length(subtract(*first, center)),
            length(subtract(*second, center)),
            length(subtract(*third, center)),
        ]
        .into_iter()
        .fold(0.0_f32, f32::max);
        for (center_index, brush_center) in centers_mm.iter().copied().enumerate() {
            let distance = length(subtract(center, brush_center));
            if distance <= radius_mm + face_radius {
                selected[face_index] = true;
            }
            if distance < nearest_distance[center_index] {
                nearest_distance[center_index] = distance;
                nearest[center_index] = face_index;
            }
        }
    }
    for face_index in nearest {
        if face_index != usize::MAX {
            selected[face_index] = true;
        }
    }
    Ok(selected
        .into_iter()
        .enumerate()
        .filter_map(|(index, value)| value.then_some(index))
        .collect())
}

/// Normalizes a selected region without progress/cancellation hooks.
pub fn normalize_face_density(
    mesh: &Mesh,
    face_indices: Option<&[usize]>,
    centers_mm: Option<&[[f32; 3]]>,
    options: NormalizeDensityOptions,
) -> Result<NormalizeDensityResult, CoreError> {
    normalize_face_density_with_hooks(mesh, face_indices, centers_mm, options, None, None)
}

/// Normalizes a selected region with optional cancellation and progress hooks.
pub fn normalize_face_density_with_hooks(
    mesh: &Mesh,
    face_indices: Option<&[usize]>,
    centers_mm: Option<&[[f32; 3]]>,
    options: NormalizeDensityOptions,
    mut cancel_check: NormalizeDensityCancel<'_>,
    mut progress: NormalizeDensityProgress<'_>,
) -> Result<NormalizeDensityResult, CoreError> {
    mesh.validate_non_empty()?;
    if mesh.face_count() > NORMALIZE_DENSITY_MAX_SOURCE_FACES {
        return Err(CoreError::OperationResourceLimit {
            operation: "normalize_density".to_owned(),
            reason: format!(
                "source mesh has {} faces, above the {} face limit",
                mesh.face_count(),
                NORMALIZE_DENSITY_MAX_SOURCE_FACES
            ),
        });
    }
    validate_options(&options)?;
    let mut selected = normalize_selection(mesh, face_indices, centers_mm, options.radius_mm)?;
    if selected.is_empty() {
        return Err(CoreError::MeshOperation(
            "no mesh faces were selected".to_owned(),
        ));
    }
    if selected.len() > NORMALIZE_DENSITY_MAX_REGION_FACES {
        return Err(CoreError::OperationResourceLimit {
            operation: "normalize_density".to_owned(),
            reason: "selected region exceeds the public face limit".to_owned(),
        });
    }
    if selected.len() > NORMALIZE_DENSITY_MAX_WORKING_REGION_FACES {
        return Err(CoreError::OperationResourceLimit {
            operation: "normalize_density".to_owned(),
            reason: format!(
                "selected region exceeds the bounded working limit of {} faces",
                NORMALIZE_DENSITY_MAX_WORKING_REGION_FACES
            ),
        });
    }
    selected.sort_unstable();
    selected.dedup();
    let mut selected_flags = vec![false; mesh.face_count()];
    for index in selected {
        if let Some(flag) = selected_flags.get_mut(index) {
            *flag = true;
        }
    }
    let selected_faces_before = selected_flags.iter().filter(|flag| **flag).count();
    let before_face_count = mesh.face_count();
    let mut vertices = mesh.vertices.clone();
    let mut faces = mesh.faces.clone();
    let mut work_units = 0_u64;
    let mut topology_passes = 0_usize;
    call_cancel(&mut cancel_check)?;
    call_progress(&mut progress, 5, "Analyzing the selected face region...");
    charge_work(
        &mut work_units,
        &mut topology_passes,
        faces.len(),
        "initial topology",
        &mut cancel_check,
        &options.limits,
    )?;
    let initial = edge_data(&faces);
    let initial_selected_counts = selected_edge_counts(&initial, &selected_flags);
    let initial_lengths = edge_lengths(&vertices, &initial);
    let boundary_flags = boundary_edge_flags(&initial, &initial_selected_counts);
    let region_lengths = initial_lengths
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (initial_selected_counts[index] > 0).then_some(*value))
        .collect::<Vec<_>>();
    let boundary_lengths = initial_lengths
        .iter()
        .enumerate()
        .filter_map(|(index, value)| boundary_flags[index].then_some(*value))
        .collect::<Vec<_>>();
    let target = options
        .target_edge_length_mm
        .or_else(|| median(&boundary_lengths))
        .or_else(|| median(&region_lengths))
        .ok_or_else(|| {
            CoreError::MeshOperation("could not determine a target edge length".to_owned())
        })?;
    if !target.is_finite() || target <= 0.0 {
        return Err(CoreError::MeshOperation(
            "target edge length must be positive and finite".to_owned(),
        ));
    }
    let before_characteristic = median(&region_lengths).unwrap_or(0.0);
    let boundary_vertices_before = boundary_vertices(&vertices, &initial, &boundary_flags);
    let mut subdivided_faces = 0_usize;
    let mut collapsed_edges = 0_usize;
    let mut geometric_deviation = 0.0_f32;
    for iteration in 0..options.max_iterations {
        call_cancel(&mut cancel_check)?;
        charge_work(
            &mut work_units,
            &mut topology_passes,
            faces.len(),
            "subdivision topology",
            &mut cancel_check,
            &options.limits,
        )?;
        let (next_vertices, next_faces, next_flags, added) =
            subdivide_region(&vertices, &faces, &selected_flags, target)?;
        vertices = next_vertices;
        faces = next_faces;
        selected_flags = next_flags;
        subdivided_faces = subdivided_faces.saturating_add(added);
        call_progress(
            &mut progress,
            ((iteration + 1) as u8).saturating_mul(70) / options.max_iterations as u8,
            &format!(
                "Normalizing face density pass {} of {}...",
                iteration + 1,
                options.max_iterations
            ),
        );
        if faces.len() > NORMALIZE_DENSITY_MAX_FACES {
            return Err(CoreError::OperationResourceLimit {
                operation: "normalize_density".to_owned(),
                reason: "face-density result exceeds the face safety limit".to_owned(),
            });
        }
        let selected_count = selected_flags.iter().filter(|flag| **flag).count();
        let collapse_limit = NORMALIZE_DENSITY_MAX_COLLAPSES_PER_ITERATION
            .min((selected_count.saturating_mul(2)).max(8));
        let mut collapsed_this_iteration = 0_usize;
        for _ in 0..collapse_limit {
            call_cancel(&mut cancel_check)?;
            charge_work(
                &mut work_units,
                &mut topology_passes,
                faces.len(),
                "collapse topology",
                &mut cancel_check,
                &options.limits,
            )?;
            let (next_vertices, next_faces, next_flags, deviation, collapsed) =
                collapse_one_edge(&vertices, &faces, &selected_flags, target)?;
            if !collapsed {
                break;
            }
            vertices = next_vertices;
            faces = next_faces;
            selected_flags = next_flags;
            collapsed_edges = collapsed_edges.saturating_add(1);
            collapsed_this_iteration = collapsed_this_iteration.saturating_add(1);
            geometric_deviation = geometric_deviation.max(deviation);
        }
        if added == 0 && collapsed_this_iteration == 0 {
            break;
        }
    }
    call_cancel(&mut cancel_check)?;
    call_progress(&mut progress, 90, "Preparing the normalized mesh report...");
    let result = Mesh::new(vertices, faces)?;
    options.limits.check_mesh("normalize_density", &result)?;
    charge_work(
        &mut work_units,
        &mut topology_passes,
        result.face_count(),
        "post-operation topology",
        &mut cancel_check,
        &options.limits,
    )?;
    let after_data = edge_data(&result.faces);
    let after_selected_counts = selected_edge_counts(&after_data, &selected_flags);
    let after_lengths = edge_lengths(&result.vertices, &after_data);
    let after_region_lengths = after_lengths
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (after_selected_counts[index] > 0).then_some(*value))
        .collect::<Vec<_>>();
    let boundary_preserved = boundary_vertices_present(
        &result,
        &after_data,
        &after_selected_counts,
        &boundary_vertices_before,
    );
    let after_characteristic = median(&after_region_lengths).unwrap_or(0.0);
    let changed = result != *mesh;
    let mut issues = Vec::new();
    if !boundary_preserved {
        issues.push("selection boundary changed".to_owned());
    }
    call_progress(&mut progress, 100, "Face density normalization complete");
    Ok(NormalizeDensityResult {
        mesh: result,
        report: NormalizeDensityReport {
            changed,
            face_count_before: before_face_count,
            face_count_after: after_data.face_edge_ids.len(),
            selected_faces_before,
            selected_faces_after: selected_flags.iter().filter(|flag| **flag).count(),
            target_edge_length_mm: target,
            boundary_edge_length_mm: median(&boundary_lengths),
            characteristic_edge_length_before_mm: before_characteristic,
            characteristic_edge_length_after_mm: after_characteristic,
            subdivided_faces,
            collapsed_edges,
            geometric_deviation_mm: geometric_deviation,
            boundary_preserved,
            selection_boundary_preserved: boundary_preserved,
            iterations: options.max_iterations,
            resource_work_units: work_units,
            topology_passes,
            resource_budget_units: NORMALIZE_DENSITY_MAX_WORK_UNITS,
            issues,
        },
    })
}

#[derive(Clone, Debug)]
struct EdgeData {
    edges: Vec<(u32, u32)>,
    face_edge_ids: Vec<[usize; 3]>,
    counts: Vec<usize>,
}

fn validate_options(options: &NormalizeDensityOptions) -> Result<(), CoreError> {
    validate_radius(options.radius_mm)?;
    if let Some(target) = options.target_edge_length_mm
        && (!target.is_finite()
            || !(NORMALIZE_DENSITY_TARGET_MIN_MM..=NORMALIZE_DENSITY_TARGET_MAX_MM)
                .contains(&target))
    {
        return Err(CoreError::MeshOperation(
            "target_edge_length_mm is outside the bounded range".to_owned(),
        ));
    }
    if !(1..=NORMALIZE_DENSITY_MAX_ITERATIONS).contains(&options.max_iterations) {
        return Err(CoreError::OperationResourceLimit {
            operation: "normalize_density".to_owned(),
            reason: format!(
                "max_iterations must be between 1 and {}",
                NORMALIZE_DENSITY_MAX_ITERATIONS
            ),
        });
    }
    options.limits.validate("normalize_density")
}

fn validate_radius(radius: f32) -> Result<(), CoreError> {
    if !radius.is_finite()
        || !(NORMALIZE_DENSITY_RADIUS_MIN_MM..=NORMALIZE_DENSITY_RADIUS_MAX_MM).contains(&radius)
    {
        return Err(CoreError::MeshOperation(
            "radius_mm is outside the bounded range".to_owned(),
        ));
    }
    Ok(())
}

fn normalize_selection(
    mesh: &Mesh,
    face_indices: Option<&[usize]>,
    centers_mm: Option<&[[f32; 3]]>,
    radius: f32,
) -> Result<Vec<usize>, CoreError> {
    let mut selected = BTreeSet::new();
    if let Some(indices) = face_indices {
        if indices.is_empty() {
            return Err(CoreError::MeshOperation(
                "face_indices must contain at least one face".to_owned(),
            ));
        }
        if indices.len() > NORMALIZE_DENSITY_MAX_REGION_FACES {
            return Err(CoreError::OperationResourceLimit {
                operation: "normalize_density".to_owned(),
                reason: "face_indices exceeds the public region limit".to_owned(),
            });
        }
        for index in indices {
            if *index >= mesh.face_count() {
                return Err(CoreError::MeshOperation(format!(
                    "face index {index} is outside the mesh"
                )));
            }
            selected.insert(*index);
        }
    } else if centers_mm.is_none() {
        selected.extend(0..mesh.face_count());
    }
    if let Some(centers) = centers_mm {
        selected.extend(select_face_region(mesh, centers, radius)?);
    }
    Ok(selected.into_iter().collect())
}

fn edge_data(faces: &[[u32; 3]]) -> EdgeData {
    let mut owners = BTreeMap::<(u32, u32), Vec<usize>>::new();
    for (face_index, face) in faces.iter().copied().enumerate() {
        for key in [
            ordered_edge(face[0], face[1]),
            ordered_edge(face[1], face[2]),
            ordered_edge(face[2], face[0]),
        ] {
            owners.entry(key).or_default().push(face_index);
        }
    }
    let edges = owners.keys().copied().collect::<Vec<_>>();
    let ids = edges
        .iter()
        .enumerate()
        .map(|(index, edge)| (*edge, index))
        .collect::<BTreeMap<_, _>>();
    let face_edge_ids = faces
        .iter()
        .map(|face| {
            [
                ids.get(&ordered_edge(face[0], face[1]))
                    .copied()
                    .unwrap_or(0),
                ids.get(&ordered_edge(face[1], face[2]))
                    .copied()
                    .unwrap_or(0),
                ids.get(&ordered_edge(face[2], face[0]))
                    .copied()
                    .unwrap_or(0),
            ]
        })
        .collect();
    let counts = edges
        .iter()
        .map(|edge| owners.get(edge).map_or(0, Vec::len))
        .collect();
    EdgeData {
        edges,
        face_edge_ids,
        counts,
    }
}

fn selected_edge_counts(data: &EdgeData, selected: &[bool]) -> Vec<usize> {
    let mut counts = vec![0_usize; data.edges.len()];
    for (face_index, face_edges) in data.face_edge_ids.iter().enumerate() {
        if !selected.get(face_index).copied().unwrap_or(false) {
            continue;
        }
        for edge_id in face_edges {
            if let Some(count) = counts.get_mut(*edge_id) {
                *count = count.saturating_add(1);
            }
        }
    }
    counts
}

fn edge_lengths(vertices: &[[f32; 3]], data: &EdgeData) -> Vec<f32> {
    data.edges
        .iter()
        .map(|(first, second)| {
            match (
                vertices.get(*first as usize),
                vertices.get(*second as usize),
            ) {
                (Some(first), Some(second)) => length(subtract(*first, *second)),
                _ => f32::NAN,
            }
        })
        .collect()
}

fn boundary_edge_flags(data: &EdgeData, selected_counts: &[usize]) -> Vec<bool> {
    data.counts
        .iter()
        .enumerate()
        .map(|(index, count)| {
            selected_counts.get(index).copied().unwrap_or(0) > 0
                && (selected_counts.get(index).copied().unwrap_or(0) < *count || *count == 1)
        })
        .collect()
}

fn boundary_vertices(
    vertices: &[[f32; 3]],
    data: &EdgeData,
    boundary_flags: &[bool],
) -> Vec<[f32; 3]> {
    let mut indices = BTreeSet::new();
    for (index, flag) in boundary_flags.iter().copied().enumerate() {
        if flag && let Some((first, second)) = data.edges.get(index) {
            indices.insert(*first);
            indices.insert(*second);
        }
    }
    indices
        .into_iter()
        .filter_map(|index| vertices.get(index as usize).copied())
        .collect()
}

fn boundary_vertices_present(
    mesh: &Mesh,
    data: &EdgeData,
    selected_counts: &[usize],
    before: &[[f32; 3]],
) -> bool {
    if before.is_empty() {
        return true;
    }
    let flags = boundary_edge_flags(data, selected_counts);
    let mut current = BTreeSet::new();
    for (index, flag) in flags.iter().copied().enumerate() {
        if flag && let Some((first, second)) = data.edges.get(index) {
            current.insert(*first);
            current.insert(*second);
        }
    }
    before.iter().all(|point| {
        current.iter().any(|index| {
            mesh.vertices
                .get(*index as usize)
                .is_some_and(|candidate| length(subtract(*candidate, *point)) <= 1.0e-5)
        })
    })
}

fn subdivide_region(
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    selected: &[bool],
    target: f32,
) -> Result<SubdivideOutput, CoreError> {
    let data = edge_data(faces);
    let selected_counts = selected_edge_counts(&data, selected);
    let lengths = edge_lengths(vertices, &data);
    let refine = data
        .counts
        .iter()
        .enumerate()
        .filter_map(|(index, count)| {
            (*count >= 2 && selected_counts[index] == *count && lengths[index] > target * 1.05)
                .then_some(index)
        })
        .collect::<BTreeSet<_>>();
    if refine.is_empty() {
        return Ok((vertices.to_vec(), faces.to_vec(), selected.to_vec(), 0));
    }
    let mut vertex_list = vertices.to_vec();
    let mut midpoint_ids = BTreeMap::<(u32, u32), u32>::new();
    for edge_id in &refine {
        let Some((first, second)) = data.edges.get(*edge_id) else {
            continue;
        };
        let (Some(first_point), Some(second_point)) = (
            vertices.get(*first as usize),
            vertices.get(*second as usize),
        ) else {
            continue;
        };
        let index =
            u32::try_from(vertex_list.len()).map_err(|_| CoreError::OperationResourceLimit {
                operation: "normalize_density".to_owned(),
                reason: "density midpoint index overflow".to_owned(),
            })?;
        midpoint_ids.insert((*first, *second), index);
        vertex_list.push([
            (first_point[0] + second_point[0]) * 0.5,
            (first_point[1] + second_point[1]) * 0.5,
            (first_point[2] + second_point[2]) * 0.5,
        ]);
    }
    let mut output_faces = Vec::new();
    let mut output_selected = Vec::new();
    for (face_index, face) in faces.iter().copied().enumerate() {
        if !selected.get(face_index).copied().unwrap_or(false) {
            output_faces.push(face);
            output_selected.push(false);
            continue;
        }
        let edge_pairs = [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])];
        let split = edge_pairs.map(|pair| {
            let key = ordered_edge(pair.0, pair.1);
            refine
                .iter()
                .any(|edge_id| data.edges.get(*edge_id).copied() == Some(key))
        });
        let count = split.iter().filter(|value| **value).count();
        if count == 0 {
            output_faces.push(face);
            output_selected.push(true);
            continue;
        }
        let source_normal = face_normal(vertices, face);
        let parts = split_face(face, edge_pairs, split, &midpoint_ids)?;
        let oriented = orient_parts(parts, &vertex_list, source_normal);
        output_selected.extend(std::iter::repeat_n(true, oriented.len()));
        output_faces.extend(oriented);
    }
    let added = output_faces.len().saturating_sub(faces.len());
    Ok((vertex_list, output_faces, output_selected, added))
}

fn split_face(
    face: [u32; 3],
    edge_pairs: [(u32, u32); 3],
    split: [bool; 3],
    midpoint_ids: &BTreeMap<(u32, u32), u32>,
) -> Result<Vec<[u32; 3]>, CoreError> {
    let midpoint = |pair: (u32, u32)| midpoint_ids.get(&ordered_edge(pair.0, pair.1)).copied();
    let count = split.iter().filter(|value| **value).count();
    if count == 1 {
        let edge_index = split.iter().position(|value| *value).unwrap_or(0);
        let (first, second) = edge_pairs[edge_index];
        let opposite = [face[0], face[1], face[2]]
            .into_iter()
            .find(|value| *value != first && *value != second)
            .unwrap_or(face[0]);
        let middle = midpoint((first, second)).ok_or_else(|| {
            CoreError::MeshOperation("density midpoint missing for one-edge split".to_owned())
        })?;
        return Ok(vec![[first, middle, opposite], [middle, second, opposite]]);
    }
    if count == 2 {
        let enabled = split
            .iter()
            .enumerate()
            .filter_map(|(index, value)| (*value).then_some(edge_pairs[index]))
            .collect::<Vec<_>>();
        let common = common_vertex(enabled[0], enabled[1]).ok_or_else(|| {
            CoreError::MeshOperation("density two-edge split has no common vertex".to_owned())
        })?;
        let first_outer = other_vertex(enabled[0], common);
        let second_outer = other_vertex(enabled[1], common);
        let first_mid = midpoint((common, first_outer))
            .ok_or_else(|| CoreError::MeshOperation("density first midpoint missing".to_owned()))?;
        let second_mid = midpoint((common, second_outer)).ok_or_else(|| {
            CoreError::MeshOperation("density second midpoint missing".to_owned())
        })?;
        return Ok(vec![
            [first_outer, first_mid, second_outer],
            [first_mid, common, second_mid],
            [first_mid, second_mid, second_outer],
        ]);
    }
    let ab = midpoint((face[0], face[1]))
        .ok_or_else(|| CoreError::MeshOperation("density AB midpoint missing".to_owned()))?;
    let bc = midpoint((face[1], face[2]))
        .ok_or_else(|| CoreError::MeshOperation("density BC midpoint missing".to_owned()))?;
    let ca = midpoint((face[2], face[0]))
        .ok_or_else(|| CoreError::MeshOperation("density CA midpoint missing".to_owned()))?;
    Ok(vec![
        [face[0], ab, ca],
        [ab, face[1], bc],
        [ca, bc, face[2]],
        [ab, bc, ca],
    ])
}

fn collapse_one_edge(
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    selected: &[bool],
    target: f32,
) -> Result<CollapseOutput, CoreError> {
    let data = edge_data(faces);
    let selected_counts = selected_edge_counts(&data, selected);
    let lengths = edge_lengths(vertices, &data);
    let boundary_flags = boundary_edge_flags(&data, &selected_counts);
    let mut boundary_vertices = BTreeSet::new();
    for (index, flag) in boundary_flags.iter().copied().enumerate() {
        if flag && let Some((first, second)) = data.edges.get(index) {
            boundary_vertices.insert(*first);
            boundary_vertices.insert(*second);
        }
    }
    let mut candidates = data
        .counts
        .iter()
        .enumerate()
        .filter(|(index, count)| {
            **count >= 2 && selected_counts[*index] == **count && lengths[*index] < target * 0.75
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    candidates.sort_by(|first, second| {
        lengths[*first]
            .total_cmp(&lengths[*second])
            .then(first.cmp(second))
    });
    for edge_id in candidates {
        let Some((first, second)) = data.edges.get(edge_id).copied() else {
            continue;
        };
        if boundary_vertices.contains(&first) || boundary_vertices.contains(&second) {
            continue;
        }
        let edge_length = lengths[edge_id];
        let Some(first_point) = vertices.get(first as usize) else {
            continue;
        };
        let Some(second_point) = vertices.get(second as usize) else {
            continue;
        };
        let midpoint = [
            (first_point[0] + second_point[0]) * 0.5,
            (first_point[1] + second_point[1]) * 0.5,
            (first_point[2] + second_point[2]) * 0.5,
        ];
        let mut candidate_vertices = vertices.to_vec();
        candidate_vertices[first as usize] = midpoint;
        let mut candidate_faces = Vec::new();
        let mut candidate_flags = Vec::new();
        let mut seen = BTreeSet::<[u32; 3]>::new();
        for (face, flag) in faces.iter().copied().zip(selected.iter().copied()) {
            let remap = |index| if index == second { first } else { index };
            let candidate = [remap(face[0]), remap(face[1]), remap(face[2])];
            if candidate[0] == candidate[1]
                || candidate[1] == candidate[2]
                || candidate[2] == candidate[0]
            {
                continue;
            }
            let Some(a) = candidate_vertices.get(candidate[0] as usize) else {
                continue;
            };
            let Some(b) = candidate_vertices.get(candidate[1] as usize) else {
                continue;
            };
            let Some(c) = candidate_vertices.get(candidate[2] as usize) else {
                continue;
            };
            let normal = cross(subtract(*b, *a), subtract(*c, *a));
            if length(normal) <= 1.0e-5 {
                continue;
            }
            let mut canonical = candidate;
            canonical.sort_unstable();
            if seen.insert(canonical) {
                candidate_faces.push(candidate);
                candidate_flags.push(flag);
            }
        }
        if candidate_faces.is_empty() {
            continue;
        }
        let (compact_vertices, compact_faces, compact_flags) =
            compact_with_flags(&candidate_vertices, &candidate_faces, &candidate_flags)?;
        return Ok((
            compact_vertices,
            compact_faces,
            compact_flags,
            edge_length * 0.5,
            true,
        ));
    }
    Ok((
        vertices.to_vec(),
        faces.to_vec(),
        selected.to_vec(),
        0.0,
        false,
    ))
}

fn compact_with_flags(
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    flags: &[bool],
) -> Result<DensityMeshBuffers, CoreError> {
    let mut remap = vec![None::<u32>; vertices.len()];
    let mut compact_vertices = Vec::new();
    for face in faces {
        for index in face {
            let slot = remap.get_mut(*index as usize).ok_or_else(|| {
                CoreError::InvalidMesh("density face references a missing vertex".to_owned())
            })?;
            if slot.is_none() {
                let value = u32::try_from(compact_vertices.len()).map_err(|_| {
                    CoreError::OperationResourceLimit {
                        operation: "normalize_density".to_owned(),
                        reason: "density compact vertex index overflow".to_owned(),
                    }
                })?;
                *slot = Some(value);
                compact_vertices.push(vertices[*index as usize]);
            }
        }
    }
    let mut compact_faces = Vec::with_capacity(faces.len());
    for face in faces {
        compact_faces.push([
            remap[face[0] as usize].unwrap_or(0),
            remap[face[1] as usize].unwrap_or(0),
            remap[face[2] as usize].unwrap_or(0),
        ]);
    }
    Ok((compact_vertices, compact_faces, flags.to_vec()))
}

fn charge_work(
    work_units: &mut u64,
    topology_passes: &mut usize,
    faces: usize,
    label: &str,
    cancel: &mut NormalizeDensityCancel<'_>,
    limits: &MeshOperationLimits,
) -> Result<(), CoreError> {
    call_cancel(cancel)?;
    *work_units = work_units
        .checked_add(
            u64::try_from(faces)
                .map_err(|_| CoreError::MeshOperation("density work count overflow".to_owned()))?,
        )
        .ok_or_else(|| CoreError::MeshOperation("density work count overflow".to_owned()))?;
    *topology_passes = topology_passes.saturating_add(1);
    if *topology_passes > NORMALIZE_DENSITY_MAX_EDGE_REBUILDS {
        return Err(CoreError::OperationResourceLimit {
            operation: "normalize_density".to_owned(),
            reason: format!("topology-work limit reached during {label}"),
        });
    }
    if *work_units > NORMALIZE_DENSITY_MAX_WORK_UNITS {
        return Err(CoreError::OperationResourceLimit {
            operation: "normalize_density".to_owned(),
            reason: format!("density budget reached during {label}"),
        });
    }
    limits.check_work("normalize_density", *work_units)
}

fn call_cancel(cancel: &mut NormalizeDensityCancel<'_>) -> Result<(), CoreError> {
    if let Some(callback) = cancel.as_mut() {
        callback()?;
    }
    Ok(())
}

fn call_progress(progress: &mut NormalizeDensityProgress<'_>, percent: u8, message: &str) {
    if let Some(callback) = progress.as_mut() {
        callback(percent, message);
    }
}

fn orient_parts(
    parts: Vec<[u32; 3]>,
    vertices: &[[f32; 3]],
    source_normal: [f32; 3],
) -> Vec<[u32; 3]> {
    parts
        .into_iter()
        .map(|mut face| {
            let normal = face_normal(vertices, face);
            if dot(normal, source_normal) < 0.0 {
                face = [face[0], face[2], face[1]];
            }
            face
        })
        .collect()
}

fn face_normal(vertices: &[[f32; 3]], face: [u32; 3]) -> [f32; 3] {
    let (Some(first), Some(second), Some(third)) = (
        vertices.get(face[0] as usize),
        vertices.get(face[1] as usize),
        vertices.get(face[2] as usize),
    ) else {
        return [0.0; 3];
    };
    cross(subtract(*second, *first), subtract(*third, *first))
}

fn common_vertex(first: (u32, u32), second: (u32, u32)) -> Option<u32> {
    [first.0, first.1]
        .into_iter()
        .find(|value| *value == second.0 || *value == second.1)
}

fn other_vertex(pair: (u32, u32), common: u32) -> u32 {
    if pair.0 == common { pair.1 } else { pair.0 }
}

fn median(values: &[f32]) -> Option<f32> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.retain(|value| value.is_finite());
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(f32::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        Some((sorted[middle - 1] + sorted[middle]) * 0.5)
    } else {
        Some(sorted[middle])
    }
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

fn length(value: [f32; 3]) -> f32 {
    dot(value, value).sqrt()
}
