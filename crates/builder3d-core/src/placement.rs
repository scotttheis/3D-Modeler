//! Bounded QHull support-face planning shared by native and semantic adapters.

use crate::CoreError;
use crate::mesh_ops::{DEFAULT_MAX_OPERATION_MEMORY_BYTES, DEFAULT_MAX_OPERATION_VERTICES};

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
}
