use std::{
    collections::{BTreeSet, HashMap, HashSet, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    sync::Arc,
};

use modeler3d_core::{Mesh, Scene, SceneObject};

use crate::{
    ViewportError,
    math::{Aabb, Mat4, Vec3, cross, dot, normalize},
};

/// Python-compatible face threshold for entering the viewport LOD tier.
pub const VIEWPORT_LOD_FACE_THRESHOLD: u64 = 100_000;
/// Target face count for a modest display proxy.
pub const VIEWPORT_LOD_FACE_TARGET: u64 = 60_000;
/// Maximum authoritative face count that may be used for active interaction.
pub const INTERACTIVE_FULL_RES_FACE_LIMIT: u64 = 30_000_000;
/// Lower bound for an active interaction display tier.
pub const INTERACTIVE_LOD_MIN_FACE_TARGET: u64 = 1_000_000;
/// Upper bound for an active interaction display tier.
pub const INTERACTIVE_LOD_MAX_FACE_TARGET: u64 = 4_000_000;
/// Deterministic proportional target used by the Python reference.
pub const INTERACTIVE_LOD_RATIO: f64 = 0.12;
/// Maximum number of flattened render objects accepted from one scene.
pub const MAX_RENDER_OBJECTS: usize = 100_000;
/// Maximum supported source group depth, enforced by the core scene/project.
pub const MAX_RENDER_GROUP_DEPTH: usize = modeler3d_core::MAX_PROJECT_GROUP_DEPTH;
/// Bound temporary adjacency data used to preserve hard surface edges.
const CREASE_AWARE_FACE_LIMIT: usize = 250_000;
/// Adjacent triangles smoother than 60 degrees share vertex normals.
const SMOOTH_NORMAL_MINIMUM_DOT: f32 = 0.5;

/// Returns the active interaction target for an authoritative face count.
#[must_use]
pub fn interactive_viewport_face_target(total_faces: u64) -> u64 {
    if total_faces <= INTERACTIVE_FULL_RES_FACE_LIMIT {
        total_faces
    } else {
        ((total_faces as f64 * INTERACTIVE_LOD_RATIO) as u64).clamp(
            INTERACTIVE_LOD_MIN_FACE_TARGET,
            INTERACTIVE_LOD_MAX_FACE_TARGET,
        )
    }
}

/// GPU-ready indexed triangle data derived from authoritative geometry.
#[derive(Clone, Debug)]
pub struct MeshGeometry {
    positions: Vec<Vec3>,
    normals: Vec<Vec3>,
    indices: Vec<u32>,
    source_face_indices: Option<Vec<usize>>,
    bounds: Aabb,
}

impl MeshGeometry {
    /// Builds finite indexed data from one authoritative mesh.
    pub fn from_mesh(mesh: &Mesh) -> Result<Self, ViewportError> {
        mesh.validate_non_empty()?;
        if mesh.vertex_count() > u32::MAX as usize {
            return Err(ViewportError::CapacityExceeded(String::from(
                "mesh vertex count exceeds indexed GPU address space",
            )));
        }
        let (minimum, maximum) = mesh
            .bounds()
            .ok_or_else(|| ViewportError::InvalidInput(String::from("mesh has no bounds")))?;
        let bounds = Aabb::new(minimum, maximum)?;
        let index_capacity =
            mesh.faces.len().checked_mul(3).ok_or_else(|| {
                ViewportError::CapacityExceeded(String::from("index count overflow"))
            })?;
        if mesh.faces.len() <= CREASE_AWARE_FACE_LIMIT {
            return Self::from_mesh_crease_aware(mesh, bounds, index_capacity);
        }
        let mut indices = Vec::with_capacity(index_capacity);
        let mut normals = vec![[0.0_f32; 3]; mesh.vertices.len()];
        for face in &mesh.faces {
            let [first, second, third] = *face;
            if usize::try_from(first)
                .ok()
                .is_none_or(|index| index >= mesh.vertices.len())
                || usize::try_from(second)
                    .ok()
                    .is_none_or(|index| index >= mesh.vertices.len())
                || usize::try_from(third)
                    .ok()
                    .is_none_or(|index| index >= mesh.vertices.len())
            {
                return Err(ViewportError::InvalidInput(String::from(
                    "mesh face references an unavailable vertex",
                )));
            }
            let a = mesh.vertices[first as usize];
            let b = mesh.vertices[second as usize];
            let c = mesh.vertices[third as usize];
            let normal = cross(
                [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
                [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
            );
            for index in [first, second, third] {
                let target = &mut normals[index as usize];
                target[0] += normal[0];
                target[1] += normal[1];
                target[2] += normal[2];
            }
            indices.extend_from_slice(&[first, second, third]);
        }
        for normal in &mut normals {
            *normal = normalize(*normal, "mesh normal").unwrap_or([0.0, 1.0, 0.0]);
        }
        Ok(Self {
            positions: mesh.vertices.clone(),
            normals,
            indices,
            source_face_indices: None,
            bounds,
        })
    }

    fn from_mesh_crease_aware(
        mesh: &Mesh,
        bounds: Aabb,
        index_capacity: usize,
    ) -> Result<Self, ViewportError> {
        let corner_count = index_capacity;
        if corner_count > u32::MAX as usize {
            return Err(ViewportError::CapacityExceeded(String::from(
                "mesh face corners exceed indexed GPU address space",
            )));
        }
        let mut face_area_normals = Vec::with_capacity(mesh.faces.len());
        let mut face_directions = Vec::with_capacity(mesh.faces.len());
        for face in &mesh.faces {
            if face.iter().any(|index| {
                usize::try_from(*index)
                    .ok()
                    .is_none_or(|index| index >= mesh.vertices.len())
            }) {
                return Err(ViewportError::InvalidInput(String::from(
                    "mesh face references an unavailable vertex",
                )));
            }
            let first = mesh.vertices[face[0] as usize];
            let second = mesh.vertices[face[1] as usize];
            let third = mesh.vertices[face[2] as usize];
            let area_normal = cross(
                [
                    second[0] - first[0],
                    second[1] - first[1],
                    second[2] - first[2],
                ],
                [
                    third[0] - first[0],
                    third[1] - first[1],
                    third[2] - first[2],
                ],
            );
            face_area_normals.push(area_normal);
            face_directions.push(normalize(area_normal, "mesh face normal").unwrap_or([0.0; 3]));
        }

        let mut parents = (0..corner_count).collect::<Vec<_>>();
        let mut edges = HashMap::<(u32, u32), (usize, u8, u8)>::with_capacity(index_capacity);
        for (face_index, face) in mesh.faces.iter().enumerate() {
            for (first_corner, second_corner) in [(0_u8, 1_u8), (1, 2), (2, 0)] {
                let first_vertex = face[first_corner as usize];
                let second_vertex = face[second_corner as usize];
                let (key, low_corner, high_corner) = if first_vertex <= second_vertex {
                    ((first_vertex, second_vertex), first_corner, second_corner)
                } else {
                    ((second_vertex, first_vertex), second_corner, first_corner)
                };
                if let Some((other_face, other_low_corner, other_high_corner)) =
                    edges.get(&key).copied()
                {
                    if dot(face_directions[face_index], face_directions[other_face])
                        >= SMOOTH_NORMAL_MINIMUM_DOT
                    {
                        join_corner_groups(
                            &mut parents,
                            face_index * 3 + usize::from(low_corner),
                            other_face * 3 + usize::from(other_low_corner),
                        );
                        join_corner_groups(
                            &mut parents,
                            face_index * 3 + usize::from(high_corner),
                            other_face * 3 + usize::from(other_high_corner),
                        );
                    }
                } else {
                    edges.insert(key, (face_index, low_corner, high_corner));
                }
            }
        }

        let mut accumulated_normals = vec![[0.0_f32; 3]; corner_count];
        for (face_index, area_normal) in face_area_normals.iter().copied().enumerate() {
            for local_corner in 0..3 {
                let root = find_corner_root(&mut parents, face_index * 3 + local_corner);
                for axis in 0..3 {
                    accumulated_normals[root][axis] += area_normal[axis];
                }
            }
        }

        let mut group_indices = HashMap::<usize, u32>::with_capacity(mesh.vertices.len());
        let mut positions = Vec::with_capacity(mesh.vertices.len());
        let mut normals = Vec::with_capacity(mesh.vertices.len());
        let mut indices = Vec::with_capacity(index_capacity);
        for (face_index, face) in mesh.faces.iter().enumerate() {
            for (local_corner, face_vertex) in face.iter().copied().enumerate() {
                let root = find_corner_root(&mut parents, face_index * 3 + local_corner);
                let output_index = if let Some(index) = group_indices.get(&root).copied() {
                    index
                } else {
                    let index = u32::try_from(positions.len()).map_err(|_| {
                        ViewportError::CapacityExceeded(String::from(
                            "split mesh vertex count exceeds indexed GPU address space",
                        ))
                    })?;
                    positions.push(mesh.vertices[face_vertex as usize]);
                    normals.push(
                        normalize(accumulated_normals[root], "mesh smooth normal")
                            .unwrap_or(face_directions[face_index]),
                    );
                    group_indices.insert(root, index);
                    index
                };
                indices.push(output_index);
            }
        }

        Ok(Self {
            positions,
            normals,
            indices,
            source_face_indices: None,
            bounds,
        })
    }

    /// Builds a bounded deterministic display proxy by sampling authoritative faces.
    pub fn interaction_lod(&self, target_faces: u64) -> Result<Self, ViewportError> {
        let full_faces = self.face_count() as u64;
        if target_faces >= full_faces {
            return Ok(self.clone());
        }
        if target_faces == 0 {
            return Err(ViewportError::InvalidInput(String::from(
                "interaction LOD target must be non-zero",
            )));
        }
        let target = usize::try_from(target_faces).map_err(|_| {
            ViewportError::CapacityExceeded(String::from("interaction LOD target is too large"))
        })?;
        let mut positions = Vec::with_capacity(target.saturating_mul(3));
        let mut normals = Vec::with_capacity(target.saturating_mul(3));
        let mut indices = Vec::with_capacity(target.saturating_mul(3));
        let mut source_face_indices = Vec::with_capacity(target);
        let stride = full_faces.div_ceil(target_faces).max(1);
        let mut sampled = 0_usize;
        let mut face_index = 0_u64;
        while face_index < full_faces && sampled < target {
            let current_face_index = usize::try_from(face_index).map_err(|_| {
                ViewportError::CapacityExceeded(String::from("LOD face index overflow"))
            })?;
            let base = current_face_index.checked_mul(3).ok_or_else(|| {
                ViewportError::CapacityExceeded(String::from("LOD index overflow"))
            })?;
            let triangle = self.indices.get(base..base + 3).ok_or_else(|| {
                ViewportError::InvalidInput(String::from("indexed mesh triangle is truncated"))
            })?;
            for &index in triangle {
                let position = *self.positions.get(index as usize).ok_or_else(|| {
                    ViewportError::InvalidInput(String::from("LOD index is out of bounds"))
                })?;
                let normal = *self.normals.get(index as usize).ok_or_else(|| {
                    ViewportError::InvalidInput(String::from("LOD normal is out of bounds"))
                })?;
                let local_index = u32::try_from(positions.len()).map_err(|_| {
                    ViewportError::CapacityExceeded(String::from("LOD vertex count exceeds u32"))
                })?;
                positions.push(position);
                normals.push(normal);
                indices.push(local_index);
            }
            source_face_indices.push(self.source_face_index(current_face_index).ok_or_else(
                || ViewportError::InvalidInput(String::from("LOD source face is unavailable")),
            )?);
            sampled = sampled.saturating_add(1);
            face_index = face_index.saturating_add(stride);
        }
        if indices.is_empty() {
            return Err(ViewportError::CapacityExceeded(String::from(
                "interaction LOD could not select a triangle",
            )));
        }
        Ok(Self {
            positions,
            normals,
            indices,
            source_face_indices: Some(source_face_indices),
            bounds: self.bounds,
        })
    }

    /// Number of indexed triangle faces.
    #[must_use]
    pub fn face_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Number of vertices.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// Approximate tightly packed GPU bytes for the geometry.
    #[must_use]
    pub fn byte_size(&self) -> u64 {
        let positions = self.positions.len().saturating_mul(12);
        let normals = self.normals.len().saturating_mul(12);
        let indices = self.indices.len().saturating_mul(4);
        let source_faces = self.source_face_indices.as_ref().map_or(0, |values| {
            values.len().saturating_mul(std::mem::size_of::<usize>())
        });
        u64::try_from(
            positions
                .saturating_add(normals)
                .saturating_add(indices)
                .saturating_add(source_faces),
        )
        .unwrap_or(u64::MAX)
    }

    /// Returns the local bounds.
    #[must_use]
    pub const fn bounds(&self) -> Aabb {
        self.bounds
    }

    /// Returns packed interleaved position/normal vertex bytes.
    pub fn vertex_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.positions.len().saturating_mul(24));
        for (position, normal) in self.positions.iter().zip(&self.normals) {
            for value in position.iter().chain(normal) {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }

    /// Returns little-endian index bytes.
    pub fn index_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.indices.len().saturating_mul(4));
        for index in &self.indices {
            bytes.extend_from_slice(&index.to_le_bytes());
        }
        bytes
    }

    /// Returns one triangle's indices.
    #[must_use]
    pub(crate) fn triangle(&self, face_index: usize) -> Option<[u32; 3]> {
        let base = face_index.checked_mul(3)?;
        Some([
            *self.indices.get(base)?,
            *self.indices.get(base + 1)?,
            *self.indices.get(base + 2)?,
        ])
    }

    /// Maps a display face back to its authoritative face, including LOD.
    #[must_use]
    pub(crate) fn source_face_index(&self, face_index: usize) -> Option<usize> {
        if face_index >= self.face_count() {
            return None;
        }
        match &self.source_face_indices {
            Some(indices) => indices.get(face_index).copied(),
            None => Some(face_index),
        }
    }

    /// Returns one vertex position.
    #[must_use]
    pub(crate) fn position(&self, index: u32) -> Option<Vec3> {
        self.positions.get(index as usize).copied()
    }

    /// Returns one normalized local vertex normal.
    #[must_use]
    pub(crate) fn normal(&self, index: u32) -> Option<Vec3> {
        self.normals.get(index as usize).copied()
    }

    /// Returns unique world-space triangle edges for the opaque wireframe
    /// overlay. Edges are deduplicated by position so hard-edge normal splits
    /// do not draw the same boundary more than once.
    pub(crate) fn wireframe_segments(
        &self,
        transform: Mat4,
    ) -> Result<Vec<(Vec3, Vec3)>, ViewportError> {
        let mut edges = BTreeSet::new();
        for face_index in 0..self.face_count() {
            let Some([first, second, third]) = self.triangle(face_index) else {
                return Err(ViewportError::InvalidInput(String::from(
                    "wireframe triangle is truncated",
                )));
            };
            for (left, right) in [(first, second), (second, third), (third, first)] {
                let left = self.position(left).ok_or_else(|| {
                    ViewportError::InvalidInput(String::from("wireframe vertex is unavailable"))
                })?;
                let right = self.position(right).ok_or_else(|| {
                    ViewportError::InvalidInput(String::from("wireframe vertex is unavailable"))
                })?;
                let left = position_key(left);
                let right = position_key(right);
                edges.insert(if left < right {
                    (left, right)
                } else {
                    (right, left)
                });
            }
        }
        edges
            .into_iter()
            .map(|(first, second)| {
                let first = position_from_key(first);
                let second = position_from_key(second);
                Ok((
                    transform.transform_point(first)?,
                    transform.transform_point(second)?,
                ))
            })
            .collect()
    }

    /// Stable address of this immutable geometry allocation for renderer
    /// cache validation. The owning `Arc` keeps the allocation alive.
    #[must_use]
    pub(crate) fn geometry_identity(&self) -> usize {
        self as *const Self as usize
    }
}

fn find_corner_root(parents: &mut [usize], corner: usize) -> usize {
    let mut root = corner;
    while parents[root] != root {
        root = parents[root];
    }
    let mut current = corner;
    while parents[current] != current {
        let next = parents[current];
        parents[current] = root;
        current = next;
    }
    root
}

fn join_corner_groups(parents: &mut [usize], first: usize, second: usize) {
    let first_root = find_corner_root(parents, first);
    let second_root = find_corner_root(parents, second);
    if first_root != second_root {
        parents[second_root] = first_root;
    }
}

fn position_key(position: Vec3) -> [u32; 3] {
    position.map(|value| if value == 0.0 { 0.0_f32 } else { value }.to_bits())
}

fn position_from_key(key: [u32; 3]) -> Vec3 {
    key.map(f32::from_bits)
}

/// One visible scene object with its authoritative and display geometry.
#[derive(Clone, Debug)]
pub struct RenderObject {
    object_id: String,
    color: [f32; 4],
    face_colors: Option<Arc<[[u8; 4]]>>,
    face_color_identity: u64,
    /// Material texture (OF-46) and its repeat size in millimetres.
    texture: Option<(Arc<modeler3d_core::TextureImage>, f32)>,
    transform: Mat4,
    full_geometry: Arc<MeshGeometry>,
    interaction_geometry: Arc<MeshGeometry>,
    /// The source object's shape and look versions, so the next
    /// [`RenderScene::from_core_reusing`] can keep this object's derived
    /// data. Zero means "never reuse" (for example a preview mesh).
    shape_version: u64,
    look_version: u64,
}

impl RenderObject {
    /// Durable scene object ID.
    #[must_use]
    pub fn object_id(&self) -> &str {
        &self.object_id
    }

    /// Object color.
    #[must_use]
    pub const fn color(&self) -> [f32; 4] {
        self.color
    }

    /// Boolean-result per-face palette, when the source metadata is valid.
    #[must_use]
    pub fn face_colors(&self) -> Option<&[[u8; 4]]> {
        self.face_colors.as_deref()
    }

    /// Returns a display face color mapped through the authoritative source
    /// face indices retained by an interaction LOD.
    #[must_use]
    pub fn face_color_for_geometry(
        &self,
        geometry: &MeshGeometry,
        face_index: usize,
    ) -> Option<[u8; 4]> {
        let source_face = geometry.source_face_index(face_index)?;
        self.face_colors.as_ref()?.get(source_face).copied()
    }

    /// The object's texture image and repeat size, when it has one. It
    /// covers every face that has no painted or boolean source colour.
    #[must_use]
    pub fn texture(&self) -> Option<(&modeler3d_core::TextureImage, f32)> {
        self.texture
            .as_ref()
            .map(|(image, tile_mm)| (image.as_ref(), *tile_mm))
    }

    /// Stable cache key for the per-face palette and source assignments.
    #[must_use]
    pub(crate) const fn face_color_identity(&self) -> u64 {
        self.face_color_identity
    }

    fn face_color_bytes(&self) -> u64 {
        self.face_colors.as_ref().map_or(0, |colors| {
            u64::try_from(colors.len())
                .unwrap_or(u64::MAX)
                .saturating_mul(4)
        })
    }

    /// Object-to-world transform.
    #[must_use]
    pub const fn transform(&self) -> Mat4 {
        self.transform
    }

    /// Authoritative full-resolution geometry.
    #[must_use]
    pub fn full_geometry(&self) -> &MeshGeometry {
        &self.full_geometry
    }

    /// Bounded active-interaction geometry.
    #[must_use]
    pub fn interaction_geometry(&self) -> &MeshGeometry {
        &self.interaction_geometry
    }

    /// Returns the transformed world-space bounds.
    pub fn world_bounds(&self) -> Result<Aabb, ViewportError> {
        self.full_geometry.bounds.transformed(self.transform)
    }
}

/// Renderer-neutral flattened scene data owned by the viewport boundary.
#[derive(Clone, Debug, Default)]
pub struct RenderScene {
    objects: Vec<RenderObject>,
    authoritative_bytes: u64,
    interaction_bytes: u64,
}

impl RenderScene {
    /// Converts a core scene without changing its authoritative contents.
    pub fn from_core(scene: &Scene) -> Result<Self, ViewportError> {
        Self::from_core_reusing(scene, None)
    }

    /// [`Self::from_core`] that keeps the display geometry, face colors and
    /// texture of every object in `previous` whose shape and look have not
    /// changed, so editing one object does not rebuild (or re-upload to the
    /// GPU) the others.
    pub fn from_core_reusing(
        scene: &Scene,
        previous: Option<&Self>,
    ) -> Result<Self, ViewportError> {
        let mut result = Self::default();
        for object in scene.objects() {
            let earlier = previous.and_then(|previous| {
                previous
                    .objects
                    .iter()
                    .find(|candidate| candidate.object_id == object.object_id())
            });
            flatten_object(object, earlier, &mut result)?;
        }
        Ok(result)
    }

    /// Creates a scene from one render object, useful for renderer tests.
    pub fn from_object(object: RenderObject) -> Result<Self, ViewportError> {
        let mut scene = Self::default();
        scene.push(object)?;
        Ok(scene)
    }

    fn push(&mut self, object: RenderObject) -> Result<(), ViewportError> {
        if self.objects.len() >= MAX_RENDER_OBJECTS {
            return Err(ViewportError::CapacityExceeded(String::from(
                "flattened render-object limit reached",
            )));
        }
        self.authoritative_bytes = self
            .authoritative_bytes
            .checked_add(object.full_geometry.byte_size())
            .and_then(|bytes| bytes.checked_add(object.face_color_bytes()))
            .ok_or_else(|| {
                ViewportError::CapacityExceeded(String::from("scene byte count overflow"))
            })?;
        self.interaction_bytes = self
            .interaction_bytes
            .checked_add(object.interaction_geometry.byte_size())
            .ok_or_else(|| {
                ViewportError::CapacityExceeded(String::from("scene LOD byte count overflow"))
            })?;
        self.objects.push(object);
        Ok(())
    }

    /// Returns flattened objects in authoritative scene order.
    #[must_use]
    pub fn objects(&self) -> &[RenderObject] {
        &self.objects
    }

    /// Returns the authoritative derived geometry bytes.
    #[must_use]
    pub const fn authoritative_bytes(&self) -> u64 {
        self.authoritative_bytes
    }

    /// Returns the active interaction derived geometry bytes.
    #[must_use]
    pub const fn interaction_bytes(&self) -> u64 {
        self.interaction_bytes
    }

    /// Returns the total authoritative face count.
    #[must_use]
    pub fn face_count(&self) -> u64 {
        self.objects
            .iter()
            .map(|object| object.full_geometry.face_count() as u64)
            .sum()
    }

    /// Creates an isolated matrix preview sharing all retained mesh buffers.
    /// Durable top-level IDs remain authoritative even when hidden objects
    /// have been filtered from this render list. GPU and CPU paths consume
    /// the same derived transforms; no core scene/history is changed.
    pub fn with_world_preview(
        &self,
        object_ids: &HashSet<String>,
        delta: Mat4,
    ) -> Result<Self, ViewportError> {
        let mut preview = self.clone();
        for object in &mut preview.objects {
            if object_ids.contains(object.object_id()) {
                object.transform = Mat4::from_rows(delta.multiply(object.transform).rows())?;
            }
        }
        Ok(preview)
    }

    /// Creates an isolated mesh preview for one visible object.
    ///
    /// The authoritative core scene and its history remain untouched.  The
    /// replacement geometry is rebuilt for both the full-resolution renderer
    /// and the bounded interaction tier, which keeps picking and preview
    /// rendering on the same temporary mesh.
    pub fn with_mesh_preview(&self, object_id: &str, mesh: &Mesh) -> Result<Self, ViewportError> {
        let full = Arc::new(MeshGeometry::from_mesh(mesh)?);
        let target = interactive_viewport_face_target(full.face_count() as u64);
        let interaction = Arc::new(full.interaction_lod(target)?);
        let mut preview = self.clone();
        let Some(object) = preview
            .objects
            .iter_mut()
            .find(|object| object.object_id() == object_id)
        else {
            return Err(ViewportError::InvalidInput(format!(
                "preview object {object_id} is not visible"
            )));
        };
        preview.authoritative_bytes = preview
            .authoritative_bytes
            .saturating_sub(object.full_geometry.byte_size())
            .saturating_sub(object.face_color_bytes())
            .checked_add(full.byte_size())
            .ok_or_else(|| {
                ViewportError::CapacityExceeded(String::from("scene byte count overflow"))
            })?;
        preview.interaction_bytes = preview
            .interaction_bytes
            .saturating_sub(object.interaction_geometry.byte_size())
            .checked_add(interaction.byte_size())
            .ok_or_else(|| {
                ViewportError::CapacityExceeded(String::from("scene LOD byte count overflow"))
            })?;
        object.full_geometry = full;
        object.interaction_geometry = interaction;
        object.face_colors = None;
        object.face_color_identity = 0;
        object.shape_version = 0;
        object.look_version = 0;
        Ok(preview)
    }

    /// Returns the union of visible world bounds.
    pub fn world_bounds(&self) -> Result<Option<Aabb>, ViewportError> {
        let mut result = None;
        for object in &self.objects {
            let bounds = object.world_bounds()?;
            result = Some(result.map_or(bounds, |current: Aabb| current.union(bounds)));
        }
        Ok(result)
    }
}

fn flatten_object(
    object: &SceneObject,
    earlier: Option<&RenderObject>,
    scene: &mut RenderScene,
) -> Result<(), ViewportError> {
    if !object.is_visible() {
        return Ok(());
    }
    let color = object.color();
    if color.iter().any(|value| !value.is_finite()) {
        return Err(ViewportError::InvalidInput(String::from(
            "scene object color must be finite",
        )));
    }
    // The reference renders the combined top-level mesh. Children retain
    // their identities/metadata for ungrouping; drawing them again duplicates
    // geometry and makes exact picking return non-top-level IDs.
    let (full, interaction) =
        match earlier.filter(|earlier| earlier.shape_version == object.shape_version()) {
            Some(earlier) => (
                Arc::clone(&earlier.full_geometry),
                Arc::clone(&earlier.interaction_geometry),
            ),
            None => {
                let full = Arc::new(MeshGeometry::from_mesh(object.mesh())?);
                let target = interactive_viewport_face_target(full.face_count() as u64);
                let interaction = Arc::new(full.interaction_lod(target)?);
                (full, interaction)
            }
        };
    // Region paint (OF-44) or the source colours of a boolean result. They
    // depend on the look and on the face count.
    let (face_colors, face_color_identity, texture) = match earlier.filter(|earlier| {
        earlier.look_version == object.look_version()
            && earlier.shape_version == object.shape_version()
    }) {
        Some(earlier) => (
            earlier.face_colors.clone(),
            earlier.face_color_identity,
            earlier.texture.clone(),
        ),
        None => {
            let face_colors =
                modeler3d_core::object_face_colors(object).map(Arc::<[[u8; 4]]>::from);
            let mut palette_hasher = DefaultHasher::new();
            face_colors.hash(&mut palette_hasher);
            let face_color_identity = if face_colors.is_some() {
                palette_hasher.finish()
            } else {
                0
            };
            (
                face_colors,
                face_color_identity,
                modeler3d_core::resolved_texture(object.metadata()),
            )
        }
    };
    scene.push(RenderObject {
        object_id: object.object_id().to_owned(),
        color,
        face_colors,
        face_color_identity,
        texture,
        transform: Mat4::from_rows(object.transform().rows())?,
        full_geometry: full,
        interaction_geometry: interaction,
        shape_version: object.shape_version(),
        look_version: object.look_version(),
    })
}

#[cfg(test)]
mod tests {
    use modeler3d_core::{
        BooleanColorProvenance, Scene, SceneObject, make_box, make_sphere,
        with_boolean_color_provenance,
    };

    use super::*;

    #[test]
    fn interaction_target_matches_reference_bounds() {
        assert_eq!(interactive_viewport_face_target(30_000_000), 30_000_000);
        assert_eq!(interactive_viewport_face_target(30_000_001), 3_600_000);
        assert_eq!(interactive_viewport_face_target(100_000_000), 4_000_000);
    }

    #[test]
    fn scene_conversion_keeps_full_data_and_builds_interaction_data() {
        let mesh = make_box([2.0, 2.0, 2.0]).map_err(|_| ()).ok();
        let Some(mesh) = mesh else {
            return;
        };
        let mut scene = Scene::new();
        let add_result = scene.add_object(SceneObject::new(mesh));
        assert!(add_result.is_ok());
        let render_scene = RenderScene::from_core(&scene).map_err(|_| ()).ok();
        let Some(render_scene) = render_scene else {
            return;
        };
        assert_eq!(render_scene.objects().len(), 1);
        assert_eq!(render_scene.face_count(), 12);
        assert!(render_scene.authoritative_bytes() > 0);
        assert_eq!(
            render_scene.objects()[0].full_geometry().face_count(),
            render_scene.objects()[0]
                .interaction_geometry()
                .face_count()
        );
    }

    #[test]
    fn box_faces_keep_hard_edges_and_wireframe_boundaries_are_deduplicated() {
        let mesh = make_box([2.0; 3]).expect("box");
        let geometry = MeshGeometry::from_mesh(&mesh).expect("geometry");
        assert_eq!(geometry.vertex_count(), 24);
        for face_index in 0..geometry.face_count() {
            let [first, second, third] = geometry.triangle(face_index).expect("triangle");
            let first = geometry.normal(first).expect("first normal");
            let second = geometry.normal(second).expect("second normal");
            let third = geometry.normal(third).expect("third normal");
            assert!(dot(first, second) > 0.999);
            assert!(dot(first, third) > 0.999);
        }
        assert_eq!(
            geometry
                .wireframe_segments(Mat4::identity())
                .expect("wireframe")
                .len(),
            18
        );
    }

    #[test]
    fn curved_sphere_keeps_smooth_vertex_normals() {
        let mesh = make_sphere(2.0, 2).expect("sphere");
        let geometry = MeshGeometry::from_mesh(&mesh).expect("geometry");
        assert_eq!(geometry.vertex_count(), mesh.vertex_count());
        for index in 0..geometry.vertex_count() {
            let index = u32::try_from(index).expect("sphere vertex index");
            let position = geometry.position(index).expect("position");
            let normal = geometry.normal(index).expect("normal");
            let radial = normalize(position, "sphere radial direction").expect("radial");
            assert!(dot(radial, normal) > 0.98);
        }
    }

    #[test]
    fn mesh_preview_rebuilds_geometry_without_mutating_core_scene() {
        let original = make_box([2.0, 2.0, 2.0]).map_err(|_| ()).ok();
        let replacement = make_box([4.0, 4.0, 4.0]).map_err(|_| ()).ok();
        let (Some(original), Some(replacement)) = (original, replacement) else {
            return;
        };
        let mut scene = Scene::new();
        assert!(scene.add_object(SceneObject::new(original)).is_ok());
        let object_id = scene.objects()[0].object_id().to_owned();
        let render = RenderScene::from_core(&scene).map_err(|_| ()).ok();
        let Some(render) = render else {
            return;
        };
        let preview = render.with_mesh_preview(&object_id, &replacement);
        assert!(preview.is_ok());
        let preview = preview.unwrap_or_default();
        assert_eq!(
            preview.objects()[0].full_geometry().bounds().maximum,
            [2.0; 3]
        );
        assert_eq!(
            preview.objects()[0].full_geometry().bounds().minimum,
            [-2.0; 3]
        );
        assert_eq!(
            render.objects()[0].full_geometry().bounds().maximum,
            [1.0; 3]
        );
        assert_eq!(
            render.objects()[0].full_geometry().bounds().minimum,
            [-1.0; 3]
        );
        assert_eq!(
            scene.objects()[0].mesh().bounds().map(|(_, max)| max[0]),
            Some(1.0)
        );
    }

    #[test]
    fn boolean_face_colors_reach_full_and_interaction_geometry() {
        let mesh = make_box([2.0, 2.0, 2.0]).expect("box");
        let source_colors = [[0.2, 0.4, 0.6, 1.0], [0.8, 0.2, 0.1, 1.0]];
        let provenance = BooleanColorProvenance {
            operation: String::from("subtract"),
            source_colors,
            face_sources: (0..mesh.face_count())
                .map(|index| u8::try_from(index % 2).unwrap_or(0))
                .collect(),
            tool_face_count: mesh.face_count() / 2,
        };
        let metadata = with_boolean_color_provenance(&serde_json::json!({}), &provenance);
        let object = SceneObject::with_details(
            mesh.clone(),
            None,
            Some(String::from("Boolean result")),
            Some(source_colors[0]),
            Some(metadata),
        )
        .expect("scene object");
        let expected_colors = provenance.face_colors_rgba8();
        let full = MeshGeometry::from_mesh(&mesh).expect("full geometry");
        let interaction = full.interaction_lod(4).expect("interaction LOD");
        let mut scene = Scene::new();
        scene.add_object(object).expect("add object");
        let render_scene = RenderScene::from_core(&scene).expect("render scene");
        let render_object = render_scene
            .objects()
            .first()
            .cloned()
            .expect("render object");

        assert_eq!(
            (0..full.face_count())
                .map(|face| render_object.face_color_for_geometry(&full, face))
                .collect::<Vec<_>>(),
            expected_colors
                .iter()
                .copied()
                .map(Some)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            (0..interaction.face_count())
                .map(|face| render_object.face_color_for_geometry(&interaction, face))
                .collect::<Vec<_>>(),
            [0, 3, 6, 9]
                .into_iter()
                .map(|face| Some(expected_colors[face]))
                .collect::<Vec<_>>()
        );

        let preview = render_scene
            .with_mesh_preview(render_object.object_id(), &mesh)
            .expect("mesh preview");
        assert!(preview.objects()[0].face_colors().is_none());
    }
}

#[cfg(test)]
mod rb04_render_regressions {
    use super::*;
    use crate::{Ray, pick_ray};
    use modeler3d_core::{Scene, SceneObject, Transform, make_box};
    #[test]
    fn group_draws_and_picks_its_combined_mesh_once_with_parent_identity() {
        let mut scene = Scene::new();
        for x in [-3., 3.] {
            let index = scene.objects().len();
            scene
                .add_object(SceneObject::new(make_box([2.; 3]).expect("box")))
                .expect("add");
            scene
                .set_transform(
                    index,
                    Transform::identity()
                        .translated([x, 0., 0.])
                        .expect("translation"),
                )
                .expect("transform");
        }
        scene.set_visible(0, false).expect("hide child");
        scene.set_selection([0, 1]);
        scene.group_selected().expect("group");
        let root = scene.objects()[0].object_id().to_owned();
        let revision = scene.revision();
        let render = RenderScene::from_core(&scene).expect("render scene");
        assert_eq!(render.objects().len(), 1);
        assert_eq!(render.face_count(), 24);
        assert_eq!(render.objects()[0].object_id(), root);
        let hit = pick_ray(
            &render,
            Ray::new([-3., 0., 10.], [0., 0., -1.]).expect("ray"),
        )
        .expect("pick")
        .expect("hit");
        assert_eq!(hit.object_id, root);
        assert!(hit.face_index < 24);
        assert!(!scene.objects()[0].children()[0].is_visible());
        assert_eq!(scene.revision(), revision);
    }
    #[test]
    fn matrix_preview_uses_ids_after_hidden_filtering_and_shares_mesh_buffers() {
        let mut scene = Scene::new();
        for _ in 0..2 {
            scene
                .add_object(SceneObject::new(make_box([2.; 3]).expect("box")))
                .expect("add");
        }
        scene.set_visible(0, false).expect("hide");
        let target = scene.objects()[1].object_id().to_owned();
        let render = RenderScene::from_core(&scene).expect("render");
        assert_eq!(render.objects()[0].object_id(), target);
        let delta = Mat4::from_rows(
            Transform::identity()
                .translated([4., 5., 6.])
                .expect("translation")
                .rows(),
        )
        .expect("matrix");
        let ids = [target].into_iter().collect::<HashSet<_>>();
        let preview = render.with_world_preview(&ids, delta).expect("preview");
        assert!(Arc::ptr_eq(
            &render.objects[0].full_geometry,
            &preview.objects[0].full_geometry
        ));
        assert!(Arc::ptr_eq(
            &render.objects[0].interaction_geometry,
            &preview.objects[0].interaction_geometry
        ));
        assert_eq!(
            preview.objects()[0]
                .transform()
                .transform_point([0.; 3])
                .expect("point"),
            [4., 5., 6.]
        );
        assert_eq!(
            render.objects()[0]
                .transform()
                .transform_point([0.; 3])
                .expect("original"),
            [0.; 3]
        );
        assert_eq!(scene.objects()[1].position(), [0.; 3]);
        assert_eq!(render.authoritative_bytes(), preview.authoritative_bytes());
    }
}
