use serde::{Deserialize, Serialize};

use crate::{CapabilityProfile, MemoryDomain, ResourceLedger};

const MAX_PLANNED_BATCHES: u64 = 4_096;
const GPU_BUDGET_HEADROOM_NUMERATOR: u64 = 9;
const GPU_BUDGET_HEADROOM_DENOMINATOR: u64 = 10;

/// The broad execution shape of a workload.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum WorkloadClass {
    /// Small or dependency-ordered work that should remain on the CPU.
    SerialCpu,
    /// Parallel work with enough arithmetic intensity to consider a device.
    ParallelCpu,
    /// Presentation and raster work.
    GpuRender,
    /// Compute work that can use a graphics adapter.
    GpuCompute,
    /// Work for which moving the source data dominates the operation.
    TransferSensitive,
    /// Work whose main constraint is memory capacity or bandwidth.
    MemoryBound,
}

/// An explicit caller preference for routing.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum RoutePreference {
    /// Let capabilities, residency, budget, and calibration decide.
    Automatic,
    /// Use the bounded CPU reference path.
    Cpu,
    /// Request one adapter, with safe CPU fallback if it cannot be used.
    Adapter(usize),
}

/// Where the authoritative input currently resides.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DataResidency {
    /// The authoritative data is present in system RAM.
    pub authoritative_on_cpu: bool,
    /// The data is already resident on this adapter, if any.
    pub resident_adapter: Option<usize>,
    /// Bytes of authoritative data in system RAM.
    pub cpu_bytes: u64,
    /// Bytes of authoritative data already resident on an adapter.
    pub gpu_bytes: u64,
    /// CPU and adapter memory refer to one physically shared pool.
    pub shared_memory: bool,
}

/// A measured comparison used by the planner.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct CalibrationSample {
    /// CPU reference duration in milliseconds.
    pub cpu_ms: f64,
    /// Adapter duration in milliseconds.
    pub gpu_ms: f64,
    /// Transfer duration in milliseconds.
    pub transfer_ms: f64,
}

impl CalibrationSample {
    fn is_valid(self) -> bool {
        self.cpu_ms.is_finite()
            && self.gpu_ms.is_finite()
            && self.transfer_ms.is_finite()
            && self.cpu_ms >= 0.0
            && self.gpu_ms >= 0.0
            && self.transfer_ms >= 0.0
    }

    fn gpu_wins(self) -> bool {
        self.is_valid() && self.gpu_ms + self.transfer_ms < self.cpu_ms
    }
}

/// The concrete implementation selected for one independently executable batch.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BatchTarget {
    /// The deterministic CPU/reference implementation.
    Cpu,
    /// A graphics adapter implementation.
    Gpu {
        /// Index in the discovered capability profile.
        adapter_index: usize,
    },
}

/// A single independently executable portion of a workload.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WorkBatch {
    /// Zero-based batch number.
    pub index: u32,
    /// First unit included in this batch.
    pub first_unit: u64,
    /// Number of units included in this batch.
    pub unit_count: u64,
    /// The implementation selected for this batch.
    pub target: BatchTarget,
}

/// The route selected for a batch or an entire workload.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Route {
    /// The deterministic CPU/reference implementation.
    Cpu,
    /// A graphics adapter implementation.
    Gpu {
        /// Index in the discovered capability profile.
        adapter_index: usize,
    },
    /// A workload split between a graphics adapter and the CPU reference path.
    Hybrid {
        /// Index in the discovered capability profile for GPU batches.
        adapter_index: usize,
    },
}

/// Why a GPU route was narrowed or rejected.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FallbackReason {
    /// The caller requested the CPU implementation.
    CpuOverride,
    /// No adapter met the current capability requirements.
    NoUsableAdapter,
    /// The measured adapter path did not beat the reference path.
    CalibrationNotBeneficial,
    /// Only part of the workload fit under the live adapter budget.
    BudgetPressure,
    /// Inputs were invalid or too large to plan within bounded limits.
    InvalidRequest,
}

/// Inputs to the resource planner.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkloadRequest {
    /// The operation's execution class.
    pub class: WorkloadClass,
    /// Number of independently processable units, such as faces or tiles.
    pub unit_count: u64,
    /// Estimated bytes needed by one unit on the selected adapter.
    pub bytes_per_unit: u64,
    /// Preferred maximum batch size for incremental processing.
    pub batch_unit_count: u64,
    /// Routing preference from the user or operation policy.
    pub preference: RoutePreference,
    /// Optional measured CPU/GPU comparison.
    pub calibration: Option<CalibrationSample>,
    /// Current authoritative-data location.
    pub residency: DataResidency,
}

/// A planned workload plus the evidence used to make the decision.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WorkloadPlan {
    /// The original workload class.
    pub class: WorkloadClass,
    /// Whole-workload summary route, including an explicit hybrid decision.
    pub route: Route,
    /// Granular work units. Each batch can safely fall back independently.
    pub batches: Vec<WorkBatch>,
    /// Bytes that must cross a non-unified residency boundary.
    pub estimated_transfer_bytes: u64,
    /// The reason a requested GPU path was narrowed, if applicable.
    pub fallback_reason: Option<FallbackReason>,
    /// Adapter budget observed by the planner.
    pub adapter_budget_bytes: u64,
    /// Adapter bytes available after existing ledger reservations.
    pub adapter_available_bytes: u64,
    /// The largest adapter allocation allowed after headroom.
    pub peak_gpu_bytes: u64,
}

/// Runtime routing diagnostics recorded by the viewport and long operations.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RoutingTelemetry {
    /// Adapter selected for the workload, if any.
    pub selected_adapter: Option<usize>,
    /// Route used by the workload.
    pub route: Option<Route>,
    /// Fallback reason, if a fallback occurred.
    pub fallback_reason: Option<FallbackReason>,
    /// Bytes transferred between non-unified domains.
    pub transfer_bytes: u64,
    /// p50 duration when measured by the caller.
    pub frame_time_p50_ms: Option<f64>,
    /// p95 duration when measured by the caller.
    pub frame_time_p95_ms: Option<f64>,
    /// Peak bytes accounted during the operation.
    pub peak_allocated_bytes: u64,
}

/// Plans a bounded CPU/GPU/hybrid workload from one capability profile and ledger.
#[must_use]
pub fn plan_workload(
    profile: &CapabilityProfile,
    request: &WorkloadRequest,
    ledger: &ResourceLedger,
) -> WorkloadPlan {
    let cpu_plan = |reason: Option<FallbackReason>| WorkloadPlan {
        class: request.class,
        route: Route::Cpu,
        batches: make_batches(
            request.unit_count,
            request.batch_unit_count,
            BatchTarget::Cpu,
        ),
        estimated_transfer_bytes: 0,
        fallback_reason: reason,
        adapter_budget_bytes: 0,
        adapter_available_bytes: 0,
        peak_gpu_bytes: 0,
    };

    if request.unit_count == 0
        || request.batch_unit_count == 0
        || request.bytes_per_unit == 0
        || request.unit_count > u64::from(u32::MAX) * MAX_PLANNED_BATCHES
    {
        return cpu_plan(Some(FallbackReason::InvalidRequest));
    }

    let candidate = choose_adapter(profile, request);
    let adapter_index = match candidate {
        Some(index) => index,
        None => {
            let reason = match request.preference {
                RoutePreference::Cpu => Some(FallbackReason::CpuOverride),
                _ => Some(FallbackReason::NoUsableAdapter),
            };
            return cpu_plan(reason);
        }
    };

    let adapter = match profile.adapters.get(adapter_index) {
        Some(adapter) => adapter,
        None => return cpu_plan(Some(FallbackReason::NoUsableAdapter)),
    };
    let adapter_available = match ledger.available_bytes(MemoryDomain::Adapter(adapter_index)) {
        Ok(available) => available,
        Err(_) => return cpu_plan(Some(FallbackReason::NoUsableAdapter)),
    };
    let peak_gpu_bytes = adapter_available.saturating_mul(GPU_BUDGET_HEADROOM_NUMERATOR)
        / GPU_BUDGET_HEADROOM_DENOMINATOR;
    let adapter_budget = adapter.memory.budget_bytes;
    if !should_use_gpu(request, adapter.kind, request.calibration) {
        let reason = match request.preference {
            RoutePreference::Cpu => FallbackReason::CpuOverride,
            _ => FallbackReason::CalibrationNotBeneficial,
        };
        return WorkloadPlan {
            class: request.class,
            route: Route::Cpu,
            batches: make_batches(
                request.unit_count,
                request.batch_unit_count,
                BatchTarget::Cpu,
            ),
            estimated_transfer_bytes: 0,
            fallback_reason: Some(reason),
            adapter_budget_bytes: adapter_budget,
            adapter_available_bytes: adapter_available,
            peak_gpu_bytes: 0,
        };
    }

    let gpu_units = peak_gpu_bytes / request.bytes_per_unit;
    if gpu_units == 0 {
        return WorkloadPlan {
            class: request.class,
            route: Route::Cpu,
            batches: make_batches(
                request.unit_count,
                request.batch_unit_count,
                BatchTarget::Cpu,
            ),
            estimated_transfer_bytes: 0,
            fallback_reason: Some(FallbackReason::BudgetPressure),
            adapter_budget_bytes: adapter_budget,
            adapter_available_bytes: adapter_available,
            peak_gpu_bytes,
        };
    }

    let gpu_units = gpu_units.min(request.unit_count);
    let transfer_bytes = if request.residency.shared_memory
        || request.residency.resident_adapter.is_some()
        || adapter.memory.unified_memory
    {
        0
    } else if request.residency.authoritative_on_cpu {
        request.bytes_per_unit.saturating_mul(gpu_units)
    } else {
        0
    };
    let gpu_route = Route::Gpu { adapter_index };
    let batches = make_hybrid_batches(
        request.unit_count,
        request.batch_unit_count,
        gpu_units,
        BatchTarget::Gpu { adapter_index },
    );
    let is_hybrid = gpu_units < request.unit_count;
    WorkloadPlan {
        class: request.class,
        route: if is_hybrid {
            Route::Hybrid { adapter_index }
        } else {
            gpu_route
        },
        batches,
        estimated_transfer_bytes: transfer_bytes,
        fallback_reason: is_hybrid.then_some(FallbackReason::BudgetPressure),
        adapter_budget_bytes: adapter_budget,
        adapter_available_bytes: adapter_available,
        peak_gpu_bytes,
    }
}

fn choose_adapter(profile: &CapabilityProfile, request: &WorkloadRequest) -> Option<usize> {
    if matches!(request.preference, RoutePreference::Cpu) {
        return None;
    }
    if let RoutePreference::Adapter(index) = request.preference {
        return profile
            .adapters
            .get(index)
            .filter(|adapter| adapter.is_usable)
            .map(|_| index);
    }

    profile
        .adapters
        .iter()
        .enumerate()
        .filter(|(_, adapter)| adapter.is_usable)
        .max_by_key(|(_, adapter)| {
            let display_score = u64::from(adapter.display.has_output) * 4;
            let memory_score = adapter.memory.budget_bytes / (1024 * 1024);
            let topology_score = u64::from(adapter.memory.unified_memory);
            display_score + memory_score + topology_score
        })
        .map(|(index, _)| index)
}

fn should_use_gpu(
    request: &WorkloadRequest,
    kind: crate::AdapterKind,
    calibration: Option<CalibrationSample>,
) -> bool {
    if matches!(request.preference, RoutePreference::Cpu)
        || matches!(request.class, WorkloadClass::SerialCpu)
    {
        return false;
    }
    if let Some(sample) = calibration {
        return sample.gpu_wins();
    }
    if matches!(kind, crate::AdapterKind::Software) {
        return false;
    }
    // Integrated/unified devices share the system pool with authoritative
    // geometry.  Without a measurement, keep CPU-parallel and memory-bound
    // work on the CPU; presentation and explicit GPU compute remain device
    // candidates.  A measured sample above takes precedence.
    matches!(
        request.class,
        WorkloadClass::GpuRender | WorkloadClass::GpuCompute
    ) || (matches!(
        kind,
        crate::AdapterKind::Discrete | crate::AdapterKind::Virtual
    ) && matches!(request.class, WorkloadClass::ParallelCpu))
}

fn make_batches(unit_count: u64, requested_batch_size: u64, target: BatchTarget) -> Vec<WorkBatch> {
    make_hybrid_batches(unit_count, requested_batch_size, 0, target)
}

fn make_hybrid_batches(
    unit_count: u64,
    requested_batch_size: u64,
    gpu_units: u64,
    gpu_target: BatchTarget,
) -> Vec<WorkBatch> {
    let minimum_batch_size =
        unit_count.saturating_add(MAX_PLANNED_BATCHES - 1) / MAX_PLANNED_BATCHES;
    let batch_size = requested_batch_size.max(minimum_batch_size).max(1);
    let mut batches = Vec::new();
    let mut first_unit = 0_u64;
    let mut batch_index = 0_u32;
    while first_unit < unit_count {
        let remaining = unit_count.saturating_sub(first_unit);
        let unit_count_for_batch = if first_unit < gpu_units {
            remaining
                .min(batch_size)
                .min(gpu_units.saturating_sub(first_unit))
        } else {
            remaining.min(batch_size)
        };
        let target = if first_unit < gpu_units {
            let gpu_remaining = gpu_units.saturating_sub(first_unit);
            if gpu_remaining >= unit_count_for_batch {
                gpu_target
            } else {
                BatchTarget::Cpu
            }
        } else {
            BatchTarget::Cpu
        };
        batches.push(WorkBatch {
            index: batch_index,
            first_unit,
            unit_count: unit_count_for_batch,
            target,
        });
        first_unit = first_unit.saturating_add(unit_count_for_batch);
        batch_index = batch_index.saturating_add(1);
        if batch_index == u32::MAX {
            break;
        }
    }
    batches
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AdapterBudget, AdapterKind, AdapterProfile, BackendKind, DisplayRelationship, MemoryProfile,
    };

    fn profile(kind: AdapterKind, unified: bool, budget: u64) -> CapabilityProfile {
        CapabilityProfile {
            cpu: crate::CpuProfile {
                logical_cores: 8,
                physical_cores: 4,
                total_memory_bytes: 1_000_000,
                available_memory_bytes: 900_000,
            },
            os_memory: crate::OsMemoryBudget {
                total_bytes: 1_000_000,
                available_bytes: 900_000,
                source: String::from("test"),
            },
            adapters: vec![AdapterProfile {
                index: 0,
                name: String::from("test adapter"),
                backend: BackendKind::Vulkan,
                kind,
                vendor_id: 0,
                device_id: 0,
                driver: String::from("test"),
                driver_info: String::new(),
                is_usable: true,
                is_display_compatible: true,
                display: DisplayRelationship {
                    has_output: true,
                    output_count: 1,
                    active_display_uncertain: true,
                },
                memory: MemoryProfile {
                    dedicated_bytes: if unified { 0 } else { budget },
                    shared_bytes: if unified { budget } else { 0 },
                    budget_bytes: budget,
                    usage_bytes: 0,
                    available_bytes: budget,
                    unified_memory: unified,
                },
                max_buffer_size: 1_000_000,
                max_texture_dimension_2d: 16_384,
            }],
            discovery_backend: String::from("test"),
        }
    }

    fn request(class: WorkloadClass) -> WorkloadRequest {
        WorkloadRequest {
            class,
            unit_count: 100,
            bytes_per_unit: 10,
            batch_unit_count: 25,
            preference: RoutePreference::Automatic,
            calibration: Some(CalibrationSample {
                cpu_ms: 20.0,
                gpu_ms: 5.0,
                transfer_ms: 1.0,
            }),
            residency: DataResidency {
                authoritative_on_cpu: true,
                ..DataResidency::default()
            },
        }
    }

    #[test]
    fn calibrated_gpu_win_uses_gpu_without_duplicate_unified_transfer() {
        let profile = profile(AdapterKind::Integrated, true, 2_000);
        let ledger = ResourceLedger::new(
            10_000,
            vec![AdapterBudget {
                budget_bytes: 2_000,
                unified_memory: true,
            }],
        );
        let plan = plan_workload(&profile, &request(WorkloadClass::ParallelCpu), &ledger);
        assert_eq!(plan.route, Route::Gpu { adapter_index: 0 });
        assert_eq!(plan.estimated_transfer_bytes, 0);
        assert!(
            plan.batches
                .iter()
                .all(|batch| { batch.target == BatchTarget::Gpu { adapter_index: 0 } })
        );
    }

    #[test]
    fn pressure_splits_work_at_batch_boundaries() {
        let profile = profile(AdapterKind::Discrete, false, 500);
        let ledger = ResourceLedger::new(
            10_000,
            vec![AdapterBudget {
                budget_bytes: 500,
                unified_memory: false,
            }],
        );
        let plan = plan_workload(&profile, &request(WorkloadClass::GpuCompute), &ledger);
        assert_eq!(plan.route, Route::Hybrid { adapter_index: 0 });
        assert_eq!(plan.fallback_reason, Some(FallbackReason::BudgetPressure));
        assert_eq!(plan.estimated_transfer_bytes, 450);
        assert!(
            plan.batches
                .iter()
                .any(|batch| batch.target == BatchTarget::Cpu)
        );
        assert!(
            plan.batches
                .iter()
                .any(|batch| { batch.target == BatchTarget::Gpu { adapter_index: 0 } })
        );
    }

    #[test]
    fn memory_bound_work_without_measurement_uses_cpu_reference() {
        let profile = profile(AdapterKind::Integrated, true, 2_000);
        let mut request = request(WorkloadClass::MemoryBound);
        request.calibration = None;
        let ledger = ResourceLedger::new(
            10_000,
            vec![AdapterBudget {
                budget_bytes: 2_000,
                unified_memory: true,
            }],
        );
        let plan = plan_workload(&profile, &request, &ledger);
        assert_eq!(plan.route, Route::Cpu);
        assert_eq!(
            plan.fallback_reason,
            Some(FallbackReason::CalibrationNotBeneficial)
        );
    }

    #[test]
    fn uncalibrated_integrated_parallel_work_stays_on_cpu_but_discrete_can_use_gpu() {
        let integrated = profile(AdapterKind::Integrated, true, 2_000);
        let discrete = profile(AdapterKind::Discrete, false, 2_000);
        let mut request = request(WorkloadClass::ParallelCpu);
        request.calibration = None;
        let integrated_ledger = ResourceLedger::new(
            10_000,
            vec![AdapterBudget {
                budget_bytes: 2_000,
                unified_memory: true,
            }],
        );
        let discrete_ledger = ResourceLedger::new(
            10_000,
            vec![AdapterBudget {
                budget_bytes: 2_000,
                unified_memory: false,
            }],
        );
        assert_eq!(
            plan_workload(&integrated, &request, &integrated_ledger).route,
            Route::Cpu
        );
        assert_eq!(
            plan_workload(&discrete, &request, &discrete_ledger).route,
            Route::Gpu { adapter_index: 0 }
        );
    }
}
