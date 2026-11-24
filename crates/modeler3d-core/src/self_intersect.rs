//! Strict triangle crossings in millimetres, shared by health and repair.

use crate::Mesh;
use crate::repair::{cross64, dot64};

const EPS: f64 = 1.0e-6;
const LEAF_SIZE: usize = 8;

#[derive(Clone, Copy)]
struct Bounds {
    lo: [f64; 3],
    hi: [f64; 3],
}

impl Bounds {
    fn empty() -> Self {
        Self {
            lo: [f64::INFINITY; 3],
            hi: [f64::NEG_INFINITY; 3],
        }
    }

    fn include(&mut self, other: Self) {
        for axis in 0..3 {
            self.lo[axis] = self.lo[axis].min(other.lo[axis]);
            self.hi[axis] = self.hi[axis].max(other.hi[axis]);
        }
    }

    fn overlaps(self, other: Self) -> bool {
        // Equivalent to growing both boxes by EPS, while retaining raw
        // bounds for half-space rejection when querying a proposed cap.
        (0..3).all(|axis| {
            self.lo[axis] - other.hi[axis] <= 2.0 * EPS
                && other.lo[axis] - self.hi[axis] <= 2.0 * EPS
        })
    }

    fn straddles(self, origin: [f64; 3], normal: [f64; 3]) -> bool {
        let mut lo = 0.0;
        let mut hi = 0.0;
        for i in 0..3 {
            let a = (self.lo[i] - origin[i]) * normal[i];
            let b = (self.hi[i] - origin[i]) * normal[i];
            lo += a.min(b);
            hi += a.max(b);
        }
        lo < -EPS && hi > EPS
    }
}

struct FaceBox {
    bounds: Bounds,
    face: u32,
    normal: [f64; 3],
}

struct Node {
    bounds: Bounds,
    start: usize,
    end: usize,
    children: Option<(usize, usize)>,
}

pub(crate) struct Search<'a> {
    mesh: &'a Mesh,
    faces: Vec<FaceBox>,
    nodes: Vec<Node>,
}

impl<'a> Search<'a> {
    pub(crate) fn new(mesh: &'a Mesh) -> Self {
        let faces = mesh
            .faces
            .iter()
            .enumerate()
            .map(|(id, face)| {
                let points = face.map(|v| mesh.vertices[v as usize].map(f64::from));
                let bounds = Bounds {
                    lo: std::array::from_fn(|i| points[0][i].min(points[1][i]).min(points[2][i])),
                    hi: std::array::from_fn(|i| points[0][i].max(points[1][i]).max(points[2][i])),
                };
                FaceBox {
                    bounds,
                    face: id as u32,
                    normal: normal(points).unwrap_or([0.0; 3]),
                }
            })
            .collect();
        let mut search = Self {
            mesh,
            faces,
            nodes: Vec::with_capacity(mesh.face_count() / 2),
        };
        if !search.faces.is_empty() {
            search.build(0, search.faces.len(), 0);
        }
        search
    }

    // Outlier separation is capped at 32 levels; subsequent median splits
    // bound the remaining depth by log2(face count), even for coincident boxes.
    fn build(&mut self, start: usize, end: usize, depth: usize) -> usize {
        let mut bounds = Bounds::empty();
        for face in &self.faces[start..end] {
            bounds.include(face.bounds);
        }
        let id = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            start,
            end,
            children: None,
        });
        if end - start > LEAF_SIZE {
            let mut axis = 0;
            for i in 1..3 {
                if bounds.hi[i] - bounds.lo[i] > bounds.hi[axis] - bounds.lo[axis] {
                    axis = i;
                }
            }
            let span = bounds.hi[axis] - bounds.lo[axis];
            let large = |face: &FaceBox| {
                (0..3).any(|i| face.bounds.hi[i] - face.bounds.lo[i] > span * 0.25)
            };
            let large_count = if depth < 32 {
                self.faces[start..end]
                    .iter()
                    .filter(|face| large(face))
                    .count()
            } else {
                0
            };
            // Isolate sparse spanning faces before centroid splitting: one long
            // triangle must not inflate every branch of a densely sampled mesh.
            let mid = if large_count > 0 && large_count < (end - start) / 4 {
                let mut split = start;
                for i in start..end {
                    if !large(&self.faces[i]) {
                        self.faces.swap(split, i);
                        split += 1;
                    }
                }
                split
            } else {
                let mid = start + (end - start) / 2;
                self.faces[start..end].select_nth_unstable_by(mid - start, |a, b| {
                    (a.bounds.lo[axis] + a.bounds.hi[axis])
                        .total_cmp(&(b.bounds.lo[axis] + b.bounds.hi[axis]))
                        .then(a.face.cmp(&b.face))
                });
                mid
            };
            let left = self.build(start, mid, depth + 1);
            let right = self.build(mid, end, depth + 1);
            self.nodes[id].children = Some((left, right));
        }
        id
    }

    fn visit(&self, mut hit: impl FnMut(u32, u32)) {
        if self.nodes.is_empty() {
            return;
        }
        let mut stack = vec![(0, 0)];
        while let Some((a, b)) = stack.pop() {
            let first = &self.nodes[a];
            let second = &self.nodes[b];
            if !first.bounds.overlaps(second.bounds) {
                continue;
            }
            match (first.children, second.children) {
                (Some((left, right)), _) if a == b => {
                    stack.extend([(right, right), (left, right), (left, left)]);
                }
                (Some((left, right)), _)
                    if second.children.is_none()
                        || first.end - first.start >= second.end - second.start =>
                {
                    stack.extend([(right, b), (left, b)]);
                }
                (_, Some((left, right))) => {
                    stack.extend([(a, right), (a, left)]);
                }
                _ => {
                    for i in first.start..first.end {
                        let start = if a == b { i + 1 } else { second.start };
                        for j in start..second.end {
                            let (x, y) = (&self.faces[i], &self.faces[j]);
                            if x.bounds.overlaps(y.bounds)
                                && crosses_prepared(
                                    self.mesh,
                                    self.mesh.faces[x.face as usize],
                                    x.normal,
                                    self.mesh.faces[y.face as usize],
                                    y.normal,
                                )
                            {
                                hit(x.face.min(y.face), x.face.max(y.face));
                            }
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn crosses_face(&self, face: [u32; 3]) -> bool {
        if self.nodes.is_empty() {
            return false;
        }
        let points = face.map(|v| self.mesh.vertices[v as usize].map(f64::from));
        let bounds = Bounds {
            lo: std::array::from_fn(|i| points[0][i].min(points[1][i]).min(points[2][i])),
            hi: std::array::from_fn(|i| points[0][i].max(points[1][i]).max(points[2][i])),
        };
        let n = normal(points).unwrap_or([0.0; 3]);
        let mut stack = vec![0];
        while let Some(id) = stack.pop() {
            let node = &self.nodes[id];
            if !node.bounds.overlaps(bounds) || !node.bounds.straddles(points[0], n) {
                continue;
            }
            if let Some((left, right)) = node.children {
                stack.extend([right, left]);
            } else {
                for other in &self.faces[node.start..node.end] {
                    if other.bounds.overlaps(bounds)
                        && crosses_prepared(
                            self.mesh,
                            face,
                            n,
                            self.mesh.faces[other.face as usize],
                            other.normal,
                        )
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Counts each unordered crossing once without storing a pair list.
pub(crate) fn count_crossing_pairs(mesh: &Mesh) -> usize {
    let mut count = 0;
    Search::new(mesh).visit(|_, _| count += 1);
    count
}

/// Returns sorted face-index pairs; touching and edge neighbours are excluded.
pub(crate) fn crossing_pairs(mesh: &Mesh) -> Vec<(u32, u32)> {
    let mut pairs = Vec::new();
    Search::new(mesh).visit(|a, b| pairs.push((a, b)));
    pairs.sort_unstable();
    pairs
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}

fn normal(p: [[f64; 3]; 3]) -> Option<[f64; 3]> {
    let n = cross64(sub(p[1], p[0]), sub(p[2], p[0]));
    let length = dot64(n, n).sqrt();
    (length >= 1.0e-12).then(|| n.map(|v| v / length))
}

fn distances(p: [[f64; 3]; 3], origin: [f64; 3], normal: [f64; 3]) -> Option<[f64; 3]> {
    let d = p.map(|point| {
        let value = dot64(sub(point, origin), normal);
        if value.abs() <= EPS { 0.0 } else { value }
    });
    (d.iter().any(|&v| v > EPS) && d.iter().any(|&v| v < -EPS)).then_some(d)
}

fn interval(p: [[f64; 3]; 3], d: [f64; 3], line: [f64; 3]) -> (f64, f64) {
    let projected = p.map(|point| dot64(point, line));
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for i in 0..3 {
        if d[i] == 0.0 {
            lo = lo.min(projected[i]);
            hi = hi.max(projected[i]);
        }
        let j = (i + 1) % 3;
        if (d[i] > 0.0 && d[j] < 0.0) || (d[i] < 0.0 && d[j] > 0.0) {
            let t = d[i] / (d[i] - d[j]);
            let v = projected[i] + t * (projected[j] - projected[i]);
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    (lo, hi)
}

#[cfg(test)]
fn crosses(mesh: &Mesh, first: u32, second: u32) -> bool {
    let a = mesh.faces[first as usize];
    let b = mesh.faces[second as usize];
    let p = a.map(|v| mesh.vertices[v as usize].map(f64::from));
    let q = b.map(|v| mesh.vertices[v as usize].map(f64::from));
    crosses_prepared(
        mesh,
        a,
        normal(p).unwrap_or([0.0; 3]),
        b,
        normal(q).unwrap_or([0.0; 3]),
    )
}

fn crosses_prepared(mesh: &Mesh, a: [u32; 3], n1: [f64; 3], b: [u32; 3], n2: [f64; 3]) -> bool {
    // Parallel planes cannot pass through each other. Cached unit normals
    // make dense planar fans cheap without an approximate coplanarity rule.
    if n1 == [0.0; 3] || n2 == [0.0; 3] || n1 == n2 || n1 == n2.map(|v| -v) {
        return false;
    }
    if a.iter().filter(|v| b.contains(v)).count() >= 2 {
        return false;
    }
    let p = a.map(|v| mesh.vertices[v as usize].map(f64::from));
    let q = b.map(|v| mesh.vertices[v as usize].map(f64::from));
    let Some(d2) = distances(q, p[0], n1) else {
        return false;
    };
    let Some(d1) = distances(p, q[0], n2) else {
        return false;
    };
    let line = cross64(n1, n2);
    let length = dot64(line, line).sqrt();
    if length == 0.0 {
        return false;
    }
    let line = line.map(|v| v / length);
    let (a, b) = interval(p, d1, line);
    let (c, d) = interval(q, d2, line);
    b.min(d) - a.max(c) > EPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spanning_face_partition_matches_exhaustive_pairs() {
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        for i in 0..128 {
            let x = i as f32;
            let first = vertices.len() as u32;
            vertices.extend([[x - 0.3, -0.3, 0.0], [x + 0.3, -0.3, 0.0], [x, 0.3, 0.0]]);
            faces.push([first, first + 1, first + 2]);
        }
        vertices.extend([[-1.0, 0.0, -1.0], [130.0, 0.0, -1.0], [64.0, 0.0, 1.0]]);
        faces.push([384, 385, 386]);
        let mesh = Mesh::new(vertices, faces).expect("spanning triangle");
        let expected: Vec<_> = (0..128)
            .filter(|&a| crosses(&mesh, a, 128))
            .map(|a| (a, 128))
            .collect();
        assert!(expected.len() > 50);
        assert_eq!(crossing_pairs(&mesh), expected);
        assert_eq!(count_crossing_pairs(&mesh), expected.len());
    }

    #[test]
    fn strict_crossings_exclude_touches_coplanarity_degeneracy_and_disjoint_intervals() {
        let a = [[-2.0, -2.0, 0.0], [2.0, -2.0, 0.0], [0.0, 2.0, 0.0]];
        for (b, expected) in [
            ([[0.0, -1.0, -1.0], [0.0, -1.0, 1.0], [0.0, 1.0, 0.0]], true),
            (
                [[0.0, 10.0, -1.0], [0.0, 10.0, 1.0], [0.0, 12.0, 0.0]],
                false,
            ),
            ([[0.0, -1.0, 0.0], [0.0, -1.0, 1.0], [0.0, 1.0, 0.0]], false),
            (
                [[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [0.0, 1.0, 0.0]],
                false,
            ),
            (
                [
                    [0.0, -1.0, -0.0000005],
                    [0.0, -1.0, 0.0000005],
                    [0.0, 1.0, 0.0],
                ],
                false,
            ),
            ([[0.0, -1.0, -1.0], [0.0, 0.0, 0.0], [0.0, 1.0, 1.0]], false),
        ] {
            let mesh = Mesh::new([a, b].concat(), vec![[0, 1, 2], [3, 4, 5]]).expect("pair");
            assert_eq!(count_crossing_pairs(&mesh), usize::from(expected), "{b:?}");
            assert_eq!(crosses(&mesh, 0, 1), expected);
            assert_eq!(crosses(&mesh, 1, 0), expected);
        }
    }

    #[test]
    fn a_shared_vertex_can_cross_but_shared_edges_and_duplicate_indices_are_skipped() {
        let mesh = Mesh::new(
            vec![
                [0.0; 3],
                [2.0, 0.0, 0.0],
                [0.0, 2.0, 0.0],
                [1.0, 0.5, 1.0],
                [0.5, 1.0, -1.0],
            ],
            vec![[0, 1, 2], [0, 3, 4]],
        )
        .expect("shared vertex");
        assert_eq!(crossing_pairs(&mesh), vec![(0, 1)]);
        for face in [[0, 1, 3], [0, 2, 1]] {
            let mut adjacent = mesh.clone();
            adjacent.faces[1] = face;
            assert_eq!(count_crossing_pairs(&adjacent), 0);
        }
    }

    #[test]
    fn bvh_visits_each_pair_once_and_matches_exhaustive_search_after_permutations() {
        let mut state = 7_u64;
        let mut vertices = Vec::new();
        for _ in 0..600 {
            vertices.push(std::array::from_fn(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((state >> 32) as i32 % 1000) as f32 / 100.0
            }));
        }
        let faces = (0..200).map(|i| [3 * i, 3 * i + 1, 3 * i + 2]).collect();
        let mut mesh = Mesh::new(vertices, faces).expect("random triangles");
        let expected: Vec<_> = (0..200)
            .flat_map(|a| (a + 1..200).map(move |b| (a, b)))
            .filter(|&(a, b)| crosses(&mesh, a, b))
            .collect();
        assert!(!expected.is_empty());
        assert_eq!(crossing_pairs(&mesh), expected);
        assert_eq!(count_crossing_pairs(&mesh), expected.len());
        mesh.faces.reverse();
        let mut remapped: Vec<_> = expected
            .into_iter()
            .map(|(a, b)| (199 - b, 199 - a))
            .collect();
        remapped.sort_unstable();
        assert_eq!(crossing_pairs(&mesh), remapped);
    }
}
