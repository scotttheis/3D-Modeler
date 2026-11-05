use crate::CoreError;

/// A row-major, homogeneous 4x4 transform using millimetres for translation.
///
/// The representation is renderer-neutral and matches the Python reference's
/// row-major JSON matrices.  Matrix composition follows the reference's
/// post-multiplication convention.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    rows: [[f32; 4]; 4],
}

impl Default for Transform {
    fn default() -> Self {
        Self::identity()
    }
}

impl Transform {
    /// Returns the identity transform.
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

    /// Validates and constructs a transform from row-major float32 values.
    pub fn from_rows(rows: [[f32; 4]; 4]) -> Result<Self, CoreError> {
        let transform = Self { rows };
        transform.validate()?;
        Ok(transform)
    }

    /// Validates and constructs a transform from project JSON precision values.
    pub fn from_f64_rows(rows: [[f64; 4]; 4]) -> Result<Self, CoreError> {
        let mut converted = [[0.0_f32; 4]; 4];
        for (converted_row, input_row) in converted.iter_mut().zip(rows) {
            for (converted_value, value) in converted_row.iter_mut().zip(input_row) {
                if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX)
                {
                    return Err(CoreError::InvalidTransform(
                        "matrix entry is not a finite float32".to_owned(),
                    ));
                }
                *converted_value = value as f32;
            }
        }
        Self::from_rows(converted)
    }

    /// Returns the matrix as row-major float32 values.
    #[must_use]
    pub const fn rows(&self) -> [[f32; 4]; 4] {
        self.rows
    }

    /// Returns the matrix widened to the precision used by `.3dp` JSON.
    #[must_use]
    pub fn f64_rows(&self) -> [[f64; 4]; 4] {
        let mut result = [[0.0_f64; 4]; 4];
        for (result_row, source_row) in result.iter_mut().zip(self.rows) {
            for (result_value, source_value) in result_row.iter_mut().zip(source_row) {
                *result_value = f64::from(source_value);
            }
        }
        result
    }

    /// Returns the translation column in millimetres.
    #[must_use]
    pub const fn position(&self) -> [f32; 3] {
        [self.rows[0][3], self.rows[1][3], self.rows[2][3]]
    }

    /// Returns a post-multiplied translation transform.
    pub fn translated(&self, offset_mm: [f32; 3]) -> Result<Self, CoreError> {
        ensure_finite_vector(offset_mm, "translation")?;
        let mut translation = Self::identity();
        translation.rows[0][3] = offset_mm[0];
        translation.rows[1][3] = offset_mm[1];
        translation.rows[2][3] = offset_mm[2];
        self.multiply(&translation)
    }

    /// Returns a post-multiplied non-uniform scale transform.
    pub fn scaled(&self, scale: [f32; 3]) -> Result<Self, CoreError> {
        ensure_finite_vector(scale, "scale")?;
        let mut scaling = Self::identity();
        scaling.rows[0][0] = scale[0];
        scaling.rows[1][1] = scale[1];
        scaling.rows[2][2] = scale[2];
        self.multiply(&scaling)
    }

    /// Returns a post-multiplied axis-angle rotation in degrees.
    pub fn rotated(&self, angle_degrees: f32, axis: [f32; 3]) -> Result<Self, CoreError> {
        if !angle_degrees.is_finite() {
            return Err(CoreError::InvalidTransform(
                "rotation angle must be finite".to_owned(),
            ));
        }
        ensure_finite_vector(axis, "rotation axis")?;
        let length = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        if !length.is_finite() || length <= f32::EPSILON {
            return Err(CoreError::InvalidTransform(
                "rotation axis must have non-zero length".to_owned(),
            ));
        }
        let [x, y, z] = [axis[0] / length, axis[1] / length, axis[2] / length];
        let radians = angle_degrees.to_radians();
        let (sine, cosine) = radians.sin_cos();
        let one_minus_cosine = 1.0 - cosine;
        let rotation = Self {
            rows: [
                [
                    cosine + x * x * one_minus_cosine,
                    x * y * one_minus_cosine - z * sine,
                    x * z * one_minus_cosine + y * sine,
                    0.0,
                ],
                [
                    y * x * one_minus_cosine + z * sine,
                    cosine + y * y * one_minus_cosine,
                    y * z * one_minus_cosine - x * sine,
                    0.0,
                ],
                [
                    z * x * one_minus_cosine - y * sine,
                    z * y * one_minus_cosine + x * sine,
                    cosine + z * z * one_minus_cosine,
                    0.0,
                ],
                [0.0, 0.0, 0.0, 1.0],
            ],
        };
        self.multiply(&rotation)
    }

    /// Multiplies two validated transforms using row-major matrix arithmetic.
    pub fn multiply(&self, other: &Self) -> Result<Self, CoreError> {
        let mut result = [[0.0_f32; 4]; 4];
        for (row_index, result_row) in result.iter_mut().enumerate() {
            for (column_index, result_value) in result_row.iter_mut().enumerate() {
                *result_value = (0..4)
                    .map(|index| self.rows[row_index][index] * other.rows[index][column_index])
                    .sum();
            }
        }
        Self::from_rows(result)
    }

    /// Transforms one point in millimetres.
    #[must_use]
    pub fn transform_point(&self, point: [f32; 3]) -> [f32; 3] {
        [
            self.rows[0][0] * point[0]
                + self.rows[0][1] * point[1]
                + self.rows[0][2] * point[2]
                + self.rows[0][3],
            self.rows[1][0] * point[0]
                + self.rows[1][1] * point[1]
                + self.rows[1][2] * point[2]
                + self.rows[1][3],
            self.rows[2][0] * point[0]
                + self.rows[2][1] * point[1]
                + self.rows[2][2] * point[2]
                + self.rows[2][3],
        ]
    }

    /// Returns the inverse of this affine transform.
    ///
    /// Boolean operations compute in world space and rebase their result into
    /// the target object's local space through this seam.  Singular transforms
    /// are rejected before a scene command can mutate the model.
    pub fn inverse(&self) -> Result<Self, CoreError> {
        let a = self.rows[0][0];
        let b = self.rows[0][1];
        let c = self.rows[0][2];
        let d = self.rows[1][0];
        let e = self.rows[1][1];
        let f = self.rows[1][2];
        let g = self.rows[2][0];
        let h = self.rows[2][1];
        let i = self.rows[2][2];
        let determinant = a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
        if !determinant.is_finite() || determinant.abs() <= 1.0e-12 {
            return Err(CoreError::InvalidTransform(
                "transform is singular and cannot be inverted".to_owned(),
            ));
        }
        let inverse_determinant = 1.0 / determinant;
        let inverse_linear = [
            [
                (e * i - f * h) * inverse_determinant,
                (c * h - b * i) * inverse_determinant,
                (b * f - c * e) * inverse_determinant,
            ],
            [
                (f * g - d * i) * inverse_determinant,
                (a * i - c * g) * inverse_determinant,
                (c * d - a * f) * inverse_determinant,
            ],
            [
                (d * h - e * g) * inverse_determinant,
                (b * g - a * h) * inverse_determinant,
                (a * e - b * d) * inverse_determinant,
            ],
        ];
        let translation = self.position();
        let inverse_translation = [
            -(inverse_linear[0][0] * translation[0]
                + inverse_linear[0][1] * translation[1]
                + inverse_linear[0][2] * translation[2]),
            -(inverse_linear[1][0] * translation[0]
                + inverse_linear[1][1] * translation[1]
                + inverse_linear[1][2] * translation[2]),
            -(inverse_linear[2][0] * translation[0]
                + inverse_linear[2][1] * translation[1]
                + inverse_linear[2][2] * translation[2]),
        ];
        Self::from_rows([
            [
                inverse_linear[0][0],
                inverse_linear[0][1],
                inverse_linear[0][2],
                inverse_translation[0],
            ],
            [
                inverse_linear[1][0],
                inverse_linear[1][1],
                inverse_linear[1][2],
                inverse_translation[1],
            ],
            [
                inverse_linear[2][0],
                inverse_linear[2][1],
                inverse_linear[2][2],
                inverse_translation[2],
            ],
            [0.0, 0.0, 0.0, 1.0],
        ])
    }

    pub(crate) fn validate(&self) -> Result<(), CoreError> {
        if self.rows.iter().flatten().any(|value| !value.is_finite()) {
            return Err(CoreError::InvalidTransform(
                "matrix entries must be finite".to_owned(),
            ));
        }
        let bottom = self.rows[3];
        if bottom
            .iter()
            .zip([0.0_f32, 0.0, 0.0, 1.0])
            .any(|(actual, expected)| (actual - expected).abs() > 1.0e-5)
        {
            return Err(CoreError::InvalidTransform(
                "homogeneous bottom row must be [0, 0, 0, 1]".to_owned(),
            ));
        }
        Ok(())
    }
}

fn ensure_finite_vector(vector: [f32; 3], label: &str) -> Result<(), CoreError> {
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(CoreError::InvalidTransform(format!(
            "{label} values must be finite"
        )));
    }
    Ok(())
}
