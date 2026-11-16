//! Absolute XYZ Euler rotation preserving the reference's signed polar stretch.
use modeler3d_core::{CoreError, Transform};

type Matrix = [[f64; 3]; 3];

fn transpose(a: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| a[j][i]))
}

fn multiply(a: Matrix, b: Matrix) -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| (0..3).map(|k| a[i][k] * b[k][j]).sum()))
}

fn determinant(a: Matrix) -> f64 {
    a[0][0] * (a[1][1] * a[2][2] - a[1][2] * a[2][1])
        - a[0][1] * (a[1][0] * a[2][2] - a[1][2] * a[2][0])
        + a[0][2] * (a[1][0] * a[2][1] - a[1][1] * a[2][0])
}

fn decompose(transform: &Transform) -> Result<(Matrix, Matrix), CoreError> {
    let rows = transform.rows();
    let a: Matrix = std::array::from_fn(|i| std::array::from_fn(|j| f64::from(rows[i][j])));
    let mut eigen = multiply(transpose(a), a);
    let mut vectors: Matrix = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    for _ in 0..32 {
        let (p, q) = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .max_by(|(i, j), (k, l)| eigen[*i][*j].abs().total_cmp(&eigen[*k][*l].abs()))
            .ok_or_else(|| CoreError::InvalidTransform("rotation decomposition failed".into()))?;
        if eigen[p][q].abs() < 1e-14 {
            break;
        }
        let angle = 0.5 * (2.0 * eigen[p][q]).atan2(eigen[q][q] - eigen[p][p]);
        let (s, c) = angle.sin_cos();
        let mut jacobi: Matrix = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        jacobi[p][p] = c;
        jacobi[q][q] = c;
        jacobi[p][q] = s;
        jacobi[q][p] = -s;
        eigen = multiply(transpose(jacobi), multiply(eigen, jacobi));
        vectors = multiply(vectors, jacobi);
    }
    let mut singular: [f64; 3] = std::array::from_fn(|i| eigen[i][i].max(0.0).sqrt());
    if singular.iter().any(|x| *x < 1e-10) {
        return Err(CoreError::InvalidTransform(
            "rotation requires a nonsingular transform".into(),
        ));
    }
    if determinant(a) < 0.0 {
        let index = (0..3)
            .min_by(|i, j| singular[*i].total_cmp(&singular[*j]))
            .unwrap_or(0);
        singular[index] *= -1.0;
    }
    let stretch = multiply(
        std::array::from_fn(|i| std::array::from_fn(|j| vectors[i][j] * singular[j])),
        transpose(vectors),
    );
    let inverse = multiply(
        std::array::from_fn(|i| std::array::from_fn(|j| vectors[i][j] / singular[j])),
        transpose(vectors),
    );
    Ok((multiply(a, inverse), stretch))
}

pub(crate) fn basis(transform: &Transform) -> Result<[[f32; 3]; 3], CoreError> {
    Ok(decompose(transform)?.0.map(|row| row.map(|v| v as f32)))
}

pub(crate) fn degrees(transform: &Transform) -> Result<[f32; 3], CoreError> {
    let (r, _) = decompose(transform)?;
    let cy = r[0][0].hypot(r[1][0]);
    let angles = if cy > 1e-7 {
        [
            r[2][1].atan2(r[2][2]),
            (-r[2][0]).atan2(cy),
            r[1][0].atan2(r[0][0]),
        ]
    } else {
        [(-r[1][2]).atan2(r[1][1]), (-r[2][0]).atan2(cy), 0.0]
    };
    Ok(angles.map(|x| x.to_degrees() as f32))
}

pub(crate) fn with_degrees(
    transform: &Transform,
    degrees: [f32; 3],
) -> Result<Transform, CoreError> {
    if degrees.iter().any(|x| !x.is_finite()) {
        return Err(CoreError::InvalidTransform(
            "rotation must be finite".into(),
        ));
    }
    let (_, stretch) = decompose(transform)?;
    let [x, y, z] = degrees.map(|v| f64::from(v).to_radians());
    let (sx, cx) = x.sin_cos();
    let (sy, cy) = y.sin_cos();
    let (sz, cz) = z.sin_cos();
    let rotation = [
        [cz * cy, cz * sy * sx - sz * cx, cz * sy * cx + sz * sx],
        [sz * cy, sz * sy * sx + cz * cx, sz * sy * cx - cz * sx],
        [-sy, cy * sx, cy * cx],
    ];
    let linear = multiply(rotation, stretch);
    let mut rows = transform.rows();
    for i in 0..3 {
        for j in 0..3 {
            rows[i][j] = linear[i][j] as f32;
        }
    }
    Transform::from_rows(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(serde::Deserialize)]
    struct Case {
        before: [[f64; 4]; 4],
        angles: [f32; 3],
        after: [[f64; 4]; 4],
    }
    #[test]
    fn absolute_rotation_matches_python_scale_shear_reflection_and_gimbal_oracles() {
        let cases: Vec<Case> = serde_json::from_str(include_str!(
            "../../../evidence/rebuild/rb-04-scene-and-precise-transforms/rotation-oracle.json"
        ))
        .expect("oracle");
        for case in cases {
            let before = Transform::from_f64_rows(case.before).expect("before");
            let actual = with_degrees(&before, case.angles).expect("rotation");
            for (a, b) in actual
                .f64_rows()
                .iter()
                .flatten()
                .zip(case.after.iter().flatten())
            {
                assert!(
                    (a - b).abs() < 2e-5,
                    "actual {actual:?}; expected {:?}",
                    case.after
                );
            }
            let reported = degrees(&actual).expect("reported angles");
            let repeated = with_degrees(&actual, reported).expect("repeat");
            for (a, b) in repeated
                .rows()
                .iter()
                .flatten()
                .zip(actual.rows().iter().flatten())
            {
                assert!((a - b).abs() < 2e-5);
            }
        }
    }
}
