//! Deterministic bounded triangle-mesh simplification.
//!
//! The Python reference delegates to trimesh's quadric decimator; the Rust
//! core uses the same family (OF-51): quadric-error edge collapse with a
//! priority queue, so the edge that changes the shape least goes first and
//! the whole run is one pass. Each collapse keeps the surface manifold (the
//! link condition) and turns no face over. The run stops at the face target,
//! or earlier when the next collapse would move the surface by more than
//! [`SIMPLIFY_SHAPE_TOLERANCE`] of the object's size, so low-poly shapes
//! such as a cube keep their form. Ties break on vertex indices, so the
//! result is deterministic on every adapter.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use crate::{CoreError, Mesh, MeshOperationLimits};

/// The default simplification target policy from the Python reference.
pub const DEFAULT_SIMPLIFY_MIN_FACES: usize = 100;

/// Largest surface movement a collapse may cause, as a fraction of the
/// object's bounding-box diagonal (1 %).
pub const SIMPLIFY_SHAPE_TOLERANCE: f64 = 0.01;

/// Face target for removing `percent` (1-95) of a mesh's faces, as the
/// desktop Simplify slider and the MCP `percent` input use it.
pub fn simplify_target_for_percent(face_count: usize, percent: u32) -> Result<usize, CoreError> {
    if !(1..=95).contains(&percent) {
        return Err(CoreError::MeshOperation(
            "percent must be between 1 and 95".to_owned(),
        ));
    }
    let keep = (face_count as u128 * u128::from(100 - percent)).div_ceil(100);
    Ok(usize::try_from(keep).unwrap_or(face_count).min(face_count))
}

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

/// Simplifies toward the optional face target with explicit limits. An
/// explicit target is honoured even where it changes the shape (as the
/// Python reference does); see [`simplify_mesh_by_percent`] for the guarded
/// desktop mode.
pub fn simplify_mesh_with_options(
    mesh: &Mesh,
    face_count: Option<usize>,
    limits: MeshOperationLimits,
) -> Result<SimplifyResult, CoreError> {
    simplify_guarded(mesh, face_count, limits, false, None)
}

/// Removes `percent` (1-95) of the faces, like the desktop Simplify slider
/// (OF-51), stopping early rather than moving the surface by more than
/// [`SIMPLIFY_SHAPE_TOLERANCE`] of the object's size.
pub fn simplify_mesh_by_percent(
    mesh: &Mesh,
    percent: u32,
    limits: MeshOperationLimits,
) -> Result<SimplifyResult, CoreError> {
    simplify_mesh_by_percent_in_area(mesh, percent, None, limits)
}

/// Removes about `percent` (1-95) of the faces in `area` (face indices; the
/// whole mesh when `None`), like the desktop Simplify window's "Area I brush
/// over" (OF-60). Only edges whose every touching face lies in the area may
/// collapse, so the area's border stays exactly where it was and joined to
/// the rest of the model, and nothing outside the area changes. The same
/// shape guard as [`simplify_mesh_by_percent`] applies.
pub fn simplify_mesh_by_percent_in_area(
    mesh: &Mesh,
    percent: u32,
    area: Option<&[usize]>,
    limits: MeshOperationLimits,
) -> Result<SimplifyResult, CoreError> {
    let Some(area) = area else {
        let target = simplify_target_for_percent(mesh.face_count(), percent)?;
        return simplify_guarded(mesh, Some(target), limits, true, None);
    };
    let face_count = mesh.face_count();
    let mut chosen = vec![false; face_count];
    for face in area {
        let Some(flag) = chosen.get_mut(*face) else {
            return Err(CoreError::MeshOperation(format!(
                "face index {face} is outside the mesh's {face_count} faces"
            )));
        };
        *flag = true;
    }
    let area_faces = chosen.iter().filter(|flag| **flag).count();
    if area_faces == 0 {
        return Err(CoreError::MeshOperation(
            "simplify needs at least one face in the area".to_owned(),
        ));
    }
    let keep = simplify_target_for_percent(area_faces, percent)?;
    let target = face_count - (area_faces - keep);
    // A vertex is free only when every face around it is in the area.
    let mut free = vec![true; mesh.vertex_count()];
    for (face, inside) in mesh.faces.iter().zip(&chosen) {
        if !inside {
            for vertex in face {
                if let Some(flag) = free.get_mut(*vertex as usize) {
                    *flag = false;
                }
            }
        }
    }
    simplify_guarded(mesh, Some(target), limits, true, Some(free))
}

/// Removes as many faces as possible while no collapse moves the surface
/// by more than `tolerance_mm` (used for Hollow's inner shell, OF-54).
/// Returns `None` when the mesh is not a clean closed manifold or nothing
/// could be removed, so the caller keeps its input.
pub(crate) fn simplify_within_tolerance(
    mesh: &Mesh,
    tolerance_mm: f64,
    limits: &MeshOperationLimits,
) -> Option<Mesh> {
    let mut edges = BTreeMap::<(u32, u32), u8>::new();
    for face in &mesh.faces {
        for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let count = edges
                .entry((first.min(second), first.max(second)))
                .or_default();
            *count = count.saturating_add(1);
        }
    }
    if edges.values().any(|count| *count != 2) {
        return None;
    }
    let work = u64::try_from(mesh.face_count())
        .ok()?
        .checked_mul(SIMPLIFY_WORK_PER_FACE)?;
    limits.check_work("simplify", work).ok()?;
    let mut state = Decimator::new(mesh, None);
    let outcome = state.run(4, tolerance_mm * tolerance_mm);
    if outcome.collapses == 0 {
        return None;
    }
    state.into_mesh().ok()
}

fn simplify_guarded(
    mesh: &Mesh,
    face_count: Option<usize>,
    limits: MeshOperationLimits,
    keep_shape: bool,
    free: Option<Vec<bool>>,
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

    let work_estimate = u64::try_from(before_faces)
        .ok()
        .and_then(|faces| faces.checked_mul(SIMPLIFY_WORK_PER_FACE))
        .ok_or_else(|| CoreError::MeshOperation("simplify work count overflow".to_owned()))?;
    limits.check_work("simplify", work_estimate)?;
    let mut state = Decimator::new(mesh, free);
    let maximum_cost = if keep_shape {
        let tolerance = state.diagonal() * SIMPLIFY_SHAPE_TOLERANCE;
        tolerance * tolerance
    } else {
        f64::INFINITY
    };
    let outcome = state.run(target, maximum_cost);
    let mut warnings = Vec::new();
    if state.alive_faces > target {
        warnings.push(if outcome.stopped_for_shape {
            format!(
                "stopped at {} faces to keep the shape (target {target})",
                state.alive_faces
            )
        } else {
            format!(
                "no further edge could be removed safely at {} faces (target {target})",
                state.alive_faces
            )
        });
    }
    let current = if outcome.collapses == 0 {
        mesh.clone()
    } else {
        state.into_mesh()?
    };
    limits.check_mesh("simplify", &current)?;
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
            collapsed_edges: outcome.collapses,
            iterations: outcome.collapses,
            work_units: work_estimate,
            geometric_deviation_mm: outcome.maximum_error.sqrt() as f32,
            before_volume_mm3,
            after_volume_mm3,
            warnings,
        },
    })
}

/// Counted work per input face (quadrics, queue entries and collapses).
const SIMPLIFY_WORK_PER_FACE: u64 = 24;

/// Symmetric 4x4 quadric: xx, xy, xz, xw, yy, yz, yw, zz, zw, ww.
type Quadric = [f64; 10];

fn plane_quadric(normal: [f64; 3], point: [f64; 3], weight: f64) -> Quadric {
    let [a, b, c] = normal;
    let d = -(a * point[0] + b * point[1] + c * point[2]);
    [
        a * a * weight,
        a * b * weight,
        a * c * weight,
        a * d * weight,
        b * b * weight,
        b * c * weight,
        b * d * weight,
        c * c * weight,
        c * d * weight,
        d * d * weight,
    ]
}

fn add_quadric(first: &mut Quadric, second: &Quadric) {
    for (value, other) in first.iter_mut().zip(second) {
        *value += other;
    }
}

fn quadric_error(q: &Quadric, p: [f64; 3]) -> f64 {
    let [x, y, z] = p;
    (q[0] * x * x
        + 2.0 * q[1] * x * y
        + 2.0 * q[2] * x * z
        + 2.0 * q[3] * x
        + q[4] * y * y
        + 2.0 * q[5] * y * z
        + 2.0 * q[6] * y
        + q[7] * z * z
        + 2.0 * q[8] * z
        + q[9])
        .max(0.0)
}

fn determinant(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

/// Point minimising the quadric, if the system is well conditioned.
fn quadric_minimum(q: &Quadric) -> Option<[f64; 3]> {
    let m = [[q[0], q[1], q[2]], [q[1], q[4], q[5]], [q[2], q[5], q[7]]];
    let rhs = [-q[3], -q[6], -q[8]];
    let det = determinant(m);
    let scale = q[0].abs() + q[4].abs() + q[7].abs();
    if !det.is_finite() || det.abs() <= 1.0e-9 * scale.powi(3).max(1.0e-30) {
        return None;
    }
    let solve = |column: usize| {
        let mut copy = m;
        for (row, value) in rhs.iter().enumerate() {
            copy[row][column] = *value;
        }
        determinant(copy) / det
    };
    let point = [solve(0), solve(1), solve(2)];
    point.iter().all(|value| value.is_finite()).then_some(point)
}

fn sub3(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[0] - second[0],
        first[1] - second[1],
        first[2] - second[2],
    ]
}

fn cross3(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn dot3(first: [f64; 3], second: [f64; 3]) -> f64 {
    first[0] * second[0] + first[1] * second[1] + first[2] * second[2]
}

fn length3(value: [f64; 3]) -> f64 {
    dot3(value, value).sqrt()
}

/// One queued collapse; ordered so the cheapest (then lowest indices) pops first.
#[derive(Clone, Copy, Debug)]
struct Candidate {
    cost: f64,
    /// Mean squared distance per surface plane, for the shape guard.
    mean_cost: f64,
    first: u32,
    second: u32,
    first_version: u32,
    second_version: u32,
    target: [f64; 3],
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.first.cmp(&self.first))
            .then_with(|| other.second.cmp(&self.second))
    }
}

struct RunOutcome {
    collapses: usize,
    maximum_error: f64,
    stopped_for_shape: bool,
}

struct Decimator {
    vertices: Vec<[f64; 3]>,
    quadrics: Vec<Quadric>,
    planes: Vec<f64>,
    faces: Vec<[u32; 3]>,
    face_alive: Vec<bool>,
    vertex_faces: Vec<Vec<u32>>,
    vertex_alive: Vec<bool>,
    /// Vertices that may move (OF-60 area mode); `None` means all.
    free: Option<Vec<bool>>,
    version: Vec<u32>,
    alive_faces: usize,
    queue: BinaryHeap<Candidate>,
}

impl Decimator {
    fn new(mesh: &Mesh, free: Option<Vec<bool>>) -> Self {
        let vertices: Vec<[f64; 3]> = mesh
            .vertices
            .iter()
            .map(|v| [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])])
            .collect();
        let mut quadrics = vec![[0.0_f64; 10]; vertices.len()];
        let mut planes = vec![0.0_f64; vertices.len()];
        let mut vertex_faces = vec![Vec::new(); vertices.len()];
        let mut edge_faces = BTreeMap::<(u32, u32), Vec<u32>>::new();
        for (index, face) in mesh.faces.iter().enumerate() {
            let face_index = index as u32;
            let [a, b, c] = face.map(|vertex| vertices[vertex as usize]);
            let normal = cross3(sub3(b, a), sub3(c, a));
            let magnitude = length3(normal);
            if magnitude > 1.0e-18 {
                let unit = normal.map(|value| value / magnitude);
                let quadric = plane_quadric(unit, a, 1.0);
                for vertex in face {
                    add_quadric(&mut quadrics[*vertex as usize], &quadric);
                    planes[*vertex as usize] += 1.0;
                }
            }
            for vertex in face {
                vertex_faces[*vertex as usize].push(face_index);
            }
            for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
                edge_faces
                    .entry((first.min(second), first.max(second)))
                    .or_default()
                    .push(face_index);
            }
        }
        // Open borders get a stiff plane through the edge, perpendicular to
        // its face, so holes and sheet edges keep their outline.
        for ((first, second), owners) in &edge_faces {
            if let [only] = owners.as_slice() {
                let face = mesh.faces[*only as usize];
                let [a, b, c] = face.map(|vertex| vertices[vertex as usize]);
                let face_normal = cross3(sub3(b, a), sub3(c, a));
                let p = vertices[*first as usize];
                let edge = sub3(vertices[*second as usize], p);
                let normal = cross3(edge, face_normal);
                let magnitude = length3(normal);
                if magnitude > 1.0e-18 {
                    let unit = normal.map(|value| value / magnitude);
                    let quadric = plane_quadric(unit, p, 10.0);
                    add_quadric(&mut quadrics[*first as usize], &quadric);
                    add_quadric(&mut quadrics[*second as usize], &quadric);
                }
            }
        }
        let mut decimator = Self {
            vertex_alive: vec![true; vertices.len()],
            free,
            version: vec![0; vertices.len()],
            vertices,
            quadrics,
            planes,
            faces: mesh.faces.clone(),
            face_alive: vec![true; mesh.faces.len()],
            vertex_faces,
            alive_faces: mesh.faces.len(),
            queue: BinaryHeap::new(),
        };
        for (first, second) in edge_faces.keys() {
            decimator.push_edge(*first, *second);
        }
        decimator
    }

    fn diagonal(&self) -> f64 {
        let mut minimum = [f64::MAX; 3];
        let mut maximum = [f64::MIN; 3];
        for vertex in &self.vertices {
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(vertex[axis]);
                maximum[axis] = maximum[axis].max(vertex[axis]);
            }
        }
        length3(sub3(maximum, minimum))
    }

    fn push_edge(&mut self, first: u32, second: u32) {
        let (a, b) = (first.min(second), first.max(second));
        if let Some(free) = &self.free
            && !(free.get(a as usize).copied().unwrap_or(false)
                && free.get(b as usize).copied().unwrap_or(false))
        {
            return;
        }
        let mut quadric = self.quadrics[a as usize];
        add_quadric(&mut quadric, &self.quadrics[b as usize]);
        let pa = self.vertices[a as usize];
        let pb = self.vertices[b as usize];
        let midpoint = [
            (pa[0] + pb[0]) * 0.5,
            (pa[1] + pb[1]) * 0.5,
            (pa[2] + pb[2]) * 0.5,
        ];
        let edge_length = length3(sub3(pb, pa));
        let mut best = (quadric_error(&quadric, midpoint), midpoint);
        for candidate in [pa, pb] {
            let error = quadric_error(&quadric, candidate);
            if error < best.0 {
                best = (error, candidate);
            }
        }
        if let Some(optimal) = quadric_minimum(&quadric)
            && length3(sub3(optimal, midpoint)) <= edge_length
        {
            let error = quadric_error(&quadric, optimal);
            if error <= best.0 {
                best = (error, optimal);
            }
        }
        let planes = (self.planes[a as usize] + self.planes[b as usize]).max(1.0);
        self.queue.push(Candidate {
            cost: best.0,
            mean_cost: best.0 / planes,
            first: a,
            second: b,
            first_version: self.version[a as usize],
            second_version: self.version[b as usize],
            target: best.1,
        });
    }

    fn live_faces(&self, vertex: u32) -> impl Iterator<Item = u32> + '_ {
        self.vertex_faces[vertex as usize]
            .iter()
            .copied()
            .filter(|face| self.face_alive[*face as usize])
    }

    fn neighbours(&self, vertex: u32) -> BTreeSet<u32> {
        self.live_faces(vertex)
            .flat_map(|face| self.faces[face as usize])
            .filter(|other| *other != vertex)
            .collect()
    }

    /// Whether collapsing `first`-`second` to `target` keeps the surface
    /// manifold and turns no face over.
    fn collapse_is_safe(&self, first: u32, second: u32, target: [f64; 3]) -> bool {
        let shared = self
            .live_faces(first)
            .filter(|face| self.faces[*face as usize].contains(&second))
            .count();
        if shared == 0 {
            return false;
        }
        let common = self
            .neighbours(first)
            .intersection(&self.neighbours(second))
            .count();
        if common != shared {
            return false;
        }
        for vertex in [first, second] {
            for face in self.live_faces(vertex) {
                let corners = self.faces[face as usize];
                if corners.contains(&first) && corners.contains(&second) {
                    continue;
                }
                let before = corners.map(|corner| self.vertices[corner as usize]);
                let after = corners.map(|corner| {
                    if corner == first || corner == second {
                        target
                    } else {
                        self.vertices[corner as usize]
                    }
                });
                let normal_before = cross3(sub3(before[1], before[0]), sub3(before[2], before[0]));
                let normal_after = cross3(sub3(after[1], after[0]), sub3(after[2], after[0]));
                let area_after = length3(normal_after);
                if area_after <= 1.0e-12 * (1.0 + length3(normal_before)) {
                    return false;
                }
                if dot3(normal_before, normal_after) <= 0.2 * length3(normal_before) * area_after {
                    return false;
                }
            }
        }
        true
    }

    fn run(&mut self, target_faces: usize, maximum_cost: f64) -> RunOutcome {
        let mut outcome = RunOutcome {
            collapses: 0,
            maximum_error: 0.0,
            stopped_for_shape: false,
        };
        while self.alive_faces > target_faces.max(4) {
            let Some(candidate) = self.queue.pop() else {
                break;
            };
            let (a, b) = (candidate.first, candidate.second);
            if !self.vertex_alive[a as usize]
                || !self.vertex_alive[b as usize]
                || self.version[a as usize] != candidate.first_version
                || self.version[b as usize] != candidate.second_version
            {
                continue;
            }
            if candidate.mean_cost > maximum_cost {
                outcome.stopped_for_shape = true;
                break;
            }
            if !self.collapse_is_safe(a, b, candidate.target) {
                continue;
            }
            // Collapse b into a.
            let removed: Vec<u32> = self
                .live_faces(a)
                .filter(|face| self.faces[*face as usize].contains(&b))
                .collect();
            for face in &removed {
                self.face_alive[*face as usize] = false;
            }
            self.alive_faces -= removed.len();
            let moved: Vec<u32> = self.live_faces(b).collect();
            for face in moved {
                for corner in &mut self.faces[face as usize] {
                    if *corner == b {
                        *corner = a;
                    }
                }
                self.vertex_faces[a as usize].push(face);
            }
            self.vertex_alive[b as usize] = false;
            self.vertices[a as usize] = candidate.target;
            let quadric_b = self.quadrics[b as usize];
            add_quadric(&mut self.quadrics[a as usize], &quadric_b);
            self.planes[a as usize] += self.planes[b as usize];
            self.version[a as usize] = self.version[a as usize].wrapping_add(1);
            let live: Vec<u32> = self.live_faces(a).collect();
            self.vertex_faces[a as usize] = live;
            outcome.collapses += 1;
            outcome.maximum_error = outcome.maximum_error.max(candidate.mean_cost);
            for neighbour in self.neighbours(a) {
                self.push_edge(a, neighbour);
            }
        }
        outcome
    }

    fn into_mesh(self) -> Result<Mesh, CoreError> {
        let vertices: Vec<[f32; 3]> = self
            .vertices
            .iter()
            .map(|v| [v[0] as f32, v[1] as f32, v[2] as f32])
            .collect();
        let faces: Vec<[u32; 3]> = self
            .faces
            .iter()
            .zip(&self.face_alive)
            .filter_map(|(face, alive)| alive.then_some(*face))
            .collect();
        compact_mesh(&vertices, &faces)
    }
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
