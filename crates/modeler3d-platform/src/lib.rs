//! Platform capability discovery and resource-aware workload planning.
//!
//! This crate deliberately owns the hardware policy inputs instead of letting
//! the viewport or a long-running core operation invent a separate GPU rule.
//! The public data model is portable; the discovery adapter supplies native
//! memory-budget and display information when the host exposes it.

mod console;
mod discovery;
mod ledger;
mod legacy_settings;
mod routing;
mod runtime;
mod slicer;
pub mod updates;

pub use console::attach_console;
pub use discovery::{
    AdapterKind, AdapterProfile, BackendKind, CapabilityProfile, CpuProfile, DiscoveryError,
    DisplayRelationship, MemoryProfile, OsMemoryBudget, discover_capability_profile,
};
pub use ledger::{
    AdapterBudget, AllocationId, AllocationKind, LedgerError, MemoryDomain, ResourceLedger,
    ResourceLedgerSnapshot,
};
pub use legacy_settings::{LegacySettings, read_legacy_settings};
pub use routing::{
    BatchTarget, CalibrationSample, DataResidency, FallbackReason, Route, RoutePreference,
    RoutingTelemetry, WorkBatch, WorkloadClass, WorkloadPlan, WorkloadRequest, plan_workload,
};
pub use runtime::{
    APP_NAME, RUNTIME_DESCRIPTOR_ENV, RUNTIME_DESCRIPTOR_NAME, RuntimeCheckError, RuntimePaths,
    current_runtime_paths, default_runtime_paths, ensure_opengl_gui_platform, process_is_alive,
    resolve_runtime_descriptor_path,
};
pub use slicer::{
    SlicerError, SlicerInfo, SlicerLaunch, discover_slicers, launch_discovered_slicer,
    launch_slicer, slicer_info_from_path,
};
