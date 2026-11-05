//! Renderer-neutral object transformation workflows.
//!
//! The Python application keeps multi-object transforms in an application
//! service rather than in widgets.  This module is the equivalent Rust seam:
//! it computes world-space bounds and new transforms first, then commits all
//! changed objects through one core history command.

use std::collections::BTreeSet;

use builder3d_core::{
    Command as CoreCommand, CoreError, GroupedCommand, Scene, Transform, TransformObjectCommand,
};
use builder3d_viewport::SnapError;
use thiserror::Error;

/// A world-space axis used by transform and alignment workflows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    /// Horizontal X axis.
    X,
    /// Vertical build-bed Y axis.
    Y,
    /// Horizontal depth Z axis.
    Z,
}

impl Axis {
    /// Returns the matrix/bounds component index.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

/// A world-space landmark used by alignment workflows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Alignment {
    /// Match the minimum edge to the first selected object.
    Minimum,
    /// Match the center to the first selected object.
    Center,
    /// Match the maximum edge to the first selected object.
    Maximum,
}

/// One mutually exclusive group transformation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GroupTransform {
    /// Translate all selected objects in world millimetres.
    Translation([f32; 3]),
    /// Rotate all selected objects around a world-space pivot in XYZ degrees.
    Rotation([f32; 3]),
    /// Scale all selected objects around a world-space pivot.
    Scale([f32; 3]),
    /// Reflect all selected objects through the world origin plane.
    Reflection([f32; 3]),
}

/// A finite world-space axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldBounds {
    /// Minimum XYZ point in millimetres.
    pub minimum: [f32; 3],
    /// Maximum XYZ point in millimetres.
    pub maximum: [f32; 3],
}

impl WorldBounds {
    /// Returns the center of the box.
    #[must_use]
    pub fn center(self) -> [f32; 3] {
        [
            self.minimum[0] * 0.5 + self.maximum[0] * 0.5,
            self.minimum[1] * 0.5 + self.maximum[1] * 0.5,
            self.minimum[2] * 0.5 + self.maximum[2] * 0.5,
        ]
    }

    /// Returns the XYZ extent of the box.
    #[must_use]
    pub fn extents(self) -> [f32; 3] {
        [
            self.maximum[0] - self.minimum[0],
            self.maximum[1] - self.minimum[1],
            self.maximum[2] - self.minimum[2],
        ]
    }

    fn include(&mut self, other: Self) {
        for axis in 0..3 {
            self.minimum[axis] = self.minimum[axis].min(other.minimum[axis]);
            self.maximum[axis] = self.maximum[axis].max(other.maximum[axis]);
        }
    }
}

/// Errors at the application transform seam.
#[derive(Debug, Error)]
pub enum TransformError {
    /// A core mesh or transform operation rejected its input.
    #[error(transparent)]
    Core(#[from] CoreError),
    /// A transform requires at least one selected object.
    #[error("{0} requires at least one selected object")]
    SelectionRequired(&'static str),
    /// An alignment/arrangement operation requires two objects.
    #[error("{0} requires at least two selected objects")]
    PairRequired(&'static str),
    /// A numeric transform value was outside the bounded contract.
    #[error("invalid {name}: {reason}")]
    InvalidValue {
        /// Field name.
        name: &'static str,
        /// Short validation reason.
        reason: &'static str,
    },
    /// Landmark snapping rejected a bounded drag query.
    #[error(transparent)]
    Snap(#[from] SnapError),
}

/// Returns ordered, deduplicated indices and validates their scene range.
pub fn ordered_indices(
    scene: &Scene,
    indices: impl IntoIterator<Item = usize>,
    operation: &'static str,
) -> Result<Vec<usize>, TransformError> {
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    for index in indices {
        if index >= scene.objects().len() {
            return Err(TransformError::InvalidValue {
                name: operation,
                reason: "object index is out of range",
            });
        }
        if seen.insert(index) {
            result.push(index);
        }
    }
    if result.is_empty() {
        return Err(TransformError::SelectionRequired(operation));
    }
    Ok(result)
}

/// Computes the combined world bounds of selected top-level objects.
pub fn world_bounds(scene: &Scene, indices: &[usize]) -> Result<WorldBounds, TransformError> {
    let mut bounds: Option<WorldBounds> = None;
    for index in indices {
        let object = scene.object(*index)?;
        let rows = object.transform().f64_rows();
        let mut minimum = [f32::INFINITY; 3];
        let mut maximum = [f32::NEG_INFINITY; 3];
        if object.mesh().vertices.is_empty() {
            return Err(TransformError::Core(CoreError::InvalidMesh(
                "object has no finite geometry bounds".into(),
            )));
        }
        for point in &object.mesh().vertices {
            for axis in 0..3 {
                let value = rows[axis][3]
                    + (0..3)
                        .map(|i| rows[axis][i] * f64::from(point[i]))
                        .sum::<f64>();
                if !value.is_finite() || value.abs() > f64::from(f32::MAX) {
                    return Err(TransformError::Core(CoreError::InvalidTransform(
                        "world bounds exceed finite millimetres".into(),
                    )));
                }
                minimum[axis] = minimum[axis].min(value as f32);
                maximum[axis] = maximum[axis].max(value as f32);
            }
        }
        let current = WorldBounds { minimum, maximum };
        if let Some(total) = &mut bounds {
            total.include(current);
        } else {
            bounds = Some(current);
        }
    }
    let bounds = bounds.ok_or(TransformError::SelectionRequired("Transform"))?;
    if bounds.extents().iter().any(|v| !v.is_finite()) {
        return Err(TransformError::Core(CoreError::InvalidTransform(
            "world extent exceeds finite millimetres".into(),
        )));
    }
    Ok(bounds)
}

/// Applies one atomic world-space transform to a selected object set.
pub fn apply_group_transform(
    scene: &mut Scene,
    indices: &[usize],
    operation: GroupTransform,
    pivot: Option<[f32; 3]>,
) -> Result<bool, TransformError> {
    let indices = ordered_indices(scene, indices.iter().copied(), "Transform")?;
    let pivot = pivot.unwrap_or(world_bounds(scene, &indices)?.center());
    validate_vector(pivot, "pivot_mm")?;
    let group = match operation {
        GroupTransform::Translation(offset) => {
            validate_vector(offset, "translation_mm")?;
            Transform::identity().translated(offset)?
        }
        GroupTransform::Rotation(angles) => {
            validate_vector(angles, "rotation_delta_deg")?;
            let rotation = euler_rotation(angles)?;
            around_pivot(rotation, pivot)?
        }
        GroupTransform::Scale(factors) => {
            validate_positive_vector(factors, "scale_factor")?;
            let scale = Transform::identity().scaled(factors)?;
            around_pivot(scale, pivot)?
        }
        GroupTransform::Reflection(factors) => {
            validate_nonzero_vector(factors, "reflection")?;
            let reflection = Transform::identity().scaled(factors)?;
            around_pivot(reflection, pivot)?
        }
    };
    let transforms = indices
        .iter()
        .map(|index| {
            let object = scene.object(*index)?;
            group
                .multiply(object.transform())
                .map_err(TransformError::from)
        })
        .collect::<Result<Vec<_>, _>>()?;
    commit_transforms(scene, &indices, transforms)
}

/// Applies one validated world matrix to selected objects as one history entry.
pub fn apply_world_matrix(
    scene: &mut Scene,
    indices: &[usize],
    matrix: Transform,
) -> Result<bool, TransformError> {
    let indices = ordered_indices(scene, indices.iter().copied(), "Transform")?;
    let transforms = indices
        .iter()
        .map(|index| matrix.multiply(scene.object(*index)?.transform()))
        .collect::<Result<Vec<_>, CoreError>>()?;
    commit_transforms(scene, &indices, transforms)
}

/// Sets an object's world-space position while preserving its linear part.
pub fn set_position(
    scene: &mut Scene,
    index: usize,
    position_mm: [f32; 3],
) -> Result<bool, TransformError> {
    validate_vector(position_mm, "position_mm")?;
    let object = scene.object(index)?;
    let mut rows = object.transform().rows();
    rows[0][3] = position_mm[0];
    rows[1][3] = position_mm[1];
    rows[2][3] = position_mm[2];
    let transform = Transform::from_rows(rows)?;
    commit_transforms(scene, &[index], vec![transform])
}

/// Resizes an object to the requested world-space extent.
pub fn set_size(
    scene: &mut Scene,
    index: usize,
    size_mm: [f32; 3],
) -> Result<bool, TransformError> {
    validate_positive_vector(size_mm, "size_mm")?;
    let object = scene.object(index)?;
    let current = world_bounds(scene, &[index])?.extents();
    if current.iter().any(|value| *value <= f32::EPSILON) {
        return Err(TransformError::InvalidValue {
            name: "size_mm",
            reason: "object has a zero world dimension",
        });
    }
    let scale = [
        size_mm[0] / current[0],
        size_mm[1] / current[1],
        size_mm[2] / current[2],
    ];
    let mut rows = Transform::identity()
        .scaled(scale)?
        .multiply(object.transform())?
        .rows();
    for (row, position) in rows.iter_mut().zip(object.position()) {
        row[3] = position;
    }
    let transform = Transform::from_rows(rows)?;
    commit_transforms(scene, &[index], vec![transform])
}

/// Sets one object's world-space position and extent as one history entry.
pub fn set_position_and_size(
    scene: &mut Scene,
    index: usize,
    position_mm: [f32; 3],
    size_mm: [f32; 3],
) -> Result<bool, TransformError> {
    validate_vector(position_mm, "position_mm")?;
    validate_positive_vector(size_mm, "size_mm")?;
    let object = scene.object(index)?;
    let current = world_bounds(scene, &[index])?.extents();
    if current.iter().any(|value| *value <= f32::EPSILON) {
        return Err(TransformError::InvalidValue {
            name: "size_mm",
            reason: "object has a zero world dimension",
        });
    }
    let scale = [
        size_mm[0] / current[0],
        size_mm[1] / current[1],
        size_mm[2] / current[2],
    ];
    let mut rows = Transform::identity()
        .scaled(scale)?
        .multiply(object.transform())?
        .rows();
    rows[0][3] = position_mm[0];
    rows[1][3] = position_mm[1];
    rows[2][3] = position_mm[2];
    let transform = Transform::from_rows(rows)?;
    commit_transforms(scene, &[index], vec![transform])
}

/// Aligns objects to the first object's world-space landmark.
pub fn align(
    scene: &mut Scene,
    indices: &[usize],
    axis: Axis,
    mode: Alignment,
) -> Result<bool, TransformError> {
    let indices = ordered_indices(scene, indices.iter().copied(), "Align")?;
    if indices.len() < 2 {
        return Err(TransformError::PairRequired("Align"));
    }
    let anchor = world_bounds(scene, &[indices[0]])?;
    let target = landmark(anchor, axis, mode);
    let mut transforms = Vec::with_capacity(indices.len());
    for index in &indices {
        let object = scene.object(*index)?;
        let bounds = world_bounds(scene, &[*index])?;
        let current = landmark(bounds, axis, mode);
        let mut offset = [0.0; 3];
        offset[axis.index()] = target - current;
        let translation = Transform::identity().translated(offset)?;
        transforms.push(translation.multiply(object.transform())?);
    }
    commit_transforms(scene, &indices, transforms)
}

/// Arranges objects in input order along one world axis with a fixed gap.
pub fn arrange(
    scene: &mut Scene,
    indices: &[usize],
    axis: Axis,
    gap_mm: f32,
) -> Result<bool, TransformError> {
    let indices = ordered_indices(scene, indices.iter().copied(), "Arrange")?;
    if indices.len() < 2 {
        return Err(TransformError::PairRequired("Arrange"));
    }
    if !gap_mm.is_finite() || gap_mm < 0.0 {
        return Err(TransformError::InvalidValue {
            name: "gap_mm",
            reason: "must be finite and non-negative",
        });
    }
    let axis_index = axis.index();
    let mut transforms = indices
        .iter()
        .map(|index| scene.object(*index).map(|object| *object.transform()))
        .collect::<Result<Vec<_>, _>>()?;
    let first = world_bounds(scene, &[indices[0]])?;
    let mut cursor = first.maximum[axis_index];
    for (position, index) in indices.iter().enumerate().skip(1) {
        let bounds = world_bounds(scene, &[*index])?;
        let delta = cursor + gap_mm - bounds.minimum[axis_index];
        let offset = axis_vector(axis, delta);
        transforms[position] = Transform::identity()
            .translated(offset)?
            .multiply(&transforms[position])?;
        cursor = bounds.maximum[axis_index] + delta;
    }
    commit_transforms(scene, &indices, transforms)
}

/// Rotates the combined convex hull's best downward face to the bed, then
/// centers X/Z and places the lowest point at Y=0 in one history entry.
pub fn settle(scene: &mut Scene, indices: &[usize]) -> Result<bool, TransformError> {
    let indices = ordered_indices(scene, indices.iter().copied(), "Settle")?;
    let count = indices.iter().try_fold(0usize, |count, index| {
        let next = count.checked_add(scene.object(*index)?.mesh().vertices.len());
        next.filter(|count| *count <= builder3d_core::MAX_PLACEMENT_POINTS)
            .ok_or_else(|| CoreError::OperationResourceLimit {
                operation: "settle".into(),
                reason: "placement point/memory budget exceeded".into(),
            })
    })?;
    let mut points = Vec::with_capacity(count);
    for index in &indices {
        let object = scene.object(*index)?;
        let rows = object.transform().f64_rows();
        for point in &object.mesh().vertices {
            points.push(std::array::from_fn::<_, 3, _>(|axis| {
                rows[axis][3]
                    + (0..3)
                        .map(|i| rows[axis][i] * f64::from(point[i]))
                        .sum::<f64>()
            }));
        }
    }
    let rotation = builder3d_core::placement_rotation(&points)?;
    if indices.len() == 1 {
        // Python lay_flat calls SceneObject.rotate (post-multiplied local
        // rotation); its multi-object service composes in world space below.
        let index = indices[0];
        let object = scene.object(index)?;
        let mut rotation_rows = Transform::identity().f64_rows();
        for axis in 0..3 {
            rotation_rows[axis][..3].copy_from_slice(&rotation[axis]);
        }
        let mut rows = object
            .transform()
            .multiply(&Transform::from_f64_rows(rotation_rows)?)?
            .f64_rows();
        let mut minimum = [f64::INFINITY; 3];
        let mut maximum = [f64::NEG_INFINITY; 3];
        for point in &object.mesh().vertices {
            for axis in 0..3 {
                let value = rows[axis][3]
                    + (0..3)
                        .map(|i| rows[axis][i] * f64::from(point[i]))
                        .sum::<f64>();
                minimum[axis] = minimum[axis].min(value);
                maximum[axis] = maximum[axis].max(value);
            }
        }
        rows[0][3] -= (minimum[0] + maximum[0]) * 0.5;
        rows[1][3] -= minimum[1];
        rows[2][3] -= (minimum[2] + maximum[2]) * 0.5;
        return commit_transforms(scene, &indices, vec![Transform::from_f64_rows(rows)?]);
    }
    let mut minimum = [f64::INFINITY; 3];
    let mut maximum = [f64::NEG_INFINITY; 3];
    for point in points {
        for axis in 0..3 {
            let value = (0..3).map(|i| rotation[axis][i] * point[i]).sum::<f64>();
            minimum[axis] = minimum[axis].min(value);
            maximum[axis] = maximum[axis].max(value);
        }
    }
    let offset = [
        -(minimum[0] + maximum[0]) * 0.5,
        -minimum[1],
        -(minimum[2] + maximum[2]) * 0.5,
    ];
    let mut transforms = Vec::with_capacity(indices.len());
    for index in &indices {
        let before = scene.object(*index)?.transform().f64_rows();
        let mut rows = before;
        for axis in 0..3 {
            for column in 0..4 {
                rows[axis][column] = (0..3)
                    .map(|i| rotation[axis][i] * before[i][column])
                    .sum::<f64>();
            }
            rows[axis][3] += offset[axis];
        }
        transforms.push(Transform::from_f64_rows(rows)?);
    }
    commit_transforms(scene, &indices, transforms)
}

/// Mirrors the reference's local basis column and world translation component.
pub fn mirror(scene: &mut Scene, indices: &[usize], axis: Axis) -> Result<bool, TransformError> {
    let indices = ordered_indices(scene, indices.iter().copied(), "Mirror")?;
    let mut transforms = Vec::with_capacity(indices.len());
    for index in &indices {
        let mut rows = scene.object(*index)?.transform().rows();
        for row in rows.iter_mut().take(3) {
            row[axis.index()] *= -1.0;
        }
        rows[axis.index()][3] *= -1.0;
        transforms.push(Transform::from_rows(rows)?);
    }
    commit_transforms(scene, &indices, transforms)
}

fn commit_transforms(
    scene: &mut Scene,
    indices: &[usize],
    transforms: Vec<Transform>,
) -> Result<bool, TransformError> {
    let mut commands: Vec<Box<dyn CoreCommand>> = Vec::new();
    for (index, transform) in indices.iter().copied().zip(transforms) {
        let current = scene.object(index)?.transform().rows();
        if !current
            .iter()
            .flatten()
            .zip(transform.rows().iter().flatten())
            .all(|(before, after)| (*before - *after).abs() <= 1e-8 + 1e-5 * after.abs())
        {
            commands.push(Box::new(TransformObjectCommand::new(index, transform)));
        }
    }
    if commands.is_empty() {
        return Ok(false);
    }
    Ok(scene.execute(GroupedCommand::new(commands))?.changed)
}

fn around_pivot(operation: Transform, pivot: [f32; 3]) -> Result<Transform, CoreError> {
    Transform::identity()
        .translated(pivot)?
        .multiply(&operation)?
        .multiply(&Transform::identity().translated([-pivot[0], -pivot[1], -pivot[2]])?)
}

fn euler_rotation(angles: [f32; 3]) -> Result<Transform, CoreError> {
    Transform::identity()
        .rotated(angles[2], [0.0, 0.0, 1.0])?
        .rotated(angles[1], [0.0, 1.0, 0.0])?
        .rotated(angles[0], [1.0, 0.0, 0.0])
}

fn landmark(bounds: WorldBounds, axis: Axis, mode: Alignment) -> f32 {
    let index = axis.index();
    match mode {
        Alignment::Minimum => bounds.minimum[index],
        Alignment::Center => (bounds.minimum[index] + bounds.maximum[index]) * 0.5,
        Alignment::Maximum => bounds.maximum[index],
    }
}

fn axis_vector(axis: Axis, value: f32) -> [f32; 3] {
    let mut vector = [0.0; 3];
    vector[axis.index()] = value;
    vector
}

fn validate_vector(value: [f32; 3], name: &'static str) -> Result<(), TransformError> {
    if value.iter().all(|component| component.is_finite()) {
        Ok(())
    } else {
        Err(TransformError::InvalidValue {
            name,
            reason: "must contain finite numbers",
        })
    }
}

fn validate_positive_vector(value: [f32; 3], name: &'static str) -> Result<(), TransformError> {
    validate_vector(value, name)?;
    if value.iter().all(|component| *component > 0.0) {
        Ok(())
    } else {
        Err(TransformError::InvalidValue {
            name,
            reason: "values must be greater than zero",
        })
    }
}

fn validate_nonzero_vector(value: [f32; 3], name: &'static str) -> Result<(), TransformError> {
    validate_vector(value, name)?;
    if value.iter().all(|component| component.abs() > f32::EPSILON) {
        Ok(())
    } else {
        Err(TransformError::InvalidValue {
            name,
            reason: "values must be non-zero",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use builder3d_core::PrimitiveKind;

    fn scene_with_two_objects() -> Scene {
        let mut scene = Scene::new();
        assert!(scene.add_primitive(PrimitiveKind::Cube, None, None).is_ok());
        assert!(scene.add_primitive(PrimitiveKind::Cube, None, None).is_ok());
        scene
    }

    #[test]
    fn group_translation_is_atomic_and_undoable() {
        let mut scene = scene_with_two_objects();
        let revision = scene.revision();
        let changed = apply_group_transform(
            &mut scene,
            &[0, 1],
            GroupTransform::Translation([10.0, 2.0, -4.0]),
            None,
        );
        assert!(matches!(changed, Ok(true)));
        assert_eq!(scene.revision(), revision + 1);
        assert!(
            scene
                .object(0)
                .is_ok_and(|object| object.position() == [10.0, 2.0, -4.0])
        );
        assert!(
            scene
                .object(1)
                .is_ok_and(|object| object.position() == [10.0, 2.0, -4.0])
        );
        assert!(
            scene
                .undo()
                .is_ok_and(|result| result.is_some_and(|item| item.changed))
        );
        assert!(
            scene
                .object(0)
                .is_ok_and(|object| object.position() == [0.0, 0.0, 0.0])
        );
    }

    #[test]
    fn align_and_arrange_use_world_bounds() {
        let mut scene = scene_with_two_objects();
        assert!(scene.translate(1, [30.0, 0.0, 0.0]).is_ok());
        assert!(align(&mut scene, &[0, 1], Axis::Y, Alignment::Minimum).is_ok());
        let first = world_bounds(&scene, &[0]);
        let second = world_bounds(&scene, &[1]);
        assert!(
            matches!((first, second), (Ok(left), Ok(right)) if (left.minimum[1] - right.minimum[1]).abs() < 1e-5)
        );
        assert!(arrange(&mut scene, &[0, 1], Axis::X, 5.0).is_ok());
        let first = world_bounds(&scene, &[0]);
        let second = world_bounds(&scene, &[1]);
        assert!(
            matches!((first, second), (Ok(left), Ok(right)) if (right.minimum[0] - left.maximum[0] - 5.0).abs() < 1e-5)
        );
    }

    #[test]
    fn settle_uses_y_as_the_build_axis_and_mirror_is_one_history_entry() {
        let mut scene = scene_with_two_objects();
        assert!(scene.translate(0, [4.0, 10.0, -6.0]).is_ok());
        assert!(scene.translate(1, [-4.0, 12.0, 8.0]).is_ok());
        let before = scene.revision();
        assert!(settle(&mut scene, &[0, 1]).is_ok());
        let bounds = world_bounds(&scene, &[0, 1]);
        assert!(matches!(bounds, Ok(value) if value.minimum[1].abs() < 1e-5));
        assert!(mirror(&mut scene, &[0, 1], Axis::X).is_ok());
        assert_eq!(scene.revision(), before + 2);
    }

    #[test]
    fn numeric_position_and_size_updates_are_undoable() {
        let mut scene = Scene::new();
        assert!(scene.add_primitive(PrimitiveKind::Cube, None, None).is_ok());
        assert!(set_position(&mut scene, 0, [4.0, 5.0, 6.0]).is_ok_and(|changed| changed));
        assert!(set_size(&mut scene, 0, [20.0, 30.0, 40.0]).is_ok_and(|changed| changed));
        assert!(
            world_bounds(&scene, &[0]).is_ok_and(|bounds| bounds.extents() == [20.0, 30.0, 40.0])
        );
    }

    #[test]
    fn combined_position_and_size_is_one_history_entry() {
        let mut scene = Scene::new();
        assert!(scene.add_primitive(PrimitiveKind::Cube, None, None).is_ok());
        let revision = scene.revision();
        assert!(
            set_position_and_size(&mut scene, 0, [4.0, 5.0, 6.0], [20.0, 30.0, 40.0])
                .is_ok_and(|changed| changed)
        );
        assert_eq!(scene.revision(), revision + 1);
        assert!(
            scene
                .object(0)
                .is_ok_and(|object| object.position() == [4.0, 5.0, 6.0])
        );
        assert!(
            world_bounds(&scene, &[0]).is_ok_and(|bounds| bounds.extents() == [20.0, 30.0, 40.0])
        );
    }
}

#[cfg(test)]
mod rb04_placement_oracle {
    use super::*;
    #[derive(serde::Deserialize)]
    struct Case {
        before: Vec<[[f64; 4]; 4]>,
        after: Vec<[[f64; 4]; 4]>,
    }
    #[test]
    fn settle_single_and_group_match_python_convex_hull_cases() {
        let cases: Vec<Case> = serde_json::from_str(include_str!(
            "../../../evidence/rebuild/rb-04-scene-and-precise-transforms/settle-oracle.json"
        ))
        .expect("oracle");
        for case in cases {
            let mut scene = Scene::new();
            for rows in &case.before {
                let index = scene.objects().len();
                scene
                    .add_object(builder3d_core::SceneObject::new(
                        builder3d_core::make_box([20., 30., 40.]).expect("box"),
                    ))
                    .expect("add");
                scene
                    .set_transform(index, Transform::from_f64_rows(*rows).expect("matrix"))
                    .expect("transform");
            }
            let before = scene.history_state_id();
            let indices = (0..scene.objects().len()).collect::<Vec<_>>();
            assert!(settle(&mut scene, &indices).expect("settle"));
            for (object, expected) in scene.objects().iter().zip(&case.after) {
                for (a, b) in object
                    .transform()
                    .f64_rows()
                    .iter()
                    .flatten()
                    .zip(expected.iter().flatten())
                {
                    assert!(
                        (a - b).abs() < 0.002,
                        "actual {:?}; expected {expected:?}",
                        object.transform()
                    );
                }
            }
            scene.undo().expect("undo");
            assert_eq!(scene.history_state_id(), before);
        }
    }
}
