//! Bounded QHull support-face planning shared by native and semantic adapters.

use crate::mesh_ops::{DEFAULT_MAX_OPERATION_MEMORY_BYTES, DEFAULT_MAX_OPERATION_VERTICES};
use crate::{CoreError, Scene, Transform};

/// Conservative placement point budget, including native hull and input copies.
pub const MAX_PLACEMENT_POINTS: usize = {
    let memory_points = (DEFAULT_MAX_OPERATION_MEMORY_BYTES / 320) as usize;
    if memory_points < DEFAULT_MAX_OPERATION_VERTICES {
        memory_points
    } else {
        DEFAULT_MAX_OPERATION_VERTICES
    }
};

const IDENTITY: [[f64; 3]; 3] = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];

/// Plans the Python reference's support-face rotation in world coordinates.
///
/// Uses double precision and QHull `QbB Pp Qt` on every host. Degenerate clouds
/// retain orientation; nonfinite or over-budget inputs return an error before
/// model mutation. The native wrapper's panics cannot cross this core seam.
pub fn placement_rotation(points: &[[f64; 3]]) -> Result<[[f64; 3]; 3], CoreError> {
    if points.len() > MAX_PLACEMENT_POINTS {
        return Err(CoreError::OperationResourceLimit {
            operation: "settle".into(),
            reason: "placement point/memory budget exceeded".into(),
        });
    }
    if points.iter().flatten().any(|value| !value.is_finite()) {
        return Err(CoreError::InvalidMesh(
            "placement points must be finite".into(),
        ));
    }
    if points.len() < 4 {
        return Ok(IDENTITY);
    }
    std::panic::catch_unwind(|| rotation_inner(points)).map_err(|_| {
        CoreError::MeshOperation("native placement kernel rejected its input".into())
    })?
}

fn rotation_inner(points: &[[f64; 3]]) -> Result<[[f64; 3]; 3], CoreError> {
    let hull = match build_hull(points, &["Qt", "QbB", "Pp"]) {
        Ok(hull) => hull,
        Err(_) => match build_hull(points, &["Qt", "QJ", "Pp"]) {
            Ok(hull) => hull,
            Err(_) => return Ok(IDENTITY),
        },
    };
    let centroid: [f64; 3] = std::array::from_fn(|axis| {
        points.iter().map(|p| p[axis]).sum::<f64>() / points.len() as f64
    });
    let mut best_score = -1.;
    let mut normal = [0., 0., 1.];
    for face in hull.simplices() {
        let vertices = face.vertices().ok_or_else(|| {
            CoreError::MeshOperation("placement hull has no facet vertices".into())
        })?;
        let mut indices = [0usize; 3];
        let mut count = 0;
        for vertex in vertices.iter() {
            // point_id handles QbB's internal scaled buffer; index() assumes
            // the original buffer and asserts in the upstream debug build.
            let id = vertex
                .point_id(&hull)
                .map_err(|error| CoreError::MeshOperation(error.to_string()))?;
            let index = usize::try_from(id)
                .ok()
                .filter(|i| *i < points.len())
                .ok_or_else(|| {
                    CoreError::MeshOperation("placement hull point is out of range".into())
                })?;
            let target = indices.get_mut(count).ok_or_else(|| {
                CoreError::MeshOperation("placement facet is not triangular".into())
            })?;
            *target = index;
            count += 1;
        }
        if count != 3 {
            return Err(CoreError::MeshOperation(
                "placement facet is not triangular".into(),
            ));
        }
        let [ia, ib, ic] = indices;
        let [a, b, c] = [points[ia], points[ib], points[ic]];
        let ab: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
        let ac: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
        let mut cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        if (0..3).map(|i| cross[i] * (a[i] - centroid[i])).sum::<f64>() < 0. {
            cross = cross.map(|v| -v);
        }
        let length = cross.iter().map(|v| v * v).sum::<f64>().sqrt();
        if length < 1e-8 {
            continue;
        }
        let score = (length / 2.) * (-cross[1] / length).max(0.);
        if score > best_score {
            best_score = score;
            normal = cross.map(|v| v / length);
        }
    }
    let cosine = (-normal[1]).clamp(-1., 1.);
    if cosine > 0.9999 {
        return Ok(IDENTITY);
    }
    let mut axis = [normal[2], 0., -normal[0]];
    let length = axis.iter().map(|v| v * v).sum::<f64>().sqrt();
    if length < 1e-8 {
        return Ok(IDENTITY);
    }
    axis = axis.map(|v| v / length);
    let sine = cosine.acos().sin();
    let skew = [
        [0., -axis[2], axis[1]],
        [axis[2], 0., -axis[0]],
        [-axis[1], axis[0], 0.],
    ];
    Ok(std::array::from_fn(|i| {
        std::array::from_fn(|j| {
            let square = (0..3).map(|k| skew[i][k] * skew[k][j]).sum::<f64>();
            IDENTITY[i][j] + sine * skew[i][j] + (1. - cosine) * square
        })
    }))
}

fn build_hull(points: &[[f64; 3]], flags: &[&str]) -> Result<qhull::Qh<'static>, CoreError> {
    qhull::Qh::builder()
        .capture_stderr(false)
        .qhull_args(flags.iter().copied())
        .map_err(|error| CoreError::MeshOperation(error.to_string()))?
        .build_from_iter(points.iter().copied())
        .map_err(|error| CoreError::MeshOperation(error.to_string()))
}

/// Rigidly settles a selection's combined support face on the Y=0 build floor.
/// Drops it straight down: the rotation that lays the support face flat is
/// pivoted about the selection's current X/Z centre, so the horizontal spot
/// does not change (OWNER-2026-11-21-PASTE-SETTLE). Preserves resized/mirrored shape and relative placement, and
/// commits once (no history for a no-op). Invalid targets or resource limits
/// leave the scene unchanged. Shared by desktop and MCP on every platform.
pub fn settle_on_floor(
    scene: &mut Scene,
    indices: &[usize],
) -> Result<crate::CommandResult, CoreError> {
    let indices = indices
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let count = indices.iter().try_fold(0usize, |count, index| {
        let next = count.checked_add(scene.object(*index)?.mesh().vertices.len());
        next.filter(|count| *count <= MAX_PLACEMENT_POINTS)
            .ok_or_else(|| CoreError::OperationResourceLimit {
                operation: "settle".into(),
                reason: "placement point/memory budget exceeded".into(),
            })
    })?;
    if count == 0 {
        return Err(CoreError::InvalidMesh("Settle needs vertices".into()));
    }
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
    let rotation = placement_rotation(&points)?;
    // The hull rotation is expressed in world coordinates. Pre-multiply it
    // for every selection size: post-multiplying a nonuniformly scaled
    // object rotates its local axes through that scale and deforms the mesh.
    let mut minimum = [f64::INFINITY; 3];
    let mut maximum = [f64::NEG_INFINITY; 3];
    let mut before_min = [f64::INFINITY; 3];
    let mut before_max = [f64::NEG_INFINITY; 3];
    for point in points {
        for axis in 0..3 {
            before_min[axis] = before_min[axis].min(point[axis]);
            before_max[axis] = before_max[axis].max(point[axis]);
            let value = (0..3).map(|i| rotation[axis][i] * point[i]).sum::<f64>();
            minimum[axis] = minimum[axis].min(value);
            maximum[axis] = maximum[axis].max(value);
        }
    }
    // Keep the current horizontal centre; only the height changes to rest on Y=0.
    let offset = [
        (before_min[0] + before_max[0]) * 0.5 - (minimum[0] + maximum[0]) * 0.5,
        -minimum[1],
        (before_min[2] + before_max[2]) * 0.5 - (minimum[2] + maximum[2]) * 0.5,
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
    let mut commands: Vec<Box<dyn crate::Command>> = Vec::new();
    for (index, transform) in indices.iter().copied().zip(transforms) {
        let current = scene.object(index)?.transform().rows();
        if !current
            .iter()
            .flatten()
            .zip(transform.rows().iter().flatten())
            .all(|(before, after)| (*before - *after).abs() <= 1e-8 + 1e-5 * after.abs())
        {
            commands.push(Box::new(crate::TransformObjectCommand::new(
                index, transform,
            )));
        }
    }
    scene.execute(crate::GroupedCommand::new(commands))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_faces_match_both_python_world_clouds() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../evidence/rebuild/rb-04-scene-and-precise-transforms/settle-hull-oracle.json"
        ))
        .expect("oracle");
        for case in cases.as_array().expect("cases") {
            let points = case["world_points"]
                .as_array()
                .expect("points")
                .iter()
                .map(|point| std::array::from_fn(|i| point[i].as_f64().expect("coordinate")))
                .collect::<Vec<[f64; 3]>>();
            let rotation = placement_rotation(&points).expect("placement");
            let normal: [f64; 3] =
                std::array::from_fn(|i| case["best_normal"][i].as_f64().expect("normal"));
            let rotated: [f64; 3] =
                std::array::from_fn(|i| (0..3).map(|j| rotation[i][j] * normal[j]).sum::<f64>());
            for (actual, expected) in rotated.into_iter().zip([0., -1., 0.]) {
                assert!((actual - expected).abs() < 1e-7);
            }
        }
    }

    #[test]
    fn invalid_and_over_budget_points_are_rejected_before_native_work() {
        assert!(matches!(
            placement_rotation(&[[f64::NAN, 0., 0.]; 4]),
            Err(CoreError::InvalidMesh(_))
        ));
        let points = vec![[0.; 3]; MAX_PLACEMENT_POINTS + 1];
        assert!(matches!(
            placement_rotation(&points),
            Err(CoreError::OperationResourceLimit { .. })
        ));
        assert_eq!(
            placement_rotation(&[[0.; 3]; 3]).expect("degenerate"),
            IDENTITY
        );
    }
    #[test]
    fn settle_invalid_target_is_atomic() {
        let mut scene = Scene::new();
        scene
            .add_primitive(crate::PrimitiveKind::Cube, None, None)
            .expect("cube");
        let transform = *scene.object(0).expect("object").transform();
        let revision = scene.revision();
        let history = scene.history_state_id();
        assert!(settle_on_floor(&mut scene, &[0, 99]).is_err());
        assert_eq!(*scene.object(0).expect("object").transform(), transform);
        assert_eq!(scene.revision(), revision);
        assert_eq!(scene.history_state_id(), history);
    }

    #[test]
    fn settle_keeps_horizontal_position_and_drops_straight_down() {
        let mut scene = Scene::new();
        scene
            .add_primitive(crate::PrimitiveKind::Cube, None, None)
            .expect("cube");
        scene.translate(0, [150.0, 40.0, -90.0]).expect("move");
        let before = scene.object(0).expect("object").position();
        let history = scene.history_state_id();
        assert!(settle_on_floor(&mut scene, &[0]).expect("settle").changed);
        let after = scene.object(0).expect("object").position();
        assert!((after[0] - before[0]).abs() < 1e-4);
        assert!((after[2] - before[2]).abs() < 1e-4);
        assert!((after[1] - (before[1] - 40.0)).abs() < 1e-3);
        scene.undo().expect("undo");
        assert_eq!(scene.history_state_id(), history);
        assert!(settle_on_floor(&mut scene, &[0]).expect("again").changed);
        let rested = scene.history_state_id();
        assert!(!settle_on_floor(&mut scene, &[0]).expect("noop").changed);
        assert_eq!(scene.history_state_id(), rested);
    }

    #[test]
    fn paste_lands_at_copy_time_spot_only_when_original_moved() {
        let mut scene = Scene::new();
        scene
            .add_primitive(crate::PrimitiveKind::Cube, None, None)
            .expect("cube");
        scene.set_selection([0]);
        let mut clipboard = scene.copy_selected().expect("copy");
        let copy_spot = clipboard[0].position();
        let unmoved = scene.paste_beside(&mut clipboard).expect("paste");
        assert!(unmoved.changed);
        assert!(scene.object(1).expect("copy").position()[0] > copy_spot[0]);
        scene
            .translate(0, [500.0, 0.0, 0.0])
            .expect("move original");
        scene.set_selection([0]);
        let mut clipboard = scene.copy_selected().expect("copy");
        let spot = clipboard[0].position();
        scene.translate(0, [-200.0, 0.0, 30.0]).expect("move again");
        let moved = scene.paste_beside(&mut clipboard).expect("paste");
        assert!(moved.changed);
        assert_eq!(scene.object(2).expect("copy").position(), spot);
    }
}
