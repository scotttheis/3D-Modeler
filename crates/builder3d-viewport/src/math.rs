use crate::ViewportError;

/// A finite three-dimensional point or vector.
pub type Vec3 = [f32; 3];

/// A row-major homogeneous matrix used at the renderer boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mat4 {
    rows: [[f32; 4]; 4],
}

impl Mat4 {
    /// Returns the identity matrix.
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            rows: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    /// Constructs a matrix after checking every entry.
    pub fn from_rows(rows: [[f32; 4]; 4]) -> Result<Self, ViewportError> {
        if rows.iter().flatten().any(|value| !value.is_finite()) {
            return Err(ViewportError::InvalidInput(String::from(
                "matrix entries must be finite",
            )));
        }
        Ok(Self { rows })
    }

    /// Returns the row-major matrix.
    #[must_use]
    pub const fn rows(self) -> [[f32; 4]; 4] {
        self.rows
    }

    /// Returns the transposed column-major bytes expected by WGSL mat4x4.
    #[must_use]
    pub fn column_major_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(16 * std::mem::size_of::<f32>());
        for column in 0..4 {
            for row in 0..4 {
                bytes.extend_from_slice(&self.rows[row][column].to_le_bytes());
            }
        }
        bytes
    }

    /// Multiplies two row-major matrices.
    #[must_use]
    pub fn multiply(self, other: Self) -> Self {
        let mut rows = [[0.0_f32; 4]; 4];
        for (row_index, row) in rows.iter_mut().enumerate() {
            for (column_index, value) in row.iter_mut().enumerate() {
                *value = (0..4)
                    .map(|index| self.rows[row_index][index] * other.rows[index][column_index])
                    .sum();
            }
        }
        Self { rows }
    }

    /// Multiplies this matrix by a homogeneous vector.
    #[must_use]
    pub fn transform4(self, value: [f32; 4]) -> [f32; 4] {
        let mut result = [0.0_f32; 4];
        for (row_index, output) in result.iter_mut().enumerate() {
            *output = self.rows[row_index]
                .iter()
                .zip(value)
                .map(|(left, right)| left * right)
                .sum();
        }
        result
    }

    /// Transforms a point and performs the homogeneous divide.
    pub fn transform_point(self, point: Vec3) -> Result<Vec3, ViewportError> {
        ensure_finite(point, "point")?;
        let transformed = self.transform4([point[0], point[1], point[2], 1.0]);
        let w = transformed[3];
        if !w.is_finite() || w.abs() <= f32::EPSILON {
            return Err(ViewportError::InvalidInput(String::from(
                "point projects at an invalid homogeneous depth",
            )));
        }
        let result = [transformed[0] / w, transformed[1] / w, transformed[2] / w];
        ensure_finite(result, "projected point")?;
        Ok(result)
    }

    /// Offsets an affine transform in world space without changing its basis.
    pub(crate) fn translated_by(self, delta: Vec3) -> Result<Self, ViewportError> {
        ensure_finite(delta, "translation delta")?;
        let mut rows = self.rows;
        for axis in 0..3 {
            rows[axis][3] += delta[axis];
        }
        Self::from_rows(rows)
    }

    /// Inverts a general 4x4 matrix with bounded Gauss-Jordan elimination.
    pub fn inverse(self) -> Result<Self, ViewportError> {
        let mut augmented = [[0.0_f64; 8]; 4];
        for (row_index, row) in augmented.iter_mut().enumerate() {
            for (column, value) in row.iter_mut().take(4).enumerate() {
                *value = f64::from(self.rows[row_index][column]);
            }
            row[4 + row_index] = 1.0;
        }
        for pivot_index in 0..4 {
            let mut best_row = pivot_index;
            let mut best_value = augmented[pivot_index][pivot_index].abs();
            for (row_index, row) in augmented.iter().enumerate().skip(pivot_index + 1) {
                let candidate = row[pivot_index].abs();
                if candidate > best_value {
                    best_row = row_index;
                    best_value = candidate;
                }
            }
            if !best_value.is_finite() || best_value <= f64::EPSILON {
                return Err(ViewportError::InvalidInput(String::from(
                    "camera matrix is singular",
                )));
            }
            if best_row != pivot_index {
                augmented.swap(best_row, pivot_index);
            }
            let pivot = augmented[pivot_index][pivot_index];
            for value in &mut augmented[pivot_index] {
                *value /= pivot;
            }
            let pivot_row = augmented[pivot_index];
            for (row_index, row) in augmented.iter_mut().enumerate() {
                if row_index == pivot_index {
                    continue;
                }
                let factor = row[pivot_index];
                for (column, value) in row.iter_mut().enumerate() {
                    *value -= factor * pivot_row[column];
                }
            }
        }
        let mut rows = [[0.0_f32; 4]; 4];
        for (row_index, row) in rows.iter_mut().enumerate() {
            for (column, output) in row.iter_mut().enumerate() {
                let value = augmented[row_index][column + 4];
                if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX)
                {
                    return Err(ViewportError::InvalidInput(String::from(
                        "inverse matrix is outside float32 range",
                    )));
                }
                *output = value as f32;
            }
        }
        Self::from_rows(rows)
    }
}

/// A normalized ray used for exact picking and gizmo hit testing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    /// Ray origin in world space.
    pub origin: Vec3,
    /// Unit direction in world space.
    pub direction: Vec3,
}

impl Ray {
    /// Constructs a ray and normalizes its direction.
    pub fn new(origin: Vec3, direction: Vec3) -> Result<Self, ViewportError> {
        ensure_finite(origin, "ray origin")?;
        Ok(Self {
            origin,
            direction: normalize(direction, "ray direction")?,
        })
    }

    /// Returns one point along the ray.
    #[must_use]
    pub fn at(self, distance: f32) -> Vec3 {
        add(self.origin, scale(self.direction, distance))
    }
}

/// An axis-aligned box used for culling and framing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    /// Minimum corner.
    pub minimum: Vec3,
    /// Maximum corner.
    pub maximum: Vec3,
}

impl Aabb {
    /// Constructs a validated axis-aligned box.
    pub fn new(minimum: Vec3, maximum: Vec3) -> Result<Self, ViewportError> {
        ensure_finite(minimum, "AABB minimum")?;
        ensure_finite(maximum, "AABB maximum")?;
        if minimum.iter().zip(maximum).any(|(low, high)| low > &high) {
            return Err(ViewportError::InvalidInput(String::from(
                "AABB minimum must not exceed maximum",
            )));
        }
        Ok(Self { minimum, maximum })
    }

    /// Returns the eight corners in a stable order.
    #[must_use]
    pub fn corners(self) -> [Vec3; 8] {
        let [x0, y0, z0] = self.minimum;
        let [x1, y1, z1] = self.maximum;
        [
            [x0, y0, z0],
            [x1, y0, z0],
            [x0, y1, z0],
            [x1, y1, z0],
            [x0, y0, z1],
            [x1, y0, z1],
            [x0, y1, z1],
            [x1, y1, z1],
        ]
    }

    /// Returns the union of two boxes.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let mut minimum = self.minimum;
        let mut maximum = self.maximum;
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(other.minimum[axis]);
            maximum[axis] = maximum[axis].max(other.maximum[axis]);
        }
        Self { minimum, maximum }
    }

    /// Returns the box transformed by a homogeneous matrix.
    pub fn transformed(self, matrix: Mat4) -> Result<Self, ViewportError> {
        let mut points = self.corners().into_iter();
        let first =
            matrix.transform_point(points.next().ok_or_else(|| {
                ViewportError::InvalidInput(String::from("AABB has no corners"))
            })?)?;
        let mut minimum = first;
        let mut maximum = first;
        for point in points {
            let transformed = matrix.transform_point(point)?;
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(transformed[axis]);
                maximum[axis] = maximum[axis].max(transformed[axis]);
            }
        }
        Self::new(minimum, maximum)
    }

    /// Tests a ray against the box and returns the nearest non-negative entry.
    #[must_use]
    pub fn ray_entry(self, ray: Ray) -> Option<f32> {
        let mut near = 0.0_f32;
        let mut far = f32::INFINITY;
        for axis in 0..3 {
            let origin = ray.origin[axis];
            let direction = ray.direction[axis];
            if direction.abs() <= f32::EPSILON {
                if origin < self.minimum[axis] || origin > self.maximum[axis] {
                    return None;
                }
                continue;
            }
            let inverse = 1.0 / direction;
            let mut first = (self.minimum[axis] - origin) * inverse;
            let mut second = (self.maximum[axis] - origin) * inverse;
            if first > second {
                std::mem::swap(&mut first, &mut second);
            }
            near = near.max(first);
            far = far.min(second);
            if near > far {
                return None;
            }
        }
        far.is_finite().then_some(near.max(0.0))
    }
}

/// Dot product of two vectors.
#[must_use]
pub fn dot(left: Vec3, right: Vec3) -> f32 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

/// Vector cross product.
#[must_use]
pub fn cross(left: Vec3, right: Vec3) -> Vec3 {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

/// Adds two vectors.
#[must_use]
pub fn add(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

/// Subtracts two vectors.
#[must_use]
pub fn subtract(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

/// Multiplies a vector by a scalar.
#[must_use]
pub fn scale(value: Vec3, factor: f32) -> Vec3 {
    [value[0] * factor, value[1] * factor, value[2] * factor]
}

/// Normalizes a finite vector.
pub fn normalize(value: Vec3, label: &str) -> Result<Vec3, ViewportError> {
    ensure_finite(value, label)?;
    let length = dot(value, value).sqrt();
    if !length.is_finite() || length <= f32::EPSILON {
        return Err(ViewportError::InvalidInput(format!(
            "{label} must be non-zero"
        )));
    }
    Ok(scale(value, 1.0 / length))
}

/// Checks all vector entries for finite values.
pub fn ensure_finite(value: Vec3, label: &str) -> Result<(), ViewportError> {
    if value.iter().any(|entry| !entry.is_finite()) {
        return Err(ViewportError::InvalidInput(format!(
            "{label} must be finite"
        )));
    }
    Ok(())
}

/// Intersects a ray with one triangle using a bounded Möller–Trumbore test.
#[must_use]
pub fn ray_triangle(ray: Ray, first: Vec3, second: Vec3, third: Vec3) -> Option<(f32, [f32; 3])> {
    let edge_one = subtract(second, first);
    let edge_two = subtract(third, first);
    let perpendicular = cross(ray.direction, edge_two);
    let determinant = dot(edge_one, perpendicular);
    if determinant.abs() <= 1.0e-7 {
        return None;
    }
    let inverse = 1.0 / determinant;
    let offset = subtract(ray.origin, first);
    let u = dot(offset, perpendicular) * inverse;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let direction = cross(offset, edge_one);
    let v = dot(ray.direction, direction) * inverse;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = dot(edge_two, direction) * inverse;
    if !distance.is_finite() || distance < 0.0 {
        return None;
    }
    Some((distance, [1.0 - u - v, u, v]))
}

/// Builds a right-handed look-at matrix matching the Python camera.
pub fn look_at(position: Vec3, target: Vec3) -> Result<Mat4, ViewportError> {
    let forward = normalize(subtract(target, position), "camera forward")?;
    let mut right = cross(forward, [0.0, 1.0, 0.0]);
    if dot(right, right) <= f32::EPSILON {
        right = cross(forward, [0.0, 0.0, 1.0]);
    }
    right = normalize(right, "camera right")?;
    let up = normalize(cross(right, forward), "camera up")?;
    Mat4::from_rows([
        [right[0], right[1], right[2], -dot(right, position)],
        [up[0], up[1], up[2], -dot(up, position)],
        [
            -forward[0],
            -forward[1],
            -forward[2],
            dot(forward, position),
        ],
        [0.0, 0.0, 0.0, 1.0],
    ])
}

/// Builds a finite perspective projection in the wgpu depth convention.
pub fn perspective(
    vertical_fov_degrees: f32,
    aspect: f32,
    near: f32,
    far: f32,
) -> Result<Mat4, ViewportError> {
    if !vertical_fov_degrees.is_finite()
        || !aspect.is_finite()
        || !near.is_finite()
        || !far.is_finite()
        || !(0.0..180.0).contains(&vertical_fov_degrees)
        || aspect <= 0.0
        || near <= 0.0
        || far <= near
    {
        return Err(ViewportError::InvalidInput(String::from(
            "perspective parameters are invalid",
        )));
    }
    let focal = 1.0 / (vertical_fov_degrees.to_radians() * 0.5).tan();
    Mat4::from_rows([
        [focal / aspect, 0.0, 0.0, 0.0],
        [0.0, focal, 0.0, 0.0],
        [0.0, 0.0, far / (near - far), (near * far) / (near - far)],
        [0.0, 0.0, -1.0, 0.0],
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_round_trips_projection_and_view() {
        let view = look_at([4.0, 3.0, 5.0], [0.0, 0.0, 0.0])
            .map_err(|_| ())
            .ok();
        let Some(view) = view else {
            return;
        };
        let projection = perspective(45.0, 1.5, 0.1, 1_000.0).map_err(|_| ()).ok();
        let Some(projection) = projection else {
            return;
        };
        let combined = projection.multiply(view);
        let inverse = combined.inverse().map_err(|_| ()).ok();
        let Some(inverse) = inverse else {
            return;
        };
        let point = combined
            .transform_point([0.2, 0.3, 0.4])
            .map_err(|_| ())
            .ok();
        let Some(point) = point else {
            return;
        };
        let recovered = inverse.transform_point(point).map_err(|_| ()).ok();
        let Some(recovered) = recovered else {
            return;
        };
        assert!((recovered[0] - 0.2).abs() < 1.0e-3);
        assert!((recovered[1] - 0.3).abs() < 1.0e-3);
        assert!((recovered[2] - 0.4).abs() < 1.0e-3);
    }

    #[test]
    fn ray_box_and_triangle_intersections_are_nearest_and_bounded() {
        let ray = Ray::new([0.0, 0.0, 2.0], [0.0, 0.0, -1.0])
            .map_err(|_| ())
            .ok();
        let Some(ray) = ray else {
            return;
        };
        let box_bounds = Aabb::new([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0])
            .map_err(|_| ())
            .ok();
        let Some(box_bounds) = box_bounds else {
            return;
        };
        assert_eq!(box_bounds.ray_entry(ray), Some(1.0));
        let hit = ray_triangle(ray, [-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [0.0, 1.0, 0.0]);
        assert!(hit.is_some_and(|(distance, barycentric)| {
            (distance - 2.0).abs() < 1.0e-5 && (barycentric[0] - 0.25).abs() < 1.0e-5
        }));
    }
}
