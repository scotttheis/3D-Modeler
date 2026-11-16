use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Instant,
};

use modeler3d_core::{Scene, SceneObject, make_box, make_sphere};
use modeler3d_platform::{
    AdapterKind, AdapterProfile, BackendKind, CapabilityProfile, CpuProfile, DataResidency,
    DisplayRelationship, MemoryProfile, OsMemoryBudget, RoutePreference, WorkloadClass,
    WorkloadPlan, WorkloadRequest, discover_capability_profile, plan_workload,
};
use modeler3d_viewport::{
    Camera, GizmoMode, InteractionState, RenderScene, RenderSpikeConfig, RendererConfig,
    ViewportFrameReport, ViewportRenderer, interactive_viewport_face_target, ledger_for_profile,
    run_render_spike,
};
use serde::Serialize;

const CONTROL_FACE_COUNT: u64 = 30_000_000;
const INTERACTION_CONTROL_FACE_COUNT: u64 = 3_600_000;
const BYTES_PER_FACE_ESTIMATE: u64 = 32;

#[derive(Debug, Serialize)]
struct RoutingMatrixEntry {
    name: String,
    render: WorkloadPlan,
    memory_bound: WorkloadPlan,
}

#[derive(Debug, Serialize)]
struct R5Report {
    report_version: String,
    capability_profile: CapabilityProfile,
    scene_authoritative_faces: u64,
    scene_authoritative_bytes: u64,
    scene_interaction_bytes: u64,
    adaptive_probe_faces: u64,
    resting_viewport: ViewportFrameReport,
    active_viewport: ViewportFrameReport,
    full_resolution_control: modeler3d_viewport::RenderMeasurement,
    interaction_control: modeler3d_viewport::RenderMeasurement,
    routing_matrix: Vec<RoutingMatrixEntry>,
    cpu_reference_ms: f64,
    binary_size_bytes: u64,
    limitations: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_dir = argument_value("--output-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("evidence/r5"));
    fs::create_dir_all(&output_dir)?;
    let profile = discover_capability_profile().await?;
    let adapter_index = selected_adapter_index(&profile);
    let (core_scene, render_scene) = build_reference_scene()?;
    let mut camera = Camera::new();
    camera.set_viewport_size(512, 512)?;
    if let Some(bounds) = render_scene.world_bounds()? {
        camera.frame_bounds(&[bounds])?;
    }
    let mut renderer = ViewportRenderer::new(
        &profile,
        &RendererConfig {
            width: 512,
            height: 512,
            adapter_index,
            capture_path: None,
        },
    )
    .await?;
    let resting = renderer.render(
        &render_scene,
        &camera,
        InteractionState::Resting,
        core_scene.revision(),
        Some([0.0, 0.0, 0.0]),
        GizmoMode::Move,
        Some(output_dir.join("viewport-resting.ppm")),
    )?;
    let active = renderer.render(
        &render_scene,
        &camera,
        InteractionState::Active,
        core_scene.revision(),
        Some([0.0, 0.0, 0.0]),
        GizmoMode::Move,
        None,
    )?;
    let full_resolution_control = run_render_spike(&RenderSpikeConfig {
        width: 512,
        height: 512,
        face_count: CONTROL_FACE_COUNT,
        adapter_index,
        warmup_frames: 1,
        measured_frames: 5,
        capture_path: output_dir.join("30m-control.ppm"),
    })
    .await?;
    let interaction_control = run_render_spike(&RenderSpikeConfig {
        width: 512,
        height: 512,
        face_count: INTERACTION_CONTROL_FACE_COUNT,
        adapter_index,
        warmup_frames: 1,
        measured_frames: 5,
        capture_path: output_dir.join("interaction-control.ppm"),
    })
    .await?;
    let routing_matrix = routing_matrix();
    let cpu_reference_ms = measure_cpu_reference(CONTROL_FACE_COUNT);
    let binary_size_bytes = fs::metadata(env::current_exe()?)?.len();
    let report = R5Report {
        report_version: String::from("R5-20261029"),
        capability_profile: profile,
        scene_authoritative_faces: render_scene.face_count(),
        scene_authoritative_bytes: render_scene.authoritative_bytes(),
        scene_interaction_bytes: render_scene.interaction_bytes(),
        adaptive_probe_faces: interactive_viewport_face_target(30_655_824),
        resting_viewport: resting,
        active_viewport: active,
        full_resolution_control,
        interaction_control,
        routing_matrix,
        cpu_reference_ms,
        binary_size_bytes,
        limitations: vec![
            String::from(
                "The 30M and 3.6M controls use procedural instancing, while the viewport frame uses retained indexed scene geometry.",
            ),
            String::from(
                "R5 does not yet provide text labels or a native window surface; offscreen captures validate the renderer and overlay paths.",
            ),
            String::from(
                "Exact picking remains CPU-authoritative and is intentionally separate from display LOD.",
            ),
        ],
    };
    write_report(&output_dir, &report)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn build_reference_scene() -> Result<(Scene, RenderScene), Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    scene.add_object(SceneObject::new(make_box([40.0, 40.0, 40.0])?))?;
    scene.add_object(SceneObject::new(make_sphere(22.0, 6)?))?;
    scene.translate(1, [60.0, 20.0, 0.0])?;
    let render_scene = RenderScene::from_core(&scene)?;
    Ok((scene, render_scene))
}

fn selected_adapter_index(profile: &CapabilityProfile) -> Option<usize> {
    profile
        .adapters
        .iter()
        .filter(|adapter| adapter.is_usable)
        .max_by_key(|adapter| {
            u64::from(adapter.display.has_output) * 1_000_000_000_000
                + adapter.memory.available_bytes
        })
        .map(|adapter| adapter.index)
}

fn routing_matrix() -> Vec<RoutingMatrixEntry> {
    [
        (
            "dedicated-discrete",
            AdapterKind::Discrete,
            false,
            8_u64 * 1024 * 1024 * 1024,
        ),
        (
            "intel-integrated",
            AdapterKind::Integrated,
            true,
            2_u64 * 1024 * 1024 * 1024,
        ),
        (
            "amd-unified",
            AdapterKind::Integrated,
            true,
            4_u64 * 1024 * 1024 * 1024,
        ),
        ("software-fallback", AdapterKind::Software, false, 0),
    ]
    .into_iter()
    .map(|(name, kind, unified, budget)| {
        let profile = synthetic_profile(kind, unified, budget);
        let ledger = ledger_for_profile(&profile);
        let render_request = WorkloadRequest {
            class: WorkloadClass::GpuRender,
            unit_count: CONTROL_FACE_COUNT,
            bytes_per_unit: BYTES_PER_FACE_ESTIMATE,
            batch_unit_count: 100_000,
            preference: RoutePreference::Automatic,
            calibration: None,
            residency: DataResidency {
                authoritative_on_cpu: true,
                cpu_bytes: CONTROL_FACE_COUNT * BYTES_PER_FACE_ESTIMATE,
                ..DataResidency::default()
            },
        };
        let memory_request = WorkloadRequest {
            class: WorkloadClass::MemoryBound,
            ..render_request.clone()
        };
        RoutingMatrixEntry {
            name: String::from(name),
            render: plan_workload(&profile, &render_request, &ledger),
            memory_bound: plan_workload(&profile, &memory_request, &ledger),
        }
    })
    .collect()
}

fn synthetic_profile(kind: AdapterKind, unified: bool, budget: u64) -> CapabilityProfile {
    CapabilityProfile {
        cpu: CpuProfile {
            logical_cores: 8,
            physical_cores: 4,
            total_memory_bytes: 16 * 1024 * 1024 * 1024,
            available_memory_bytes: 8 * 1024 * 1024 * 1024,
        },
        os_memory: OsMemoryBudget {
            total_bytes: 16 * 1024 * 1024 * 1024,
            available_bytes: 8 * 1024 * 1024 * 1024,
            source: String::from("R5 synthetic matrix"),
        },
        adapters: if matches!(kind, AdapterKind::Software) {
            Vec::new()
        } else {
            vec![AdapterProfile {
                index: 0,
                name: String::from("R5 synthetic adapter"),
                backend: BackendKind::Other,
                kind,
                vendor_id: 0,
                device_id: 0,
                driver: String::from("synthetic"),
                driver_info: String::new(),
                is_usable: true,
                is_display_compatible: true,
                display: DisplayRelationship {
                    has_output: true,
                    output_count: 1,
                    active_display_uncertain: false,
                },
                memory: MemoryProfile {
                    dedicated_bytes: if unified { 0 } else { budget },
                    shared_bytes: if unified { budget } else { 0 },
                    budget_bytes: budget,
                    usage_bytes: 0,
                    available_bytes: budget,
                    unified_memory: unified,
                },
                max_buffer_size: u64::MAX,
                max_texture_dimension_2d: 16_384,
            }]
        },
        discovery_backend: String::from("R5 synthetic matrix"),
    }
}

fn measure_cpu_reference(face_count: u64) -> f64 {
    let started = Instant::now();
    let mut checksum = 0_u64;
    for index in 0..face_count {
        checksum = checksum
            .wrapping_add(index.wrapping_mul(0x9E37_79B9))
            .rotate_left((index & 31) as u32);
    }
    std::hint::black_box(checksum);
    started.elapsed().as_secs_f64() * 1_000.0
}

fn write_report(output_dir: &Path, report: &R5Report) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        output_dir.join("report.json"),
        serde_json::to_vec_pretty(report)?,
    )?;
    let mut markdown = String::new();
    markdown.push_str("# 3D Modeler Rust R5 viewport\n\n");
    markdown.push_str(&format!(
        "- Scene faces: {}\n- Scene derived bytes: authoritative {} / interaction {}\n- 30,655,824-face adaptive probe target: {} faces\n- Real viewport resting: {:.3} ms (CPU fallback: {})\n- Real viewport active: {:.3} ms (CPU fallback: {})\n- 30M control p50/p95/max: {:.3}/{:.3}/{:.3} ms\n- Interaction control p50/p95/max: {:.3}/{:.3}/{:.3} ms\n- CPU reference: {:.3} ms\n- Binary size: {} bytes\n\n",
        report.scene_authoritative_faces,
        report.scene_authoritative_bytes,
        report.scene_interaction_bytes,
        report.adaptive_probe_faces,
        report.resting_viewport.frame_time_ms,
        report.resting_viewport.cpu_fallback,
        report.active_viewport.frame_time_ms,
        report.active_viewport.cpu_fallback,
        report.full_resolution_control.p50_ms,
        report.full_resolution_control.p95_ms,
        report.full_resolution_control.max_ms,
        report.interaction_control.p50_ms,
        report.interaction_control.p95_ms,
        report.interaction_control.max_ms,
        report.cpu_reference_ms,
        report.binary_size_bytes,
    ));
    markdown.push_str("## Routing matrix\n\n");
    for entry in &report.routing_matrix {
        markdown.push_str(&format!(
            "- **{}**: render `{:?}` / fallback `{:?}`; memory-bound `{:?}` / fallback `{:?}`.\n",
            entry.name,
            entry.render.route,
            entry.render.fallback_reason,
            entry.memory_bound.route,
            entry.memory_bound.fallback_reason,
        ));
    }
    markdown.push_str("\n## Limitations\n\n");
    for limitation in &report.limitations {
        markdown.push_str(&format!("- {limitation}\n"));
    }
    fs::write(output_dir.join("report.md"), markdown)?;
    Ok(())
}

fn argument_value(name: &str) -> Option<String> {
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == name {
            return arguments.next();
        }
    }
    None
}
