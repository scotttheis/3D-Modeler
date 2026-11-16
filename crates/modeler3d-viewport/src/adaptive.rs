use modeler3d_platform::{
    AdapterBudget, AllocationKind, CalibrationSample, CapabilityProfile, DataResidency,
    MemoryDomain, ResourceLedger, Route, RoutePreference, RoutingTelemetry, WorkloadClass,
    WorkloadPlan, WorkloadRequest, plan_workload,
};

use crate::{
    ViewportError,
    scene::{INTERACTIVE_FULL_RES_FACE_LIMIT, RenderScene, interactive_viewport_face_target},
};

/// Display tier chosen for one viewport frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum InteractionTier {
    /// Full authoritative display geometry.
    Full,
    /// Deterministic display LOD while camera input is active.
    InteractionLod,
    /// Planner split the workload at bounded batch boundaries.
    Hybrid,
    /// The renderer is unavailable or the planner selected the CPU path.
    CpuFallback,
}

/// Whether the camera is moving or settled.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum InteractionState {
    /// Full detail and settled derived cues may be shown.
    #[default]
    Resting,
    /// Camera motion uses the prepared interaction tier.
    Active,
}

/// One central-ledger reservation set associated with a viewport scene.
#[derive(Clone, Debug, Default)]
pub struct ViewportReservationSet {
    /// Reservations made in the central ledger.
    ids: Vec<modeler3d_platform::AllocationId>,
}

impl ViewportReservationSet {
    /// Reserves the retained authoritative and prepared display bytes.
    pub fn reserve(
        ledger: &mut ResourceLedger,
        scene: &RenderScene,
    ) -> Result<Self, ViewportError> {
        let mut ids = Vec::with_capacity(3);
        let authoritative = ledger.reserve(
            AllocationKind::Authoritative,
            MemoryDomain::System,
            scene.authoritative_bytes(),
        )?;
        ids.push(authoritative);
        let interaction = ledger.reserve(
            AllocationKind::Cache,
            MemoryDomain::System,
            scene.interaction_bytes(),
        );
        match interaction {
            Ok(id) => ids.push(id),
            Err(error) => {
                let _ = ledger.release(authoritative);
                return Err(error.into());
            }
        }
        Ok(Self { ids })
    }

    /// Releases every reservation exactly once.
    pub fn release(self, ledger: &mut ResourceLedger) -> Result<(), ViewportError> {
        for id in self.ids {
            ledger.release(id)?;
        }
        Ok(())
    }

    /// Returns the number of live reservations in this set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Returns whether this set owns no reservations.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

/// Adaptive viewport decision plus platform routing evidence.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct AdaptiveInteractionPlan {
    /// Current interaction state.
    pub state: InteractionStateReport,
    /// Selected display tier.
    pub tier: InteractionTier,
    /// Total authoritative faces retained by the scene.
    pub authoritative_faces: u64,
    /// Faces submitted by the active display path.
    pub display_faces: u64,
    /// Whether full authoritative display geometry is allowed by the threshold.
    pub full_resolution_allowed: bool,
    /// Platform CPU/GPU/hybrid work plan.
    pub workload: WorkloadPlan,
    /// Runtime routing telemetry seed.
    pub telemetry: RoutingTelemetry,
}

/// Serializable interaction state used in reports.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum InteractionStateReport {
    /// Camera settled.
    Resting,
    /// Camera active.
    Active,
}

/// Stateful transition helper that controls the interaction tier boundary.
#[derive(Clone, Copy, Debug, Default)]
pub struct AdaptiveInteractionController {
    state: InteractionState,
}

impl AdaptiveInteractionController {
    /// Creates a settled controller.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: InteractionState::Resting,
        }
    }

    /// Marks camera input active.
    pub const fn begin(&mut self) {
        self.state = InteractionState::Active;
    }

    /// Marks camera input settled.
    pub const fn settle(&mut self) {
        self.state = InteractionState::Resting;
    }

    /// Returns the current state.
    #[must_use]
    pub const fn state(self) -> InteractionState {
        self.state
    }
}

/// Creates a tier and CPU/GPU/hybrid plan from one central ledger snapshot.
pub fn plan_interaction(
    profile: &CapabilityProfile,
    scene: &RenderScene,
    state: InteractionState,
    ledger: &ResourceLedger,
    calibration: Option<CalibrationSample>,
) -> AdaptiveInteractionPlan {
    plan_interaction_with_preference(
        profile,
        scene,
        state,
        ledger,
        calibration,
        RoutePreference::Automatic,
    )
}

/// Creates an adaptive plan while pinning work to the renderer's opened adapter.
pub fn plan_interaction_on_adapter(
    profile: &CapabilityProfile,
    scene: &RenderScene,
    state: InteractionState,
    ledger: &ResourceLedger,
    calibration: Option<CalibrationSample>,
    adapter_index: usize,
) -> AdaptiveInteractionPlan {
    plan_interaction_with_preference(
        profile,
        scene,
        state,
        ledger,
        calibration,
        RoutePreference::Adapter(adapter_index),
    )
}

fn plan_interaction_with_preference(
    profile: &CapabilityProfile,
    scene: &RenderScene,
    state: InteractionState,
    ledger: &ResourceLedger,
    calibration: Option<CalibrationSample>,
    preference: RoutePreference,
) -> AdaptiveInteractionPlan {
    let authoritative_faces = scene.face_count();
    let display_faces = match state {
        InteractionState::Active => interactive_viewport_face_target(authoritative_faces),
        InteractionState::Resting => authoritative_faces,
    };
    let display_bytes = if display_faces == authoritative_faces {
        scene.authoritative_bytes()
    } else {
        proportional_bytes(scene.interaction_bytes(), scene.face_count(), display_faces)
    };
    let bytes_per_unit = match display_bytes.checked_div(display_faces.max(1)) {
        Some(bytes) => bytes.max(1),
        None => 1,
    };
    let request = WorkloadRequest {
        class: WorkloadClass::GpuRender,
        unit_count: display_faces,
        bytes_per_unit,
        batch_unit_count: 100_000,
        preference,
        calibration,
        residency: DataResidency {
            authoritative_on_cpu: true,
            cpu_bytes: scene.authoritative_bytes(),
            gpu_bytes: 0,
            shared_memory: false,
            resident_adapter: None,
        },
    };
    // An empty document has no triangle batches to route, but the viewport
    // still uses the adapter to clear its target and draw the floor overlay.
    // Treat that as a successful zero-work adapter plan instead of letting
    // the planner's invalid zero-unit guard report a misleading CPU fallback.
    let empty_scene_adapter = match preference {
        RoutePreference::Cpu => None,
        RoutePreference::Adapter(index) => profile
            .adapters
            .get(index)
            .filter(|adapter| adapter.is_usable)
            .map(|adapter| adapter.index),
        RoutePreference::Automatic => profile
            .adapters
            .iter()
            .enumerate()
            .filter(|(_, adapter)| adapter.is_usable)
            .max_by_key(|(_, adapter)| {
                u64::from(adapter.display.has_output) * 4
                    + adapter.memory.budget_bytes / (1024 * 1024)
                    + u64::from(adapter.memory.unified_memory)
            })
            .map(|(_, adapter)| adapter.index),
    };
    let workload = if display_faces == 0
        && let Some(adapter_index) = empty_scene_adapter
    {
        let adapter = &profile.adapters[adapter_index];
        let adapter_available_bytes = ledger
            .available_bytes(MemoryDomain::Adapter(adapter_index))
            .unwrap_or(0);
        WorkloadPlan {
            class: request.class,
            route: Route::Gpu { adapter_index },
            batches: Vec::new(),
            estimated_transfer_bytes: 0,
            fallback_reason: None,
            adapter_budget_bytes: adapter.memory.budget_bytes,
            adapter_available_bytes,
            peak_gpu_bytes: 0,
        }
    } else {
        plan_workload(profile, &request, ledger)
    };
    let tier = match workload.route {
        Route::Cpu => InteractionTier::CpuFallback,
        Route::Hybrid { .. } => InteractionTier::Hybrid,
        Route::Gpu { .. } => {
            if state == InteractionState::Active && display_faces < authoritative_faces {
                InteractionTier::InteractionLod
            } else {
                InteractionTier::Full
            }
        }
    };
    let selected_adapter = match workload.route {
        Route::Cpu => None,
        Route::Gpu { adapter_index } | Route::Hybrid { adapter_index } => Some(adapter_index),
    };
    AdaptiveInteractionPlan {
        state: match state {
            InteractionState::Resting => InteractionStateReport::Resting,
            InteractionState::Active => InteractionStateReport::Active,
        },
        tier,
        authoritative_faces,
        display_faces,
        full_resolution_allowed: authoritative_faces <= INTERACTIVE_FULL_RES_FACE_LIMIT,
        telemetry: RoutingTelemetry {
            selected_adapter,
            route: Some(workload.route),
            fallback_reason: workload.fallback_reason,
            transfer_bytes: workload.estimated_transfer_bytes,
            frame_time_p50_ms: None,
            frame_time_p95_ms: None,
            peak_allocated_bytes: display_bytes,
        },
        workload,
    }
}

fn proportional_bytes(bytes: u64, full_faces: u64, display_faces: u64) -> u64 {
    if full_faces == 0 {
        0
    } else {
        bytes
            .saturating_mul(display_faces)
            .checked_div(full_faces)
            .unwrap_or_default()
    }
}

/// Builds a ledger from a discovered profile without inventing GPU capacity.
#[must_use]
pub fn ledger_for_profile(profile: &CapabilityProfile) -> ResourceLedger {
    let adapter_budgets = profile
        .adapters
        .iter()
        .map(|adapter| AdapterBudget {
            budget_bytes: adapter.memory.available_bytes,
            unified_memory: adapter.memory.unified_memory,
        })
        .collect();
    ResourceLedger::new(profile.os_memory.available_bytes, adapter_budgets)
}

#[cfg(test)]
mod tests {
    use modeler3d_core::{Scene, SceneObject, make_box};
    use modeler3d_platform::{
        AdapterKind, AdapterProfile, BackendKind, CpuProfile, DisplayRelationship, MemoryProfile,
        OsMemoryBudget,
    };

    use super::*;
    use crate::scene::RenderScene;

    fn profile(kind: AdapterKind) -> CapabilityProfile {
        CapabilityProfile {
            cpu: CpuProfile {
                logical_cores: 8,
                physical_cores: 4,
                total_memory_bytes: 100_000,
                available_memory_bytes: 90_000,
            },
            os_memory: OsMemoryBudget {
                total_bytes: 100_000,
                available_bytes: 90_000,
                source: String::from("test"),
            },
            adapters: vec![AdapterProfile {
                index: 0,
                name: String::from("test"),
                backend: BackendKind::Vulkan,
                kind,
                vendor_id: 1,
                device_id: 1,
                driver: String::new(),
                driver_info: String::new(),
                is_usable: true,
                is_display_compatible: true,
                display: DisplayRelationship {
                    has_output: true,
                    output_count: 1,
                    active_display_uncertain: true,
                },
                memory: MemoryProfile {
                    dedicated_bytes: 80_000,
                    shared_bytes: 0,
                    budget_bytes: 80_000,
                    usage_bytes: 0,
                    available_bytes: 80_000,
                    unified_memory: false,
                },
                max_buffer_size: 1_000_000,
                max_texture_dimension_2d: 16_384,
            }],
            discovery_backend: String::from("test"),
        }
    }

    fn scene() -> RenderScene {
        let mesh = match make_box([2.0, 2.0, 2.0]) {
            Ok(mesh) => mesh,
            Err(_) => return RenderScene::default(),
        };
        let mut source = Scene::new();
        if source.add_object(SceneObject::new(mesh)).is_err() {
            return RenderScene::default();
        }
        RenderScene::from_core(&source).unwrap_or_default()
    }

    #[test]
    fn controller_switches_only_the_display_tier() {
        let mut controller = AdaptiveInteractionController::new();
        assert_eq!(controller.state(), InteractionState::Resting);
        controller.begin();
        assert_eq!(controller.state(), InteractionState::Active);
        controller.settle();
        assert_eq!(controller.state(), InteractionState::Resting);
    }

    #[test]
    fn planner_reports_a_cpu_fallback_without_usable_adapters() {
        let profile = profile(AdapterKind::Discrete);
        let mut no_adapter = profile.clone();
        no_adapter.adapters[0].is_usable = false;
        let plan = plan_interaction(
            &no_adapter,
            &scene(),
            InteractionState::Active,
            &ledger_for_profile(&profile),
            None,
        );
        assert_eq!(plan.tier, InteractionTier::CpuFallback);
        assert_eq!(plan.telemetry.selected_adapter, None);
    }

    #[test]
    fn empty_scene_keeps_adapter_presentation_without_a_cpu_fallback() {
        let profile = profile(AdapterKind::Discrete);
        let plan = plan_interaction_on_adapter(
            &profile,
            &RenderScene::default(),
            InteractionState::Resting,
            &ledger_for_profile(&profile),
            None,
            0,
        );

        assert_eq!(plan.tier, InteractionTier::Full);
        assert_eq!(plan.authoritative_faces, 0);
        assert!(plan.workload.batches.is_empty());
        assert_eq!(plan.workload.route, Route::Gpu { adapter_index: 0 });
        assert_eq!(plan.workload.fallback_reason, None);
        assert_eq!(plan.telemetry.selected_adapter, Some(0));
    }
}
