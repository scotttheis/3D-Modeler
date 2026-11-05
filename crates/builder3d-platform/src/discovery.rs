use serde::{Deserialize, Serialize};
use sysinfo::System;

/// Broad adapter class reported by the graphics API, independent of vendor name.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum AdapterKind {
    /// An integrated graphics processor.
    Integrated,
    /// A device with dedicated local video memory.
    Discrete,
    /// A virtual adapter exposed by the operating system or hypervisor.
    Virtual,
    /// A software or CPU adapter.
    Software,
    /// An adapter whose API type is not classified by the backend.
    Other,
}

/// The graphics backend selected by the API adapter.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum BackendKind {
    /// Direct3D 12 on Windows.
    Dx12,
    /// Vulkan.
    Vulkan,
    /// OpenGL or GLES.
    Gl,
    /// Metal.
    Metal,
    /// WebGPU in a browser host.
    WebGpu,
    /// The software/no-op backend.
    Noop,
    /// A backend added after this build's mapping.
    Other,
}

/// Relationship between an adapter and display outputs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DisplayRelationship {
    /// Whether DXGI/wgpu found at least one output on this adapter.
    pub has_output: bool,
    /// Number of outputs reported by the native adapter query.
    pub output_count: u32,
    /// Windows output enumeration does not by itself prove which output is
    /// currently driving the desktop; this remains explicit in diagnostics.
    pub active_display_uncertain: bool,
}

/// Adapter-local and shared-memory measurements.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct MemoryProfile {
    /// Dedicated local video memory from the native adapter query.
    pub dedicated_bytes: u64,
    /// Shared system memory visible to the adapter.
    pub shared_bytes: u64,
    /// Current OS-reported budget for the chosen memory segment.
    pub budget_bytes: u64,
    /// Current usage in that memory segment.
    pub usage_bytes: u64,
    /// Budget minus current usage.
    pub available_bytes: u64,
    /// Whether CPU and adapter allocations draw from one physical pool.
    pub unified_memory: bool,
}

/// A discovered graphics adapter and its routing-relevant capabilities.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdapterProfile {
    /// Stable index within this discovery result.
    pub index: usize,
    /// API-reported adapter name.
    pub name: String,
    /// API backend.
    pub backend: BackendKind,
    /// API-reported device category.
    pub kind: AdapterKind,
    /// PCI/vendor identifier when the API provides it.
    pub vendor_id: u32,
    /// Device identifier when the API provides it.
    pub device_id: u32,
    /// Driver name/version string supplied by the backend.
    pub driver: String,
    /// Backend-specific driver details.
    pub driver_info: String,
    /// Whether the adapter can create a usable device with the current build.
    pub is_usable: bool,
    /// Whether it is compatible with at least one display output.
    pub is_display_compatible: bool,
    /// Output/display relationship.
    pub display: DisplayRelationship,
    /// Memory topology and live budget.
    pub memory: MemoryProfile,
    /// Maximum buffer allocation supported by the adapter.
    pub max_buffer_size: u64,
    /// Maximum 2D texture dimension supported by the adapter.
    pub max_texture_dimension_2d: u32,
}

/// CPU topology and system-memory observations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CpuProfile {
    /// Logical processors available to this process.
    pub logical_cores: usize,
    /// Physical cores when the host query provides them.
    pub physical_cores: usize,
    /// Total system memory in bytes.
    pub total_memory_bytes: u64,
    /// Memory currently available to the process/system in bytes.
    pub available_memory_bytes: u64,
}

/// Live OS memory values used for CPU and unified-memory planning.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OsMemoryBudget {
    /// Total physical memory in bytes.
    pub total_bytes: u64,
    /// Available system memory in bytes.
    pub available_bytes: u64,
    /// Native query used to populate the values.
    pub source: String,
}

/// Complete capability snapshot consumed by resource planners and diagnostics.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CapabilityProfile {
    /// CPU and RAM capabilities.
    pub cpu: CpuProfile,
    /// OS-level memory budget.
    pub os_memory: OsMemoryBudget,
    /// Every graphics adapter visible to wgpu.
    pub adapters: Vec<AdapterProfile>,
    /// Description of the discovery paths used for this snapshot.
    pub discovery_backend: String,
}

/// Discovery failures that should be surfaced without panicking.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DiscoveryError {
    /// The host returned no usable CPU parallelism information.
    #[error("the host reported no logical processors")]
    NoLogicalProcessors,
    /// The system information provider could not initialize.
    #[error("system capability discovery could not initialize")]
    SystemUnavailable,
}

#[derive(Clone, Debug)]
struct NativeAdapter {
    vendor_id: u32,
    device_id: u32,
    dedicated_bytes: u64,
    shared_bytes: u64,
    budget_bytes: u64,
    usage_bytes: u64,
    output_count: u32,
}

/// Enumerates the current CPU, RAM, graphics, display, and memory-budget profile.
///
/// The wgpu adapter query is portable. On Windows, the native supplement uses
/// DXGI's live local/non-local budget counters and output enumeration. A host
/// that cannot provide the native supplement is still returned with an explicit
/// unknown/zero native field rather than a guessed vendor policy.
pub async fn discover_capability_profile() -> Result<CapabilityProfile, DiscoveryError> {
    let mut system = System::new_all().map_err(|_| DiscoveryError::SystemUnavailable)?;
    system.refresh_memory();
    let logical_cores = system.cpus().len();
    if logical_cores == 0 {
        return Err(DiscoveryError::NoLogicalProcessors);
    }
    let physical_cores = System::physical_core_count().unwrap_or(logical_cores);
    let total_memory_bytes = system.total_memory();
    let available_memory_bytes = system.available_memory();
    let os_memory = OsMemoryBudget {
        total_bytes: total_memory_bytes,
        available_bytes: available_memory_bytes,
        source: String::from("sysinfo"),
    };
    let cpu = CpuProfile {
        logical_cores,
        physical_cores,
        total_memory_bytes,
        available_memory_bytes,
    };

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapters = instance.enumerate_adapters(wgpu::Backends::all()).await;
    let native_adapters = native_adapters();
    let profiles = adapters
        .iter()
        .enumerate()
        .map(|(index, adapter)| {
            let info = adapter.get_info();
            let kind = adapter_kind(info.device_type);
            let native = native_adapters.iter().find(|candidate| {
                candidate.vendor_id == info.vendor && candidate.device_id == info.device
            });
            let memory = memory_profile(native, &kind, &os_memory);
            let display = DisplayRelationship {
                has_output: native.is_some_and(|candidate| candidate.output_count > 0),
                output_count: native.map_or(0, |candidate| candidate.output_count),
                active_display_uncertain: native.is_some(),
            };
            let limits = adapter.limits();
            AdapterProfile {
                index,
                name: info.name,
                backend: backend_kind(info.backend),
                kind,
                vendor_id: info.vendor,
                device_id: info.device,
                driver: info.driver,
                driver_info: info.driver_info,
                is_usable: !matches!(kind, AdapterKind::Software) && limits.max_buffer_size > 0,
                is_display_compatible: display.has_output,
                display,
                memory,
                max_buffer_size: limits.max_buffer_size,
                max_texture_dimension_2d: limits.max_texture_dimension_2d,
            }
        })
        .collect();

    Ok(CapabilityProfile {
        cpu,
        os_memory,
        adapters: profiles,
        discovery_backend: if cfg!(windows) {
            String::from("wgpu + DXGI + sysinfo")
        } else {
            String::from("wgpu + sysinfo")
        },
    })
}

fn adapter_kind(device_type: wgpu::DeviceType) -> AdapterKind {
    match device_type {
        wgpu::DeviceType::DiscreteGpu => AdapterKind::Discrete,
        wgpu::DeviceType::IntegratedGpu => AdapterKind::Integrated,
        wgpu::DeviceType::VirtualGpu => AdapterKind::Virtual,
        wgpu::DeviceType::Cpu => AdapterKind::Software,
        wgpu::DeviceType::Other => AdapterKind::Other,
    }
}

fn backend_kind(backend: wgpu::Backend) -> BackendKind {
    match format!("{backend:?}").as_str() {
        "Dx12" => BackendKind::Dx12,
        "Vulkan" => BackendKind::Vulkan,
        "Gl" | "Gles" => BackendKind::Gl,
        "Metal" => BackendKind::Metal,
        "BrowserWebGpu" | "WebGpu" => BackendKind::WebGpu,
        "Noop" => BackendKind::Noop,
        _ => BackendKind::Other,
    }
}

fn memory_profile(
    native: Option<&NativeAdapter>,
    kind: &AdapterKind,
    os_memory: &OsMemoryBudget,
) -> MemoryProfile {
    let (dedicated_bytes, shared_bytes, budget_bytes, usage_bytes) = match native {
        Some(native) => (
            native.dedicated_bytes,
            native.shared_bytes,
            native.budget_bytes,
            native.usage_bytes,
        ),
        None => (0, os_memory.available_bytes, os_memory.available_bytes, 0),
    };
    let unified_memory = native.is_some()
        && shared_bytes > dedicated_bytes
        && !matches!(kind, AdapterKind::Discrete | AdapterKind::Software);
    MemoryProfile {
        dedicated_bytes,
        shared_bytes,
        budget_bytes,
        usage_bytes,
        available_bytes: budget_bytes.saturating_sub(usage_bytes),
        unified_memory,
    }
}

#[cfg(windows)]
fn native_adapters() -> Vec<NativeAdapter> {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_MEMORY_SEGMENT_GROUP_NON_LOCAL,
        IDXGIAdapter3, IDXGIFactory6,
    };
    use windows::core::Interface;

    let factory = match unsafe { CreateDXGIFactory1::<IDXGIFactory6>() } {
        Ok(factory) => factory,
        Err(_) => return Vec::new(),
    };
    let mut result = Vec::new();
    let mut index = 0_u32;
    while let Ok(adapter) = unsafe { factory.EnumAdapters1(index) } {
        index = index.saturating_add(1);
        let desc = match unsafe { adapter.GetDesc1() } {
            Ok(desc) => desc,
            Err(_) => continue,
        };
        let adapter3 = adapter.cast::<IDXGIAdapter3>().ok();
        let local = query_memory(adapter3.as_ref(), DXGI_MEMORY_SEGMENT_GROUP_LOCAL);
        let non_local = query_memory(adapter3.as_ref(), DXGI_MEMORY_SEGMENT_GROUP_NON_LOCAL);
        let dedicated_bytes = desc.DedicatedVideoMemory as u64;
        let shared_bytes = desc.SharedSystemMemory as u64;
        let budget_bytes = local
            .map(|memory| memory.Budget)
            .filter(|budget| *budget > 0)
            .or_else(|| non_local.map(|memory| memory.Budget))
            .unwrap_or(dedicated_bytes.max(shared_bytes));
        let usage_bytes = local
            .map(|memory| memory.CurrentUsage)
            .or_else(|| non_local.map(|memory| memory.CurrentUsage))
            .unwrap_or(0);
        let mut output_count = 0_u32;
        while unsafe { adapter.EnumOutputs(output_count) }.is_ok() {
            output_count = output_count.saturating_add(1);
        }
        result.push(NativeAdapter {
            vendor_id: desc.VendorId,
            device_id: desc.DeviceId,
            dedicated_bytes,
            shared_bytes,
            budget_bytes,
            usage_bytes,
            output_count,
        });
    }
    result
}

#[cfg(windows)]
fn query_memory(
    adapter: Option<&windows::Win32::Graphics::Dxgi::IDXGIAdapter3>,
    group: windows::Win32::Graphics::Dxgi::DXGI_MEMORY_SEGMENT_GROUP,
) -> Option<windows::Win32::Graphics::Dxgi::DXGI_QUERY_VIDEO_MEMORY_INFO> {
    let adapter = adapter?;
    let mut memory = std::mem::MaybeUninit::<
        windows::Win32::Graphics::Dxgi::DXGI_QUERY_VIDEO_MEMORY_INFO,
    >::uninit();
    if unsafe { adapter.QueryVideoMemoryInfo(0, group, memory.as_mut_ptr()) }.is_ok() {
        Some(unsafe { memory.assume_init() })
    } else {
        None
    }
}

#[cfg(not(windows))]
fn native_adapters() -> Vec<NativeAdapter> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_native_memory_keeps_system_budget_visible() {
        let profile = memory_profile(
            None,
            &AdapterKind::Integrated,
            &OsMemoryBudget {
                total_bytes: 100,
                available_bytes: 70,
                source: String::from("test"),
            },
        );
        assert_eq!(profile.budget_bytes, 70);
        assert!(!profile.unified_memory);
    }
}
