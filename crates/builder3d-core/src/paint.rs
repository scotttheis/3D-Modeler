//! Paint (OF-44): colour a whole object or a region of its faces.
//!
//! Region colours live in object metadata under [`FACE_PAINT_KEY`], next to
//! the boolean colour provenance, so the `.3dp` layout is unchanged: an older
//! reader that ignores the key shows the object's own colour. Face entry `0`
//! always means "the object's own colour", so the Material dialog still
//! recolours every face that was not painted separately.

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    Command, CoreError, Mesh, ObjectId, SceneObject, boolean_color_provenance, color_rgba8,
    project::MAX_PROJECT_OBJECT_METADATA_BYTES,
};

/// Reserved object metadata key for region paint.
pub const FACE_PAINT_KEY: &str = "face_paint";

/// Most distinct painted colours one object keeps besides its own colour.
pub const MAX_FACE_PAINT_COLORS: usize = 255;

/// Smallest and largest normal angle a region grows across, in degrees,
/// reached at coverage 0 and just below coverage 1.
const REGION_ANGLE_MIN_DEG: f32 = 1.0;
const REGION_ANGLE_MAX_DEG: f32 = 180.0;

/// Per-face paint for one object.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FacePaint {
    /// Painted colours. Face entry `n >= 1` uses `palette[n - 1]`.
    pub palette: Vec<[f32; 4]>,
    /// Run-length face entries, flat as `[entry, count, entry, count, ..]`.
    pub runs: Vec<u32>,
}

impl FacePaint {
    /// Builds region paint from one entry per face.
    fn encode(palette: Vec<[f32; 4]>, entries: &[u16]) -> Self {
        let mut runs = Vec::new();
        let mut iter = entries.iter().copied();
        if let Some(mut current) = iter.next() {
            let mut count = 1_u32;
            for entry in iter {
                if entry == current && count < u32::MAX {
                    count += 1;
                } else {
                    runs.push(u32::from(current));
                    runs.push(count);
                    current = entry;
                    count = 1;
                }
            }
            runs.push(u32::from(current));
            runs.push(count);
        }
        Self { palette, runs }
    }

    /// Expands the runs to one entry per face, or `None` when the runs do not
    /// describe exactly `face_count` faces with known palette entries.
    fn entries(&self, face_count: usize) -> Option<Vec<u16>> {
        if !self.runs.len().is_multiple_of(2) || self.palette.len() > MAX_FACE_PAINT_COLORS {
            return None;
        }
        let mut entries = Vec::with_capacity(face_count);
        for pair in self.runs.as_chunks::<2>().0 {
            let entry = u16::try_from(pair[0]).ok()?;
            if usize::from(entry) > self.palette.len() {
                return None;
            }
            let count = usize::try_from(pair[1]).ok()?;
            if count == 0 || entries.len().checked_add(count)? > face_count {
                return None;
            }
            entries.extend(std::iter::repeat_n(entry, count));
        }
        (entries.len() == face_count).then_some(entries)
    }

    fn is_valid_for_face_count(&self, face_count: usize) -> bool {
        self.palette.iter().all(|color| valid_color(*color)) && self.entries(face_count).is_some()
    }
}

/// Parses valid region paint for a known mesh face count.
#[must_use]
pub fn face_paint(metadata: &Value, face_count: usize) -> Option<FacePaint> {
    let value = metadata.get(FACE_PAINT_KEY)?.clone();
    let paint: FacePaint = serde_json::from_value(value).ok()?;
    paint.is_valid_for_face_count(face_count).then_some(paint)
}

/// Returns one RGBA8 colour per face when the object has more than one face
/// colour (region paint, or the source colours of a boolean result), and
/// `None` when the whole object is drawn in its own colour.
#[must_use]
pub fn object_face_colors(object: &SceneObject) -> Option<Vec<[u8; 4]>> {
    face_colors_f32(object).map(|colors| colors.into_iter().map(color_rgba8).collect())
}

fn face_colors_f32(object: &SceneObject) -> Option<Vec<[f32; 4]>> {
    let face_count = object.mesh().face_count();
    if let Some(paint) = face_paint(object.metadata(), face_count) {
        let entries = paint.entries(face_count)?;
        return Some(
            entries
                .into_iter()
                .map(|entry| match entry {
                    0 => object.color(),
                    other => paint.palette[usize::from(other) - 1],
                })
                .collect(),
        );
    }
    boolean_color_provenance(object.metadata(), face_count).map(|provenance| {
        provenance
            .face_sources
            .iter()
            .map(|source| provenance.source_colors[usize::from(*source)])
            .collect()
    })
}

/// Removes region paint, for edits that change the face list.
#[must_use]
pub fn without_face_paint(metadata: &Value) -> Value {
    let mut updated = metadata.clone();
    if let Some(fields) = updated.as_object_mut() {
        fields.remove(FACE_PAINT_KEY);
    }
    updated
}

/// Finds the faces one Paint click colours.
///
/// `coverage` is the Coverage slider, `0.0..=1.0`. At `1.0` every face of the
/// object is painted. Below that, the region grows from `seed_face` across
/// shared edges to faces whose normal is within an angle of the clicked
/// face's normal; the angle widens with coverage (about 1 degree at 0, a
/// flat side of a box; about 46 degrees at 0.5).
pub fn paint_region_faces(
    mesh: &Mesh,
    seed_face: usize,
    coverage: f32,
) -> Result<Vec<usize>, CoreError> {
    let face_count = mesh.face_count();
    if seed_face >= face_count {
        return Err(CoreError::InvalidObject(format!(
            "face {seed_face} is not on this object ({face_count} faces)"
        )));
    }
    if !coverage.is_finite() || !(0.0..=1.0).contains(&coverage) {
        return Err(CoreError::InvalidObject(
            "coverage must be between 0 and 1".to_owned(),
        ));
    }
    if coverage >= 1.0 {
        return Ok((0..face_count).collect());
    }
    let limit_deg =
        REGION_ANGLE_MIN_DEG + (REGION_ANGLE_MAX_DEG - REGION_ANGLE_MIN_DEG) * coverage * coverage;
    let cos_limit = limit_deg.to_radians().cos();
    let seed_normal = mesh.face_normal(mesh.faces[seed_face]);

    // Weld by exact position so split-vertex meshes (flat-shaded primitives,
    // STL imports) still connect across their edges.
    let mut welded = HashMap::with_capacity(mesh.vertex_count());
    let vertex_key: Vec<u32> = mesh
        .vertices
        .iter()
        .map(|position| {
            let key = position.map(f32::to_bits);
            let next = u32::try_from(welded.len()).unwrap_or(u32::MAX);
            *welded.entry(key).or_insert(next)
        })
        .collect();
    let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::with_capacity(face_count * 2);
    for (face_index, face) in mesh.faces.iter().enumerate() {
        let ids = face.map(|vertex| vertex_key.get(vertex as usize).copied().unwrap_or(u32::MAX));
        for (a, b) in [(ids[0], ids[1]), (ids[1], ids[2]), (ids[2], ids[0])] {
            if a != b {
                edge_faces
                    .entry((a.min(b), a.max(b)))
                    .or_default()
                    .push(face_index);
            }
        }
    }

    let mut included = vec![false; face_count];
    included[seed_face] = true;
    let mut queue = VecDeque::from([seed_face]);
    let mut region = Vec::new();
    while let Some(face_index) = queue.pop_front() {
        region.push(face_index);
        let face = mesh.faces[face_index];
        let ids = face.map(|vertex| vertex_key.get(vertex as usize).copied().unwrap_or(u32::MAX));
        for (a, b) in [(ids[0], ids[1]), (ids[1], ids[2]), (ids[2], ids[0])] {
            let Some(neighbours) = edge_faces.get(&(a.min(b), a.max(b))) else {
                continue;
            };
            for &neighbour in neighbours {
                if included[neighbour] {
                    continue;
                }
                let normal = mesh.face_normal(mesh.faces[neighbour]);
                let dot = normal[0] * seed_normal[0]
                    + normal[1] * seed_normal[1]
                    + normal[2] * seed_normal[2];
                if dot >= cos_limit {
                    included[neighbour] = true;
                    queue.push_back(neighbour);
                }
            }
        }
    }
    region.sort_unstable();
    Ok(region)
}

/// Works out the colour and metadata after painting `faces` with `color`.
/// When every face ends up one colour, the object simply takes that colour
/// and loses its region paint and boolean source colours.
fn painted_appearance(
    object: &SceneObject,
    faces: Option<&[usize]>,
    color: [f32; 4],
) -> Result<([f32; 4], Value), CoreError> {
    if !valid_color(color) {
        return Err(CoreError::InvalidColor(
            "RGBA values must be finite and between 0 and 1".to_owned(),
        ));
    }
    let face_count = object.mesh().face_count();
    let mut colors = face_colors_f32(object).unwrap_or_else(|| vec![object.color(); face_count]);
    match faces {
        None => colors.iter_mut().for_each(|face| *face = color),
        Some(faces) => {
            for &face in faces {
                let slot = colors.get_mut(face).ok_or_else(|| {
                    CoreError::InvalidObject(format!(
                        "face {face} is not on this object ({face_count} faces)"
                    ))
                })?;
                *slot = color;
            }
        }
    }
    let cleared = crate::without_boolean_color_provenance(&without_face_paint(object.metadata()));
    let Some(first) = colors.first().copied() else {
        return Ok((object.color(), cleared));
    };
    if colors.iter().all(|face| *face == first) {
        return Ok((first, cleared));
    }
    let base = object.color();
    let mut palette: Vec<[f32; 4]> = Vec::new();
    let mut entries = Vec::with_capacity(face_count);
    for face in &colors {
        if *face == base {
            entries.push(0_u16);
            continue;
        }
        let position = match palette.iter().position(|known| known == face) {
            Some(position) => position,
            None => {
                if palette.len() >= MAX_FACE_PAINT_COLORS {
                    return Err(CoreError::InvalidObject(format!(
                        "an object can hold at most {MAX_FACE_PAINT_COLORS} painted colours"
                    )));
                }
                palette.push(*face);
                palette.len() - 1
            }
        };
        entries.push(u16::try_from(position + 1).unwrap_or(u16::MAX));
    }
    let paint = FacePaint::encode(palette, &entries);
    let value = serde_json::to_value(&paint)
        .map_err(|error| CoreError::InvalidObject(format!("paint could not be stored: {error}")))?;
    let mut metadata = cleared;
    let Some(fields) = metadata.as_object_mut() else {
        return Err(CoreError::InvalidObject(
            "metadata must be a JSON object".to_owned(),
        ));
    };
    fields.insert(FACE_PAINT_KEY.to_owned(), value);
    let size = serde_json::to_vec(&metadata).map_or(usize::MAX, |bytes| bytes.len());
    if size > MAX_PROJECT_OBJECT_METADATA_BYTES {
        return Err(CoreError::InvalidObject(
            "this object has too many separately painted areas to save; paint a larger area"
                .to_owned(),
        ));
    }
    Ok((base, metadata))
}

/// Paints a whole object or some of its faces as one undo entry. Painting
/// that changes nothing is a no-op with no history entry.
pub struct PaintFacesCommand {
    index: usize,
    faces: Option<Vec<usize>>,
    color: [f32; 4],
    applied: Option<([f32; 4], Value)>,
    previous: Option<([f32; 4], Value)>,
    target_id: Option<ObjectId>,
}

impl PaintFacesCommand {
    /// Paints every face of the object at `index`.
    #[must_use]
    pub const fn whole_object(index: usize, color: [f32; 4]) -> Self {
        Self {
            index,
            faces: None,
            color,
            applied: None,
            previous: None,
            target_id: None,
        }
    }

    /// Paints the listed faces of the object at `index`.
    #[must_use]
    pub const fn faces(index: usize, faces: Vec<usize>, color: [f32; 4]) -> Self {
        Self {
            index,
            faces: Some(faces),
            color,
            applied: None,
            previous: None,
            target_id: None,
        }
    }
}

impl Command for PaintFacesCommand {
    fn name(&self) -> &str {
        "paint"
    }

    fn execute(&mut self, scene: &mut crate::Scene) -> Result<bool, CoreError> {
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        if self.target_id.is_none() {
            self.target_id = Some(object.object_id().to_owned());
        }
        let (color, metadata) = match &self.applied {
            Some(applied) => applied.clone(),
            None => painted_appearance(object, self.faces.as_deref(), self.color)?,
        };
        if object.color() == color && object.metadata() == &metadata {
            return Ok(false);
        }
        if self.previous.is_none() {
            self.previous = Some((object.color(), object.metadata().clone()));
        }
        object.set_color_raw(color);
        object.set_metadata_raw(metadata.clone());
        self.applied = Some((color, metadata));
        Ok(true)
    }

    fn undo(&mut self, scene: &mut crate::Scene) -> Result<(), CoreError> {
        let (color, metadata) = self
            .previous
            .clone()
            .ok_or_else(|| CoreError::CommandFailed {
                command: self.name().to_owned(),
                reason: "paint command has not executed".to_owned(),
            })?;
        let object = scene
            .objects_mut()
            .get_mut(self.index)
            .ok_or(CoreError::ObjectIndex(self.index))?;
        object.set_color_raw(color);
        object.set_metadata_raw(metadata);
        Ok(())
    }

    fn affected_object_ids(&self) -> Vec<ObjectId> {
        self.target_id.iter().cloned().collect()
    }
}

fn valid_color(color: [f32; 4]) -> bool {
    color
        .iter()
        .all(|channel| channel.is_finite() && (0.0..=1.0).contains(channel))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Scene, make_box};

    fn cube_scene() -> Scene {
        let mesh = match make_box([20.0, 20.0, 20.0]) {
            Ok(mesh) => mesh,
            Err(error) => panic!("box failed: {error}"),
        };
        let object = match SceneObject::with_details(
            mesh,
            None,
            Some(String::from("Cube")),
            Some([0.5, 0.5, 0.5, 1.0]),
            None,
        ) {
            Ok(object) => object,
            Err(error) => panic!("object failed: {error}"),
        };
        let mut scene = Scene::new();
        assert!(scene.add_object(object).is_ok());
        scene
    }

    const RED: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

    #[test]
    fn low_coverage_paints_one_side_of_a_box_and_full_coverage_all_of_it() {
        let scene = cube_scene();
        let mesh = scene.objects()[0].mesh();
        let side = paint_region_faces(mesh, 0, 0.0).unwrap_or_default();
        assert_eq!(side.len(), 2, "one flat side of a box is two triangles");
        let all = paint_region_faces(mesh, 0, 1.0).unwrap_or_default();
        assert_eq!(all.len(), mesh.face_count());
        // Just under full coverage still stays on connected faces with a
        // normal less than 180 degrees from the clicked one.
        let most = paint_region_faces(mesh, 0, 0.99).unwrap_or_default();
        assert!(most.len() > side.len());
        assert!(paint_region_faces(mesh, mesh.face_count(), 0.5).is_err());
        assert!(paint_region_faces(mesh, 0, 1.5).is_err());
    }

    #[test]
    fn region_paint_is_one_undo_entry_and_keeps_the_rest_of_the_object() {
        let mut scene = cube_scene();
        let history = scene.history_state_id();
        let faces = paint_region_faces(scene.objects()[0].mesh(), 0, 0.0).unwrap_or_default();
        let result = scene.execute(PaintFacesCommand::faces(0, faces.clone(), RED));
        assert!(result.is_ok_and(|result| result.changed));
        assert_eq!(scene.history_state_id(), history + 1);
        let object = &scene.objects()[0];
        assert_eq!(object.color(), [0.5, 0.5, 0.5, 1.0]);
        let colors = object_face_colors(object).unwrap_or_default();
        assert_eq!(colors.len(), object.mesh().face_count());
        for (index, color) in colors.iter().enumerate() {
            let expected = if faces.contains(&index) {
                [255, 0, 0, 255]
            } else {
                [128, 128, 128, 255]
            };
            assert_eq!(*color, expected);
        }
        // The same paint again changes nothing and makes no history entry.
        let again = scene.execute(PaintFacesCommand::faces(0, faces, RED));
        assert!(again.is_ok_and(|result| !result.changed));
        assert_eq!(scene.history_state_id(), history + 1);
        assert!(scene.undo().is_ok_and(|result| result.is_some()));
        assert!(object_face_colors(&scene.objects()[0]).is_none());
        assert!(scene.redo().is_ok_and(|result| result.is_some()));
        assert!(object_face_colors(&scene.objects()[0]).is_some());
    }

    #[test]
    fn painting_every_face_one_colour_becomes_the_object_colour() {
        let mut scene = cube_scene();
        let faces = paint_region_faces(scene.objects()[0].mesh(), 0, 0.0).unwrap_or_default();
        assert!(
            scene
                .execute(PaintFacesCommand::faces(0, faces, RED))
                .is_ok()
        );
        assert!(
            scene
                .execute(PaintFacesCommand::whole_object(0, RED))
                .is_ok()
        );
        let object = &scene.objects()[0];
        assert_eq!(object.color(), RED);
        assert!(object.metadata().get(FACE_PAINT_KEY).is_none());
        assert!(object_face_colors(object).is_none());
    }

    #[test]
    fn material_colour_still_recolours_the_unpainted_faces() {
        let mut scene = cube_scene();
        let faces = paint_region_faces(scene.objects()[0].mesh(), 0, 0.0).unwrap_or_default();
        assert!(
            scene
                .execute(PaintFacesCommand::faces(0, faces, RED))
                .is_ok()
        );
        let blue = [0.0, 0.0, 1.0, 1.0];
        assert!(scene.set_color(0, blue).is_ok());
        let colors = object_face_colors(&scene.objects()[0]).unwrap_or_default();
        assert_eq!(colors.iter().filter(|c| **c == [255, 0, 0, 255]).count(), 2);
        assert_eq!(
            colors.iter().filter(|c| **c == [0, 0, 255, 255]).count(),
            colors.len() - 2
        );
    }

    #[test]
    fn a_mesh_change_drops_region_paint() {
        let mut scene = cube_scene();
        let faces = paint_region_faces(scene.objects()[0].mesh(), 0, 0.0).unwrap_or_default();
        assert!(
            scene
                .execute(PaintFacesCommand::faces(0, faces, RED))
                .is_ok()
        );
        let replacement = match make_box([10.0, 10.0, 10.0]) {
            Ok(mesh) => mesh,
            Err(error) => panic!("box failed: {error}"),
        };
        assert!(scene.replace_mesh(0, replacement).is_ok());
        assert!(scene.objects()[0].metadata().get(FACE_PAINT_KEY).is_none());
    }

    #[test]
    fn region_paint_survives_a_project_save_and_reopen() {
        let mut scene = cube_scene();
        let faces = paint_region_faces(scene.objects()[0].mesh(), 0, 0.0).unwrap_or_default();
        assert!(scene.execute(PaintFacesCommand::faces(0, faces, RED)).is_ok());
        let before = object_face_colors(&scene.objects()[0]);
        let bytes = match crate::save_project_bytes(&scene) {
            Ok(bytes) => bytes,
            Err(error) => panic!("save failed: {error}"),
        };
        let reopened = match crate::load_project_bytes(&bytes) {
            Ok(scene) => scene,
            Err(error) => panic!("load failed: {error}"),
        };
        assert!(before.is_some());
        assert_eq!(object_face_colors(&reopened.objects()[0]), before);
    }

    #[test]
    fn invalid_stored_paint_is_ignored() {
        let metadata = serde_json::json!({ FACE_PAINT_KEY: { "palette": [[1.0, 0.0, 0.0, 1.0]], "runs": [1, 3] } });
        assert!(face_paint(&metadata, 4).is_none());
        assert!(face_paint(&metadata, 3).is_some());
        let bad_entry = serde_json::json!({ FACE_PAINT_KEY: { "palette": [], "runs": [1, 3] } });
        assert!(face_paint(&bad_entry, 3).is_none());
    }
}
