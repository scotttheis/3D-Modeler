//! Renderer-neutral landmark snapping for interactive translation.
//!
//! Geometry is captured once at drag start. Pointer updates then operate on
//! scalar bounds and never rescan authoritative mesh vertices. The desktop
//! and MCP adapters can therefore share the same deterministic correction
//! policy without importing egui, wgpu, or a windowing API.

use thiserror::Error;

/// Default ordinary object-snap tolerance in millimetres.
pub const OBJECT_SNAP_TOLERANCE_MM: f32 = 4.0;
/// Fixed build-bed snap tolerance in millimetres.
pub const FLOOR_SNAP_TOLERANCE_MM: f32 = OBJECT_SNAP_TOLERANCE_MM;
/// View-scaled normalized projected span used by the reference policy.
pub const SNAP_NORMALIZED_PROJECTED_SPAN: f32 = 0.02;
/// Fixed world-unit tolerance mode.
pub const SNAP_TOLERANCE_FIXED_MM: &str = "fixed_mm";
/// Camera-scaled tolerance mode.
pub const SNAP_TOLERANCE_VIEW_SCALED: &str = "view_scaled";
/// Fixed build-floor tolerance mode.
pub const SNAP_TOLERANCE_FLOOR_FIXED: &str = "floor_fixed";

/// A finite world-space bounds pair.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapBounds {
    /// Minimum XYZ point.
    pub minimum: [f32; 3],
    /// Maximum XYZ point.
    pub maximum: [f32; 3],
}

impl SnapBounds {
    /// Validates a bounds pair before it enters a drag snapshot.
    pub fn new(minimum: [f32; 3], maximum: [f32; 3]) -> Result<Self, SnapError> {
        if minimum
            .iter()
            .chain(maximum.iter())
            .any(|value| !value.is_finite())
            || minimum
                .iter()
                .zip(maximum)
                .any(|(lower, upper)| lower > &upper)
        {
            return Err(SnapError::InvalidBounds);
        }
        Ok(Self { minimum, maximum })
    }

    fn landmark(self, axis: usize, name: SnapLandmark) -> f32 {
        match name {
            SnapLandmark::Minimum => self.minimum[axis],
            SnapLandmark::Center => (self.minimum[axis] + self.maximum[axis]) * 0.5,
            SnapLandmark::Maximum => self.maximum[axis],
        }
    }
}

/// One immutable eligible snap target captured at drag start.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapTarget {
    /// Stable scene object identity.
    pub object_id: String,
    /// Original top-level scene order used for deterministic ties.
    pub scene_order: usize,
    /// World-space target bounds.
    pub bounds: SnapBounds,
}

/// View data used to resolve a camera-scaled tolerance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnapViewContext {
    /// World millimetres represented by one normalized vertical unit.
    pub world_units_per_normalized_unit: f32,
    /// Aggregate moving-object midpoint.
    pub aggregate_midpoint: [f32; 3],
    /// Optional normalized span override.
    pub normalized_span: f32,
}

impl SnapViewContext {
    /// Creates a validated view context using the reference normalized span.
    pub fn new(
        world_units_per_normalized_unit: f32,
        aggregate_midpoint: [f32; 3],
    ) -> Result<Self, SnapError> {
        if !world_units_per_normalized_unit.is_finite()
            || world_units_per_normalized_unit <= 0.0
            || aggregate_midpoint.iter().any(|value| !value.is_finite())
        {
            return Err(SnapError::InvalidViewContext);
        }
        Ok(Self {
            world_units_per_normalized_unit,
            aggregate_midpoint,
            normalized_span: SNAP_NORMALIZED_PROJECTED_SPAN,
        })
    }

    /// Returns the resolved world-unit tolerance.
    pub fn tolerance_mm(self) -> Result<f32, SnapError> {
        if !self.normalized_span.is_finite() || self.normalized_span <= 0.0 {
            return Err(SnapError::InvalidViewContext);
        }
        Ok(self.world_units_per_normalized_unit * self.normalized_span)
    }
}

/// Immutable drag-start geometry.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapSceneSnapshot {
    /// Union bounds of the moving selection.
    pub moving_bounds: SnapBounds,
    /// Eligible object targets in scene order.
    pub targets: Vec<SnapTarget>,
    /// Stable identities of moving objects, used to exclude self targets.
    pub moving_object_ids: Vec<String>,
    /// Whether the origin should be an eligible landmark.
    pub include_origin: bool,
}

impl SnapSceneSnapshot {
    /// Builds a snapshot and removes targets that belong to the moving set.
    pub fn new(
        moving_bounds: SnapBounds,
        moving_object_ids: Vec<String>,
        targets: impl IntoIterator<Item = SnapTarget>,
        include_origin: bool,
    ) -> Self {
        let targets = targets
            .into_iter()
            .filter(|target| !moving_object_ids.iter().any(|id| id == &target.object_id))
            .collect();
        Self {
            moving_bounds,
            targets,
            moving_object_ids,
            include_origin,
        }
    }
}

/// Inputs for one stateless or stateful snap query.
pub struct SnapQuery<'a> {
    /// Immutable drag-start geometry.
    pub snapshot: &'a SnapSceneSnapshot,
    /// Requested movement axis.
    pub axis: [f32; 3],
    /// Pointer-proposed scalar movement.
    pub proposed_delta: f32,
    /// Previous pointer-proposed scalar movement.
    pub previous_delta: f32,
    /// Optional explicit tolerance override.
    pub tolerance_mm: Option<f32>,
    /// Tolerance policy name.
    pub tolerance_mode: &'a str,
    /// Optional camera-derived view context.
    pub view_context: Option<SnapViewContext>,
    /// Optional Y coordinate of the build floor.
    pub floor_y: Option<f32>,
    /// Candidate excluded after a release recapture.
    pub excluded: Option<&'a SnapCandidateKey>,
}

/// A landmark on a moving or target object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapLandmark {
    /// Minimum edge.
    Minimum,
    /// Center point.
    Center,
    /// Maximum edge.
    Maximum,
}

/// The semantic kind of the selected snap target.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapTargetKind {
    /// Another scene object.
    Object,
    /// The build floor.
    Floor,
    /// The world origin.
    Origin,
}

/// A stable key used to exclude the just-released candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapCandidateKey {
    /// Target semantic kind.
    pub target_kind: SnapTargetKind,
    /// Target object identity when applicable.
    pub target_object_id: Option<String>,
    /// Moving landmark involved in the correction.
    pub moving_landmark: SnapLandmark,
    /// Target landmark involved in the correction.
    pub target_landmark: Option<SnapLandmark>,
}

/// Structured output from the scalar solver.
#[derive(Clone, Debug, PartialEq)]
pub struct SnapTranslationResult {
    /// Corrected translation along the requested axis.
    pub applied_delta: f32,
    /// Whether a candidate was captured.
    pub snapped: bool,
    /// Resolved tolerance in millimetres.
    pub tolerance_mm: f32,
    /// Policy that supplied the tolerance.
    pub tolerance_source: &'static str,
    /// Semantic target kind, when snapped.
    pub target_kind: Option<SnapTargetKind>,
    /// Target object identity, when snapped to an object.
    pub target_object_id: Option<String>,
    /// Moving landmark, when snapped.
    pub moving_landmark: Option<SnapLandmark>,
    /// Target landmark, when snapped.
    pub target_landmark: Option<SnapLandmark>,
    /// Signed pre-correction error.
    pub error: Option<f32>,
    /// Candidate key for capture/release state.
    pub candidate_key: Option<SnapCandidateKey>,
}

/// Stateful capture/release memory for one pointer gesture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SnapState {
    active_delta: Option<f32>,
    active_key: Option<SnapCandidateKey>,
    release_distance: f32,
    released_delta: Option<f32>,
    released_direction: f32,
    released_key: Option<SnapCandidateKey>,
}

impl SnapState {
    /// Creates an empty gesture state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            active_delta: None,
            active_key: None,
            release_distance: 0.05,
            released_delta: None,
            released_direction: 0.0,
            released_key: None,
        }
    }

    /// Clears capture state at the end of a gesture.
    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

/// Typed snapping failures.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum SnapError {
    /// Bounds contain NaN, infinity, or reversed components.
    #[error("snap bounds must be finite and ordered")]
    InvalidBounds,
    /// View-scaled tolerance input is unusable.
    #[error("snap view context must contain a positive finite scale")]
    InvalidViewContext,
    /// Tolerance input is unusable.
    #[error("snap tolerance must be finite and non-negative")]
    InvalidTolerance,
    /// The axis vector is unusable.
    #[error("snap axis must contain a non-zero finite vector")]
    InvalidAxis,
    /// Floor input is unusable.
    #[error("snap floor must be finite")]
    InvalidFloor,
    /// A tolerance mode name is not part of the contract.
    #[error("unknown snap tolerance mode: {0}")]
    UnknownMode(String),
}

/// Selects the nearest eligible scalar correction without mutating state.
pub fn select_snap_correction(query: SnapQuery<'_>) -> Result<SnapTranslationResult, SnapError> {
    let SnapQuery {
        snapshot,
        axis,
        proposed_delta,
        previous_delta,
        tolerance_mm,
        tolerance_mode,
        view_context,
        floor_y,
        excluded,
    } = query;
    let axis_length = axis.iter().map(|value| value * value).sum::<f32>().sqrt();
    if axis.iter().any(|value| !value.is_finite())
        || !proposed_delta.is_finite()
        || !previous_delta.is_finite()
        || !axis_length.is_finite()
        || axis_length <= f32::EPSILON
    {
        return Err(SnapError::InvalidAxis);
    }
    let normalized = axis.map(|value| value / axis_length);
    let axis_index = normalized
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| left.abs().total_cmp(&right.abs()))
        .map(|(index, _)| index)
        .ok_or(SnapError::InvalidAxis)?;
    let axis_component = normalized[axis_index];
    let (tolerance, source) = resolve_tolerance(tolerance_mm, tolerance_mode, view_context, false)?;
    let moving_names = [
        SnapLandmark::Minimum,
        SnapLandmark::Center,
        SnapLandmark::Maximum,
    ];
    let target_names = [
        SnapLandmark::Minimum,
        SnapLandmark::Maximum,
        SnapLandmark::Center,
    ];
    let mut best: Option<SnapTranslationResult> = None;
    let mut best_distance = f32::INFINITY;

    let mut candidates = Vec::new();
    if snapshot.include_origin {
        candidates.push(Candidate {
            scalar: 0.0,
            target_kind: SnapTargetKind::Origin,
            target_object_id: None,
            target_landmark: None,
        });
    }
    for target in &snapshot.targets {
        for name in target_names.iter().copied() {
            candidates.push(Candidate {
                scalar: target.bounds.landmark(axis_index, name),
                target_kind: SnapTargetKind::Object,
                target_object_id: Some(target.object_id.clone()),
                target_landmark: Some(name),
            });
        }
    }
    for candidate in candidates {
        for moving_name in moving_names {
            let key = candidate.key(moving_name);
            if excluded.is_some_and(|item| item == &key) {
                continue;
            }
            let query = snapshot.moving_bounds.landmark(axis_index, moving_name)
                + axis_component * proposed_delta;
            let error = query - candidate.scalar;
            let candidate_delta = proposed_delta - error / axis_component;
            let pointer_distance = (candidate_delta - proposed_delta).abs();
            let delta_tolerance = tolerance / axis_component.abs();
            let (low, high) = if previous_delta <= proposed_delta {
                (previous_delta, proposed_delta)
            } else {
                (proposed_delta, previous_delta)
            };
            let near = pointer_distance < delta_tolerance;
            let crossed = (proposed_delta - previous_delta).abs() > 1.0e-8
                && candidate_delta >= low - delta_tolerance
                && candidate_delta <= high + delta_tolerance;
            if !(near || (crossed && candidate.target_kind == SnapTargetKind::Floor)) {
                continue;
            }
            let distance = pointer_distance;
            if distance < best_distance {
                best_distance = distance;
                best = Some(SnapTranslationResult {
                    applied_delta: candidate_delta,
                    snapped: true,
                    tolerance_mm: tolerance,
                    tolerance_source: source,
                    target_kind: Some(candidate.target_kind),
                    target_object_id: candidate.target_object_id.clone(),
                    moving_landmark: Some(moving_name),
                    target_landmark: candidate.target_landmark,
                    error: Some(error),
                    candidate_key: Some(key),
                });
            }
        }
    }

    if let Some(floor) = floor_y {
        if !floor.is_finite() {
            return Err(SnapError::InvalidFloor);
        }
        if axis_index == 1 && axis_component.abs() > f32::EPSILON {
            let (floor_tolerance, floor_source) =
                resolve_tolerance(tolerance_mm, SNAP_TOLERANCE_FLOOR_FIXED, view_context, true)?;
            let candidate = Candidate {
                scalar: floor,
                target_kind: SnapTargetKind::Floor,
                target_object_id: None,
                target_landmark: None,
            };
            for moving_name in moving_names {
                let key = candidate.key(moving_name);
                if excluded.is_some_and(|item| item.target_kind == SnapTargetKind::Floor) {
                    continue;
                }
                let query = snapshot.moving_bounds.landmark(axis_index, moving_name)
                    + axis_component * proposed_delta;
                let error = query - candidate.scalar;
                let candidate_delta = proposed_delta - error / axis_component;
                let pointer_distance = (candidate_delta - proposed_delta).abs();
                let delta_tolerance = floor_tolerance / axis_component.abs();
                let (low, high) = if previous_delta <= proposed_delta {
                    (previous_delta, proposed_delta)
                } else {
                    (proposed_delta, previous_delta)
                };
                let near = pointer_distance < delta_tolerance;
                let crossed = (proposed_delta - previous_delta).abs() > 1.0e-8
                    && candidate_delta >= low - delta_tolerance
                    && candidate_delta <= high + delta_tolerance;
                if !(near || crossed) {
                    continue;
                }
                if pointer_distance < best_distance {
                    best_distance = pointer_distance;
                    best = Some(SnapTranslationResult {
                        applied_delta: candidate_delta,
                        snapped: true,
                        tolerance_mm: floor_tolerance,
                        tolerance_source: floor_source,
                        target_kind: Some(SnapTargetKind::Floor),
                        target_object_id: None,
                        moving_landmark: Some(moving_name),
                        target_landmark: None,
                        error: Some(error),
                        candidate_key: Some(key),
                    });
                }
            }
        }
    }

    Ok(best.unwrap_or(SnapTranslationResult {
        applied_delta: proposed_delta,
        snapped: false,
        tolerance_mm: tolerance,
        tolerance_source: source,
        target_kind: None,
        target_object_id: None,
        moving_landmark: None,
        target_landmark: None,
        error: None,
        candidate_key: None,
    }))
}

/// Applies capture/release hysteresis to the stateless solver.
pub fn snap_translation_result(
    query: SnapQuery<'_>,
    state: &mut SnapState,
) -> Result<SnapTranslationResult, SnapError> {
    let SnapQuery {
        snapshot,
        axis,
        proposed_delta,
        previous_delta,
        tolerance_mm,
        tolerance_mode,
        view_context,
        floor_y,
        excluded: _,
    } = query;
    if let Some(active) = state.active_delta {
        let movement = proposed_delta - previous_delta;
        let moved_away = (proposed_delta - active) * movement.signum();
        if moved_away <= state.release_distance.max(0.05) + 1.0e-6 {
            let (tolerance, source) =
                resolve_tolerance(tolerance_mm, tolerance_mode, view_context, false)?;
            return Ok(SnapTranslationResult {
                applied_delta: active,
                snapped: true,
                tolerance_mm: tolerance,
                tolerance_source: source,
                target_kind: state.active_key.as_ref().map(|key| key.target_kind),
                target_object_id: state
                    .active_key
                    .as_ref()
                    .and_then(|key| key.target_object_id.clone()),
                moving_landmark: state.active_key.as_ref().map(|key| key.moving_landmark),
                target_landmark: state
                    .active_key
                    .as_ref()
                    .and_then(|key| key.target_landmark),
                error: None,
                candidate_key: state.active_key.clone(),
            });
        }
        state.released_delta = Some(active);
        state.released_direction = movement.signum();
        state.released_key = state.active_key.take();
        state.active_delta = None;
    }
    let excluded = state.released_key.as_ref();
    let result = select_snap_correction(SnapQuery {
        snapshot,
        axis,
        proposed_delta,
        previous_delta,
        tolerance_mm,
        tolerance_mode,
        view_context,
        floor_y,
        excluded,
    })?;
    if result.snapped {
        state.active_delta = Some(result.applied_delta);
        state.active_key = result.candidate_key.clone();
        state.release_distance = result.tolerance_mm;
        state.released_delta = None;
        state.released_direction = 0.0;
        state.released_key = None;
    }
    Ok(result)
}

struct Candidate {
    scalar: f32,
    target_kind: SnapTargetKind,
    target_object_id: Option<String>,
    target_landmark: Option<SnapLandmark>,
}

impl Candidate {
    fn key(&self, moving_landmark: SnapLandmark) -> SnapCandidateKey {
        SnapCandidateKey {
            target_kind: self.target_kind,
            target_object_id: self.target_object_id.clone(),
            moving_landmark,
            target_landmark: self.target_landmark,
        }
    }
}

fn resolve_tolerance(
    tolerance_mm: Option<f32>,
    mode: &str,
    view_context: Option<SnapViewContext>,
    floor: bool,
) -> Result<(f32, &'static str), SnapError> {
    let (default, source) = if floor || mode == SNAP_TOLERANCE_FLOOR_FIXED {
        (FLOOR_SNAP_TOLERANCE_MM, SNAP_TOLERANCE_FLOOR_FIXED)
    } else if mode == SNAP_TOLERANCE_VIEW_SCALED {
        (
            view_context
                .ok_or(SnapError::InvalidViewContext)?
                .tolerance_mm()?,
            SNAP_TOLERANCE_VIEW_SCALED,
        )
    } else if mode == SNAP_TOLERANCE_FIXED_MM {
        (OBJECT_SNAP_TOLERANCE_MM, SNAP_TOLERANCE_FIXED_MM)
    } else {
        return Err(SnapError::UnknownMode(mode.to_owned()));
    };
    let tolerance = tolerance_mm.unwrap_or(default);
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err(SnapError::InvalidTolerance);
    }
    Ok((tolerance, source))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> SnapSceneSnapshot {
        SnapSceneSnapshot::new(
            SnapBounds::new([0.0, 0.0, 0.0], [10.0, 10.0, 10.0]).unwrap_or(SnapBounds {
                minimum: [0.0; 3],
                maximum: [10.0; 3],
            }),
            vec![String::from("moving")],
            [
                SnapTarget {
                    object_id: String::from("moving"),
                    scene_order: 0,
                    bounds: SnapBounds {
                        minimum: [0.0; 3],
                        maximum: [10.0; 3],
                    },
                },
                SnapTarget {
                    object_id: String::from("target"),
                    scene_order: 1,
                    bounds: SnapBounds {
                        minimum: [14.0, 0.0, 0.0],
                        maximum: [24.0, 10.0, 10.0],
                    },
                },
            ],
            false,
        )
    }

    #[test]
    fn snapshot_excludes_moving_objects_and_snaps_nearest_landmark() {
        let scene = snapshot();
        let result = select_snap_correction(SnapQuery {
            snapshot: &scene,
            axis: [1.0, 0.0, 0.0],
            proposed_delta: 3.8,
            previous_delta: 0.0,
            tolerance_mm: Some(4.0),
            tolerance_mode: SNAP_TOLERANCE_FIXED_MM,
            view_context: None,
            floor_y: None,
            excluded: None,
        });
        assert!(result.is_ok());
        if let Ok(value) = result {
            assert!(value.snapped);
            assert!((value.applied_delta - 4.0).abs() < 1.0e-5);
            assert_eq!(value.target_object_id.as_deref(), Some("target"));
        }
        assert_eq!(snapshot().targets.len(), 1);
    }

    #[test]
    fn floor_and_view_scaled_tolerances_are_typed() {
        let context = SnapViewContext::new(100.0, [5.0, 5.0, 5.0]);
        assert!(context.is_ok());
        let scene = snapshot();
        let result = select_snap_correction(SnapQuery {
            snapshot: &scene,
            axis: [0.0, 1.0, 0.0],
            proposed_delta: -7.0,
            previous_delta: 0.0,
            tolerance_mm: None,
            tolerance_mode: SNAP_TOLERANCE_VIEW_SCALED,
            view_context: context.ok(),
            floor_y: Some(0.0),
            excluded: None,
        });
        assert!(
            result.is_ok_and(
                |value| value.snapped && value.target_kind == Some(SnapTargetKind::Floor)
            )
        );
    }

    #[test]
    fn capture_holds_until_release_distance_then_excludes_previous_target() {
        let mut state = SnapState::new();
        let scene = snapshot();
        let first = snap_translation_result(
            SnapQuery {
                snapshot: &scene,
                axis: [1.0, 0.0, 0.0],
                proposed_delta: 3.8,
                previous_delta: 0.0,
                tolerance_mm: Some(4.0),
                tolerance_mode: SNAP_TOLERANCE_FIXED_MM,
                view_context: None,
                floor_y: None,
                excluded: None,
            },
            &mut state,
        );
        assert!(first.is_ok_and(|value| value.snapped));
        let held = snap_translation_result(
            SnapQuery {
                snapshot: &scene,
                axis: [1.0, 0.0, 0.0],
                proposed_delta: 3.9,
                previous_delta: 3.8,
                tolerance_mm: Some(4.0),
                tolerance_mode: SNAP_TOLERANCE_FIXED_MM,
                view_context: None,
                floor_y: None,
                excluded: None,
            },
            &mut state,
        );
        assert!(held.is_ok_and(|value| (value.applied_delta - 4.0).abs() < 1.0e-5));
        state.clear();
        assert!(state.active_delta.is_none());
    }
}
