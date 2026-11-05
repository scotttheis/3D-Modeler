//! Renderer-facing code, adaptive viewport policy, and measured wgpu controls.

mod adaptive;
mod camera;
mod cpu;
mod error;
mod floor;
mod gizmo;
mod math;
mod picking;
mod renderer;
mod scene;
mod snapping;
mod spike;

pub use adaptive::{
    AdaptiveInteractionController, AdaptiveInteractionPlan, InteractionState,
    InteractionStateReport, InteractionTier, ViewportReservationSet, ledger_for_profile,
    plan_interaction, plan_interaction_on_adapter,
};
pub use camera::{
    CAMERA_FOV_DEGREES, CAMERA_MAX_DISTANCE, CAMERA_MIN_DISTANCE, Camera, CameraState,
    VIEW_PICK_TOLERANCE_PIXELS,
};
pub use cpu::{
    CpuFrame, CpuRenderOptions, MAX_CPU_RENDER_DIMENSION, MAX_CPU_RENDER_TRIANGLES, render_cpu,
    render_cpu_with_options, write_ppm,
};
pub use error::ViewportError;
pub use floor::{
    ContactCue, ContactCueCache, DEFAULT_FLOOR_SIZE, FLOOR_GRID_TARGET_PIXELS,
    FLOOR_MAJOR_GRID_SPACING, FloorGeometry, FloorLine, FloorSurface, MAX_CONTACT_CUES,
    MAX_FLOOR_GRID_LINES, build_floor,
};
pub use gizmo::{
    GIZMO_TARGET_PIXELS, GizmoAxis, GizmoGeometry, GizmoHit, GizmoLine, GizmoMode, GizmoTriangle,
    MOVE_ARROW_HALF_LENGTH_FACTOR, MOVE_ARROW_OFFSET_FACTOR, ROTATE_RING_GAP_FACTOR,
    ROTATE_RING_SEGMENTS, X_AXIS_COLOR, Y_AXIS_COLOR, Z_AXIS_COLOR, build_gizmo,
    build_gizmo_with_bounds, hit_test, hit_test_with_bounds,
};
pub use math::{
    Aabb, Mat4, Ray, Vec3, add, cross, dot, normalize, perspective, ray_triangle, scale, subtract,
};
pub use picking::{MAX_EXACT_PICK_TRIANGLES, PickHit, pick_ray, pick_scene};
pub use renderer::{
    RendererAdapterInfo, RendererConfig, ViewportFrameReport, ViewportRenderOptions,
    ViewportRenderer, ViewportTheme,
};
pub use scene::{
    INTERACTIVE_FULL_RES_FACE_LIMIT, INTERACTIVE_LOD_MAX_FACE_TARGET,
    INTERACTIVE_LOD_MIN_FACE_TARGET, INTERACTIVE_LOD_RATIO, MAX_RENDER_GROUP_DEPTH,
    MAX_RENDER_OBJECTS, MeshGeometry, RenderObject, RenderScene, VIEWPORT_LOD_FACE_TARGET,
    VIEWPORT_LOD_FACE_THRESHOLD, interactive_viewport_face_target,
};
pub use snapping::{
    FLOOR_SNAP_TOLERANCE_MM, OBJECT_SNAP_TOLERANCE_MM, SNAP_NORMALIZED_PROJECTED_SPAN,
    SNAP_TOLERANCE_FIXED_MM, SNAP_TOLERANCE_FLOOR_FIXED, SNAP_TOLERANCE_VIEW_SCALED, SnapBounds,
    SnapCandidateKey, SnapError, SnapLandmark, SnapQuery, SnapSceneSnapshot, SnapState, SnapTarget,
    SnapTargetKind, SnapTranslationResult, SnapViewContext, select_snap_correction,
    snap_translation_result,
};
pub use spike::{RenderError, RenderMeasurement, RenderSpikeConfig, run_render_spike};
