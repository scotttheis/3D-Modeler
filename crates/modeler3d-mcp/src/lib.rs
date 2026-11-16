//! The R0b MCP transport spike.

mod r4;

use modeler3d_platform::CapabilityProfile;
use rmcp::{handler::server::wrapper::Parameters, schemars::JsonSchema, tool, tool_router};
use serde::{Deserialize, Serialize};

pub use r4::{
    AdapterError, ApprovedRoots, CATALOG_VERSION, Profile, R4Config, R4Document, R4Gateway,
    R4Server, SHELL_TOOLS, SavedSlicer, TOOL_CATALOG, ToolSpec, ViewState,
};

/// Input for the bounded R0b device-profile tool.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct DeviceProfileRequest {
    /// Include the full adapter and budget snapshot when true.
    pub detailed: Option<bool>,
}

/// The one semantic tool exposed by the R0b server.
#[derive(Clone, Debug)]
pub struct R0bServer {
    profile: CapabilityProfile,
}

impl R0bServer {
    /// Creates a server over one immutable capability snapshot.
    #[must_use]
    pub const fn new(profile: CapabilityProfile) -> Self {
        Self { profile }
    }
}

#[tool_router(server_handler)]
impl R0bServer {
    /// Returns the immutable startup device/resource profile used by the spike.
    #[tool(
        name = "device_profile",
        description = "Return the measured CPU, RAM, adapter, display, and memory-budget profile used by 3D Modeler R0b."
    )]
    fn device_profile(&self, Parameters(request): Parameters<DeviceProfileRequest>) -> String {
        let detail = request.detailed.unwrap_or(false);
        let result = if detail {
            serde_json::to_string(&self.profile)
        } else {
            serde_json::to_string(&ProfileSummary::from(&self.profile))
        };
        match result {
            Ok(json) => json,
            Err(error) => format!("profile serialization failed: {error}"),
        }
    }
}

#[derive(Debug, Serialize)]
struct ProfileSummary {
    logical_cores: usize,
    physical_cores: usize,
    total_memory_bytes: u64,
    available_memory_bytes: u64,
    adapter_count: usize,
    usable_adapter_count: usize,
    discovery_backend: String,
}

impl From<&CapabilityProfile> for ProfileSummary {
    fn from(profile: &CapabilityProfile) -> Self {
        Self {
            logical_cores: profile.cpu.logical_cores,
            physical_cores: profile.cpu.physical_cores,
            total_memory_bytes: profile.cpu.total_memory_bytes,
            available_memory_bytes: profile.cpu.available_memory_bytes,
            adapter_count: profile.adapters.len(),
            usable_adapter_count: profile
                .adapters
                .iter()
                .filter(|adapter| adapter.is_usable)
                .count(),
            discovery_backend: profile.discovery_backend.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use modeler3d_platform::{CpuProfile, OsMemoryBudget};

    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            cpu: CpuProfile {
                logical_cores: 2,
                physical_cores: 1,
                total_memory_bytes: 100,
                available_memory_bytes: 80,
            },
            os_memory: OsMemoryBudget {
                total_bytes: 100,
                available_bytes: 80,
                source: String::from("test"),
            },
            adapters: Vec::new(),
            discovery_backend: String::from("test"),
        }
    }

    #[test]
    fn summary_is_compact_and_detail_is_explicit() {
        let server = R0bServer::new(profile());
        let summary = server.device_profile(Parameters(DeviceProfileRequest { detailed: None }));
        assert!(summary.contains("adapter_count"));
        let detailed = server.device_profile(Parameters(DeviceProfileRequest {
            detailed: Some(true),
        }));
        assert!(detailed.contains("os_memory"));
    }
}
