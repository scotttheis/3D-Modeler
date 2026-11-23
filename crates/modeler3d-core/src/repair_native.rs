//! Integer geometry for the portable Enhanced/Extended repair stages.

use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};

use crate::{CoreError, Mesh, MeshOperationLimits};

type Point = [i64; 3];
type Edge = (u32, u32);
type Owners = BTreeMap<Edge, Vec<(usize, bool)>>;

#[derive(Clone, Copy)]
pub(super) struct RepairGrid {
    origin: [f64; 3],
    step: f64,
}

impl RepairGrid {
    pub(super) fn from_mesh(mesh: &Mesh) -> Self {
        let (minimum, maximum) = mesh.bounds().unwrap_or(([0.0; 3], [0.0; 3]));
        let length = (0..3)
            .map(|axis| f64::from(maximum[axis]) - f64::from(minimum[axis]))
            .fold(0.0, f64::max);
        Self {
            origin: minimum.map(f64::from),
            step: if length.is_finite() && length > 0.0 {
                0.002_f64.min(length / 2500.0)
            } else {
                0.0
            },
        }
    }

    fn points(self, mesh: &Mesh) -> Result<Option<Vec<Point>>, CoreError> {
        if self.step == 0.0 {
            return Ok(None);
        }
        mesh.vertices
            .iter()
            .map(|point| {
                let mut key = [0; 3];
                for axis in 0..3 {
                    let coordinate =
                        ((f64::from(point[axis]) - self.origin[axis]) / self.step).round();
                    // Leave room for signed differences and neighbouring hash cells.
                    if !coordinate.is_finite() || coordinate.abs() >= (i64::MAX / 4) as f64 {
                        return Err(arithmetic_limit());
                    }
                    key[axis] = coordinate as i64;
                }
                Ok(key)
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    fn mesh(self, points: &[Point], faces: Vec<[u32; 3]>) -> Result<Mesh, CoreError> {
        let vertices = points
            .iter()
            .map(|point| {
                std::array::from_fn(|axis| {
                    (self.origin[axis] + point[axis] as f64 * self.step) as f32
                })
            })
            .collect();
        Mesh::new(vertices, faces)
    }
}

fn arithmetic_limit() -> CoreError {
    CoreError::OperationResourceLimit {
        operation: "repair".to_owned(),
        reason: "exact lattice arithmetic exceeds the integer range".to_owned(),
    }
}

fn difference(a: Point, b: Point) -> Point {
    std::array::from_fn(|axis| a[axis] - b[axis])
}

fn cross(a: Point, b: Point) -> Result<[i128; 3], CoreError> {
    let mut result = [0; 3];
    for (axis, coordinate) in result.iter_mut().enumerate() {
        let j = (axis + 1) % 3;
        let k = (axis + 2) % 3;
        *coordinate = (i128::from(a[j]) * i128::from(b[k]))
            .checked_sub(i128::from(a[k]) * i128::from(b[j]))
            .ok_or_else(arithmetic_limit)?;
    }
    Ok(result)
}

fn dot(a: [i128; 3], b: [i128; 3]) -> Result<i128, CoreError> {
    a.into_iter().zip(b).try_fold(0_i128, |sum, (a, b)| {
        sum.checked_add(a.checked_mul(b).ok_or_else(arithmetic_limit)?)
            .ok_or_else(arithmetic_limit)
    })
}

fn distance_squared(a: Point, b: Point) -> Result<i128, CoreError> {
    let delta = difference(a, b).map(i128::from);
    dot(delta, delta)
}

fn edge(a: u32, b: u32) -> Edge {
    (a.min(b), a.max(b))
}

fn edges(faces: &[[u32; 3]]) -> Owners {
    let mut result = BTreeMap::new();
    for (index, face) in faces.iter().enumerate() {
        for i in 0..3 {
            let (a, b) = (face[i], face[(i + 1) % 3]);
            result
                .entry(edge(a, b))
                .or_insert_with(Vec::new)
                .push((index, a < b));
        }
    }
    result
}

// Sorting once stores all edge owners contiguously without allocating one Vec
// per edge. Face order within each edge matches the original ordered map.
pub(super) fn edge_records(faces: &[[u32; 3]]) -> Vec<(Edge, usize, bool)> {
    let mut records = Vec::with_capacity(faces.len().saturating_mul(3));
    for (id, face) in faces.iter().enumerate() {
        for i in 0..3 {
            let (a, b) = (face[i], face[(i + 1) % 3]);
            records.push((edge(a, b), id, a <= b));
        }
    }
    records.sort_unstable();
    records
}

fn open_points(faces: &[[u32; 3]]) -> BTreeSet<u32> {
    edge_records(faces)
        .chunk_by(|a, b| a.0 == b.0)
        .filter(|owners| owners.len() == 1)
        .flat_map(|owners| [owners[0].0.0, owners[0].0.1])
        .collect()
}

fn boundary_points(owners: &Owners) -> BTreeSet<u32> {
    owners
        .iter()
        .filter(|(_, owners)| owners.len() == 1)
        .flat_map(|(&(a, b), _)| [a, b])
        .collect()
}

fn index(value: usize) -> Result<u32, CoreError> {
    u32::try_from(value).map_err(|_| arithmetic_limit())
}

// Retain source vertex order: representative tie breaking must not depend on
// the first triangle that happens to reference a point.
fn compact(mesh: &Mesh) -> Result<Mesh, CoreError> {
    let mut used = vec![false; mesh.vertices.len()];
    for face in &mesh.faces {
        for &vertex in face {
            used[vertex as usize] = true;
        }
    }
    let mut vertices = Vec::new();
    let mut remap = vec![0; used.len()];
    for (old, used) in used.into_iter().enumerate() {
        if used {
            remap[old] = index(vertices.len())?;
            vertices.push(mesh.vertices[old]);
        }
    }
    Mesh::new(
        vertices,
        mesh.faces
            .iter()
            .map(|face| face.map(|v| remap[v as usize]))
            .collect(),
    )
}

pub(super) fn clean_faces(mesh: &Mesh) -> Result<Mesh, CoreError> {
    let mut groups = BTreeMap::<[u32; 3], (Vec<usize>, Vec<usize>)>::new();
    for (index, face) in mesh.faces.iter().copied().enumerate() {
        let [a, b, c] = face;
        if a == b || b == c || c == a {
            continue;
        }
        let mut key = face;
        key.sort_unstable();
        let positive = (a < b) ^ (a < c) ^ (b < c);
        let group = groups.entry(key).or_default();
        if positive {
            group.0.push(index);
        } else {
            group.1.push(index);
        }
    }
    let mut keep = vec![false; mesh.faces.len()];
    for (positive, negative) in groups.values() {
        let majority = if positive.len() > negative.len() {
            positive
        } else {
            negative
        };
        if positive.len() != negative.len()
            && let Some(&first) = majority.first()
        {
            keep[first] = true;
        }
    }
    compact(&Mesh::new(
        mesh.vertices.clone(),
        mesh.faces
            .iter()
            .enumerate()
            .filter_map(|(i, &face)| keep[i].then_some(face))
            .collect(),
    )?)
}

fn snap(mesh: &Mesh, grid: RepairGrid) -> Result<Mesh, CoreError> {
    let Some(points) = grid.points(mesh)? else {
        return Ok(mesh.clone());
    };
    let mut representatives = HashMap::new();
    let mut remap = Vec::with_capacity(points.len());
    for (i, point) in points.iter().copied().enumerate() {
        remap.push(*representatives.entry(point).or_insert(index(i)?));
    }
    clean_faces(
        &grid.mesh(
            &points,
            mesh.faces
                .iter()
                .map(|face| face.map(|v| remap[v as usize]))
                .collect(),
        )?,
    )
}

/// Moves every point onto the grid without merging any two vertices.
fn align(mesh: &Mesh, grid: RepairGrid) -> Result<Mesh, CoreError> {
    let Some(points) = grid.points(mesh)? else {
        return Ok(mesh.clone());
    };
    grid.mesh(&points, mesh.faces.clone())
}

/// Inserts a point into a directed edge without changing triangle winding.
fn split(face: [u32; 3], boundary: Edge, middle: u32) -> Option<[[u32; 3]; 2]> {
    for i in 0..3 {
        let (a, b, x) = (face[i], face[(i + 1) % 3], face[(i + 2) % 3]);
        if edge(a, b) == boundary {
            return Some([[a, middle, x], [middle, b, x]]);
        }
    }
    None
}

// Mutable edge ownership makes collapse local even on a large book mesh.
struct Topology {
    faces: Vec<Option<[u32; 3]>>,
    owners: Owners,
}

impl Topology {
    fn new(faces: &[[u32; 3]]) -> Self {
        Self {
            faces: faces.iter().copied().map(Some).collect(),
            owners: edges(faces),
        }
    }

    fn remove(&mut self, index: usize) {
        if let Some(face) = self.faces[index].take() {
            for i in 0..3 {
                let key = edge(face[i], face[(i + 1) % 3]);
                if let Some(owners) = self.owners.get_mut(&key) {
                    owners.retain(|&(owner, _)| owner != index);
                    if owners.is_empty() {
                        self.owners.remove(&key);
                    }
                }
            }
        }
    }

    fn insert(&mut self, face: [u32; 3]) -> usize {
        let index = self.faces.len();
        self.faces.push(Some(face));
        for i in 0..3 {
            let (a, b) = (face[i], face[(i + 1) % 3]);
            self.owners
                .entry(edge(a, b))
                .or_default()
                .push((index, a < b));
        }
        index
    }

    fn finish(self) -> Vec<[u32; 3]> {
        self.faces.into_iter().flatten().collect()
    }
}

pub(super) fn collapse(mesh: &Mesh, grid: RepairGrid) -> Result<Mesh, CoreError> {
    let Some(points) = grid.points(mesh)? else {
        return Ok(mesh.clone());
    };
    let mut collinear = false;
    for face in &mesh.faces {
        let [a, b, c] = face.map(|v| points[v as usize]);
        if cross(difference(b, a), difference(c, a))? == [0; 3] {
            collinear = true;
            break;
        }
    }
    if !collinear {
        return clean_faces(&grid.mesh(&points, mesh.faces.clone())?);
    }
    let mut topology = Topology::new(&mesh.faces);
    let mut queue: VecDeque<_> = (0..mesh.faces.len()).collect();
    let mut iterations = 0;
    let cap = mesh.face_count().saturating_mul(2);
    while let Some(id) = queue.pop_front() {
        let Some(face) = topology.faces[id] else {
            continue;
        };
        let [a, b, c] = face.map(|v| points[v as usize]);
        if cross(difference(b, a), difference(c, a))? != [0; 3] {
            continue;
        }
        if iterations >= cap {
            break;
        }
        iterations += 1;
        let mut long_edge = None;
        for i in 0..3 {
            let (m, a, b) = (face[i], face[(i + 1) % 3], face[(i + 2) % 3]);
            let (pm, pa, pb) = (points[m as usize], points[a as usize], points[b as usize]);
            if dot(
                difference(pm, pa).map(i128::from),
                difference(pm, pb).map(i128::from),
            )? < 0
            {
                long_edge = Some((edge(a, b), m, a < b));
                break;
            }
        }
        if let Some((key, middle, direction)) = long_edge
            && let Some(owners) = topology.owners.get(&key)
            && owners.len() == 2
            && let Some(&(neighbour, _)) = owners
                .iter()
                .find(|&&(other, dir)| other != id && dir != direction)
            && let Some(neighbour_face) = topology.faces[neighbour]
            && let Some(replacement) = split(neighbour_face, key, middle)
        {
            topology.remove(neighbour);
            for face in replacement {
                queue.push_back(topology.insert(face));
            }
        }
        topology.remove(id);
    }
    clean_faces(&grid.mesh(&points, topology.finish())?)
}

// The loop direction belongs to the existing surface. Reverse every emitted
// triangle so the cap already agrees across the boundary, even for inward cups.
pub(super) fn fill_holes(
    mesh: &Mesh,
    grid: RepairGrid,
    limits: MeshOperationLimits,
    loops: &[Vec<u32>],
    maximum: usize,
) -> Result<Mesh, CoreError> {
    if !loops
        .iter()
        .any(|boundary| (3..=maximum).contains(&boundary.len()))
    {
        return Ok(mesh.clone());
    }
    let mut count = mesh.face_count();
    let mut work = 0_u64;
    for boundary in loops.iter().filter(|b| (3..=maximum).contains(&b.len())) {
        let n = boundary.len();
        count = count.checked_add(n - 2).ok_or_else(arithmetic_limit)?;
        if n > limits.max_vertices || count > limits.max_faces {
            return Err(fill_limit("hole fill exceeds the vertex or face limit"));
        }
        let size = u64::try_from(n).map_err(|_| arithmetic_limit())?;
        let (units, scratch) = if n <= 300 {
            (
                size.checked_mul(size - 1)
                    .and_then(|v| v.checked_mul(size - 2))
                    .map(|v| v / 6),
                size.checked_mul(size).and_then(|v| v.checked_mul(24)),
            )
        } else {
            (
                size.checked_mul(u64::from(size.ilog2()) + 1)
                    .and_then(|v| v.checked_mul(3)),
                size.checked_mul(512),
            )
        };
        work = work
            .checked_add(units.ok_or_else(arithmetic_limit)?)
            .ok_or_else(arithmetic_limit)?;
        limits.check_work("repair", work)?;
        // Include the original/candidate buffers, lattice and edge membership.
        let memory = (mesh.vertices.len() as u64)
            .checked_mul(48)
            .and_then(|v| v.checked_add((count as u64).checked_mul(192)?))
            .and_then(|v| v.checked_add(scratch?))
            .ok_or_else(arithmetic_limit)?;
        if memory > limits.max_memory_bytes {
            return Err(fill_limit("hole fill exceeds the working-memory limit"));
        }
    }
    let Some(points) = grid.points(mesh)? else {
        return Ok(mesh.clone());
    };
    let mut existing: HashSet<Edge> = mesh
        .faces
        .iter()
        .flat_map(|f| [edge(f[0], f[1]), edge(f[1], f[2]), edge(f[2], f[0])])
        .collect();
    // A cap face on the same three points as an existing face would only
    // cancel or double it (a lone triangle's "hole" is its own back side).
    let sorted = |face: &[u32; 3]| {
        let mut key = *face;
        key.sort_unstable();
        key
    };
    let mut present: HashSet<[u32; 3]> = mesh.faces.iter().map(sorted).collect();
    let mut faces = mesh.faces.clone();
    for boundary in loops.iter().filter(|b| (3..=maximum).contains(&b.len())) {
        let hole = Hole {
            mesh,
            points: &points,
            boundary,
            existing: &existing,
        };
        let cap = if boundary.len() <= 300 {
            hole.minimum_area()?
        } else {
            hole.ears()?
        };
        if let Some(cap) = cap {
            // A triangulation of distinct cyclic positions uses a diagonal at
            // most twice. Check before committing this hole, and also prevent
            // later holes from adding any of its diagonals again.
            let mut uses = HashMap::<Edge, usize>::new();
            for face in &cap {
                for i in 0..3 {
                    *uses.entry(edge(face[i], face[(i + 1) % 3])).or_default() += 1;
                }
            }
            if uses.values().any(|&count| count > 2)
                || cap.iter().any(|face| present.contains(&sorted(face)))
            {
                continue;
            }
            present.extend(cap.iter().map(sorted));
            existing.extend(uses.keys().copied());
            faces.extend(cap);
        }
    }
    Mesh::new(mesh.vertices.clone(), faces)
}

fn fill_limit(reason: &str) -> CoreError {
    CoreError::OperationResourceLimit {
        operation: "repair".to_owned(),
        reason: reason.to_owned(),
    }
}

struct Hole<'a> {
    mesh: &'a Mesh,
    points: &'a [Point],
    boundary: &'a [u32],
    existing: &'a HashSet<Edge>,
}

impl Hole<'_> {
    fn allowed_edge(&self, i: usize, j: usize) -> bool {
        let n = self.boundary.len();
        i.abs_diff(j) == 1
            || i.abs_diff(j) == n - 1
            || !self
                .existing
                .contains(&edge(self.boundary[i], self.boundary[j]))
    }

    fn area(&self, i: usize, j: usize, k: usize) -> Result<Option<f64>, CoreError> {
        let [a, b, c] = [i, j, k].map(|i| self.points[self.boundary[i] as usize]);
        if cross(difference(b, a), difference(c, a))? == [0; 3] {
            return Ok(None);
        }
        let [a, b, c] =
            [i, j, k].map(|i| self.mesh.vertices[self.boundary[i] as usize].map(f64::from));
        let ab = std::array::from_fn(|axis| b[axis] - a[axis]);
        let ac = std::array::from_fn(|axis| c[axis] - a[axis]);
        let normal = crate::repair::cross64(ab, ac);
        Ok(Some(crate::repair::dot64(normal, normal).sqrt() * 0.5))
    }

    fn triangle(&self, i: usize, j: usize, k: usize) -> [u32; 3] {
        [self.boundary[i], self.boundary[k], self.boundary[j]]
    }

    fn minimum_area(&self) -> Result<Option<Vec<[u32; 3]>>, CoreError> {
        let n = self.boundary.len();
        let mut weights = vec![f64::INFINITY; n * n];
        let mut splits = vec![0; n * n];
        let allowed: Vec<_> = (0..n)
            .flat_map(|i| (0..n).map(move |j| self.allowed_edge(i, j)))
            .collect();
        for i in 0..n - 1 {
            weights[i * n + i + 1] = 0.0;
        }
        for span in 2..n {
            for i in 0..n - span {
                let k = i + span;
                if !allowed[i * n + k] {
                    continue;
                }
                for m in i + 1..k {
                    let subtotal = weights[i * n + m] + weights[m * n + k];
                    if !subtotal.is_finite() {
                        continue;
                    }
                    if let Some(area) = self.area(i, m, k)? {
                        let candidate = subtotal + area;
                        // Strict comparison and ascending m preserve the tie rule.
                        if candidate < weights[i * n + k] {
                            weights[i * n + k] = candidate;
                            splits[i * n + k] = m;
                        }
                    }
                }
            }
        }
        if !weights[n - 1].is_finite() {
            return Ok(None);
        }
        let mut pending = vec![(0, n - 1)];
        let mut cap = Vec::with_capacity(n - 2);
        while let Some((i, k)) = pending.pop() {
            if k - i < 2 {
                continue;
            }
            let m = splits[i * n + k];
            cap.push(self.triangle(i, m, k));
            pending.extend([(m, k), (i, m)]);
        }
        Ok(Some(cap))
    }

    fn ear(&self, i: usize, previous: &[usize], next: &[usize]) -> Result<Option<Ear>, CoreError> {
        let (a, b) = (previous[i], next[i]);
        if !self.allowed_edge(a, b) {
            return Ok(None);
        }
        Ok(self.area(a, i, b)?.map(|area| Ear {
            area,
            position: i,
            previous: a,
            next: b,
        }))
    }

    fn ears(&self) -> Result<Option<Vec<[u32; 3]>>, CoreError> {
        let n = self.boundary.len();
        let mut previous: Vec<_> = (0..n).map(|i| if i == 0 { n - 1 } else { i - 1 }).collect();
        let mut next: Vec<_> = (0..n).map(|i| (i + 1) % n).collect();
        let mut alive = vec![true; n];
        let mut heap = BinaryHeap::new();
        for i in 0..n {
            if let Some(ear) = self.ear(i, &previous, &next)? {
                heap.push(ear);
            }
        }
        let mut cap = Vec::with_capacity(n - 2);
        for _ in 0..n - 3 {
            let ear = loop {
                let Some(ear) = heap.pop() else {
                    return Ok(None);
                };
                if alive[ear.position]
                    && previous[ear.position] == ear.previous
                    && next[ear.position] == ear.next
                {
                    break ear;
                }
            };
            let (a, i, b) = (ear.previous, ear.position, ear.next);
            cap.push(self.triangle(a, i, b));
            alive[i] = false;
            next[a] = b;
            previous[b] = a;
            for j in [a, b] {
                if let Some(ear) = self.ear(j, &previous, &next)? {
                    heap.push(ear);
                }
            }
        }
        let Some(i) = alive.iter().position(|&v| v) else {
            return Ok(None);
        };
        if self.area(i, next[i], next[next[i]])?.is_none() {
            return Ok(None);
        }
        cap.push(self.triangle(i, next[i], next[next[i]]));
        Ok(Some(cap))
    }
}

#[derive(Clone, Copy)]
struct Ear {
    area: f64,
    position: usize,
    previous: usize,
    next: usize,
}
impl PartialEq for Ear {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl Eq for Ear {}
impl PartialOrd for Ear {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Ear {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .area
            .total_cmp(&self.area)
            .then_with(|| other.position.cmp(&self.position))
            .then_with(|| other.previous.cmp(&self.previous))
            .then_with(|| other.next.cmp(&self.next))
    }
}

fn cell(point: Point) -> Point {
    point.map(|coordinate| coordinate.div_euclid(5))
}

fn weld(mesh: &Mesh, grid: RepairGrid) -> Result<Mesh, CoreError> {
    let Some(points) = grid.points(mesh)? else {
        return Ok(mesh.clone());
    };
    let open = open_points(&mesh.faces);
    if open.is_empty() {
        return collapse(mesh, grid);
    }
    let mut buckets = HashMap::<Point, Vec<u32>>::new();
    let mut remap = (0..points.len())
        .map(index)
        .collect::<Result<Vec<_>, _>>()?;
    for vertex in open {
        let point = points[vertex as usize];
        let key = cell(point);
        let mut nearest = None;
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(candidates) = buckets.get(&[key[0] + x, key[1] + y, key[2] + z]) {
                        for &candidate in candidates {
                            let distance = distance_squared(point, points[candidate as usize])?;
                            let choice = (distance, candidate);
                            if distance <= 25 && nearest.is_none_or(|previous| choice < previous) {
                                nearest = Some(choice);
                            }
                        }
                    }
                }
            }
        }
        if let Some((_, representative)) = nearest {
            remap[vertex as usize] = representative;
        } else {
            buckets.entry(key).or_default().push(vertex);
        }
    }
    let welded = grid.mesh(
        &points,
        mesh.faces
            .iter()
            .map(|face| face.map(|v| remap[v as usize]))
            .collect(),
    )?;
    collapse(&clean_faces(&welded)?, grid)
}

fn on_segment(p: Point, a: Point, b: Point) -> Result<Option<i128>, CoreError> {
    if distance_squared(p, a)? <= 25 || distance_squared(p, b)? <= 25 {
        return Ok(None);
    }
    let e = difference(b, a);
    let w = difference(p, a);
    let length = dot(e.map(i128::from), e.map(i128::from))?;
    let projection = dot(e.map(i128::from), w.map(i128::from))?;
    if projection <= 0 || projection >= length {
        return Ok(None);
    }
    let area = cross(e, w)?;
    let limit = length.checked_mul(25).ok_or_else(arithmetic_limit)?;
    // An individual cross component already over the limit cannot qualify.
    // Overflow in this nonnegative sum proves it exceeds the i128 limit.
    let mut squared = 0_i128;
    for component in area {
        let Some(term) = component.checked_mul(component) else {
            return Ok(None);
        };
        let Some(sum) = squared.checked_add(term) else {
            return Ok(None);
        };
        if sum > limit {
            return Ok(None);
        }
        squared = sum;
    }
    Ok(Some(projection))
}

fn segment_candidates(buckets: &HashMap<Point, Vec<u32>>, a: Point, b: Point) -> BTreeSet<u32> {
    let delta = difference(b, a);
    let span = delta.into_iter().map(i64::unsigned_abs).max().unwrap_or(0);
    let samples = span.div_ceil(5).max(1);
    let mut candidates = BTreeSet::new();
    if samples.saturating_add(1).saturating_mul(125) <= buckets.len() as u64 {
        let mut visited = HashSet::new();
        for sample in 0..=samples {
            // Integer interpolation is within one unit of the segment.
            // Consecutive samples differ by at most five per axis, so every
            // point within radius five is at most two cells from a sample.
            let point = std::array::from_fn(|i| {
                a[i] + (i128::from(delta[i]) * i128::from(sample) / i128::from(samples)) as i64
            });
            let key = cell(point);
            for x in -2..=2 {
                for y in -2..=2 {
                    for z in -2..=2 {
                        let key = [key[0] + x, key[1] + y, key[2] + z];
                        if visited.insert(key)
                            && let Some(points) = buckets.get(&key)
                        {
                            candidates.extend(points.iter().copied());
                        }
                    }
                }
            }
        }
    } else {
        // A very long edge in a sparse mesh is cheaper to query by occupied
        // cells than by traversing its empty length.
        let low = cell(std::array::from_fn(|i| a[i].min(b[i]) - 5));
        let high = cell(std::array::from_fn(|i| a[i].max(b[i]) + 5));
        for (key, vertices) in buckets {
            if (0..3).all(|i| key[i] >= low[i] && key[i] <= high[i]) {
                candidates.extend(vertices.iter().copied());
            }
        }
    }
    candidates
}

fn junctions(mesh: &Mesh, grid: RepairGrid) -> Result<Mesh, CoreError> {
    let Some(points) = grid.points(mesh)? else {
        return Ok(mesh.clone());
    };
    if open_points(&mesh.faces).is_empty() {
        return clean_faces(&grid.mesh(&points, mesh.faces.clone())?);
    }
    let mut topology = Topology::new(&mesh.faces);
    let cap = mesh.face_count().saturating_mul(2);
    let mut splits = 0;
    loop {
        let open = boundary_points(&topology.owners);
        let mut buckets = HashMap::<Point, Vec<u32>>::new();
        for vertex in open {
            buckets
                .entry(cell(points[vertex as usize]))
                .or_default()
                .push(vertex);
        }
        let mut changed = false;
        let boundaries: Vec<_> = topology
            .owners
            .iter()
            .filter(|(_, owners)| owners.len() == 1)
            .map(|(&edge, owners)| (edge, owners[0].0))
            .collect();
        for (key @ (a, b), owner) in boundaries {
            if splits >= cap {
                break;
            }
            let Some(face) = topology.faces[owner] else {
                continue;
            };
            let (pa, pb) = (points[a as usize], points[b as usize]);
            let mut candidates = Vec::new();
            for p in segment_candidates(&buckets, pa, pb) {
                if p != a
                    && p != b
                    && let Some(t) = on_segment(points[p as usize], pa, pb)?
                {
                    candidates.push((t, p));
                }
            }
            candidates.sort_unstable();
            if let Some(&(_, middle)) = candidates.first()
                && let Some(replacement) = split(face, key, middle)
            {
                topology.remove(owner);
                for face in replacement {
                    topology.insert(face);
                }
                splits += 1;
                changed = true;
            }
        }
        if !changed || splits >= cap {
            break;
        }
    }
    let split = grid.mesh(&points, topology.finish())?;
    clean_faces(&weld(&split, grid)?)
}

/// `insert` is the first pass: merge points sharing a grid cell, collapse
/// thin triangles, weld and split T-junctions. The second pass only moves
/// off-grid points (hole-fill centres) onto the grid and welds open edges;
/// it must not merge by cell, or it would rejoin parts that the non-manifold
/// split separated on purpose.
pub(super) fn stitch(mesh: &Mesh, grid: RepairGrid, insert: bool) -> Result<Mesh, CoreError> {
    let collapsed = if insert {
        collapse(&snap(mesh, grid)?, grid)?
    } else {
        align(mesh, grid)?
    };
    let welded = weld(&collapsed, grid)?;
    if insert {
        junctions(&welded, grid)
    } else {
        clean_faces(&welded)
    }
}

fn root(parent: &mut [usize], mut vertex: usize) -> usize {
    while parent[vertex] != vertex {
        parent[vertex] = parent[parent[vertex]];
        vertex = parent[vertex];
    }
    vertex
}

fn join(parent: &mut [usize], a: usize, b: usize) {
    let (a, b) = (root(parent, a), root(parent, b));
    parent[a.max(b)] = a.min(b);
}

fn paired_faces(mesh: &Mesh, key: Edge, owners: &[(usize, bool)]) -> Vec<(usize, usize)> {
    let (a, b) = (
        mesh.vertices[key.0 as usize].map(f64::from),
        mesh.vertices[key.1 as usize].map(f64::from),
    );
    let axis = std::array::from_fn::<_, 3, _>(|i| b[i] - a[i]);
    let reference_axis = (0..3)
        .min_by(|&i, &j| axis[i].abs().total_cmp(&axis[j].abs()))
        .unwrap_or(0);
    let mut reference = [0.0; 3];
    reference[reference_axis] = 1.0;
    let cross = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let dot = |a: [f64; 3], b: [f64; 3]| a.into_iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
    let mut u = cross(axis, reference);
    let length = dot(u, u).sqrt();
    if length == 0.0 {
        return Vec::new();
    }
    u = u.map(|x| x / length);
    let mut v = cross(axis, u);
    let length = dot(v, v).sqrt();
    v = v.map(|x| x / length);
    let mut angular = Vec::new();
    for &(face, direction) in owners {
        if let Some(&third) = mesh.faces[face].iter().find(|&&p| p != key.0 && p != key.1) {
            let p = std::array::from_fn(|i| f64::from(mesh.vertices[third as usize][i]) - a[i]);
            angular.push((dot(p, v).atan2(dot(p, u)), face, direction));
        }
    }
    angular.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut used = BTreeSet::new();
    let mut pairs = Vec::new();
    for (i, &(_, face, direction)) in angular.iter().enumerate() {
        if direction {
            continue;
        }
        for offset in 1..angular.len() {
            let (_, other, forward) = angular[(i + offset) % angular.len()];
            if forward && used.insert(other) {
                pairs.push((face, other));
                break;
            }
        }
    }
    pairs
}

pub(super) fn split_non_manifold(mesh: &Mesh, grid: RepairGrid) -> Result<Mesh, CoreError> {
    let Some(points) = grid.points(mesh)? else {
        return Ok(mesh.clone());
    };
    let snapped = grid.mesh(&points, mesh.faces.clone())?;
    let records = edge_records(&mesh.faces);
    if records
        .chunk_by(|a, b| a.0 == b.0)
        .all(|owners| owners.len() == 2)
    {
        return Ok(snapped);
    }
    let owners = edges(&mesh.faces);
    let mut endpoints = BTreeSet::new();
    let mut links = BTreeMap::<u32, Vec<(usize, usize)>>::new();
    for (&key @ (a, b), faces) in &owners {
        let pairs = if faces.len() > 2 {
            endpoints.extend([a, b]);
            paired_faces(&snapped, key, faces)
        } else if faces.len() == 2 {
            vec![(faces[0].0, faces[1].0)]
        } else {
            Vec::new()
        };
        for vertex in [a, b] {
            links.entry(vertex).or_default().extend(&pairs);
        }
    }
    let mut incident = BTreeMap::<u32, Vec<usize>>::new();
    for (id, face) in mesh.faces.iter().enumerate() {
        for &vertex in face {
            if endpoints.contains(&vertex) {
                incident.entry(vertex).or_default().push(id);
            }
        }
    }
    let mut vertices = snapped.vertices;
    let mut faces = mesh.faces.clone();
    for vertex in endpoints {
        let Some(fan) = incident.get(&vertex) else {
            continue;
        };
        let local: BTreeMap<_, _> = fan.iter().enumerate().map(|(i, &face)| (face, i)).collect();
        let mut parent: Vec<_> = (0..fan.len()).collect();
        if let Some(links) = links.get(&vertex) {
            for &(a, b) in links {
                join(&mut parent, local[&a], local[&b]);
            }
        }
        let mut copies = BTreeMap::new();
        for (i, &face) in fan.iter().enumerate() {
            let group = root(&mut parent, i);
            let replacement = if let Some(&copy) = copies.get(&group) {
                copy
            } else {
                let copy = if copies.is_empty() {
                    vertex
                } else {
                    let copy = index(vertices.len())?;
                    vertices.push(vertices[vertex as usize]);
                    copy
                };
                copies.insert(group, copy);
                copy
            };
            for corner in &mut faces[face] {
                if *corner == vertex {
                    *corner = replacement;
                }
            }
        }
    }
    remove_fins(&Mesh::new(vertices, faces)?, grid)
}

fn remove_fins(mesh: &Mesh, grid: RepairGrid) -> Result<Mesh, CoreError> {
    let Some(points) = grid.points(mesh)? else {
        return Ok(mesh.clone());
    };
    let owners = edges(&mesh.faces);
    let mut parent: Vec<_> = (0..mesh.face_count()).collect();
    for faces in owners.values() {
        if let Some(&(first, _)) = faces.first() {
            for &(other, _) in &faces[1..] {
                join(&mut parent, first, other);
            }
        }
    }
    let mut components = BTreeMap::<usize, Vec<usize>>::new();
    for face in 0..mesh.face_count() {
        components
            .entry(root(&mut parent, face))
            .or_default()
            .push(face);
    }
    let mut open = BTreeSet::new();
    for faces in owners.values().filter(|faces| faces.len() == 1) {
        open.insert(root(&mut parent, faces[0].0));
    }
    if components.keys().all(|component| open.contains(component)) {
        return grid.mesh(&points, mesh.faces.clone());
    }
    let mut removed = BTreeSet::<usize>::new();
    for (component, faces) in &components {
        if !open.contains(component) {
            continue;
        }
        let vertices: BTreeSet<_> = faces.iter().flat_map(|&face| mesh.faces[face]).collect();
        let mut vertices = vertices.into_iter();
        let Some(first) = vertices.next() else {
            continue;
        };
        let a = points[first as usize];
        let rest: Vec<_> = vertices.map(|v| points[v as usize]).collect();
        let mut normal = [0; 3];
        if let Some(&b) = rest.iter().find(|&&p| p != a) {
            for &c in &rest {
                normal = cross(difference(b, a), difference(c, a))?;
                if normal != [0; 3] {
                    break;
                }
            }
        }
        let mut planar = true;
        for &point in &rest {
            if dot(normal, difference(point, a).map(i128::from))? != 0 {
                planar = false;
                break;
            }
        }
        if planar {
            removed.extend(faces.iter().copied());
        }
    }
    compact(
        &grid.mesh(
            &points,
            mesh.faces
                .iter()
                .enumerate()
                .filter_map(|(i, &face)| (!removed.contains(&i)).then_some(face))
                .collect(),
        )?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(points: &[Point], faces: &[[u32; 3]]) -> Mesh {
        Mesh::new(
            points.iter().map(|p| p.map(|v| v as f32)).collect(),
            faces.to_vec(),
        )
        .expect("mesh")
    }

    fn unit_grid() -> RepairGrid {
        RepairGrid {
            origin: [0.0; 3],
            step: 1.0,
        }
    }

    #[test]
    fn duplicate_windings_cancel_in_pairs_and_keep_only_the_majority() {
        for (positive, negative, expected) in
            [(1, 1, 0), (3, 1, 1), (1, 3, 1), (3, 3, 0), (3, 0, 1)]
        {
            let mut faces = vec![[0, 1, 2]; positive];
            faces.extend(vec![[2, 1, 0]; negative]);
            faces.push([0, 0, 1]);
            let input = fixture(&[[0, 0, 0], [10, 0, 0], [0, 10, 0]], &faces);
            let cleaned = clean_faces(&input).expect("cleanup");
            assert_eq!(cleaned.face_count(), expected);
            if expected == 1 {
                let face = cleaned.faces[0].map(|v| cleaned.vertices[v as usize]);
                let a = face[1][0] - face[0][0];
                let b = face[2][1] - face[0][1];
                let c = face[1][1] - face[0][1];
                let d = face[2][0] - face[0][0];
                assert_eq!(a * b - c * d > 0.0, positive > negative);
            }
        }
    }

    #[test]
    fn grid_rounds_to_nearest_and_keeps_the_original_origin() {
        let input = fixture(&[[10, 20, 30], [20, 30, 40], [15, 25, 35]], &[[0, 1, 2]]);
        let grid = RepairGrid::from_mesh(&input);
        assert_eq!(grid.origin, [10.0, 20.0, 30.0]);
        assert_eq!(grid.step, 0.002);
        let later = Mesh::new(vec![[12.0008, 22.0012, 32.0]], vec![]).expect("later mesh");
        assert_eq!(
            grid.points(&later).expect("lattice"),
            Some(vec![[1000, 1001, 1000]])
        );
        let points = grid.points(&input).expect("lattice").expect("valid grid");
        let snapped = grid.mesh(&points, input.faces.clone()).expect("snapped");
        assert_eq!(grid.points(&snapped).expect("lattice"), Some(points));
        let zero = fixture(&[[0, 0, 0]], &[[0, 0, 0]]);
        assert!(
            RepairGrid::from_mesh(&zero)
                .points(&zero)
                .expect("zero extent")
                .is_none()
        );
    }

    #[test]
    fn weld_uses_nearest_representative_and_never_chains() {
        // Independent open triangles make 0,1,2,3 boundary points. The
        // remaining corners are far apart and cannot merge accidentally.
        let points = [
            [0, 0, 0],
            [8, 0, 0],
            [4, 0, 0],
            [12, 0, 0],
            [0, 100, 0],
            [0, 0, 100],
            [100, 100, 0],
            [100, 0, 100],
            [200, 100, 0],
            [200, 0, 100],
            [300, 100, 0],
            [300, 0, 100],
        ];
        let input = fixture(&points, &[[0, 4, 5], [1, 6, 7], [2, 8, 9], [3, 10, 11]]);
        let result = weld(&input, unit_grid()).expect("weld");
        let tip = |i: usize| result.vertices[result.faces[i][0] as usize];
        assert_eq!(tip(0), [0.0; 3]);
        assert_eq!(tip(1), [8.0, 0.0, 0.0]);
        assert_eq!(tip(2), [0.0; 3]); // tie to lower index
        assert_eq!(tip(3), [8.0, 0.0, 0.0]); // 4 was mapped, never a representative
        let input = fixture(
            &[
                [0, 0, 0],
                [4, 0, 0],
                [8, 0, 0],
                [0, 100, 0],
                [0, 0, 100],
                [100, 100, 0],
                [100, 0, 100],
                [200, 100, 0],
                [200, 0, 100],
            ],
            &[[0, 3, 4], [1, 5, 6], [2, 7, 8]],
        );
        let result = weld(&input, unit_grid()).expect("no chaining");
        assert!(result.vertices.contains(&[8.0, 0.0, 0.0]));
    }

    #[test]
    fn segment_test_uses_exact_distance_projection_and_endpoint_exclusion() {
        assert!(
            on_segment([50, 5, 0], [0, 0, 0], [100, 0, 0])
                .expect("segment")
                .is_some()
        );
        assert!(
            on_segment([50, 5, 1], [0, 0, 0], [100, 0, 0])
                .expect("segment")
                .is_none()
        );
        assert!(
            on_segment([5, 0, 0], [0, 0, 0], [100, 0, 0])
                .expect("segment")
                .is_none()
        );
        assert!(
            on_segment([0, 5, 0], [0, 0, 0], [100, 0, 0])
                .expect("segment")
                .is_none()
        );
        assert!(
            on_segment([-1, 0, 0], [0, 0, 0], [100, 0, 0])
                .expect("segment")
                .is_none()
        );
        assert!(
            on_segment([50, 5, 0], [100, 0, 0], [0, 0, 0])
                .expect("segment")
                .is_some()
        );
    }

    #[test]
    fn sampled_spatial_hash_query_includes_every_point_within_the_segment_radius() {
        let mut points = Vec::new();
        for x in -10..=10 {
            for y in -5..=5 {
                for z in -5..=5 {
                    points.push([x, x + y, x + z]);
                }
            }
        }
        // Occupied distant cells force the sampled hash route, rather than
        // the cheaper sparse-cell scan used on tiny meshes.
        for i in 0..1000 {
            points.push([1000 + i * 10, 1000, 1000]);
        }
        let mut buckets = HashMap::<Point, Vec<u32>>::new();
        for (id, &point) in points.iter().enumerate() {
            buckets.entry(cell(point)).or_default().push(id as u32);
        }
        for (a, b) in [([-10; 3], [10; 3]), ([10; 3], [-10; 3])] {
            let candidates = segment_candidates(&buckets, a, b);
            for (id, &point) in points.iter().enumerate() {
                if on_segment(point, a, b).expect("exact segment").is_some() {
                    assert!(candidates.contains(&(id as u32)), "missed {point:?}");
                }
            }
        }
    }

    #[test]
    fn collapse_splits_only_an_oppositely_directed_manifold_neighbour() {
        let points = [[0, 0, 0], [20, 0, 0], [10, 0, 0], [0, 20, 0], [0, -20, 0]];
        let input = fixture(&points, &[[0, 1, 2], [1, 0, 3]]);
        let result = collapse(&input, unit_grid()).expect("collapse");
        assert_eq!(result.face_count(), 2);
        assert!(result.faces.iter().all(|face| {
            face.iter()
                .any(|&v| result.vertices[v as usize] == [10.0, 0.0, 0.0])
        }));
        for faces in [
            vec![[0, 1, 2]],
            vec![[0, 1, 2], [0, 1, 3]],
            vec![[0, 1, 2], [1, 0, 3], [0, 1, 4]],
        ] {
            let input = fixture(&points, &faces);
            let result = collapse(&input, unit_grid()).expect("open/nonmanifold collapse");
            assert_eq!(result.face_count(), faces.len() - 1);
        }
    }

    #[test]
    fn planar_sheets_are_retained_without_any_closed_component() {
        let input = fixture(&[[0, 0, 0], [100, 0, 0], [0, 100, 0]], &[[0, 1, 2]]);
        assert_eq!(
            remove_fins(&input, unit_grid()).expect("no closed component"),
            input
        );
    }

    #[test]
    fn non_manifold_pairing_separates_touching_cubes_and_removes_only_flat_fins() {
        let cube = |lo: Point, hi: Point| {
            let [x, y, z] = lo;
            let [xx, yy, zz] = hi;
            fixture(
                &[
                    [x, y, z],
                    [xx, y, z],
                    [xx, yy, z],
                    [x, yy, z],
                    [x, y, zz],
                    [xx, y, zz],
                    [xx, yy, zz],
                    [x, yy, zz],
                ],
                &[
                    [0, 2, 1],
                    [0, 3, 2],
                    [4, 5, 6],
                    [4, 6, 7],
                    [0, 1, 5],
                    [0, 5, 4],
                    [1, 2, 6],
                    [1, 6, 5],
                    [2, 3, 7],
                    [2, 7, 6],
                    [3, 0, 4],
                    [3, 4, 7],
                ],
            )
        };
        let mut input =
            Mesh::concatenate(&[cube([0; 3], [10; 3]), cube([10, 10, 0], [20, 20, 10])])
                .expect("cubes");
        for face in &mut input.faces {
            for v in face {
                if *v == 8 {
                    *v = 2;
                } else if *v == 12 {
                    *v = 6;
                }
            }
        }
        input.vertices.push([5.0, 5.0, 15.0]);
        input.faces.push([4, 5, 16]);
        let grid = RepairGrid::from_mesh(&input);
        let split = split_non_manifold(&input, grid).expect("split");
        let health = crate::inspect_mesh(&split).expect("health");
        assert_eq!(health.non_manifold_edges, 0);
        assert_eq!(health.boundary_edges, 0);
        assert_eq!(health.body_count, 2);
        assert_eq!(split.face_count(), 24);
        assert!(!split.vertices.contains(&[5.0, 5.0, 15.0]));

        let curved = fixture(
            &[[30, 0, 0], [40, 0, 0], [30, 10, 0], [40, 10, 5]],
            &[[0, 1, 2], [1, 3, 2]],
        );
        let with_sheet = Mesh::concatenate(&[cube([0; 3], [10; 3]), curved]).expect("curved sheet");
        let kept = remove_fins(&with_sheet, unit_grid()).expect("fins");
        assert_eq!(kept.face_count(), 14);
        assert!(kept.vertices.contains(&[40.0, 10.0, 5.0]));
    }

    #[test]
    fn fill_opposes_boundary_edges_before_any_winding_stage() {
        for inward in [false, true] {
            let mut input = fixture(
                &[[0, 0, 0], [10, 0, 0], [0, 10, 0], [0, 0, 10]],
                &[[0, 2, 1], [0, 1, 3], [0, 3, 2]],
            );
            let mut boundary = vec![2, 1, 3];
            if inward {
                for face in &mut input.faces {
                    face.swap(1, 2);
                }
                boundary.reverse();
            }
            let filled = fill_holes(
                &input,
                unit_grid(),
                MeshOperationLimits::default(),
                &[boundary],
                4,
            )
            .expect("fill");
            let health = crate::inspect_mesh(&filled).expect("health");
            assert_eq!(health.boundary_edges, 0);
            assert_eq!(health.flipped_edges, 0);
            assert_eq!(health.non_manifold_edges, 0);
            assert_eq!(filled.vertex_count(), 4);
            assert_eq!(filled.face_count(), 4);
        }
    }

    #[test]
    fn minimum_area_ties_choose_the_smallest_split_position() {
        let input = fixture(
            &[
                [0, 0, 10],
                [10, 0, 10],
                [10, 10, 10],
                [0, 10, 10],
                [5, 5, 0],
            ],
            &[[4, 0, 1], [4, 1, 2], [4, 2, 3], [4, 3, 0]],
        );
        let filled = fill_holes(
            &input,
            unit_grid(),
            MeshOperationLimits::default(),
            &[vec![0, 1, 2, 3]],
            4,
        )
        .expect("tie fill");
        assert_eq!(&filled.faces[4..], &[[0, 3, 1], [1, 3, 2]]);
    }

    #[test]
    fn impossible_fills_leave_no_partial_cap_or_centre_point() {
        let line = fixture(&[[0, 0, 0], [10, 0, 0], [20, 0, 0]], &[[0, 1, 2]]);
        assert_eq!(
            fill_holes(
                &line,
                unit_grid(),
                MeshOperationLimits::default(),
                &[vec![0, 1, 2]],
                4
            )
            .expect("collinear hole"),
            line
        );
        let blocked = fixture(
            &[[0, 0, 0], [10, 0, 0], [10, 10, 0], [0, 10, 0]],
            &[[0, 1, 2], [0, 2, 3], [0, 3, 1]],
        );
        assert_eq!(
            fill_holes(
                &blocked,
                unit_grid(),
                MeshOperationLimits::default(),
                &[vec![0, 1, 2, 3]],
                4
            )
            .expect("blocked diagonals"),
            blocked
        );
    }

    #[test]
    fn hole_lattice_overflow_is_a_typed_resource_error() {
        let input = Mesh::new(
            vec![[0.0; 3], [f32::MAX, 0.0, 0.0], [0.0, 1.0, 0.0]],
            vec![[0, 1, 2]],
        )
        .expect("finite mesh");
        assert!(matches!(
            fill_holes(
                &input,
                unit_grid(),
                MeshOperationLimits::default(),
                &[vec![0, 1, 2]],
                4
            ),
            Err(CoreError::OperationResourceLimit { .. })
        ));
    }
}
