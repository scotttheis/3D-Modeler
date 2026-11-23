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
#[cfg(windows)]
mod window_chrome;
#[cfg(windows)]
pub use window_chrome::set_title_bar_colors;
pub mod updates;

pub use console::attach_console;

/// Reports whether the Windows repair engine DLL is installed under SystemRoot.
/// Returns false on other platforms; Windows activation may still fail at runtime.
#[must_use]
pub fn windows_repair_supported() -> bool {
    #[cfg(windows)]
    {
        std::env::var_os("SystemRoot").is_some_and(|root| {
            std::path::PathBuf::from(root)
                .join("System32/Windows.Graphics.Printing.3D.dll")
                .is_file()
        })
    }
    #[cfg(not(windows))]
    {
        false
    }
}

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
    current_runtime_paths, default_runtime_paths, documents_directory, ensure_opengl_gui_platform,
    process_is_alive, resolve_runtime_descriptor_path,
};
pub use slicer::{
    SlicerError, SlicerInfo, SlicerLaunch, discover_slicers, launch_discovered_slicer,
    launch_slicer, slicer_info_from_path,
};

#[cfg(test)]
mod repair_tests {
    #[test]
    fn repair_support_matches_the_host_installation() {
        #[cfg(windows)]
        let expected = std::env::var_os("SystemRoot").is_some_and(|root| {
            std::path::PathBuf::from(root)
                .join("System32/Windows.Graphics.Printing.3D.dll")
                .is_file()
        });
        #[cfg(not(windows))]
        let expected = false;
        assert_eq!(super::windows_repair_supported(), expected);
    }
}
