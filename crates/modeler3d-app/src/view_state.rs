//! Semantic camera and display-state workflows for the application layer.
//!
//! Mouse adapters can drive the viewport camera directly, while menus,
//! automation, and tests use this typed controller for the same state,
//! presets, and frame behavior.

use modeler3d_core::{CoreError, Scene};
use modeler3d_viewport::{Aabb, Camera, CameraState, ViewportError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable display toggles owned by the application adapter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ViewDisplayState {
    /// Whether the build grid is visible.
    pub show_grid: bool,
    /// Whether wireframe edges are visible.
    pub wireframe: bool,
    /// Whether the viewport uses translucent x-ray display.
    pub xray: bool,
}

impl Default for ViewDisplayState {
    fn default() -> Self {
        Self {
            show_grid: true,
            wireframe: false,
            xray: false,
        }
    }
}

/// Complete view state exposed by the application service.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct ViewState {
    /// Horizontal orbit angle in degrees.
    pub yaw_deg: f32,
    /// Vertical orbit angle in degrees.
    pub pitch_deg: f32,
    /// Camera distance in millimetres.
    pub distance_mm: f32,
    /// Camera target center in millimetres.
    pub center_mm: [f32; 3],
    /// Display toggles.
    pub display: ViewDisplayState,
}

/// Target set for a semantic frame command.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameTarget {
    /// Frame every top-level scene object.
    All,
    /// Frame the visible selection, falling back to all visible objects when empty.
    Selected,
}

/// Typed errors from camera and frame workflows.
#[derive(Debug, Error)]
pub enum ViewStateError {
    /// A named preset was not part of the bounded catalog.
    #[error("preset must be reset, top, front, or side")]
    InvalidPreset,
    /// A scene object could not provide a frame bound.
    #[error(transparent)]
    Core(#[from] CoreError),
    /// The renderer-neutral camera rejected a bounded state.
    #[error(transparent)]
    Viewport(#[from] ViewportError),
}

/// Application-owned camera and display-state controller.
pub struct ViewStateController {
    camera: Camera,
    display: ViewDisplayState,
}

impl Default for ViewStateController {
    fn default() -> Self {
        Self::new()
    }
}

impl ViewStateController {
    /// Creates the reference opening pose and display toggles.
    #[must_use]
    pub fn new() -> Self {
        Self {
            camera: Camera::new(),
            display: ViewDisplayState::default(),
        }
    }

    /// Returns a copy safe for UI or automation adapters.
    #[must_use]
    pub fn state(&self) -> ViewState {
        let camera = self.camera.state();
        ViewState {
            yaw_deg: camera.yaw_degrees,
            pitch_deg: camera.pitch_degrees,
            distance_mm: camera.distance,
            center_mm: camera.center,
            display: self.display,
        }
    }

    /// Returns the renderer-neutral camera for interaction adapters.
    #[must_use]
    pub const fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Sets the viewport size used by frame and projection calculations.
    pub fn set_viewport_size(&mut self, width: u32, height: u32) -> Result<(), ViewStateError> {
        self.camera.set_viewport_size(width, height)?;
        Ok(())
    }

    /// Replaces camera values while preserving or replacing display toggles.
    pub fn set_state(&mut self, state: ViewState) -> Result<ViewState, ViewStateError> {
        self.camera.set_state(CameraState {
            yaw_degrees: state.yaw_deg,
            pitch_degrees: state.pitch_deg,
            distance: state.distance_mm,
            center: state.center_mm,
        })?;
        self.display = state.display;
        Ok(self.state())
    }

    /// Updates only the application display toggles.
    pub fn set_display(&mut self, display: ViewDisplayState) -> ViewState {
        self.display = display;
        self.state()
    }

    /// Restores the camera reset pose without changing display toggles.
    pub fn reset(&mut self) -> ViewState {
        self.camera.reset_view();
        self.state()
    }

    /// Applies one of the reference named camera presets.
    pub fn preset(&mut self, name: &str) -> Result<ViewState, ViewStateError> {
        match name.trim().to_ascii_lowercase().as_str() {
            "reset" => Ok(self.reset()),
            "top" => self.set_camera_angles(0.0, 89.0),
            "front" => self.set_camera_angles(0.0, 0.0),
            "side" => self.set_camera_angles(90.0, 0.0),
            _ => Err(ViewStateError::InvalidPreset),
        }
    }

    /// Frames all objects or the current selection through the camera.
    pub fn frame(
        &mut self,
        scene: &Scene,
        target: FrameTarget,
    ) -> Result<ViewState, ViewStateError> {
        let indices = match target {
            FrameTarget::All => (0..scene.objects().len()).collect::<Vec<_>>(),
            FrameTarget::Selected if scene.selection().is_empty() => {
                (0..scene.objects().len()).collect::<Vec<_>>()
            }
            FrameTarget::Selected => scene.selection().iter().copied().collect::<Vec<_>>(),
        };
        let mut bounds = Vec::new();
        for index in indices {
            let object = scene.object(index)?;
            // Framing is a presentation operation. Hidden objects, including
            // a hidden-only selection, must not pull the camera away from the
            // visible scene or turn an empty visible scene into a fake frame.
            if !object.is_visible() {
                continue;
            }
            if let Some((minimum, maximum)) = object.transformed_mesh()?.bounds() {
                bounds.push(Aabb::new(minimum, maximum)?);
            }
        }
        if !bounds.is_empty() {
            self.camera.frame_bounds(&bounds)?;
        }
        Ok(self.state())
    }

    fn set_camera_angles(
        &mut self,
        yaw_deg: f32,
        pitch_deg: f32,
    ) -> Result<ViewState, ViewStateError> {
        let current = self.camera.state();
        self.camera.set_state(CameraState {
            yaw_degrees: yaw_deg,
            pitch_degrees: pitch_deg,
            distance: current.distance,
            center: current.center,
        })?;
        Ok(self.state())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modeler3d_core::{PrimitiveKind, Scene};

    #[test]
    fn state_updates_and_presets_keep_camera_bounds() {
        let mut controller = ViewStateController::default();
        let initial = controller.state();
        // Yaw 0 since 2026-11-09 (floor axes match the 3D Builder reference);
        // the Python build opens at 45 degrees.
        assert_eq!(initial.yaw_deg, 0.0);
        assert_eq!(initial.pitch_deg, 35.0);
        assert_eq!(initial.distance_mm, 5.0);
        let updated = controller.set_state(ViewState {
            yaw_deg: 12.0,
            pitch_deg: 20.0,
            distance_mm: 40.0,
            center_mm: [1.0, 2.0, 3.0],
            display: ViewDisplayState {
                show_grid: false,
                wireframe: true,
                xray: true,
            },
        });
        assert!(updated.is_ok());
        assert_eq!(updated.ok().map(|state| state.distance_mm), Some(40.0));
        assert!(matches!(
            controller.set_state(ViewState {
                pitch_deg: 90.0,
                ..controller.state()
            }),
            Err(ViewStateError::Viewport(_))
        ));
        assert!(
            controller
                .preset("top")
                .is_ok_and(|state| state.pitch_deg == 89.0)
        );
        assert!(
            controller
                .preset("side")
                .is_ok_and(|state| state.yaw_deg == 90.0)
        );
        assert!(matches!(
            controller.preset("diagonal"),
            Err(ViewStateError::InvalidPreset)
        ));
    }

    #[test]
    fn frame_selected_and_empty_selection_fallback_use_world_bounds() {
        let mut scene = Scene::new();
        assert!(
            scene
                .add_primitive(PrimitiveKind::Cube, Some([20.0, 20.0, 20.0]), None)
                .is_ok()
        );
        assert!(scene.translate(0, [100.0, 10.0, 0.0]).is_ok());
        let mut controller = ViewStateController::default();
        assert!(controller.set_viewport_size(800, 600).is_ok());
        let framed = controller.frame(&scene, FrameTarget::Selected);
        assert!(framed.is_ok());
        assert!(framed.is_ok_and(|state| state.center_mm[0].abs() > 90.0));
        scene.clear_selection();
        let fallback = controller.frame(&scene, FrameTarget::Selected);
        assert!(fallback.is_ok());
        assert!(fallback.is_ok_and(|state| state.center_mm[0].abs() > 90.0));
    }

    #[test]
    fn empty_scene_frame_is_a_stable_noop() {
        let scene = Scene::new();
        let mut controller = ViewStateController::default();
        let before = controller.state();
        let after = controller.frame(&scene, FrameTarget::All);
        assert!(after.is_ok());
        assert_eq!(after.ok(), Some(before));
    }

    #[test]
    fn framing_ignores_hidden_objects_and_hidden_only_selection() {
        let mut scene = Scene::new();
        assert!(
            scene
                .add_primitive(PrimitiveKind::Cube, Some([4.0, 4.0, 4.0]), None)
                .is_ok()
        );
        assert!(scene.translate(0, [400.0, 0.0, 0.0]).is_ok());
        assert!(scene.set_visible(0, false).is_ok());
        scene.set_selection([0]);
        let mut controller = ViewStateController::default();
        assert!(controller.set_viewport_size(800, 600).is_ok());
        let before = controller.state();
        assert!(controller.frame(&scene, FrameTarget::Selected).is_ok());
        assert_eq!(controller.state(), before);
        assert!(controller.frame(&scene, FrameTarget::All).is_ok());
        assert_eq!(controller.state(), before);
    }
}
