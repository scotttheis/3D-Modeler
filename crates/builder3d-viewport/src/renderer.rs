use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

use builder3d_core::color_rgba8;
use builder3d_platform::{
    AllocationId, AllocationKind, CapabilityProfile, MemoryDomain, ResourceLedger,
};
use wgpu::util::DeviceExt;

use crate::{
    ViewportError,
    adaptive::{
        AdaptiveInteractionPlan, InteractionState, InteractionTier, ViewportReservationSet,
        ledger_for_profile, plan_interaction_on_adapter,
    },
    camera::Camera,
    cpu::{CpuRenderOptions, render_cpu_with_options, write_ppm},
    floor::{
        CONTACT_SHADOW_LAYERS, ContactCueCache, DEFAULT_FLOOR_SIZE, FLOOR_SURFACE_SUBDIVISIONS,
        FloorGeometry, build_floor, expand_shadow_polygon, shadow_light_direction_world,
    },
    gizmo::{GizmoMode, build_gizmo},
    lighting::ViewportLighting,
    math::{Mat4, subtract},
    scene::{MeshGeometry, RenderObject, RenderScene},
};

const MAX_RENDER_DIMENSION: u32 = 4_096;
const MAX_OVERLAY_VERTICES: usize = 32_768;
const OVERLAY_VERTEX_STRIDE: usize = 36;
const BYTES_PER_PIXEL: u64 = 4;
const COPY_BYTES_PER_ROW_ALIGNMENT: u64 = 256;
const MESH_UNIFORM_BYTES: usize = 240;

/// Configuration for the real offscreen wgpu viewport.
#[derive(Clone, Debug)]
pub struct RendererConfig {
    /// Target width.
    pub width: u32,
    /// Target height.
    pub height: u32,
    /// Profile adapter index, or automatic selection.
    pub adapter_index: Option<usize>,
    /// Optional capture written after each rendered frame.
    pub capture_path: Option<PathBuf>,
}

/// Viewport theme used by both the GPU renderer and the bounded CPU fallback.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ViewportTheme {
    /// Original dark Builder3D viewport colors.
    #[default]
    Dark,
    /// Original light Builder3D viewport colors.
    Light,
}

/// Presentation options owned by the desktop adapter and consumed by the
/// renderer without changing authoritative scene data.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewportRenderOptions {
    /// Whether the camera-relative build-plane grid is visible.
    pub grid_visible: bool,
    /// Whether opaque black mesh edges are overlaid with depth testing.
    pub wireframe: bool,
    /// Whether mesh colors use the translucent X-Ray pass.
    pub xray: bool,
    /// Durable IDs receiving the viewport selection tint.
    pub selected_object_ids: Vec<String>,
    /// RGBA tint mixed into selected authored colors.
    pub selected_tint: [f32; 4],
    /// Active shell theme.
    pub theme: ViewportTheme,
    /// User-adjustable screen-aligned lighting and floor contact shadows.
    pub lighting: ViewportLighting,
}

impl Default for ViewportRenderOptions {
    fn default() -> Self {
        Self {
            grid_visible: true,
            wireframe: false,
            xray: false,
            selected_object_ids: Vec::new(),
            selected_tint: [0.0, 0.55, 1.0, 1.0],
            theme: ViewportTheme::Dark,
            lighting: ViewportLighting::default(),
        }
    }
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            width: 512,
            height: 512,
            adapter_index: None,
            capture_path: None,
        }
    }
}

/// Adapter identity used in viewport evidence.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct RendererAdapterInfo {
    /// API name.
    pub name: String,
    /// Backend.
    pub backend: String,
    /// Device class.
    pub device_type: String,
    /// Driver name.
    pub driver: String,
    /// Driver detail.
    pub driver_info: String,
}

/// Measured result of one viewport frame.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ViewportFrameReport {
    /// Adapter identity.
    pub adapter: RendererAdapterInfo,
    /// Selected adaptive plan.
    pub plan: AdaptiveInteractionPlan,
    /// Wall-clock CPU submission/completion duration.
    pub frame_time_ms: f64,
    /// Number of triangles submitted to the GPU path.
    pub submitted_triangles: u64,
    /// Number of overlay vertices submitted.
    pub overlay_vertices: u64,
    /// Whether CPU fallback rendered this frame.
    pub cpu_fallback: bool,
    /// CPU fallback triangles when applicable.
    pub cpu_triangles: u64,
    /// CPU fallback shaded pixels when applicable.
    pub cpu_shaded_pixels: u64,
    /// Current central resource ledger snapshot.
    pub ledger: builder3d_platform::ResourceLedgerSnapshot,
    /// Capture path, if one was requested.
    pub capture_path: Option<String>,
}

struct GpuGeometry {
    vertex: wgpu::Buffer,
    batches: Vec<GpuMaterialBatch>,
    ledger_id: AllocationId,
    face_count: u64,
    geometry_identity: usize,
    face_color_identity: u64,
}

struct GpuMaterialBatch {
    authored_color: Option<[u8; 4]>,
    index: wgpu::Buffer,
    index_count: u32,
    uniform: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

struct FaceIndexBatch {
    authored_color: Option<[u8; 4]>,
    indices: Vec<u32>,
}

/// Samples per pixel for the mesh pass. Four-sample MSAA removes stair-stepped
/// silhouettes; the resolved single-sample colour target is what the shell
/// presents, reads back, and the CPU fallback writes.
const VIEWPORT_SAMPLE_COUNT: u32 = 4;

struct RenderTargets {
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    multisample_texture: wgpu::Texture,
    multisample_view: wgpu::TextureView,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
}

/// A real offscreen wgpu scene renderer with CPU fallback and central budgets.
pub struct ViewportRenderer {
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
    profile: CapabilityProfile,
    adapter_info: RendererAdapterInfo,
    adapter_index: usize,
    max_texture_dimension_2d: u32,
    width: u32,
    height: u32,
    clear_color: [f64; 4],
    color_texture: wgpu::Texture,
    color_view: wgpu::TextureView,
    _multisample_texture: wgpu::Texture,
    multisample_view: wgpu::TextureView,
    _depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    mesh_pipeline: wgpu::RenderPipeline,
    mesh_xray_pipeline: wgpu::RenderPipeline,
    overlay_line_pipeline: wgpu::RenderPipeline,
    overlay_triangle_pipeline: wgpu::RenderPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    ledger: ResourceLedger,
    render_target_ledger_id: AllocationId,
    gpu_objects: HashMap<String, GpuGeometry>,
    contact_cache: ContactCueCache,
    scene_reservations: Option<(u64, ViewportReservationSet)>,
}

impl ViewportRenderer {
    /// Opens the selected adapter and creates bounded offscreen attachments.
    pub async fn new(
        profile: &CapabilityProfile,
        config: &RendererConfig,
    ) -> Result<Self, ViewportError> {
        validate_config(config)?;
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapters = instance.enumerate_adapters(wgpu::Backends::all()).await;
        let (adapter_index, adapter) = select_adapter(adapters, profile, config.adapter_index)?;
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("builder3d-r5-viewport-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
            ..Default::default()
        };
        let (device, queue) = adapter
            .request_device(&descriptor)
            .await
            .map_err(|error| ViewportError::Device(error.to_string()))?;
        Self::from_device(
            profile,
            config,
            instance,
            adapter,
            device,
            queue,
            adapter_index,
        )
    }

    /// Adapts the R5 renderer to an existing native presentation device.
    ///
    /// The adapter, device, and queue must belong to `instance` and be the same
    /// device used by the native UI renderer. This path is intended for eframe
    /// integration and avoids opening a second device or window.
    pub fn new_on_shared_device(
        profile: &CapabilityProfile,
        config: &RendererConfig,
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Self, ViewportError> {
        validate_config(config)?;
        let info = adapter.get_info();
        let adapter_index = profile_adapter_index(profile, info.vendor, info.device, info.backend)
            .ok_or_else(|| {
                ViewportError::Unavailable(format!(
                    "eframe adapter {} was not present in the discovered capability profile",
                    info.name
                ))
            })?;
        Self::from_device(
            profile,
            config,
            instance.clone(),
            adapter.clone(),
            device.clone(),
            queue.clone(),
            adapter_index,
        )
    }

    fn from_device(
        profile: &CapabilityProfile,
        config: &RendererConfig,
        instance: wgpu::Instance,
        adapter: wgpu::Adapter,
        device: wgpu::Device,
        queue: wgpu::Queue,
        adapter_index: usize,
    ) -> Result<Self, ViewportError> {
        validate_config(config)?;
        let info = adapter.get_info();
        let max_texture_dimension_2d = adapter.limits().max_texture_dimension_2d;
        if config.width > max_texture_dimension_2d || config.height > max_texture_dimension_2d {
            return Err(ViewportError::InvalidInput(format!(
                "renderer dimensions exceed the device limit of {max_texture_dimension_2d} pixels"
            )));
        }
        let mut ledger = ledger_for_profile(profile);
        let target_bytes = render_target_size_bytes(config.width, config.height)?;
        let render_target_ledger_id = ledger.reserve(
            AllocationKind::RenderTarget,
            MemoryDomain::Adapter(adapter_index),
            target_bytes,
        )?;
        let targets = create_render_targets(&device, config.width, config.height);
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("builder3d-r5-uniform-layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let mesh_pipeline = create_mesh_pipeline(&device, &uniform_layout, false);
        let mesh_xray_pipeline = create_mesh_pipeline(&device, &uniform_layout, true);
        let overlay_line_pipeline = create_overlay_pipeline(
            &device,
            &uniform_layout,
            wgpu::PrimitiveTopology::LineList,
            "builder3d-r5-overlay-lines",
        );
        let overlay_triangle_pipeline = create_overlay_pipeline(
            &device,
            &uniform_layout,
            wgpu::PrimitiveTopology::TriangleList,
            "builder3d-r5-overlay-triangles",
        );
        Ok(Self {
            instance,
            device,
            queue,
            profile: profile.clone(),
            adapter_info: RendererAdapterInfo {
                name: info.name,
                backend: format!("{:?}", info.backend),
                device_type: format!("{:?}", info.device_type),
                driver: info.driver,
                driver_info: info.driver_info,
            },
            adapter_index,
            max_texture_dimension_2d,
            width: config.width,
            height: config.height,
            clear_color: [0.015, 0.02, 0.035, 1.0],
            color_texture: targets.color_texture,
            color_view: targets.color_view,
            _multisample_texture: targets.multisample_texture,
            multisample_view: targets.multisample_view,
            _depth_texture: targets.depth_texture,
            depth_view: targets.depth_view,
            mesh_pipeline,
            mesh_xray_pipeline,
            overlay_line_pipeline,
            overlay_triangle_pipeline,
            uniform_layout,
            ledger,
            render_target_ledger_id,
            gpu_objects: HashMap::new(),
            contact_cache: ContactCueCache::new(),
            scene_reservations: None,
        })
    }

    /// Returns the color texture view for presentation in egui on the same device.
    #[must_use]
    pub const fn color_view(&self) -> &wgpu::TextureView {
        &self.color_view
    }

    /// Returns the renderer's physical pixel dimensions.
    #[must_use]
    pub const fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the graphics adapter selected by eframe or the standalone renderer.
    #[must_use]
    pub const fn adapter_info(&self) -> &RendererAdapterInfo {
        &self.adapter_info
    }

    /// Resizes both offscreen attachments while preserving the old target on budget failure.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<(), ViewportError> {
        let config = RendererConfig {
            width,
            height,
            adapter_index: Some(self.adapter_index),
            capture_path: None,
        };
        validate_config(&config)?;
        if width > self.max_texture_dimension_2d || height > self.max_texture_dimension_2d {
            return Err(ViewportError::InvalidInput(format!(
                "renderer dimensions exceed the device limit of {} pixels",
                self.max_texture_dimension_2d
            )));
        }
        if (self.width, self.height) == (width, height) {
            return Ok(());
        }
        let target_bytes = render_target_size_bytes(width, height)?;
        let next_ledger_id = self.ledger.reserve(
            AllocationKind::RenderTarget,
            MemoryDomain::Adapter(self.adapter_index),
            target_bytes,
        )?;
        let targets = create_render_targets(&self.device, width, height);
        if let Err(error) = self.ledger.release(self.render_target_ledger_id) {
            let _ = self.ledger.release(next_ledger_id);
            return Err(error.into());
        }
        self.color_texture = targets.color_texture;
        self.color_view = targets.color_view;
        self._multisample_texture = targets.multisample_texture;
        self.multisample_view = targets.multisample_view;
        self._depth_texture = targets.depth_texture;
        self.depth_view = targets.depth_view;
        self.render_target_ledger_id = next_ledger_id;
        self.width = width;
        self.height = height;
        Ok(())
    }

    /// Sets the sRGB-encoded clear color used by the native viewport.
    pub fn set_clear_color(&mut self, color: [f32; 4]) -> Result<(), ViewportError> {
        self.clear_color = normalize_clear_color(color)?;
        Ok(())
    }

    /// Returns the live central ledger snapshot.
    #[must_use]
    pub fn ledger_snapshot(&self) -> builder3d_platform::ResourceLedgerSnapshot {
        self.ledger.snapshot()
    }

    /// Returns the selected adapter identity.
    #[must_use]
    pub const fn adapter_index(&self) -> usize {
        self.adapter_index
    }

    /// Renders one scene frame, selecting full/LOD/CPU/hybrid work centrally.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        scene: &RenderScene,
        camera: &Camera,
        interaction: InteractionState,
        scene_revision: u64,
        gizmo_center: Option<[f32; 3]>,
        gizmo_mode: GizmoMode,
        capture_path: Option<PathBuf>,
    ) -> Result<ViewportFrameReport, ViewportError> {
        let options = ViewportRenderOptions::default();
        self.render_internal(
            scene,
            camera,
            interaction,
            scene_revision,
            gizmo_center,
            gizmo_mode,
            capture_path,
            None,
            &options,
        )
    }

    /// Renders one frame with explicit display, selection, and theme state.
    #[allow(clippy::too_many_arguments)]
    pub fn render_with_options(
        &mut self,
        scene: &RenderScene,
        camera: &Camera,
        interaction: InteractionState,
        scene_revision: u64,
        gizmo_center: Option<[f32; 3]>,
        gizmo_mode: GizmoMode,
        capture_path: Option<PathBuf>,
        options: &ViewportRenderOptions,
    ) -> Result<ViewportFrameReport, ViewportError> {
        self.render_internal(
            scene,
            camera,
            interaction,
            scene_revision,
            gizmo_center,
            gizmo_mode,
            capture_path,
            None,
            options,
        )
    }

    /// Renders a temporary world-space translation without changing the scene.
    #[allow(clippy::too_many_arguments)]
    pub fn render_with_translation_preview(
        &mut self,
        scene: &RenderScene,
        camera: &Camera,
        interaction: InteractionState,
        scene_revision: u64,
        gizmo_center: Option<[f32; 3]>,
        gizmo_mode: GizmoMode,
        capture_path: Option<PathBuf>,
        preview_translation: Option<(&str, [f32; 3])>,
    ) -> Result<ViewportFrameReport, ViewportError> {
        let options = ViewportRenderOptions::default();
        self.render_internal(
            scene,
            camera,
            interaction,
            scene_revision,
            gizmo_center,
            gizmo_mode,
            capture_path,
            preview_translation,
            &options,
        )
    }

    /// Renders a temporary world-space translation with explicit presentation
    /// options while leaving the authoritative scene untouched.
    #[allow(clippy::too_many_arguments)]
    pub fn render_with_translation_preview_options(
        &mut self,
        scene: &RenderScene,
        camera: &Camera,
        interaction: InteractionState,
        scene_revision: u64,
        gizmo_center: Option<[f32; 3]>,
        gizmo_mode: GizmoMode,
        capture_path: Option<PathBuf>,
        preview_translation: Option<(&str, [f32; 3])>,
        options: &ViewportRenderOptions,
    ) -> Result<ViewportFrameReport, ViewportError> {
        self.render_internal(
            scene,
            camera,
            interaction,
            scene_revision,
            gizmo_center,
            gizmo_mode,
            capture_path,
            preview_translation,
            options,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn render_internal(
        &mut self,
        scene: &RenderScene,
        camera: &Camera,
        interaction: InteractionState,
        scene_revision: u64,
        gizmo_center: Option<[f32; 3]>,
        gizmo_mode: GizmoMode,
        capture_path: Option<PathBuf>,
        preview_translation: Option<(&str, [f32; 3])>,
        options: &ViewportRenderOptions,
    ) -> Result<ViewportFrameReport, ViewportError> {
        let started = Instant::now();
        let scene_bytes = scene
            .authoritative_bytes()
            .saturating_add(scene.interaction_bytes());
        if self
            .scene_reservations
            .as_ref()
            .is_none_or(|(bytes, _)| *bytes != scene_bytes)
        {
            if let Some((_, reservation)) = self.scene_reservations.take() {
                reservation.release(&mut self.ledger)?;
            }
            let reservation = ViewportReservationSet::reserve(&mut self.ledger, scene)?;
            self.scene_reservations = Some((scene_bytes, reservation));
        }
        let plan = plan_interaction_on_adapter(
            &self.profile,
            scene,
            interaction,
            &self.ledger,
            None,
            self.adapter_index,
        );
        let use_interaction = matches!(
            plan.tier,
            InteractionTier::InteractionLod
                | InteractionTier::Hybrid
                | InteractionTier::CpuFallback
        );
        let scene_radius = scene
            .world_bounds()?
            .map(|bounds| {
                let extent = [
                    bounds.maximum[0] - bounds.minimum[0],
                    bounds.maximum[1] - bounds.minimum[1],
                    bounds.maximum[2] - bounds.minimum[2],
                ];
                (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt() * 0.5
            })
            .unwrap_or(1.0);
        if matches!(plan.tier, InteractionTier::CpuFallback) {
            return self.render_cpu_report(
                scene,
                camera,
                &plan,
                use_interaction,
                started,
                capture_path,
                options,
            );
        }
        let mut submitted_triangles = 0_u64;
        let mut upload_failed = false;
        let object_ids = scene
            .objects()
            .iter()
            .map(|object| object.object_id().to_owned())
            .collect::<Vec<_>>();
        for object in scene.objects() {
            let geometry = if use_interaction {
                object.interaction_geometry()
            } else {
                object.full_geometry()
            };
            submitted_triangles = submitted_triangles.saturating_add(geometry.face_count() as u64);
            if self.ensure_gpu_object(object, geometry).is_err() {
                upload_failed = true;
                break;
            }
        }
        if upload_failed {
            return self.render_cpu_report(
                scene,
                camera,
                &plan,
                use_interaction,
                started,
                capture_path,
                options,
            );
        }
        let stale_ids = self
            .gpu_objects
            .keys()
            .filter(|id| !object_ids.iter().any(|candidate| candidate == *id))
            .cloned()
            .collect::<Vec<_>>();
        for stale_id in stale_ids {
            if let Some(stale) = self.gpu_objects.remove(&stale_id) {
                let _ = self.ledger.release(stale.ledger_id);
            }
        }
        let floor = build_floor(camera, DEFAULT_FLOOR_SIZE, options.grid_visible, true)?;
        if interaction == InteractionState::Resting {
            self.contact_cache.settle(scene, scene_revision)?;
        }
        let cues = self.contact_cache.cues_for_frame(
            camera,
            scene,
            interaction == InteractionState::Active,
            scene_revision,
        )?;
        let overlay = build_overlay_vertices(
            &floor,
            &cues,
            camera,
            gizmo_center,
            gizmo_mode,
            scene_radius,
            self.width,
            self.height,
            scene,
            options,
        )?;
        let overlay_reservation = if overlay.total_vertices() > 0 {
            let bytes = overlay.total_bytes()?;
            self.ledger
                .reserve(
                    AllocationKind::Staging,
                    MemoryDomain::Adapter(self.adapter_index),
                    bytes,
                )
                .ok()
        } else {
            None
        };
        let overlay_enabled = overlay_reservation.is_some() || overlay.total_vertices() == 0;
        let overlay = if overlay_enabled {
            overlay
        } else {
            OverlayBuffers {
                lines: Vec::new(),
                triangles: Vec::new(),
            }
        };
        let view_matrix = camera.view_matrix()?;
        let view_projection = camera.view_projection(scene_radius)?;
        let light_direction = options.lighting.direction_in_view(view_matrix);
        let material_parameters = options.lighting.shader_parameters();
        let uniform = make_uniform(view_projection, [1.0, 1.0, 1.0, 1.0]);
        let uniform_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("builder3d-r5-overlay-uniform"),
                contents: &uniform,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
        let overlay_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("builder3d-r5-overlay-bind-group"),
            layout: &self.uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let line_buffer = overlay.line_bytes().map(|bytes| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("builder3d-r5-floor-line-buffer"),
                    contents: &bytes,
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
        let triangle_buffer = overlay.triangle_bytes().map(|bytes| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("builder3d-r5-overlay-triangle-buffer"),
                    contents: &bytes,
                    usage: wgpu::BufferUsages::VERTEX,
                })
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("builder3d-r5-frame-encoder"),
            });
        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view: &self.multisample_view,
            depth_slice: None,
            resolve_target: Some(&self.color_view),
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: self.clear_color[0],
                    g: self.clear_color[1],
                    b: self.clear_color[2],
                    a: self.clear_color[3],
                }),
                store: wgpu::StoreOp::Store,
            },
        })];
        let depth_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("builder3d-r5-mesh-pass"),
                color_attachments: &color_attachments,
                depth_stencil_attachment: depth_attachment,
                ..Default::default()
            });
            pass.set_pipeline(if options.xray {
                &self.mesh_xray_pipeline
            } else {
                &self.mesh_pipeline
            });
            for object in scene.objects() {
                let Some(gpu) = self.gpu_objects.get(object.object_id()) else {
                    continue;
                };
                let mut transform = object.transform();
                if let Some((object_id, position)) = preview_translation
                    && object.object_id() == object_id
                {
                    let origin = transform.transform_point([0.0; 3])?;
                    transform = transform.translated_by(subtract(position, origin))?;
                }
                let model_view = view_matrix.multiply(transform);
                let normal_transform = model_view.inverse()?.transpose();
                let selected = options
                    .selected_object_ids
                    .iter()
                    .any(|id| id == object.object_id());
                pass.set_vertex_buffer(0, gpu.vertex.slice(..));
                for batch in &gpu.batches {
                    let authored_color = batch
                        .authored_color
                        .map(color_from_rgba8)
                        .unwrap_or_else(|| object.color());
                    let color = display_color(authored_color, selected, options);
                    let bytes = make_mesh_uniform(
                        view_projection.multiply(transform),
                        model_view,
                        normal_transform,
                        color,
                        light_direction,
                        material_parameters,
                    );
                    self.queue.write_buffer(&batch.uniform, 0, &bytes);
                    pass.set_bind_group(0, &batch.bind_group, &[]);
                    pass.set_index_buffer(batch.index.slice(..), wgpu::IndexFormat::Uint32);
                    pass.draw_indexed(0..batch.index_count, 0, 0..1);
                }
            }
            if let Some(triangle_buffer) = &triangle_buffer {
                pass.set_pipeline(&self.overlay_triangle_pipeline);
                pass.set_bind_group(0, &overlay_bind_group, &[]);
                pass.set_vertex_buffer(0, triangle_buffer.slice(..));
                pass.draw(0..overlay.triangle_vertex_count(), 0..1);
            }
            // Draw surfaces first.  Grid, contact, wireframe, and gizmo lines
            // are depth-tested over the floor so the opaque floor cannot hide
            // them when both primitives occupy the build plane.
            if let Some(line_buffer) = &line_buffer {
                pass.set_pipeline(&self.overlay_line_pipeline);
                pass.set_bind_group(0, &overlay_bind_group, &[]);
                pass.set_vertex_buffer(0, line_buffer.slice(..));
                pass.draw(0..overlay.line_vertex_count(), 0..1);
            }
        }
        self.queue.submit([encoder.finish()]);
        if let Some(reservation) = overlay_reservation {
            let _ = self.ledger.release(reservation);
        }
        let capture_path = if let Some(path) = capture_path {
            self.capture(&path)?;
            Some(path.display().to_string())
        } else {
            None
        };
        let elapsed = started.elapsed().as_secs_f64() * 1_000.0;
        Ok(ViewportFrameReport {
            adapter: self.adapter_info.clone(),
            plan,
            frame_time_ms: elapsed,
            submitted_triangles,
            overlay_vertices: overlay.total_vertices() as u64,
            cpu_fallback: false,
            cpu_triangles: 0,
            cpu_shaded_pixels: 0,
            ledger: self.ledger.snapshot(),
            capture_path,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn render_cpu_report(
        &mut self,
        scene: &RenderScene,
        camera: &Camera,
        plan: &AdaptiveInteractionPlan,
        interaction: bool,
        started: Instant,
        capture_path: Option<PathBuf>,
        options: &ViewportRenderOptions,
    ) -> Result<ViewportFrameReport, ViewportError> {
        let frame = render_cpu_with_options(
            scene,
            camera,
            self.width,
            self.height,
            interaction,
            &CpuRenderOptions {
                clear_color: cpu_clear_color(options.theme),
                wireframe: options.wireframe,
                xray: options.xray,
                selected_object_ids: options.selected_object_ids.clone(),
                selected_tint: options.selected_tint,
                lighting: options.lighting,
                floor_enabled: true,
                grid_visible: options.grid_visible,
                floor_color: cpu_floor_color(options.theme),
                grid_major_color: cpu_grid_major_color(options.theme),
                grid_minor_color: cpu_grid_minor_color(options.theme),
                contact_shadow_color: cpu_contact_shadow_color(options.theme),
            },
        )?;
        let capture_path = if let Some(path) = capture_path {
            write_ppm(&frame, &path)?;
            Some(path.display().to_string())
        } else {
            None
        };
        Ok(ViewportFrameReport {
            adapter: self.adapter_info.clone(),
            plan: plan.clone(),
            frame_time_ms: started.elapsed().as_secs_f64() * 1_000.0,
            submitted_triangles: frame.triangles,
            overlay_vertices: 0,
            cpu_fallback: true,
            cpu_triangles: frame.triangles,
            cpu_shaded_pixels: frame.shaded_pixels,
            ledger: self.ledger.snapshot(),
            capture_path,
        })
    }

    fn ensure_gpu_object(
        &mut self,
        object: &RenderObject,
        geometry: &MeshGeometry,
    ) -> Result<(), ViewportError> {
        let needs_upload = self
            .gpu_objects
            .get(object.object_id())
            .is_none_or(|cached| {
                cached.face_count != geometry.face_count() as u64
                    || cached.geometry_identity != geometry.geometry_identity()
                    || cached.face_color_identity != object.face_color_identity()
            });
        if !needs_upload {
            return Ok(());
        }
        let old = self.gpu_objects.remove(object.object_id());
        let result = self.create_gpu_object(object, geometry);
        match result {
            Ok(gpu) => {
                if let Some(old) = old {
                    let _ = self.ledger.release(old.ledger_id);
                }
                self.gpu_objects.insert(object.object_id().to_owned(), gpu);
                Ok(())
            }
            Err(error) => {
                if let Some(old) = old {
                    self.gpu_objects.insert(object.object_id().to_owned(), old);
                }
                Err(error)
            }
        }
    }

    fn create_gpu_object(
        &mut self,
        object: &RenderObject,
        geometry: &MeshGeometry,
    ) -> Result<GpuGeometry, ViewportError> {
        let index_batches = face_index_batches(object, geometry)?;
        if index_batches.is_empty() {
            return Err(ViewportError::InvalidInput(String::from(
                "render geometry did not produce a material batch",
            )));
        }
        let index_counts = index_batches
            .iter()
            .map(|batch| {
                u32::try_from(batch.indices.len()).map_err(|_| {
                    ViewportError::CapacityExceeded(String::from(
                        "GPU material batch index count exceeds u32",
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let batch_uniform_bytes = u64::try_from(index_batches.len())
            .map_err(|_| {
                ViewportError::CapacityExceeded(String::from(
                    "GPU material batch count exceeds u64",
                ))
            })?
            .checked_mul(MESH_UNIFORM_BYTES as u64)
            .ok_or_else(|| {
                ViewportError::CapacityExceeded(String::from(
                    "GPU material uniform allocation overflow",
                ))
            })?;
        let bytes = geometry
            .byte_size()
            .checked_add(batch_uniform_bytes)
            .ok_or_else(|| {
                ViewportError::CapacityExceeded(String::from("GPU geometry allocation overflow"))
            })?;
        let ledger_id = self.ledger.reserve(
            AllocationKind::Cache,
            MemoryDomain::Adapter(self.adapter_index),
            bytes,
        )?;
        let vertex_bytes = geometry.vertex_bytes();
        let vertex = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("builder3d-r5-mesh-vertices"),
                contents: &vertex_bytes,
                usage: wgpu::BufferUsages::VERTEX,
            });
        let batches = index_batches
            .into_iter()
            .zip(index_counts)
            .map(|(batch, index_count)| {
                let index_bytes = encode_indices(&batch.indices);
                let index = self
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("builder3d-r7-material-indices"),
                        contents: &index_bytes,
                        usage: wgpu::BufferUsages::INDEX,
                    });
                let uniform = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("builder3d-r7-material-uniform"),
                    size: MESH_UNIFORM_BYTES as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("builder3d-r7-material-bind-group"),
                    layout: &self.uniform_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    }],
                });
                GpuMaterialBatch {
                    authored_color: batch.authored_color,
                    index,
                    index_count,
                    uniform,
                    bind_group,
                }
            })
            .collect();
        Ok(GpuGeometry {
            vertex,
            batches,
            ledger_id,
            face_count: geometry.face_count() as u64,
            geometry_identity: geometry.geometry_identity(),
            face_color_identity: object.face_color_identity(),
        })
    }

    fn capture(&self, path: &PathBuf) -> Result<(), ViewportError> {
        let padded = padded_bytes_per_row(self.width)?;
        let bytes = padded.checked_mul(u64::from(self.height)).ok_or_else(|| {
            ViewportError::CapacityExceeded(String::from("capture buffer overflow"))
        })?;
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("builder3d-r5-readback"),
            size: bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("builder3d-r5-readback-encoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.color_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(u32::try_from(padded).map_err(|_| {
                        ViewportError::CapacityExceeded(String::from(
                            "capture row pitch exceeds u32",
                        ))
                    })?),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        wait_for_gpu(&self.instance);
        let slice = buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result.map_err(|error| error.to_string()));
        });
        let started = Instant::now();
        loop {
            match receiver.try_recv() {
                Ok(result) => result.map_err(ViewportError::Graphics)?,
                Err(mpsc::TryRecvError::Empty) => {
                    if started.elapsed() > Duration::from_secs(30) {
                        return Err(ViewportError::ReadbackTimeout);
                    }
                    wait_for_gpu(&self.instance);
                    std::thread::yield_now();
                    continue;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(ViewportError::ReadbackTimeout);
                }
            }
            break;
        }
        let mapped = buffer
            .slice(..)
            .get_mapped_range()
            .map_err(|error| ViewportError::Graphics(error.to_string()))?;
        let row_stride = usize::try_from(padded).map_err(|_| {
            ViewportError::CapacityExceeded(String::from("capture row stride too large"))
        })?;
        let width = usize::try_from(self.width)
            .map_err(|_| ViewportError::InvalidInput(String::from("capture width is too large")))?;
        let height = usize::try_from(self.height).map_err(|_| {
            ViewportError::InvalidInput(String::from("capture height is too large"))
        })?;
        let mut output = Vec::new();
        output.extend_from_slice(format!("P6\n{} {}\n255\n", self.width, self.height).as_bytes());
        output.reserve(width.saturating_mul(height).saturating_mul(3));
        for row in 0..height {
            let start = row.saturating_mul(row_stride);
            let end = start.saturating_add(width.saturating_mul(4));
            let source = mapped
                .get(start..end)
                .ok_or_else(|| ViewportError::Graphics(String::from("capture row is truncated")))?;
            for pixel in source.as_chunks::<4>().0 {
                output.extend_from_slice(&pixel[..3]);
            }
        }
        drop(mapped);
        buffer.unmap();
        std::fs::write(path, output).map_err(|error| ViewportError::io(path, error))
    }
}

impl Drop for ViewportRenderer {
    fn drop(&mut self) {
        if let Some((_, reservation)) = self.scene_reservations.take() {
            let _ = reservation.release(&mut self.ledger);
        }
        for (_, object) in self.gpu_objects.drain() {
            let _ = self.ledger.release(object.ledger_id);
        }
        let _ = self.ledger.release(self.render_target_ledger_id);
    }
}

fn validate_config(config: &RendererConfig) -> Result<(), ViewportError> {
    if config.width == 0
        || config.height == 0
        || config.width > MAX_RENDER_DIMENSION
        || config.height > MAX_RENDER_DIMENSION
    {
        return Err(ViewportError::InvalidInput(String::from(
            "renderer dimensions must be between 1 and 4096",
        )));
    }
    Ok(())
}

fn render_target_size_bytes(width: u32, height: u32) -> Result<u64, ViewportError> {
    u64::from(width)
        .checked_mul(u64::from(height))
        // One resolved colour target, plus multisampled colour and depth.
        .and_then(|pixels| {
            pixels.checked_mul(BYTES_PER_PIXEL * (1 + 2 * u64::from(VIEWPORT_SAMPLE_COUNT)))
        })
        .ok_or_else(|| ViewportError::CapacityExceeded(String::from("render target size overflow")))
}

fn normalize_clear_color(color: [f32; 4]) -> Result<[f64; 4], ViewportError> {
    if !color
        .iter()
        .all(|value| value.is_finite() && (0.0..=1.0).contains(value))
    {
        return Err(ViewportError::InvalidInput(String::from(
            "clear color channels must be finite values between zero and one",
        )));
    }
    Ok(color.map(f64::from))
}

fn create_render_targets(device: &wgpu::Device, width: u32, height: u32) -> RenderTargets {
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let color_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("builder3d-r5-color"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let multisample_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("builder3d-r5-color-msaa"),
        size,
        mip_level_count: 1,
        sample_count: VIEWPORT_SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let multisample_view = multisample_texture.create_view(&wgpu::TextureViewDescriptor::default());
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("builder3d-r5-depth"),
        size,
        mip_level_count: 1,
        sample_count: VIEWPORT_SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth24Plus,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
    RenderTargets {
        color_texture,
        color_view,
        multisample_texture,
        multisample_view,
        depth_texture,
        depth_view,
    }
}

fn select_adapter(
    adapters: Vec<wgpu::Adapter>,
    profile: &CapabilityProfile,
    requested: Option<usize>,
) -> Result<(usize, wgpu::Adapter), ViewportError> {
    if let Some(index) = requested {
        let profile_adapter = profile
            .adapters
            .get(index)
            .ok_or(ViewportError::NoAdapter)?;
        let adapter = adapters
            .iter()
            .find(|adapter| {
                let info = adapter.get_info();
                info.vendor == profile_adapter.vendor_id
                    && info.device == profile_adapter.device_id
                    && backend_matches(profile_adapter.backend, info.backend)
            })
            .cloned()
            .ok_or(ViewportError::NoAdapter)?;
        return Ok((index, adapter));
    }
    let selected = profile
        .adapters
        .iter()
        .filter(|profile_adapter| profile_adapter.is_usable)
        .max_by_key(|profile_adapter| {
            u64::from(profile_adapter.display.has_output) * 1_000_000_000
                + profile_adapter.memory.available_bytes
        });
    if let Some(profile_adapter) = selected {
        let adapter = adapters
            .iter()
            .find(|adapter| {
                let info = adapter.get_info();
                info.vendor == profile_adapter.vendor_id
                    && info.device == profile_adapter.device_id
                    && backend_matches(profile_adapter.backend, info.backend)
            })
            .cloned();
        if let Some(adapter) = adapter {
            return Ok((profile_adapter.index, adapter));
        }
    }
    adapters
        .into_iter()
        .enumerate()
        .find(|(_, adapter)| !matches!(adapter.get_info().device_type, wgpu::DeviceType::Cpu))
        .ok_or(ViewportError::NoAdapter)
}

fn backend_matches(kind: builder3d_platform::BackendKind, backend: wgpu::Backend) -> bool {
    matches!(
        (kind, backend),
        (builder3d_platform::BackendKind::Dx12, wgpu::Backend::Dx12)
            | (
                builder3d_platform::BackendKind::Vulkan,
                wgpu::Backend::Vulkan
            )
            | (builder3d_platform::BackendKind::Gl, wgpu::Backend::Gl)
            | (builder3d_platform::BackendKind::Metal, wgpu::Backend::Metal)
            | (
                builder3d_platform::BackendKind::WebGpu,
                wgpu::Backend::BrowserWebGpu
            )
            | (builder3d_platform::BackendKind::Noop, wgpu::Backend::Noop)
            | (builder3d_platform::BackendKind::Other, _)
    )
}

fn profile_adapter_index(
    profile: &CapabilityProfile,
    vendor_id: u32,
    device_id: u32,
    backend: wgpu::Backend,
) -> Option<usize> {
    profile
        .adapters
        .iter()
        .find(|candidate| {
            candidate.vendor_id == vendor_id
                && candidate.device_id == device_id
                && backend_matches(candidate.backend, backend)
        })
        .map(|candidate| candidate.index)
}

fn make_uniform(matrix: Mat4, color: [f32; 4]) -> Vec<u8> {
    let mut bytes = matrix.column_major_bytes();
    for value in color {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn make_mesh_uniform(
    view_projection: Mat4,
    model_view: Mat4,
    normal_transform: Mat4,
    color: [f32; 4],
    light_direction: [f32; 3],
    material_parameters: [f32; 4],
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(MESH_UNIFORM_BYTES);
    bytes.extend_from_slice(&view_projection.column_major_bytes());
    bytes.extend_from_slice(&model_view.column_major_bytes());
    bytes.extend_from_slice(&normal_transform.column_major_bytes());
    for value in color
        .into_iter()
        .chain(light_direction)
        .chain([0.0])
        .chain(material_parameters)
    {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn padded_bytes_per_row(width: u32) -> Result<u64, ViewportError> {
    let row_bytes = u64::from(width)
        .checked_mul(BYTES_PER_PIXEL)
        .ok_or_else(|| ViewportError::CapacityExceeded(String::from("row size overflow")))?;
    let padding = COPY_BYTES_PER_ROW_ALIGNMENT - row_bytes % COPY_BYTES_PER_ROW_ALIGNMENT;
    Ok(if padding == COPY_BYTES_PER_ROW_ALIGNMENT {
        row_bytes
    } else {
        row_bytes + padding
    })
}

fn wait_for_gpu(instance: &wgpu::Instance) {
    let _ = instance.poll_all(true);
}

fn create_mesh_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    xray: bool,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("builder3d-r5-mesh-shader"),
        source: wgpu::ShaderSource::Wgsl(MESH_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("builder3d-r5-mesh-pipeline-layout"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    let targets = [Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Rgba8Unorm,
        blend: xray.then_some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(if xray {
            "builder3d-r5-mesh-xray-pipeline"
        } else {
            "builder3d-r5-mesh-pipeline"
        }),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 24,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 12,
                        shader_location: 1,
                    },
                ],
            })],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: Some(!xray),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: VIEWPORT_SAMPLE_COUNT,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment_main"),
            compilation_options: Default::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn create_overlay_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    topology: wgpu::PrimitiveTopology,
    label: &'static str,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("builder3d-r5-overlay-shader"),
        source: wgpu::ShaderSource::Wgsl(OVERLAY_SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("builder3d-r5-overlay-pipeline-layout"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    let targets = [Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Rgba8Unorm,
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
    })];
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex_main"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: OVERLAY_VERTEX_STRIDE as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 12,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: 28,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32,
                        offset: 32,
                        shader_location: 3,
                    },
                ],
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth24Plus,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: VIEWPORT_SAMPLE_COUNT,
            ..Default::default()
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment_main"),
            compilation_options: Default::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}

struct OverlayBuffers {
    lines: Vec<OverlayVertex>,
    triangles: Vec<OverlayVertex>,
}

impl OverlayBuffers {
    fn total_vertices(&self) -> usize {
        self.lines.len().saturating_add(self.triangles.len())
    }

    fn total_bytes(&self) -> Result<u64, ViewportError> {
        u64::try_from(self.total_vertices().saturating_mul(OVERLAY_VERTEX_STRIDE)).map_err(|_| {
            ViewportError::CapacityExceeded(String::from("overlay byte count overflow"))
        })
    }

    fn line_bytes(&self) -> Option<Vec<u8>> {
        (!self.lines.is_empty()).then(|| pack_overlay(&self.lines))
    }

    fn triangle_bytes(&self) -> Option<Vec<u8>> {
        (!self.triangles.is_empty()).then(|| pack_overlay(&self.triangles))
    }

    fn line_vertex_count(&self) -> u32 {
        u32::try_from(self.lines.len()).unwrap_or(u32::MAX)
    }

    fn triangle_vertex_count(&self) -> u32 {
        u32::try_from(self.triangles.len()).unwrap_or(u32::MAX)
    }
}

#[derive(Clone, Copy)]
struct OverlayVertex {
    position: [f32; 3],
    color: [f32; 4],
    surface_fade: f32,
    floor_pattern: f32,
}

impl OverlayVertex {
    fn new(position: [f32; 3], color: [f32; 4]) -> Self {
        Self {
            position,
            color,
            surface_fade: 0.0,
            floor_pattern: 0.0,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_overlay_vertices(
    floor: &FloorGeometry,
    cues: &[crate::floor::ContactCue],
    camera: &Camera,
    gizmo_center: Option<[f32; 3]>,
    gizmo_mode: GizmoMode,
    scene_radius: f32,
    width: u32,
    height: u32,
    scene: &RenderScene,
    options: &ViewportRenderOptions,
) -> Result<OverlayBuffers, ViewportError> {
    let mut lines = Vec::new();
    let mut triangles = Vec::new();
    for line in floor.lines.iter().chain(&floor.front_cue) {
        let mut start_color = line.color;
        start_color[3] *= line.start_alpha;
        let mut end_color = line.color;
        end_color[3] *= line.end_alpha;
        lines.push(OverlayVertex {
            ..OverlayVertex::new(line.start, floor_line_color(start_color, options.theme))
        });
        lines.push(OverlayVertex {
            ..OverlayVertex::new(line.end, floor_line_color(end_color, options.theme))
        });
    }
    if let Some(surface) = floor.surface {
        let [minimum_x, _, minimum_z] = surface.minimum;
        let [maximum_x, y, maximum_z] = surface.maximum;
        let color = floor_surface_color(options.theme);
        let subdivisions = FLOOR_SURFACE_SUBDIVISIONS as f32;
        for z_index in 0..FLOOR_SURFACE_SUBDIVISIONS {
            let z0 = minimum_z + (maximum_z - minimum_z) * z_index as f32 / subdivisions;
            let z1 = minimum_z + (maximum_z - minimum_z) * (z_index + 1) as f32 / subdivisions;
            for x_index in 0..FLOOR_SURFACE_SUBDIVISIONS {
                let x0 = minimum_x + (maximum_x - minimum_x) * x_index as f32 / subdivisions;
                let x1 = minimum_x + (maximum_x - minimum_x) * (x_index + 1) as f32 / subdivisions;
                let first_position = [x0, y, z0];
                let second_position = [x1, y, z0];
                let third_position = [x1, y, z1];
                let fourth_position = [x0, y, z1];
                let first = floor_surface_vertex(
                    first_position,
                    color,
                    surface.fade_parameter_at(first_position),
                );
                let second = floor_surface_vertex(
                    second_position,
                    color,
                    surface.fade_parameter_at(second_position),
                );
                let third = floor_surface_vertex(
                    third_position,
                    color,
                    surface.fade_parameter_at(third_position),
                );
                let fourth = floor_surface_vertex(
                    fourth_position,
                    color,
                    surface.fade_parameter_at(fourth_position),
                );
                triangles.extend([first, second, third, first, third, fourth]);
            }
        }
    }
    let shadow_strength = f32::from(options.lighting.normalized().contact_shadow_percent) / 100.0;
    if shadow_strength > 0.0 {
        let light_world = shadow_light_direction_world(camera, options.lighting);
        for cue in cues {
            let polygon = cue.shadow_polygon(light_world);
            for (layer_scale, layer_alpha) in CONTACT_SHADOW_LAYERS {
                let expanded = expand_shadow_polygon(&polygon, layer_scale);
                let mut shadow_color = cue_color(cue.color, options.theme);
                shadow_color[3] = (shadow_color[3] * shadow_strength * layer_alpha).clamp(0.0, 1.0);
                for pair in expanded.windows(2) {
                    triangles.push(OverlayVertex {
                        ..OverlayVertex::new(expanded[0], shadow_color)
                    });
                    triangles.push(OverlayVertex {
                        ..OverlayVertex::new(pair[0], shadow_color)
                    });
                    triangles.push(OverlayVertex {
                        ..OverlayVertex::new(pair[1], shadow_color)
                    });
                }
            }
        }
    }
    if options.wireframe {
        for object in scene.objects() {
            for (first, second) in object
                .full_geometry()
                .wireframe_segments(object.transform())?
            {
                lines.extend([
                    OverlayVertex {
                        ..OverlayVertex::new(first, [0.0, 0.0, 0.0, 1.0])
                    },
                    OverlayVertex {
                        ..OverlayVertex::new(second, [0.0, 0.0, 0.0, 1.0])
                    },
                ]);
                if lines.len().saturating_add(triangles.len()) >= MAX_OVERLAY_VERTICES {
                    break;
                }
            }
            if lines.len().saturating_add(triangles.len()) >= MAX_OVERLAY_VERTICES {
                break;
            }
        }
    }
    if let Some(center) = gizmo_center {
        let geometry = build_gizmo(camera, center, gizmo_mode)?;
        for line in geometry.lines {
            lines.push(OverlayVertex {
                ..OverlayVertex::new(line.start, line.color)
            });
            lines.push(OverlayVertex {
                ..OverlayVertex::new(line.end, line.color)
            });
        }
        for triangle in geometry.triangles {
            triangles.extend_from_slice(&[
                OverlayVertex {
                    ..OverlayVertex::new(triangle.first, triangle.color)
                },
                OverlayVertex {
                    ..OverlayVertex::new(triangle.second, triangle.color)
                },
                OverlayVertex {
                    ..OverlayVertex::new(triangle.third, triangle.color)
                },
            ]);
        }
    }
    if lines.len().saturating_add(triangles.len()) > MAX_OVERLAY_VERTICES {
        return Err(ViewportError::CapacityExceeded(String::from(
            "viewport overlay vertex budget reached",
        )));
    }
    let _ = (scene_radius, width, height);
    Ok(OverlayBuffers { lines, triangles })
}

fn face_index_batches(
    object: &RenderObject,
    geometry: &MeshGeometry,
) -> Result<Vec<FaceIndexBatch>, ViewportError> {
    let capacity = geometry
        .face_count()
        .checked_mul(3)
        .ok_or_else(|| ViewportError::CapacityExceeded(String::from("face index overflow")))?;
    if object.face_colors().is_none() {
        let mut indices = Vec::with_capacity(capacity);
        for face_index in 0..geometry.face_count() {
            let triangle = geometry.triangle(face_index).ok_or_else(|| {
                ViewportError::InvalidInput(String::from("GPU geometry triangle is truncated"))
            })?;
            indices.extend_from_slice(&triangle);
        }
        return Ok(vec![FaceIndexBatch {
            authored_color: None,
            indices,
        }]);
    }

    let default_color = color_rgba8(object.color());
    let mut groups = BTreeMap::<[u8; 4], Vec<u32>>::new();
    for face_index in 0..geometry.face_count() {
        let triangle = geometry.triangle(face_index).ok_or_else(|| {
            ViewportError::InvalidInput(String::from("GPU geometry triangle is truncated"))
        })?;
        let color = object
            .face_color_for_geometry(geometry, face_index)
            .unwrap_or(default_color);
        groups
            .entry(color)
            .or_default()
            .extend_from_slice(&triangle);
    }
    Ok(groups
        .into_iter()
        .map(|(authored_color, indices)| FaceIndexBatch {
            authored_color: Some(authored_color),
            indices,
        })
        .collect())
}

fn encode_indices(indices: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(indices.len().saturating_mul(4));
    for index in indices {
        bytes.extend_from_slice(&index.to_le_bytes());
    }
    bytes
}

fn color_from_rgba8(color: [u8; 4]) -> [f32; 4] {
    color.map(|channel| f32::from(channel) / 255.0)
}

fn display_color(authored: [f32; 4], selected: bool, options: &ViewportRenderOptions) -> [f32; 4] {
    let mut color = authored;
    if selected {
        for (channel, tint) in color[..3].iter_mut().zip(options.selected_tint) {
            *channel = *channel * 0.62 + tint * 0.38;
        }
    }
    if options.xray {
        color[3] = (color[3] * 0.30).clamp(0.05, 0.45);
    }
    color
}

fn floor_surface_vertex(position: [f32; 3], color: [f32; 4], fade_parameter: f32) -> OverlayVertex {
    OverlayVertex {
        position,
        color,
        surface_fade: fade_parameter,
        floor_pattern: 1.0,
    }
}

fn floor_line_color(color: [f32; 4], theme: ViewportTheme) -> [f32; 4] {
    let _ = theme;
    [0.35, 0.35, 0.38, (color[3] * 0.22).min(0.13)]
}

fn floor_surface_color(theme: ViewportTheme) -> [f32; 4] {
    let _ = theme;
    [0.74, 0.74, 0.74, 1.0]
}

fn cue_color(color: [f32; 4], theme: ViewportTheme) -> [f32; 4] {
    match theme {
        ViewportTheme::Dark => color,
        ViewportTheme::Light => [0.25, 0.35, 0.60, color[3]],
    }
}

fn cpu_contact_shadow_color(theme: ViewportTheme) -> [f32; 4] {
    cue_color([0.05, 0.08, 0.12, 0.28], theme)
}

fn cpu_clear_color(theme: ViewportTheme) -> [u8; 4] {
    let _ = theme;
    [240, 240, 240, 255]
}

fn cpu_floor_color(theme: ViewportTheme) -> [f32; 4] {
    let _ = theme;
    [0.875, 0.875, 0.875, 1.0]
}

fn cpu_grid_major_color(theme: ViewportTheme) -> [f32; 4] {
    let _ = theme;
    [0.35, 0.35, 0.38, 0.13]
}

fn cpu_grid_minor_color(theme: ViewportTheme) -> [f32; 4] {
    let _ = theme;
    [0.35, 0.35, 0.38, 0.07]
}

fn pack_overlay(vertices: &[OverlayVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len().saturating_mul(OVERLAY_VERTEX_STRIDE));
    for vertex in vertices {
        for value in vertex
            .position
            .iter()
            .chain(vertex.color.iter())
            .chain([&vertex.surface_fade, &vertex.floor_pattern])
        {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
    }
    bytes
}

const MESH_SHADER: &str = r#"
struct Uniforms {
    view_projection: mat4x4<f32>,
    model_view: mat4x4<f32>,
    normal_transform: mat4x4<f32>,
    color: vec4<f32>,
    light_direction: vec4<f32>,
    material: vec4<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) view_position: vec3<f32>,
};
@vertex
fn vertex_main(@location(0) position: vec3<f32>, @location(1) normal: vec3<f32>) -> VertexOutput {
    let local_position = vec4<f32>(position, 1.0);
    let view_position = uniforms.model_view * local_position;
    let view_normal = uniforms.normal_transform * vec4<f32>(normal, 0.0);
    return VertexOutput(
        uniforms.view_projection * local_position,
        view_normal.xyz,
        view_position.xyz,
    );
}
@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let light = normalize(uniforms.light_direction.xyz);
    let toward_viewer = normalize(-input.view_position);
    let facing = dot(normal, light);
    let lambert = max(facing, 0.0);
    let wrapped = (facing * 0.5 + 0.5) * (facing * 0.5 + 0.5);
    let fill = max(dot(normal, normalize(vec3<f32>(-light.x, 0.0, light.z) + vec3<f32>(0.0, 0.0, 0.35))), 0.0);
    let diffuse = 0.55 * lambert + 0.33 * wrapped + 0.12 * fill;
    var specular = 0.0;
    if lambert > 0.0 {
        let half_vector = light + toward_viewer;
        if dot(half_vector, half_vector) > 0.000001 {
            let halfway = normalize(half_vector);
            specular = pow(max(dot(normal, halfway), 0.0), uniforms.material.w)
                * uniforms.material.z
                * max(uniforms.material.x, 0.25);
        }
    }
    let intensity = uniforms.material.y + uniforms.material.x * diffuse;
    let linear = max(
        uniforms.color.rgb * intensity + vec3<f32>(specular),
        vec3<f32>(0.0),
    );
    let low = linear * 12.92;
    let high = 1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return vec4<f32>(select(high, low, linear <= vec3<f32>(0.0031308)), uniforms.color.a);
}
"#;

const OVERLAY_SHADER: &str = r#"
struct Uniforms {
    view_projection: mat4x4<f32>,
    color: vec4<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) surface_fade: f32,
    @location(3) floor_pattern: f32,
};
@vertex
fn vertex_main(
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) surface_fade: f32,
    @location(3) floor_pattern: f32,
) -> VertexOutput {
    return VertexOutput(
        uniforms.view_projection * vec4<f32>(position, 1.0),
        color,
        position,
        surface_fade,
        floor_pattern,
    );
}
@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    var color = input.color;
    if input.floor_pattern > 0.5 {
        let checker_cell = vec2<i32>(floor(input.world_position.xz / 50.0));
        let alternate = ((checker_cell.x + checker_cell.y) & 1) == 1;
        let cell_position = input.world_position.xz / 50.0;
        let footprint = max(fwidth(cell_position.x), fwidth(cell_position.y));
        let contrast = 1.0 - smoothstep(0.55, 1.15, footprint);
        let shade = select(-0.04, 0.04, alternate) * contrast;
        color = vec4<f32>(
            color.rgb + vec3<f32>(shade),
            color.a * (1.0 - smoothstep(0.0, 1.0, input.surface_fade)),
        );
    }
    let linear = max(color.rgb, vec3<f32>(0.0));
    let low = linear * 12.92;
    let high = 1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return vec4<f32>(select(high, low, linear <= vec3<f32>(0.0031308)), color.a);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use builder3d_platform::{
        AdapterKind, AdapterProfile, BackendKind, CpuProfile, DisplayRelationship, MemoryProfile,
        OsMemoryBudget,
    };

    #[test]
    fn uniform_bytes_match_wgsl_alignment() {
        let bytes = make_uniform(Mat4::identity(), [1.0, 0.5, 0.25, 1.0]);
        assert_eq!(bytes.len(), 80);
    }

    #[test]
    fn display_options_tint_selection_and_keep_xray_alpha_bounded() {
        let options = ViewportRenderOptions {
            xray: true,
            selected_object_ids: vec![String::from("cube")],
            ..ViewportRenderOptions::default()
        };
        let selected = display_color([0.8, 0.2, 0.1, 1.0], true, &options);
        let plain = display_color([0.8, 0.2, 0.1, 1.0], false, &options);
        assert_ne!(selected, plain);
        assert!((0.05..=0.45).contains(&selected[3]));
        assert!((0.05..=0.45).contains(&plain[3]));
        assert_eq!(
            floor_surface_color(ViewportTheme::Light),
            floor_surface_color(ViewportTheme::Dark)
        );
        assert_eq!(
            cpu_clear_color(ViewportTheme::Dark),
            cpu_clear_color(ViewportTheme::Light)
        );
    }

    #[test]
    fn boolean_face_colors_build_two_shared_vertex_gpu_batches() {
        let mesh = builder3d_core::make_box([2.0, 2.0, 2.0]).expect("box");
        let provenance = builder3d_core::BooleanColorProvenance {
            operation: String::from("subtract"),
            source_colors: [[0.2, 0.4, 0.6, 1.0], [0.8, 0.2, 0.1, 1.0]],
            face_sources: (0..mesh.face_count())
                .map(|index| u8::try_from(index % 2).unwrap_or(0))
                .collect(),
            tool_face_count: mesh.face_count() / 2,
        };
        let metadata =
            builder3d_core::with_boolean_color_provenance(&serde_json::json!({}), &provenance);
        let object = builder3d_core::SceneObject::with_details(
            mesh,
            None,
            Some(String::from("Boolean result")),
            Some(provenance.source_colors[0]),
            Some(metadata),
        )
        .expect("scene object");
        let mut scene = builder3d_core::Scene::new();
        scene.add_object(object).expect("add object");
        let render = RenderScene::from_core(&scene).expect("render scene");
        let object = &render.objects()[0];
        let batches = face_index_batches(object, object.full_geometry()).expect("index batches");

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].authored_color, Some([51, 102, 153, 255]));
        assert_eq!(batches[1].authored_color, Some([204, 51, 26, 255]));
        assert_eq!(batches[0].indices.len(), 18);
        assert_eq!(batches[1].indices.len(), 18);
        assert_eq!(
            batches
                .iter()
                .map(|batch| batch.indices.len())
                .sum::<usize>(),
            36
        );
    }

    #[test]
    fn padded_rows_are_copy_aligned() {
        assert_eq!(padded_bytes_per_row(64).ok(), Some(256));
        assert_eq!(padded_bytes_per_row(65).ok(), Some(512));
    }

    #[test]
    fn shared_device_uses_the_matching_profile_backend_and_identity() {
        let profile = test_profile();
        assert_eq!(
            profile_adapter_index(&profile, 0x10de, 0x1eb1, wgpu::Backend::Dx12),
            Some(2)
        );
        assert_eq!(
            profile_adapter_index(&profile, 0x10de, 0x1eb1, wgpu::Backend::Vulkan),
            None
        );
        assert_eq!(
            profile_adapter_index(&profile, 0x1002, 0x1eb1, wgpu::Backend::Dx12),
            None
        );
    }

    #[test]
    fn render_target_budget_counts_color_and_depth_attachments() {
        assert_eq!(render_target_size_bytes(4, 2).ok(), Some(288));
        assert!(render_target_size_bytes(u32::MAX, u32::MAX).is_err());
    }

    #[test]
    fn default_floor_overlay_fits_the_vertex_budget() {
        let camera = Camera::new();
        let floor = build_floor(&camera, DEFAULT_FLOOR_SIZE, true, true).expect("floor");
        let scene = RenderScene::from_core(&builder3d_core::Scene::new()).expect("empty scene");
        let overlay = build_overlay_vertices(
            &floor,
            &[],
            &camera,
            None,
            GizmoMode::Move,
            1.0,
            2_560,
            1_440,
            &scene,
            &ViewportRenderOptions::default(),
        )
        .expect("default floor overlay should render");

        assert_eq!(
            overlay.triangles.len(),
            FLOOR_SURFACE_SUBDIVISIONS * FLOOR_SURFACE_SUBDIVISIONS * 6
        );
        assert!(overlay.total_vertices() <= MAX_OVERLAY_VERTICES);
    }

    #[test]
    fn clear_color_accepts_normalized_channels_and_rejects_invalid_values() {
        assert_eq!(
            normalize_clear_color([0.0, 0.25, 0.5, 1.0]).ok(),
            Some([0.0, 0.25, 0.5, 1.0])
        );
        assert!(normalize_clear_color([0.0, f32::NAN, 0.5, 1.0]).is_err());
        assert!(normalize_clear_color([0.0, 0.25, 1.01, 1.0]).is_err());
    }

    fn test_profile() -> CapabilityProfile {
        CapabilityProfile {
            cpu: CpuProfile {
                logical_cores: 8,
                physical_cores: 4,
                total_memory_bytes: 16_000,
                available_memory_bytes: 8_000,
            },
            os_memory: OsMemoryBudget {
                total_bytes: 16_000,
                available_bytes: 8_000,
                source: String::from("test"),
            },
            adapters: vec![AdapterProfile {
                index: 2,
                name: String::from("test RTX"),
                backend: BackendKind::Dx12,
                kind: AdapterKind::Discrete,
                vendor_id: 0x10de,
                device_id: 0x1eb1,
                driver: String::from("test driver"),
                driver_info: String::new(),
                is_usable: true,
                is_display_compatible: true,
                display: DisplayRelationship {
                    has_output: true,
                    output_count: 1,
                    active_display_uncertain: false,
                },
                memory: MemoryProfile {
                    dedicated_bytes: 8_000,
                    shared_bytes: 4_000,
                    budget_bytes: 8_000,
                    usage_bytes: 0,
                    available_bytes: 8_000,
                    unified_memory: false,
                },
                max_buffer_size: 8_000,
                max_texture_dimension_2d: 4_096,
            }],
            discovery_backend: String::from("test"),
        }
    }
}
