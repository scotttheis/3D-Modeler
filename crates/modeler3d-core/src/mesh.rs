use std::collections::HashMap;

use crate::{CoreError, Transform};

/// A validated indexed triangle mesh in millimetres.
#[derive(Clone, Debug)]
pub struct Mesh {
    /// Vertex positions in model space.
    pub vertices: Vec<[f32; 3]>,
    /// Zero-based triangle indices into [`Mesh::vertices`].
    pub faces: Vec<[u32; 3]>,
}

impl PartialEq for Mesh {
    fn eq(&self, other: &Self) -> bool {
        self.vertices == other.vertices && self.faces == other.faces
    }
}

impl Mesh {
    /// Constructs a mesh after checking finite coordinates and face indices.
    pub fn new(vertices: Vec<[f32; 3]>, faces: Vec<[u32; 3]>) -> Result<Self, CoreError> {
        let mesh = Self { vertices, faces };
        mesh.validate()?;
        Ok(mesh)
    }

    /// Number of vertices in the mesh.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Number of triangular faces in the mesh.
    #[must_use]
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Returns whether the mesh has no vertices or faces.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() || self.faces.is_empty()
    }

    /// Validates that the mesh is suitable as a project mesh.
    pub fn validate_non_empty(&self) -> Result<(), CoreError> {
        self.validate()?;
        if self.is_empty() {
            return Err(CoreError::InvalidMesh(
                "project meshes must contain vertices and faces".to_owned(),
            ));
        }
        Ok(())
    }

    /// Returns the axis-aligned bounds, or `None` for an empty vertex set.
    #[must_use]
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let first = *self.vertices.first()?;
        let mut minimum = first;
        let mut maximum = first;
        for vertex in &self.vertices[1..] {
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(vertex[axis]);
                maximum[axis] = maximum[axis].max(vertex[axis]);
            }
        }
        Some((minimum, maximum))
    }

    /// Returns axis-aligned extents in millimetres.
    #[must_use]
    pub fn extents(&self) -> [f32; 3] {
        self.bounds()
            .map(|(minimum, maximum)| {
                [
                    maximum[0] - minimum[0],
                    maximum[1] - minimum[1],
                    maximum[2] - minimum[2],
                ]
            })
            .unwrap_or([0.0; 3])
    }

    /// Returns a transformed copy without changing the authoritative mesh.
    pub fn transformed(&self, transform: &Transform) -> Result<Self, CoreError> {
        let vertices = self
            .vertices
            .iter()
            .map(|point| transform.transform_point(*point))
            .collect();
        Self::new(vertices, self.faces.clone())
    }

    /// Concatenates meshes in order, preserving each mesh's local triangle
    /// ordering and producing one indexed mesh for grouped scene objects.
    pub fn concatenate(meshes: &[Self]) -> Result<Self, CoreError> {
        if meshes.is_empty() {
            return Err(CoreError::InvalidMesh(
                "at least one mesh is required for concatenation".to_owned(),
            ));
        }
        let vertex_capacity = meshes
            .iter()
            .try_fold(0_usize, |total, mesh| {
                total.checked_add(mesh.vertices.len())
            })
            .ok_or_else(|| CoreError::InvalidMesh("mesh vertex count overflow".to_owned()))?;
        let face_capacity = meshes
            .iter()
            .try_fold(0_usize, |total, mesh| total.checked_add(mesh.faces.len()))
            .ok_or_else(|| CoreError::InvalidMesh("mesh face count overflow".to_owned()))?;
        let mut vertices = Vec::with_capacity(vertex_capacity);
        let mut faces = Vec::with_capacity(face_capacity);
        for mesh in meshes {
            mesh.validate_non_empty()?;
            let offset = u32::try_from(vertices.len())
                .map_err(|_| CoreError::InvalidMesh("mesh has too many vertices".to_owned()))?;
            vertices.extend_from_slice(&mesh.vertices);
            for face in &mesh.faces {
                faces.push([
                    face[0]
                        .checked_add(offset)
                        .ok_or_else(|| CoreError::InvalidMesh("face index overflow".to_owned()))?,
                    face[1]
                        .checked_add(offset)
                        .ok_or_else(|| CoreError::InvalidMesh("face index overflow".to_owned()))?,
                    face[2]
                        .checked_add(offset)
                        .ok_or_else(|| CoreError::InvalidMesh("face index overflow".to_owned()))?,
                ]);
            }
        }
        Self::new(vertices, faces)
    }

    /// Returns the normalized geometric normal for a face, or zero for a
    /// degenerate triangle.
    #[must_use]
    pub fn face_normal(&self, face: [u32; 3]) -> [f32; 3] {
        let Some(first) = self.vertices.get(face[0] as usize) else {
            return [0.0; 3];
        };
        let Some(second) = self.vertices.get(face[1] as usize) else {
            return [0.0; 3];
        };
        let Some(third) = self.vertices.get(face[2] as usize) else {
            return [0.0; 3];
        };
        let a = [
            second[0] - first[0],
            second[1] - first[1],
            second[2] - first[2],
        ];
        let b = [
            third[0] - first[0],
            third[1] - first[1],
            third[2] - first[2],
        ];
        let cross = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        let length = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
        if !length.is_finite() || length <= f32::EPSILON {
            [0.0; 3]
        } else {
            [
                normal_component(cross[0] / length),
                normal_component(cross[1] / length),
                normal_component(cross[2] / length),
            ]
        }
    }

    /// Encodes the mesh as the binary STL representation used by the Python
    /// project writer: an all-zero 80-byte header, little-endian float32
    /// coordinates, and zero attribute bytes.
    pub fn to_binary_stl(&self) -> Result<Vec<u8>, CoreError> {
        self.validate_non_empty()?;
        let face_count = u32::try_from(self.faces.len())
            .map_err(|_| CoreError::InvalidMesh("too many faces for binary STL".to_owned()))?;
        let payload = self
            .faces
            .len()
            .checked_mul(50)
            .and_then(|size| size.checked_add(84))
            .ok_or_else(|| CoreError::InvalidMesh("binary STL size overflow".to_owned()))?;
        let mut bytes = Vec::with_capacity(payload);
        bytes.extend_from_slice(&[0_u8; 80]);
        bytes.extend_from_slice(&face_count.to_le_bytes());
        for face in &self.faces {
            let normal = self.face_normal(*face);
            for value in normal {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            for index in face {
                let vertex = self.vertices[*index as usize];
                for value in vertex {
                    bytes.extend_from_slice(&value.to_le_bytes());
                }
            }
            bytes.extend_from_slice(&[0_u8, 0_u8]);
        }
        Ok(bytes)
    }

    /// Decodes a binary STL mesh into indexed vertices and triangle faces.
    pub fn from_binary_stl(bytes: &[u8]) -> Result<Self, CoreError> {
        if bytes.len() < 84 {
            return Err(CoreError::InvalidMesh(
                "binary STL is shorter than its header".to_owned(),
            ));
        }
        let count_bytes: [u8; 4] = bytes[80..84]
            .try_into()
            .map_err(|_| CoreError::InvalidMesh("binary STL count is truncated".to_owned()))?;
        let face_count = usize::try_from(u32::from_le_bytes(count_bytes))
            .map_err(|_| CoreError::InvalidMesh("binary STL face count is too large".to_owned()))?;
        let expected = face_count
            .checked_mul(50)
            .and_then(|size| size.checked_add(84))
            .ok_or_else(|| CoreError::InvalidMesh("binary STL size overflow".to_owned()))?;
        if bytes.len() != expected {
            return Err(CoreError::InvalidMesh(format!(
                "binary STL length {} does not match its face count ({expected})",
                bytes.len()
            )));
        }
        if face_count == 0 {
            return Err(CoreError::InvalidMesh(
                "binary STL contains no faces".to_owned(),
            ));
        }

        let mut vertices = Vec::with_capacity(face_count.saturating_mul(3));
        let mut faces = Vec::with_capacity(face_count);
        let mut vertex_map: HashMap<[u32; 3], u32> = HashMap::new();
        for face_index in 0..face_count {
            let base = 84 + face_index * 50 + 12;
            let mut indices = [0_u32; 3];
            for (corner, slot) in indices.iter_mut().enumerate() {
                let point_offset = base + corner * 12;
                let point = [
                    read_f32(bytes, point_offset)?,
                    read_f32(bytes, point_offset + 4)?,
                    read_f32(bytes, point_offset + 8)?,
                ];
                if point.iter().any(|value| !value.is_finite()) {
                    return Err(CoreError::InvalidMesh(
                        "binary STL contains a non-finite vertex".to_owned(),
                    ));
                }
                let key = [point[0].to_bits(), point[1].to_bits(), point[2].to_bits()];
                let index = if let Some(index) = vertex_map.get(&key) {
                    *index
                } else {
                    let index = u32::try_from(vertices.len()).map_err(|_| {
                        CoreError::InvalidMesh("binary STL has too many vertices".to_owned())
                    })?;
                    vertices.push(point);
                    vertex_map.insert(key, index);
                    index
                };
                *slot = index;
            }
            faces.push(indices);
        }

        Self::new(vertices, faces)
    }

    pub(crate) fn validate(&self) -> Result<(), CoreError> {
        if self
            .vertices
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(CoreError::InvalidMesh(
                "vertex coordinates must be finite".to_owned(),
            ));
        }
        let vertex_count = self.vertices.len();
        for (face_index, face) in self.faces.iter().enumerate() {
            if face.iter().any(|index| (*index as usize) >= vertex_count) {
                return Err(CoreError::InvalidMesh(format!(
                    "face {face_index} references a missing vertex"
                )));
            }
        }
        Ok(())
    }
}

fn read_f32(bytes: &[u8], offset: usize) -> Result<f32, CoreError> {
    let raw: [u8; 4] = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| CoreError::InvalidMesh("binary STL vertex is truncated".to_owned()))?
        .try_into()
        .map_err(|_| CoreError::InvalidMesh("binary STL vertex is truncated".to_owned()))?;
    Ok(f32::from_le_bytes(raw))
}

fn normal_component(value: f32) -> f32 {
    if value == 0.0 { 0.0 } else { value }
}
