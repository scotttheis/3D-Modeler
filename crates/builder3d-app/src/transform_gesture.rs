//! Bounded, renderer-neutral drag mathematics. Pointer motion never edits a scene.
use std::collections::{BTreeSet, HashSet};

use builder3d_core::{Scene, Transform};
use builder3d_viewport::{
    Ray, SNAP_TOLERANCE_VIEW_SCALED, SnapQuery, SnapSceneSnapshot, SnapState,
    SnapTranslationResult, SnapViewContext, add, cross, dot, normalize, scale,
    snap_translation_result, subtract,
};

use crate::transforms::{TransformError, world_bounds};

#[derive(Clone, Debug)]
pub(crate) struct TransformSelectionSnapshot {
    pub(crate) revision: u64,
    pub(crate) document_generation: u64,
    pub(crate) selection: BTreeSet<usize>,
    pub(crate) objects: Vec<(String, Transform)>,
}

impl TransformSelectionSnapshot {
    pub(crate) fn ids(&self) -> HashSet<String> {
        self.objects.iter().map(|(id, _)| id.clone()).collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ResizeFrame {
    pub(crate) center: [f32; 3],
    /// Orthonormal basis columns, in world coordinates.
    pub(crate) axes: [[f32; 3]; 3],
    pub(crate) extents: [f32; 3],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ResizeHandle {
    Face { axis: usize, side: f32 },
    Edge { axis: usize, sides: [f32; 3] },
    Corner([f32; 3]),
}

impl ResizeFrame {
    pub(crate) fn from_scene(
        scene: &Scene,
        indices: &[usize],
        object_basis: bool,
    ) -> Result<Self, TransformError> {
        let bounds = world_bounds(scene, indices)?;
        let mut frame = Self {
            center: bounds.center(),
            extents: bounds.extents(),
            axes: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        };
        if object_basis && indices.len() == 1 {
            let object = scene.object(indices[0])?;
            let basis = crate::rotation::basis(object.transform())?;
            frame.axes = std::array::from_fn(|i| std::array::from_fn(|j| basis[j][i]));
            let origin = object.position();
            let rows = object.transform().f64_rows();
            let mut minimum = [f64::INFINITY; 3];
            let mut maximum = [f64::NEG_INFINITY; 3];
            for vertex in &object.mesh().vertices {
                let offset: [f64; 3] = std::array::from_fn(|i| {
                    (0..3).map(|j| rows[i][j] * f64::from(vertex[j])).sum()
                });
                for i in 0..3 {
                    let value: f64 = (0..3)
                        .map(|j| offset[j] * f64::from(frame.axes[i][j]))
                        .sum();
                    minimum[i] = minimum[i].min(value);
                    maximum[i] = maximum[i].max(value);
                }
            }
            frame.extents = std::array::from_fn(|i| (maximum[i] - minimum[i]) as f32);
            frame.center = origin;
            for i in 0..3 {
                frame.center = add(
                    frame.center,
                    scale(frame.axes[i], ((minimum[i] + maximum[i]) * 0.5) as f32),
                );
            }
        }
        if frame
            .center
            .iter()
            .chain(frame.extents.iter())
            .any(|v| !v.is_finite())
        {
            return Err(invalid("resize bounds exceed finite millimetres"));
        }
        Ok(frame)
    }

    pub(crate) fn handles(self) -> Vec<(ResizeHandle, [f32; 3])> {
        let mut handles = Vec::with_capacity(26);
        for axis in 0..3 {
            for side in [-1., 1.] {
                handles.push((
                    ResizeHandle::Face { axis, side },
                    add(
                        self.center,
                        scale(self.axes[axis], side * self.extents[axis] * 0.5),
                    ),
                ));
            }
        }
        for x in [-1., 1.] {
            for y in [-1., 1.] {
                for z in [-1., 1.] {
                    let sides = [x, y, z];
                    handles.push((ResizeHandle::Corner(sides), self.point(sides)));
                }
            }
        }
        for axis in 0..3 {
            for a in [-1., 1.] {
                for b in [-1., 1.] {
                    let mut sides = [0.; 3];
                    sides[(axis + 1) % 3] = a;
                    sides[(axis + 2) % 3] = b;
                    handles.push((ResizeHandle::Edge { axis, sides }, self.point(sides)));
                }
            }
        }
        handles
    }

    /// The reference's visible picker exposes faces and corners. Its edge
    /// calculations are internal and are not an additional desktop affordance.
    pub(crate) fn interactive_handles(self) -> impl Iterator<Item = (ResizeHandle, [f32; 3])> {
        self.handles()
            .into_iter()
            .filter(|(handle, _)| !matches!(handle, ResizeHandle::Edge { .. }))
    }

    fn point(self, sides: [f32; 3]) -> [f32; 3] {
        let mut point = self.center;
        for (i, side) in sides.into_iter().enumerate() {
            point = add(point, scale(self.axes[i], side * self.extents[i] * 0.5));
        }
        point
    }
}

#[derive(Clone, Debug)]
enum GestureKind {
    Move {
        axis: [f32; 3],
        start: f32,
    },
    Rotate {
        axis: [f32; 3],
        previous: [f32; 3],
        accumulated: f32,
    },
    Resize {
        frame: ResizeFrame,
        handle: ResizeHandle,
        start_parameter: Option<f32>,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct TransformGesture {
    pub(crate) snapshot: TransformSelectionSnapshot,
    pub(crate) center: [f32; 3],
    pub(crate) delta: Transform,
    /// Immutable landmark data captured when a Move drag begins.
    pub(crate) snap_snapshot: Option<SnapSceneSnapshot>,
    /// Stateful capture/release memory for the current Move drag.
    pub(crate) snap_state: SnapState,
    /// Camera-derived tolerance context captured at drag start.
    pub(crate) snap_view_context: Option<SnapViewContext>,
    /// Most recent structured snap feedback for the native status/overlay.
    pub(crate) last_snap: Option<SnapTranslationResult>,
    snap_enabled: bool,
    last_move_delta: f32,
    previous_move_delta: f32,
    kind: GestureKind,
}

impl TransformGesture {
    pub(crate) fn move_axis(
        snapshot: TransformSelectionSnapshot,
        center: [f32; 3],
        axis: [f32; 3],
        ray: Ray,
    ) -> Option<Self> {
        let start = axis_parameter(ray, center, axis)?;
        Some(Self {
            snapshot,
            center,
            delta: Transform::identity(),
            snap_snapshot: None,
            snap_state: SnapState::new(),
            snap_view_context: None,
            last_snap: None,
            snap_enabled: false,
            last_move_delta: 0.0,
            previous_move_delta: 0.0,
            kind: GestureKind::Move { axis, start },
        })
    }
    pub(crate) fn rotate(
        snapshot: TransformSelectionSnapshot,
        center: [f32; 3],
        axis: [f32; 3],
        ray: Ray,
    ) -> Option<Self> {
        let previous = rotation_vector(ray, center, axis)?;
        Some(Self {
            snapshot,
            center,
            delta: Transform::identity(),
            snap_snapshot: None,
            snap_state: SnapState::new(),
            snap_view_context: None,
            last_snap: None,
            snap_enabled: false,
            last_move_delta: 0.0,
            previous_move_delta: 0.0,
            kind: GestureKind::Rotate {
                axis,
                previous,
                accumulated: 0.,
            },
        })
    }
    pub(crate) fn resize(
        snapshot: TransformSelectionSnapshot,
        frame: ResizeFrame,
        handle: ResizeHandle,
        start_ray: Ray,
    ) -> Option<Self> {
        let start_parameter = match handle {
            ResizeHandle::Face { axis, .. } => {
                Some(axis_parameter(start_ray, frame.center, frame.axes[axis])?)
            }
            ResizeHandle::Corner(sides) => {
                let vector = subtract(frame.point(sides), frame.center);
                let length = dot(vector, vector).sqrt();
                if length <= 1e-8 {
                    return None;
                }
                Some(axis_parameter(
                    start_ray,
                    frame.center,
                    scale(vector, 1.0 / length),
                )?)
            }
            ResizeHandle::Edge { .. } => None,
        };
        Some(Self {
            snapshot,
            center: frame.center,
            delta: Transform::identity(),
            snap_snapshot: None,
            snap_state: SnapState::new(),
            snap_view_context: None,
            last_snap: None,
            snap_enabled: false,
            last_move_delta: 0.0,
            previous_move_delta: 0.0,
            kind: GestureKind::Resize {
                frame,
                handle,
                start_parameter,
            },
        })
    }

    /// Enables landmark/floor snapping for a Move gesture using immutable
    /// drag-start geometry and the camera context from that same moment.
    pub(crate) fn configure_move_snapping(
        &mut self,
        snapshot: SnapSceneSnapshot,
        view_context: Option<SnapViewContext>,
        enabled: bool,
    ) {
        self.snap_snapshot = Some(snapshot);
        self.snap_view_context = view_context;
        self.snap_enabled = enabled;
        self.snap_state.clear();
        self.last_snap = None;
        self.last_move_delta = 0.0;
        self.previous_move_delta = 0.0;
    }

    pub(crate) fn active_axis_index(&self) -> Option<usize> {
        match &self.kind {
            GestureKind::Move { axis, .. } | GestureKind::Rotate { axis, .. } => {
                let (index, magnitude) = axis
                    .iter()
                    .copied()
                    .enumerate()
                    .max_by(|(_, first), (_, second)| first.abs().total_cmp(&second.abs()))?;
                (magnitude.abs() > 0.999).then_some(index)
            }
            GestureKind::Resize {
                handle: ResizeHandle::Face { axis, .. } | ResizeHandle::Edge { axis, .. },
                ..
            } => Some(*axis),
            GestureKind::Resize {
                handle: ResizeHandle::Corner(_),
                ..
            } => None,
        }
    }

    pub(crate) fn active_resize_handle(&self) -> Option<ResizeHandle> {
        match &self.kind {
            GestureKind::Resize { handle, .. } => Some(*handle),
            GestureKind::Move { .. } | GestureKind::Rotate { .. } => None,
        }
    }

    pub(crate) fn update(&mut self, ray: Ray) -> Result<(), TransformError> {
        let matrix = match &mut self.kind {
            GestureKind::Move { axis, start } => {
                let Some(parameter) = axis_parameter(ray, self.center, *axis) else {
                    return Ok(());
                };
                let delta = parameter - *start;
                self.last_move_delta = delta;
                Transform::identity().translated(scale(*axis, delta))?
            }
            GestureKind::Rotate {
                axis,
                previous,
                accumulated,
            } => {
                let Some(current) = rotation_vector(ray, self.center, *axis) else {
                    return Ok(());
                };
                *accumulated +=
                    dot(*axis, cross(*previous, current)).atan2(dot(*previous, current));
                *previous = current;
                let rotation = Transform::identity().rotated(accumulated.to_degrees(), *axis)?;
                around_center(rotation, self.center, [0.; 3])?
            }
            GestureKind::Resize {
                frame,
                handle,
                start_parameter,
            } => {
                let (axis, side) = match *handle {
                    ResizeHandle::Face { axis, side } => (Some(axis), Some(side)),
                    ResizeHandle::Edge { axis, .. } => (Some(axis), None),
                    ResizeHandle::Corner(_) => (None, None),
                };
                let mut factors = [1.; 3];
                let mut offset = [0.; 3];
                if let Some(axis) = axis {
                    let extent = frame.extents[axis];
                    if extent <= 1e-8 {
                        return Err(invalid("cannot resize a zero dimension"));
                    }
                    let Some(parameter) = axis_parameter(ray, frame.center, frame.axes[axis])
                    else {
                        return Ok(());
                    };
                    let new_extent = side
                        .map_or(parameter.abs(), |s| {
                            start_parameter.map_or_else(
                                || s * (parameter + s * extent * 0.5),
                                |start| extent + s * (parameter - start),
                            )
                        })
                        .max(0.01);
                    factors[axis] = new_extent / extent;
                    if let Some(side) = side {
                        offset = scale(frame.axes[axis], side * (new_extent - extent) * 0.5);
                    }
                } else if let ResizeHandle::Corner(sides) = *handle {
                    let vector = subtract(frame.point(sides), frame.center);
                    let length = dot(vector, vector).sqrt();
                    if length <= 1e-8 {
                        return Err(invalid("cannot resize a zero dimension"));
                    }
                    let Some(parameter) =
                        axis_parameter(ray, frame.center, scale(vector, 1. / length))
                    else {
                        return Ok(());
                    };
                    let new_extent =
                        length + start_parameter.map_or(0.0, |start| parameter - start);
                    factors = [new_extent.max(0.01) / length; 3];
                }
                let mut rows = Transform::identity().rows();
                for (i, row) in rows.iter_mut().take(3).enumerate() {
                    for (j, value) in row.iter_mut().take(3).enumerate() {
                        *value = (0..3)
                            .map(|k| frame.axes[k][i] * factors[k] * frame.axes[k][j])
                            .sum();
                    }
                }
                around_center(Transform::from_rows(rows)?, frame.center, offset)?
            }
        };
        self.delta = matrix;
        Ok(())
    }

    /// Updates a Move gesture and applies landmark/floor correction without
    /// feeding the corrected position back as the next pointer proposal.
    pub(crate) fn update_with_snap(
        &mut self,
        ray: Ray,
        bypass_snap: bool,
    ) -> Result<Option<SnapTranslationResult>, TransformError> {
        self.update(ray)?;
        let Some(snapshot) = self.snap_snapshot.as_ref() else {
            return Ok(None);
        };
        let axis = match &self.kind {
            GestureKind::Move { axis, .. } => *axis,
            GestureKind::Rotate { .. } | GestureKind::Resize { .. } => return Ok(None),
        };
        let proposed_delta = self.last_move_delta;
        if bypass_snap || !self.snap_enabled {
            self.snap_state.clear();
            self.last_snap = None;
            self.previous_move_delta = proposed_delta;
            return Ok(None);
        }
        let result = snap_translation_result(
            SnapQuery {
                snapshot,
                axis,
                proposed_delta,
                previous_delta: self.previous_move_delta,
                tolerance_mm: None,
                tolerance_mode: SNAP_TOLERANCE_VIEW_SCALED,
                view_context: self.snap_view_context,
                floor_y: Some(0.0),
                excluded: None,
            },
            &mut self.snap_state,
        )?;
        self.previous_move_delta = proposed_delta;
        if (result.applied_delta - proposed_delta).abs() > f32::EPSILON {
            self.delta = Transform::identity().translated(scale(axis, result.applied_delta))?;
        }
        self.last_snap = Some(result.clone());
        Ok(Some(result))
    }
}

fn invalid(reason: &'static str) -> TransformError {
    TransformError::InvalidValue {
        name: "viewport transform",
        reason,
    }
}

fn around_center(
    linear: Transform,
    center: [f32; 3],
    offset: [f32; 3],
) -> Result<Transform, TransformError> {
    Ok(Transform::identity()
        .translated(add(center, offset))?
        .multiply(&linear)?
        .multiply(&Transform::identity().translated(scale(center, -1.))?)?)
}

fn axis_parameter(ray: Ray, center: [f32; 3], axis: [f32; 3]) -> Option<f32> {
    let projection = dot(axis, ray.direction);
    let denominator = 1. - projection * projection;
    if denominator.abs() <= 1e-5 {
        return None;
    }
    let offset = subtract(ray.origin, center);
    let parameter = (dot(axis, offset) - projection * dot(ray.direction, offset)) / denominator;
    parameter.is_finite().then_some(parameter)
}

fn rotation_vector(ray: Ray, center: [f32; 3], axis: [f32; 3]) -> Option<[f32; 3]> {
    let denominator = dot(axis, ray.direction);
    if denominator.abs() <= 1e-5 {
        return None;
    }
    let distance = dot(axis, subtract(center, ray.origin)) / denominator;
    let vector = subtract(add(ray.origin, scale(ray.direction, distance)), center);
    if dot(vector, vector) <= 1e-12 {
        return None;
    }
    normalize(vector, "rotation drag").ok()
}
