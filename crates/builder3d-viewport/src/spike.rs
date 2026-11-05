use std::{
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};

const BYTES_PER_PIXEL: u64 = 4;
const COPY_BYTES_PER_ROW_ALIGNMENT: u64 = 256;
const MAX_DIMENSION: u32 = 4_096;

/// Configuration for the offscreen R0b renderer control.
#[derive(Clone, Debug)]
pub struct RenderSpikeConfig {
    /// Width of the offscreen capture.
    pub width: u32,
    /// Height of the offscreen capture.
    pub height: u32,
    /// Number of triangle instances submitted to the GPU.
    pub face_count: u64,
    /// Adapter index from the capability profile, or automatic selection.
    pub adapter_index: Option<usize>,
    /// Number of frames to warm before timing.
    pub warmup_frames: u32,
    /// Number of frames included in the timing sample.
    pub measured_frames: u32,
    /// PPM output path for the final rendered frame.
    pub capture_path: PathBuf,
}

impl Default for RenderSpikeConfig {
    fn default() -> Self {
        Self {
            width: 512,
            height: 512,
            face_count: 30_000_000,
            adapter_index: None,
            warmup_frames: 1,
            measured_frames: 7,
            capture_path: PathBuf::from("evidence/r0b/30m-control.ppm"),
        }
    }
}

/// Measurements and adapter identity from one render spike.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RenderMeasurement {
    /// Adapter name from wgpu.
    pub adapter_name: String,
    /// Backend name from wgpu.
    pub backend: String,
    /// Device class from wgpu.
    pub device_type: String,
    /// Driver name from wgpu.
    pub driver: String,
    /// Driver details from wgpu.
    pub driver_info: String,
    /// Number of submitted triangle instances.
    pub face_count: u64,
    /// Capture width.
    pub width: u32,
    /// Capture height.
    pub height: u32,
    /// Time to submit and complete the first useful frame in milliseconds.
    pub first_useful_frame_ms: f64,
    /// Steady-state frame completion samples in milliseconds.
    pub frame_times_ms: Vec<f64>,
    /// Median steady-state completion time.
    pub p50_ms: f64,
    /// P95 steady-state completion time.
    pub p95_ms: f64,
    /// Maximum steady-state completion time.
    pub max_ms: f64,
    /// Total readback bytes copied for the capture.
    pub capture_readback_bytes: u64,
    /// Capture path as displayed in the report.
    pub capture_path: String,
}

/// Errors from configuration, adapter/device setup, rendering, readback, or capture.
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    /// The requested control would exceed bounded spike limits.
    #[error("invalid render spike configuration: {0}")]
    InvalidConfiguration(String),
    /// No adapter matched the requested index or automatic policy.
    #[error("no usable wgpu adapter was found")]
    NoAdapter,
    /// A graphics device could not be opened.
    #[error("wgpu device creation failed: {0}")]
    Device(String),
    /// The readback callback did not complete in the bounded wait.
    #[error("wgpu readback did not complete")]
    ReadbackTimeout,
    /// The readback failed at the graphics API boundary.
    #[error("wgpu readback failed: {0}")]
    Readback(String),
    /// The capture file could not be written.
    #[error("capture write failed: {0}")]
    Io(#[from] std::io::Error),
}

/// Renders the triangle-control workload on a real wgpu adapter and captures it.
pub async fn run_render_spike(
    config: &RenderSpikeConfig,
) -> Result<RenderMeasurement, RenderError> {
    validate_config(config)?;
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapters = instance.enumerate_adapters(wgpu::Backends::all()).await;
    let adapter = select_adapter(adapters, config.adapter_index)?;
    let adapter_info = adapter.get_info();
    let device_descriptor = wgpu::DeviceDescriptor {
        label: Some("builder3d-r0b-spike"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        memory_hints: wgpu::MemoryHints::MemoryUsage,
        trace: wgpu::Trace::Off,
        ..Default::default()
    };
    let (device, queue) = adapter
        .request_device(&device_descriptor)
        .await
        .map_err(|error| RenderError::Device(error.to_string()))?;

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("builder3d-r0b-capture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("builder3d-r0b-procedural-control"),
        source: wgpu::ShaderSource::Wgsl(CONTROL_SHADER.into()),
    });
    let color_targets = [Some(wgpu::ColorTargetState {
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    })];
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("builder3d-r0b-procedural-pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment_main"),
            compilation_options: Default::default(),
            targets: &color_targets,
        }),
        multiview_mask: None,
        cache: None,
    });

    let first_started = Instant::now();
    submit_frame(
        &device,
        &queue,
        &texture,
        &view,
        &pipeline,
        config.face_count,
        None,
    );
    wait_for_gpu(&instance);
    let first_useful_frame_ms = elapsed_ms(first_started);

    for _ in 0..config.warmup_frames {
        submit_frame(
            &device,
            &queue,
            &texture,
            &view,
            &pipeline,
            config.face_count,
            None,
        );
        wait_for_gpu(&instance);
    }

    let mut frame_times_ms = Vec::with_capacity(config.measured_frames as usize);
    for _ in 0..config.measured_frames {
        let started = Instant::now();
        submit_frame(
            &device,
            &queue,
            &texture,
            &view,
            &pipeline,
            config.face_count,
            None,
        );
        wait_for_gpu(&instance);
        frame_times_ms.push(elapsed_ms(started));
    }

    let padded_bytes_per_row = padded_bytes_per_row(config.width)?;
    let readback_bytes = padded_bytes_per_row
        .checked_mul(u64::from(config.height))
        .ok_or_else(|| RenderError::InvalidConfiguration(String::from("readback size overflow")))?;
    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("builder3d-r0b-readback"),
        size: readback_bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    submit_frame(
        &device,
        &queue,
        &texture,
        &view,
        &pipeline,
        config.face_count,
        Some((&readback_buffer, padded_bytes_per_row)),
    );
    wait_for_gpu(&instance);
    readback_to_ppm(
        &instance,
        &readback_buffer,
        config.width,
        config.height,
        padded_bytes_per_row,
        &config.capture_path,
    )?;

    let (p50_ms, p95_ms, max_ms) = summarize(&frame_times_ms).ok_or_else(|| {
        RenderError::InvalidConfiguration(String::from("no timing samples were collected"))
    })?;
    Ok(RenderMeasurement {
        adapter_name: adapter_info.name,
        backend: format!("{:?}", adapter_info.backend),
        device_type: format!("{:?}", adapter_info.device_type),
        driver: adapter_info.driver,
        driver_info: adapter_info.driver_info,
        face_count: config.face_count,
        width: config.width,
        height: config.height,
        first_useful_frame_ms,
        frame_times_ms,
        p50_ms,
        p95_ms,
        max_ms,
        capture_readback_bytes: readback_bytes,
        capture_path: config.capture_path.display().to_string(),
    })
}

fn validate_config(config: &RenderSpikeConfig) -> Result<(), RenderError> {
    if config.width == 0
        || config.height == 0
        || config.width > MAX_DIMENSION
        || config.height > MAX_DIMENSION
    {
        return Err(RenderError::InvalidConfiguration(String::from(
            "capture dimensions must be between 1 and 4096",
        )));
    }
    if config.face_count == 0 || config.face_count > u64::from(u32::MAX) {
        return Err(RenderError::InvalidConfiguration(String::from(
            "face count must fit one bounded instanced draw",
        )));
    }
    if config.measured_frames == 0 || config.measured_frames > 1_024 {
        return Err(RenderError::InvalidConfiguration(String::from(
            "measured frame count must be between 1 and 1024",
        )));
    }
    if config.warmup_frames > 1_024 {
        return Err(RenderError::InvalidConfiguration(String::from(
            "warmup frame count must be at most 1024",
        )));
    }
    let _ = padded_bytes_per_row(config.width)?;
    Ok(())
}

fn padded_bytes_per_row(width: u32) -> Result<u64, RenderError> {
    let row_bytes = u64::from(width)
        .checked_mul(BYTES_PER_PIXEL)
        .ok_or_else(|| RenderError::InvalidConfiguration(String::from("row size overflow")))?;
    let padding = COPY_BYTES_PER_ROW_ALIGNMENT - (row_bytes % COPY_BYTES_PER_ROW_ALIGNMENT);
    Ok(if padding == COPY_BYTES_PER_ROW_ALIGNMENT {
        row_bytes
    } else {
        row_bytes + padding
    })
}

fn select_adapter(
    adapters: Vec<wgpu::Adapter>,
    requested_index: Option<usize>,
) -> Result<wgpu::Adapter, RenderError> {
    if let Some(index) = requested_index {
        return adapters
            .into_iter()
            .nth(index)
            .ok_or(RenderError::NoAdapter);
    }
    adapters
        .iter()
        .find(|adapter| !matches!(adapter.get_info().device_type, wgpu::DeviceType::Cpu))
        .cloned()
        .or_else(|| adapters.into_iter().next())
        .ok_or(RenderError::NoAdapter)
}

fn submit_frame(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    view: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    face_count: u64,
    readback: Option<(&wgpu::Buffer, u64)>,
) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("builder3d-r0b-frame"),
    });
    let color_attachments = [Some(wgpu::RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.015,
                g: 0.02,
                b: 0.035,
                a: 1.0,
            }),
            store: wgpu::StoreOp::Store,
        },
    })];
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("builder3d-r0b-control-pass"),
            color_attachments: &color_attachments,
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.draw(0..3, 0..face_count as u32);
    }
    if let Some((buffer, bytes_per_row)) = readback {
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row as u32),
                    rows_per_image: Some(texture.size().height),
                },
            },
            texture.size(),
        );
    }
    queue.submit([encoder.finish()]);
}

fn wait_for_gpu(instance: &wgpu::Instance) {
    let _ = instance.poll_all(true);
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1_000.0
}

fn readback_to_ppm(
    instance: &wgpu::Instance,
    buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    padded_bytes_per_row: u64,
    capture_path: &Path,
) -> Result<(), RenderError> {
    let slice = buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result.map_err(|error| format!("{error:?}")));
    });
    let started = Instant::now();
    let mapping = loop {
        match receiver.try_recv() {
            Ok(result) => break result,
            Err(mpsc::TryRecvError::Empty) => {
                if started.elapsed() > Duration::from_secs(30) {
                    return Err(RenderError::ReadbackTimeout);
                }
                let _ = instance.poll_all(true);
                std::thread::yield_now();
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(RenderError::ReadbackTimeout);
            }
        }
    };
    mapping.map_err(RenderError::Readback)?;
    let mapped = buffer
        .slice(..)
        .get_mapped_range()
        .map_err(|error| RenderError::Readback(format!("{error:?}")))?;
    let row_stride = usize::try_from(padded_bytes_per_row)
        .map_err(|_| RenderError::InvalidConfiguration(String::from("row stride too large")))?;
    let width = usize::try_from(width)
        .map_err(|_| RenderError::InvalidConfiguration(String::from("width too large")))?;
    let height = usize::try_from(height)
        .map_err(|_| RenderError::InvalidConfiguration(String::from("height too large")))?;
    let output_size = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or_else(|| RenderError::InvalidConfiguration(String::from("capture size overflow")))?;
    let mut output = Vec::with_capacity(output_size + 32);
    output.extend_from_slice(format!("P6\n{width} {height}\n255\n").as_bytes());
    for row in 0..height {
        let start = row.checked_mul(row_stride).ok_or_else(|| {
            RenderError::InvalidConfiguration(String::from("row offset overflow"))
        })?;
        let end = start
            .checked_add(width.checked_mul(4).ok_or_else(|| {
                RenderError::InvalidConfiguration(String::from("pixel row overflow"))
            })?)
            .ok_or_else(|| RenderError::InvalidConfiguration(String::from("row end overflow")))?;
        let source = mapped
            .get(start..end)
            .ok_or_else(|| RenderError::Readback(String::from("mapped row was truncated")))?;
        for pixel in source.as_chunks::<4>().0 {
            output.extend_from_slice(&pixel[..3]);
        }
    }
    drop(mapped);
    buffer.unmap();
    if let Some(parent) = capture_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(capture_path, output)?;
    Ok(())
}

fn summarize(values: &[f64]) -> Option<(f64, f64, f64)> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let p50 = sorted[(sorted.len() - 1) / 2];
    let p95_index = ((sorted.len() * 95).saturating_add(99) / 100).saturating_sub(1);
    let p95 = sorted[p95_index.min(sorted.len() - 1)];
    let max = *sorted.last()?;
    Some((p50, p95, max))
}

const CONTROL_SHADER: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vertex_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
    let local = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
    );
    let grid_width = 4096.0;
    let grid_height = 8192.0;
    let x = f32(instance_index % 4096u);
    let y = f32(instance_index / 4096u);
    let cell = local[vertex_index];
    let position = vec2<f32>(
        -1.0 + (x + cell.x) * (2.0 / grid_width),
        -1.0 + (y + cell.y) * (2.0 / grid_height),
    );
    let hue = f32(instance_index % 97u) / 96.0;
    return VertexOutput(
        vec4<f32>(position, 0.0, 1.0),
        vec3<f32>(0.2 + hue * 0.7, 0.35 + (1.0 - hue) * 0.55, 0.75),
    );
}

@fragment
fn fragment_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_pitch_is_aligned_to_wgpu_copy_requirement() {
        assert_eq!(padded_bytes_per_row(512).ok(), Some(2_048));
        assert_eq!(padded_bytes_per_row(513).ok(), Some(2_304));
    }

    #[test]
    fn summary_uses_bounded_percentiles() {
        assert_eq!(summarize(&[4.0, 1.0, 2.0, 3.0]), Some((2.0, 4.0, 4.0)));
        assert_eq!(summarize(&[]), None);
    }
}
