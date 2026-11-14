use crate::math::{Mat4, Vec3, dot, normalize};

/// User-adjustable mesh lighting, independent of the active viewport theme.
///
/// The default key light is view-aligned so it stays in a fixed place on the
/// screen while the camera orbits. Contact shadow strength only affects the
/// bounded floor footprint cues; it does not alter model geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ViewportLighting {
    /// Keep the key light fixed in view space as the camera moves.
    pub screen_aligned: bool,
    /// Horizontal screen angle in degrees; positive values place light left.
    pub azimuth_degrees: i16,
    /// Light height above the screen plane, in degrees.
    pub elevation_degrees: u8,
    /// Key light strength as a percentage; accepted range is 0 to 150.
    pub intensity_percent: u8,
    /// Ambient fill as a percentage; accepted range is 0 to 80.
    pub ambient_percent: u8,
    /// Specular highlight strength as a percentage; accepted range is 0 to 100.
    pub specular_percent: u8,
    /// Highlight sharpness as a percentage; accepted range is 0 to 100.
    pub highlight_sharpness: u8,
    /// Floor contact shadow strength as a percentage; accepted range is 0 to 200.
    pub contact_shadow_percent: u8,
}

impl Default for ViewportLighting {
    fn default() -> Self {
        Self {
            screen_aligned: true,
            azimuth_degrees: 30,
            elevation_degrees: 55,
            intensity_percent: 100,
            ambient_percent: 32,
            specular_percent: 0,
            highlight_sharpness: 36,
            contact_shadow_percent: 100,
        }
    }
}

impl ViewportLighting {
    /// Clamps deserialized or externally supplied values to supported ranges.
    #[must_use]
    pub fn normalized(self) -> Self {
        Self {
            screen_aligned: self.screen_aligned,
            azimuth_degrees: self.azimuth_degrees.clamp(-80, 80),
            elevation_degrees: self.elevation_degrees.clamp(10, 85),
            intensity_percent: self.intensity_percent.min(150),
            ambient_percent: self.ambient_percent.min(80),
            specular_percent: self.specular_percent.min(100),
            highlight_sharpness: self.highlight_sharpness.min(100),
            contact_shadow_percent: self.contact_shadow_percent.min(200),
        }
    }

    /// Returns the normalized key-light direction in camera view space.
    #[must_use]
    pub(crate) fn direction_in_view(self, view_matrix: Mat4) -> Vec3 {
        let settings = self.normalized();
        let azimuth = f32::from(settings.azimuth_degrees).to_radians();
        let elevation = f32::from(settings.elevation_degrees).to_radians();
        let horizontal = elevation.cos();
        let direction = [
            -azimuth.sin() * horizontal,
            elevation.sin(),
            azimuth.cos() * horizontal,
        ];
        let direction = if settings.screen_aligned {
            direction
        } else {
            let transformed =
                view_matrix.transform4([direction[0], direction[1], direction[2], 0.0]);
            [transformed[0], transformed[1], transformed[2]]
        };
        normalize(direction, "viewport key light").unwrap_or([0.0, 0.0, 1.0])
    }

    /// Returns key strength, ambient fill, highlight strength, and exponent.
    #[must_use]
    pub(crate) fn shader_parameters(self) -> [f32; 4] {
        let settings = self.normalized();
        [
            0.72 * f32::from(settings.intensity_percent) / 100.0,
            f32::from(settings.ambient_percent) / 100.0,
            0.55 * f32::from(settings.specular_percent) / 100.0,
            8.0 + 88.0 * f32::from(settings.highlight_sharpness) / 100.0,
        ]
    }

    /// Shades one CPU fallback sample with the same view-space model as WGSL.
    #[must_use]
    pub(crate) fn shade_sample(
        color: [f32; 4],
        normal: Vec3,
        view_position: Vec3,
        light_direction: Vec3,
        [key_strength, ambient, specular_strength, exponent]: [f32; 4],
    ) -> [f32; 4] {
        let normal = normalize(normal, "viewport surface normal").unwrap_or([0.0, 0.0, 1.0]);
        let light = normalize(light_direction, "viewport key light").unwrap_or([0.0, 0.0, 1.0]);
        let toward_viewer = normalize(
            [-view_position[0], -view_position[1], -view_position[2]],
            "viewport view direction",
        )
        .unwrap_or([0.0, 0.0, 1.0]);
        // Keep this in step with `fragment_main` in the renderer's WGSL. A
        // plain Lambert term leaves every face turned away from the key light
        // at the same flat ambient value, so vertical faces lose their form.
        // The wrapped term grades those faces and a weak mirrored fill light
        // separates the two sides of a box.
        let facing = dot(normal, light);
        let lambert = facing.max(0.0);
        let wrapped = (facing * 0.5 + 0.5) * (facing * 0.5 + 0.5);
        let fill_direction = normalize([-light[0], 0.0, light[2] + 0.35], "viewport fill light")
            .unwrap_or([0.0, 0.0, 1.0]);
        let fill = dot(normal, fill_direction).max(0.0);
        let diffuse = 0.55 * lambert + 0.33 * wrapped + 0.12 * fill;
        let half_vector = [
            light[0] + toward_viewer[0],
            light[1] + toward_viewer[1],
            light[2] + toward_viewer[2],
        ];
        let specular = if lambert > 0.0 && dot(half_vector, half_vector) > 0.000_001 {
            let halfway = normalize(half_vector, "viewport half vector").unwrap_or(light);
            dot(normal, halfway).max(0.0).powf(exponent)
                * specular_strength
                * key_strength.max(0.25)
        } else {
            0.0
        };
        let illumination = ambient + key_strength * diffuse;
        let mut shaded = color;
        for (input, output) in color.iter().zip(shaded.iter_mut()).take(3) {
            let linear = (*input * illumination + specular).max(0.0);
            *output = if linear <= 0.003_130_8 {
                linear * 12.92
            } else {
                1.055 * linear.powf(1.0 / 2.4) - 0.055
            }
            .clamp(0.0, 1.0);
        }
        shaded
    }
}

/// How much darker a selected object is drawn than its own colour (OF-34):
/// selection keeps the object's colour family, as 3D Builder does, instead
/// of mixing in a highlight colour.
pub const SELECTED_SHADE: f32 = 0.82;

/// The display colour of a selected object or face: the authored colour,
/// slightly darker, alpha unchanged. The GPU renderer and the CPU fallback
/// both use this rule.
#[must_use]
pub fn selection_shade(color: [f32; 4]) -> [f32; 4] {
    [
        color[0] * SELECTED_SHADE,
        color[1] * SELECTED_SHADE,
        color[2] * SELECTED_SHADE,
        color[3],
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn selection_darkens_slightly_and_keeps_the_colour_family() {
        let red = [0.8, 0.2, 0.1, 1.0];
        let shaded = super::selection_shade(red);
        assert!(shaded[0] < red[0] && shaded[0] > red[0] * 0.7);
        // Same hue: channel ratios are unchanged.
        assert!((shaded[0] / shaded[1] - red[0] / red[1]).abs() < 1.0e-5);
        assert!((shaded[1] / shaded[2] - red[1] / red[2]).abs() < 1.0e-5);
        assert_eq!(shaded[3], red[3]);
    }
}
