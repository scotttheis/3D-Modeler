//! Selection outline for X-ray (OF-47, as in 3D Builder): the visible
//! silhouette and sharp edges of a selected object.
//!
//! Edge adjacency is built once per uploaded geometry; each frame only the
//! facing test runs. Both the GPU renderer and the CPU fallback use it.

use std::collections::HashMap;

use crate::{
    math::{Vec3, dot, subtract},
    scene::MeshGeometry,
};

/// Largest mesh, in faces, that gets an outline. Bigger meshes draw without
/// one rather than spend the frame on it.
pub(crate) const MAX_OUTLINE_FACES: usize = 400_000;
/// Two faces meeting at more than this angle form a visible sharp edge.
const CREASE_COS: f32 = 0.866; // 30 degrees

/// Outline colour: 3D Builder's light blue.
pub(crate) const OUTLINE_COLOR: [f32; 4] = [0.50, 0.74, 0.96, 1.0];

const NO_FACE: u32 = u32::MAX;

#[derive(Clone, Copy, Debug)]
struct Edge {
    a: Vec3,
    b: Vec3,
    faces: [u32; 2],
    crease: bool,
}

/// Edge adjacency of one geometry.
#[derive(Debug)]
pub(crate) struct OutlineEdges {
    edges: Vec<Edge>,
    normals: Vec<Vec3>,
    points: Vec<Vec3>,
}

impl OutlineEdges {
    /// Builds the adjacency, or `None` for an empty or too-large mesh.
    pub(crate) fn build(geometry: &MeshGeometry) -> Option<Self> {
        let face_count = geometry.face_count();
        if face_count == 0 || face_count > MAX_OUTLINE_FACES {
            return None;
        }
        let mut normals = Vec::with_capacity(face_count);
        let mut points = Vec::with_capacity(face_count);
        // Weld by exact position so split-vertex meshes still connect.
        let mut welded: HashMap<[u32; 3], u32> = HashMap::new();
        let mut open: HashMap<(u32, u32), usize> = HashMap::with_capacity(face_count * 2);
        let mut edges: Vec<Edge> = Vec::with_capacity(face_count * 3 / 2);
        for face in 0..face_count {
            let triangle = geometry.triangle(face)?;
            let corners = [
                geometry.position(triangle[0])?,
                geometry.position(triangle[1])?,
                geometry.position(triangle[2])?,
            ];
            let normal = face_normal(corners);
            normals.push(normal);
            points.push(corners[0]);
            let ids = corners.map(|corner| {
                let next = u32::try_from(welded.len()).unwrap_or(u32::MAX);
                *welded.entry(corner.map(f32::to_bits)).or_insert(next)
            });
            let face_id = u32::try_from(face).ok()?;
            for (first, second) in [(0, 1), (1, 2), (2, 0)] {
                let (low, high) = (ids[first].min(ids[second]), ids[first].max(ids[second]));
                if low == high {
                    continue;
                }
                match open.get(&(low, high)).copied() {
                    Some(index) if edges[index].faces[1] == NO_FACE => {
                        let other = edges[index].faces[0] as usize;
                        edges[index].faces[1] = face_id;
                        edges[index].crease = dot(normals[other], normal) < CREASE_COS;
                    }
                    _ => {
                        open.insert((low, high), edges.len());
                        edges.push(Edge {
                            a: corners[first],
                            b: corners[second],
                            faces: [face_id, NO_FACE],
                            crease: false,
                        });
                    }
                }
            }
        }
        Some(Self {
            edges,
            normals,
            points,
        })
    }

    /// Edges to draw when seen from `eye` (in the geometry's own space):
    /// silhouette edges, open edges and sharp edges on the visible side.
    pub(crate) fn visible_segments(&self, eye: Vec3) -> Vec<(Vec3, Vec3)> {
        let facing: Vec<bool> = self
            .normals
            .iter()
            .zip(&self.points)
            .map(|(normal, point)| dot(*normal, subtract(eye, *point)) > 0.0)
            .collect();
        self.edges
            .iter()
            .filter(|edge| {
                let first = facing[edge.faces[0] as usize];
                if edge.faces[1] == NO_FACE {
                    return first;
                }
                let second = facing[edge.faces[1] as usize];
                first != second || (edge.crease && (first || second))
            })
            .map(|edge| (edge.a, edge.b))
            .collect()
    }
}

fn face_normal(corners: [Vec3; 3]) -> Vec3 {
    let u = subtract(corners[1], corners[0]);
    let v = subtract(corners[2], corners[0]);
    let cross = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let length = dot(cross, cross).sqrt();
    if length.is_finite() && length > f32::EPSILON {
        [cross[0] / length, cross[1] / length, cross[2] / length]
    } else {
        [0.0; 3]
    }
}

#[cfg(test)]
mod tests {
    use builder3d_core::make_box;

    use super::*;

    #[test]
    fn a_box_seen_from_a_corner_outlines_its_nine_visible_edges() {
        let mesh = match make_box([20.0, 20.0, 20.0]) {
            Ok(mesh) => mesh,
            Err(error) => panic!("box failed: {error}"),
        };
        let geometry = match MeshGeometry::from_mesh(&mesh) {
            Ok(geometry) => geometry,
            Err(error) => panic!("geometry failed: {error}"),
        };
        let Some(edges) = OutlineEdges::build(&geometry) else {
            panic!("box should get an outline");
        };
        // Three faces visible: 6 silhouette edges + 3 edges between them.
        // Face diagonals are flat and never drawn.
        let segments = edges.visible_segments([100.0, 100.0, 100.0]);
        assert_eq!(segments.len(), 9);
        // Straight on, one face shows: its 4 edges are the silhouette.
        let segments = edges.visible_segments([0.0, 0.0, 100.0]);
        assert_eq!(segments.len(), 4);
    }
}
