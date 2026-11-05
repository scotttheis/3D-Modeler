use crate::{
    ViewportError,
    math::{
        Aabb, Mat4, Ray, Vec3, add, cross, dot, look_at, normalize, perspective, scale, subtract,
    },
};

/// Reference camera field of view in degrees.
pub const CAMERA_FOV_DEGREES: f32 = 45.0;
/// Minimum camera distance in scene millimetres.
pub const CAMERA_MIN_DISTANCE: f32 = 0.5;
/// Maximum camera distance accepted by the interaction API.
pub const CAMERA_MAX_DISTANCE: f32 = 5_000.0;
/// Pixel tolerance used by viewport snapping and gizmo hit testing.
pub const VIEW_PICK_TOLERANCE_PIXELS: f32 = 12.0;

/// A validated orbit camera matching the Python reference's world-up model.
#[derive(Clone, Debug)]
pub struct Camera {
    yaw_degrees: f32,
    pitch_degrees: f32,
    distance: f32,
    center: Vec3,
    default_yaw_degrees: f32,
    default_pitch_degrees: f32,
    default_distance: f32,
    viewport_width: u32,
    viewport_height: u32,
    vertical_fov_degrees: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self::new()
    }
}

impl Camera {
    /// Creates the reference opening view: yaw 45°, pitch 35°, distance 5mm.
    #[must_use]
    pub fn new() -> Self {
        Self {
            yaw_degrees: 45.0,
            pitch_degrees: 35.0,
            distance: 5.0,
            center: [0.0; 3],
            default_yaw_degrees: 45.0,
            default_pitch_degrees: 35.0,
            default_distance: 500.0,
            viewport_width: 1,
            viewport_height: 1,
            vertical_fov_degrees: CAMERA_FOV_DEGREES,
        }
    }

    /// Returns yaw, pitch, distance, and target center.
    #[must_use]
    pub fn state(&self) -> CameraState {
        CameraState {
            yaw_degrees: self.yaw_degrees,
            pitch_degrees: self.pitch_degrees,
            distance: self.distance,
            center: self.center,
        }
    }

    /// Returns the camera position in world space.
    #[must_use]
    pub fn position(&self) -> Vec3 {
        let yaw = self.yaw_degrees.to_radians();
        let pitch = self.pitch_degrees.to_radians();
        add(
            self.center,
            [
                self.distance * pitch.cos() * yaw.sin(),
                self.distance * pitch.sin(),
                self.distance * pitch.cos() * yaw.cos(),
            ],
        )
    }

    /// Returns the orbit yaw in degrees.
    #[must_use]
    pub const fn yaw_degrees(&self) -> f32 {
        self.yaw_degrees
    }

    /// Returns the orbit pitch in degrees.
    #[must_use]
    pub const fn pitch_degrees(&self) -> f32 {
        self.pitch_degrees
    }

    /// Returns the target center.
    #[must_use]
    pub const fn center(&self) -> Vec3 {
        self.center
    }

    /// Returns the orbit distance.
    #[must_use]
    pub const fn distance(&self) -> f32 {
        self.distance
    }

    /// Changes the viewport size used by projection and screen rays.
    pub fn set_viewport_size(&mut self, width: u32, height: u32) -> Result<(), ViewportError> {
        if width == 0 || height == 0 {
            return Err(ViewportError::InvalidInput(String::from(
                "camera viewport dimensions must be non-zero",
            )));
        }
        self.viewport_width = width;
        self.viewport_height = height;
        Ok(())
    }

    /// Returns the current viewport dimensions.
    #[must_use]
    pub const fn viewport_size(&self) -> (u32, u32) {
        (self.viewport_width, self.viewport_height)
    }

    /// Orbits with the Python reference sensitivity and pitch clamp.
    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) -> Result<(), ViewportError> {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return Err(ViewportError::InvalidInput(String::from(
                "orbit deltas must be finite",
            )));
        }
        self.yaw_degrees -= delta_x / 6.0;
        self.pitch_degrees = (self.pitch_degrees + delta_y * 0.25).clamp(-89.0, 89.0);
        Ok(())
    }

    /// Pans in the camera plane using distance-scaled motion.
    pub fn pan(&mut self, delta_x: f32, delta_y: f32) -> Result<(), ViewportError> {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return Err(ViewportError::InvalidInput(String::from(
                "pan deltas must be finite",
            )));
        }
        let forward = normalize(subtract(self.center, self.position()), "camera forward")?;
        let mut right = cross(forward, [0.0, 1.0, 0.0]);
        if dot(right, right) <= f32::EPSILON {
            right = cross(forward, [0.0, 0.0, 1.0]);
        }
        right = normalize(right, "camera right")?;
        let up = normalize(cross(right, forward), "camera up")?;
        let speed = self.distance * 0.001;
        self.center = add(
            self.center,
            scale(add(scale(right, delta_x), scale(up, delta_y)), speed),
        );
        Ok(())
    }

    /// Applies the wheel zoom policy and clamps to the bounded range.
    pub fn zoom(&mut self, delta: f32) -> Result<(), ViewportError> {
        if !delta.is_finite() {
            return Err(ViewportError::InvalidInput(String::from(
                "zoom delta must be finite",
            )));
        }
        let absolute = delta.abs();
        if absolute < 1.0 {
            return Ok(());
        }
        let sensitivity = if absolute < 20.0 { 0.008 } else { 0.001 };
        self.distance = (self.distance * (1.0 - delta * sensitivity))
            .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
        Ok(())
    }

    /// Applies a native gesture zoom factor.
    pub fn zoom_factor(&mut self, delta: f32) -> Result<(), ViewportError> {
        if !delta.is_finite() {
            return Err(ViewportError::InvalidInput(String::from(
                "gesture zoom delta must be finite",
            )));
        }
        if delta == 0.0 {
            return Ok(());
        }
        self.distance = (self.distance * (1.0 - delta).max(0.05))
            .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
        Ok(())
    }

    /// Restores the persisted reset-view pose.
    pub fn reset_view(&mut self) {
        self.yaw_degrees = self.default_yaw_degrees;
        self.pitch_degrees = self.default_pitch_degrees;
        self.distance = self.default_distance;
        self.center = [0.0; 3];
    }

    /// Sets an orbit pose while preserving the target and distance.
    pub fn set_view(&mut self, yaw_degrees: f32, pitch_degrees: f32) -> Result<(), ViewportError> {
        if !yaw_degrees.is_finite() || !pitch_degrees.is_finite() {
            return Err(ViewportError::InvalidInput(String::from(
                "camera angles must be finite",
            )));
        }
        self.yaw_degrees = yaw_degrees;
        self.pitch_degrees = pitch_degrees.clamp(-89.0, 89.0);
        Ok(())
    }

    /// Replaces the complete camera pose after validating its bounded state.
    pub fn set_state(&mut self, state: CameraState) -> Result<(), ViewportError> {
        if !state.yaw_degrees.is_finite()
            || !state.pitch_degrees.is_finite()
            || !state.distance.is_finite()
            || !state.center.iter().all(|value| value.is_finite())
        {
            return Err(ViewportError::InvalidInput(String::from(
                "camera state must contain finite values",
            )));
        }
        if !(-89.0..=89.0).contains(&state.pitch_degrees) {
            return Err(ViewportError::InvalidInput(String::from(
                "camera pitch must be between -89 and 89 degrees",
            )));
        }
        if !(CAMERA_MIN_DISTANCE..=CAMERA_MAX_DISTANCE).contains(&state.distance) {
            return Err(ViewportError::InvalidInput(String::from(
                "camera distance is outside the bounded range",
            )));
        }
        self.yaw_degrees = state.yaw_degrees;
        self.pitch_degrees = state.pitch_degrees;
        self.distance = state.distance;
        self.center = state.center;
        Ok(())
    }

    /// Frames world bounds with the current camera angle.
    pub fn frame_bounds(&mut self, bounds: &[Aabb]) -> Result<(), ViewportError> {
        let first = bounds.first().copied().ok_or_else(|| {
            ViewportError::InvalidInput(String::from("at least one frame bound is required"))
        })?;
        let combined = bounds
            .iter()
            .skip(1)
            .fold(first, |current, next| current.union(*next));
        let center = scale(add(combined.minimum, combined.maximum), 0.5);
        let extent = subtract(combined.maximum, combined.minimum);
        let radius = (dot(extent, extent).sqrt() * 0.5).max(0.01);
        let aspect = self.viewport_width as f32 / self.viewport_height.max(1) as f32;
        let horizontal = 2.0
            * ((self.vertical_fov_degrees.to_radians() * 0.5).tan() * aspect.max(1.0e-3)).atan();
        let limiting_fov = self.vertical_fov_degrees.to_radians().min(horizontal);
        self.center = center;
        self.distance = (radius / (limiting_fov * 0.5).tan() * 1.15)
            .max(radius * 2.0)
            .clamp(CAMERA_MIN_DISTANCE, CAMERA_MAX_DISTANCE);
        Ok(())
    }

    /// Builds the camera view matrix.
    pub fn view_matrix(&self) -> Result<Mat4, ViewportError> {
        look_at(self.position(), self.center)
    }

    /// Builds the perspective projection matrix.
    pub fn projection_matrix(&self, scene_radius: f32) -> Result<Mat4, ViewportError> {
        let radius = if scene_radius.is_finite() && scene_radius > 0.0 {
            scene_radius
        } else {
            1.0
        };
        let near = (self.distance - radius * 4.0).max(0.01);
        let far = (self.distance + radius * 4.0).max(near + 1.0);
        perspective(
            self.vertical_fov_degrees,
            self.viewport_width as f32 / self.viewport_height.max(1) as f32,
            near,
            far,
        )
    }

    /// Builds the matrix used by the renderer and screen projection.
    pub fn view_projection(&self, scene_radius: f32) -> Result<Mat4, ViewportError> {
        Ok(self
            .projection_matrix(scene_radius)?
            .multiply(self.view_matrix()?))
    }

    /// Returns the world-space ray through a top-left-origin screen point.
    pub fn screen_ray(&self, x: f32, y: f32, scene_radius: f32) -> Result<Ray, ViewportError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(ViewportError::InvalidInput(String::from(
                "screen coordinates must be finite",
            )));
        }
        let inverse = self.view_projection(scene_radius)?.inverse()?;
        let width = self.viewport_width as f32;
        let height = self.viewport_height as f32;
        let near_ndc = [2.0 * x / width - 1.0, 1.0 - 2.0 * y / height, 0.0];
        let far_ndc = [near_ndc[0], near_ndc[1], 1.0];
        let near = inverse.transform_point(near_ndc)?;
        let far = inverse.transform_point(far_ndc)?;
        Ray::new(near, subtract(far, near))
    }

    /// Converts a world distance to pixels at the target center depth.
    #[must_use]
    pub fn pixels_per_world_unit(&self) -> f32 {
        let height = self.viewport_height.max(1) as f32;
        height
            / (2.0
                * self.distance.max(CAMERA_MIN_DISTANCE)
                * (self.vertical_fov_degrees.to_radians() * 0.5).tan())
    }

    /// Returns a view-scaled world tolerance for a pixel-sized interaction.
    #[must_use]
    pub fn world_tolerance(&self, pixels: f32) -> f32 {
        pixels.max(0.0) / self.pixels_per_world_unit().max(f32::EPSILON)
    }

    /// Returns the current vertical field of view.
    #[must_use]
    pub const fn vertical_fov_degrees(&self) -> f32 {
        self.vertical_fov_degrees
    }
}

/// Serializable camera state used by viewport telemetry and semantic adapters.
#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CameraState {
    /// Orbit yaw in degrees.
    pub yaw_degrees: f32,
    /// Orbit pitch in degrees.
    pub pitch_degrees: f32,
    /// Target distance.
    pub distance: f32,
    /// Orbit target center.
    pub center: Vec3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_orbit_and_zoom_clamps_match_python_contract() {
        let mut camera = Camera::new();
        assert_eq!(
            camera.position(),
            [
                5.0 * 35.0_f32.to_radians().cos() * 45.0_f32.to_radians().sin(),
                5.0 * 35.0_f32.to_radians().sin(),
                5.0 * 35.0_f32.to_radians().cos() * 45.0_f32.to_radians().cos(),
            ]
        );
        let result = camera.orbit(6.0, 1_000.0);
        assert!(result.is_ok());
        assert_eq!(camera.yaw_degrees(), 44.0);
        assert_eq!(camera.pitch_degrees(), 89.0);
        let result = camera.zoom(-10_000_000.0);
        assert!(result.is_ok());
        assert_eq!(camera.distance(), CAMERA_MAX_DISTANCE);
    }

    #[test]
    fn screen_ray_and_frame_are_finite() {
        let mut camera = Camera::new();
        let size_result = camera.set_viewport_size(800, 600);
        assert!(size_result.is_ok());
        let bounds_result = Aabb::new([-1.0, 0.0, -1.0], [1.0, 2.0, 1.0]);
        let Some(bounds) = bounds_result.map_err(|_| ()).ok() else {
            return;
        };
        let frame_result = camera.frame_bounds(&[bounds]);
        assert!(frame_result.is_ok());
        let ray = camera.screen_ray(400.0, 300.0, 2.0);
        assert!(ray.is_ok());
        assert!(ray.is_ok_and(|value| { value.direction.iter().all(|item| item.is_finite()) }));
    }
}
