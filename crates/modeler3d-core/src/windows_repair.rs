//! Isolated adapter for the public Windows Printing3D repair API.

#[cfg(not(windows))]
use std::time::Duration;

use crate::Mesh;

#[derive(Debug)]
pub(crate) enum WindowsRepairError {
    Unavailable,
    #[cfg_attr(not(windows), allow(dead_code))]
    TimedOut,
    #[cfg_attr(not(windows), allow(dead_code))]
    Cancelled,
    #[cfg_attr(not(windows), allow(dead_code))]
    Failed(String),
}

#[cfg(not(windows))]
pub(crate) fn run_windows_repair(
    _mesh: &Mesh,
    _time_limit: Duration,
    _cancelled: &dyn Fn() -> bool,
    _elapsed: &mut dyn FnMut(Duration),
) -> Result<Mesh, WindowsRepairError> {
    Err(WindowsRepairError::Unavailable)
}

#[cfg(windows)]
pub(crate) use native::run_windows_repair;

#[cfg(windows)]
mod native {
    use std::time::{Duration, Instant};

    use windows::{
        Graphics::Printing3D::{
            Printing3DBufferDescription, Printing3DBufferFormat, Printing3DMesh, Printing3DModel,
            Printing3DModelUnit,
        },
        Storage::Streams::{ByteOrder, DataReader, IBuffer},
        Win32::System::WinRT::IBufferByteAccess,
        core::Interface,
    };

    use super::{Mesh, WindowsRepairError};

    impl From<windows::core::Error> for WindowsRepairError {
        fn from(error: windows::core::Error) -> Self {
            Self::Failed(error.to_string())
        }
    }

    fn failed(reason: &str) -> WindowsRepairError {
        WindowsRepairError::Failed(reason.to_owned())
    }

    pub(crate) fn run_windows_repair(
        mesh: &Mesh,
        time_limit: Duration,
        cancelled: &dyn Fn() -> bool,
        elapsed: &mut dyn FnMut(Duration),
    ) -> Result<Mesh, WindowsRepairError> {
        mesh.validate_non_empty()
            .map_err(|error| failed(&error.to_string()))?;
        if cancelled() {
            return Err(WindowsRepairError::Cancelled);
        }
        // windows-rs safely activates apartment-agnostic WinRT classes, including
        // on the desktop worker's fresh thread; no manual COM calls are needed.
        let model = Printing3DModel::new().map_err(|_| WindowsRepairError::Unavailable)?;
        let native = Printing3DMesh::new()?;
        model.SetUnit(Printing3DModelUnit::Millimeter)?;
        let vertex_count =
            u32::try_from(mesh.vertex_count()).map_err(|_| failed("Too many vertices"))?;
        let face_count =
            u32::try_from(mesh.face_count()).map_err(|_| failed("Too many triangles"))?;
        native.SetVertexCount(vertex_count)?;
        native.SetIndexCount(face_count)?;
        native.SetVertexPositionsDescription(Printing3DBufferDescription {
            Format: Printing3DBufferFormat::Printing3DDouble,
            Stride: 3,
        })?;
        // The engine's own layout (what it returns): doubles and unsigned
        // integers with a stride of three components per record. A float32
        // layout is rejected with "The parameter is incorrect".
        native.SetTriangleIndicesDescription(Printing3DBufferDescription {
            Format: Printing3DBufferFormat::Printing3DUInt,
            Stride: 3,
        })?;
        native.CreateVertexPositions(
            vertex_count
                .checked_mul(24)
                .ok_or_else(|| failed("Vertex buffer overflow"))?,
        )?;
        native.CreateTriangleIndices(
            face_count
                .checked_mul(12)
                .ok_or_else(|| failed("Index buffer overflow"))?,
        )?;
        write_triples::<f32, 8>(&native.GetVertexPositions()?, &mesh.vertices, |value| {
            f64::from(value).to_le_bytes()
        })?;
        write_triples::<u32, 4>(&native.GetTriangleIndices()?, &mesh.faces, |value| {
            value.to_le_bytes()
        })?;
        model.Meshes()?.Append(&native)?;

        let started = Instant::now();
        let action = model.RepairAsync()?;
        let mut last_progress = Duration::ZERO;
        elapsed(Duration::ZERO);
        loop {
            if cancelled() {
                let _ = action.Cancel();
                return Err(WindowsRepairError::Cancelled);
            }
            let running = started.elapsed();
            if running >= time_limit {
                let _ = action.Cancel();
                return Err(WindowsRepairError::TimedOut);
            }
            // Public Windows.Foundation.AsyncStatus values. windows 0.62
            // exposes this return type through its transitive windows-future.
            match action.Status()?.0 {
                1 => {
                    action.GetResults().map_err(|error| {
                        failed(&format!("Reading the completed repair: {error}"))
                    })?;
                    break;
                }
                2 => return Err(WindowsRepairError::Cancelled),
                3 => {
                    action
                        .GetResults()
                        .map_err(|error| failed(&format!("Running Windows repair: {error}")))?;
                    return Err(failed("Windows repair reported an error"));
                }
                0 => {}
                _ => return Err(failed("Unknown Windows repair status")),
            }
            if running.saturating_sub(last_progress) >= Duration::from_millis(100) {
                elapsed(running);
                last_progress = running;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        elapsed(started.elapsed());
        if cancelled() {
            return Err(WindowsRepairError::Cancelled);
        }
        let meshes = model.Meshes()?;
        let mut parts = Vec::new();
        for index in 0..meshes.Size()? {
            parts.push(read_mesh(&meshes.GetAt(index)?)?);
        }
        let repaired = Mesh::concatenate(&parts).map_err(|error| failed(&error.to_string()))?;
        repaired
            .validate_non_empty()
            .map_err(|error| failed(&error.to_string()))?;
        Ok(repaired)
    }

    #[allow(unsafe_code)]
    fn write_triples<T: Copy, const N: usize>(
        buffer: &IBuffer,
        triples: &[[T; 3]],
        encode: impl Fn(T) -> [u8; N],
    ) -> Result<(), WindowsRepairError> {
        let length = triples
            .len()
            .checked_mul(3 * N)
            .and_then(|bytes| u32::try_from(bytes).ok())
            .ok_or_else(|| failed("Buffer size overflow"))?;
        if buffer.Capacity()? < length {
            return Err(failed("Windows allocated a short mesh buffer"));
        }
        buffer.SetLength(length)?;
        let access: IBufferByteAccess = buffer.cast()?;
        // SAFETY: IBufferByteAccess is the public WinRT buffer access interface.
        // The owned IBuffer and access interface outlive this exclusive write;
        // Capacity was checked, and repair has not started. Byte writes impose
        // no alignment requirement and stay within the allocated buffer.
        unsafe {
            let pointer = access.Buffer()?;
            if pointer.is_null() {
                return Err(failed("Windows returned a null mesh buffer"));
            }
            let destination = std::slice::from_raw_parts_mut(pointer, length as usize);
            for (slot, value) in destination
                .as_chunks_mut::<N>()
                .0
                .iter_mut()
                .zip(triples.iter().flatten())
            {
                slot.copy_from_slice(&encode(*value));
            }
        }
        Ok(())
    }

    fn reader(buffer: &IBuffer, count: u32, stride: u32) -> Result<DataReader, WindowsRepairError> {
        let length = count
            .checked_mul(stride)
            .ok_or_else(|| failed("Result buffer overflow"))?;
        if count == 0 || buffer.Length()? != length {
            return Err(failed("Empty or inconsistent Windows mesh buffer"));
        }
        let reader = DataReader::FromBuffer(buffer)?;
        reader.SetByteOrder(ByteOrder::LittleEndian)?;
        Ok(reader)
    }

    fn read_mesh(mesh: &Printing3DMesh) -> Result<Mesh, WindowsRepairError> {
        let vertex_description = mesh.VertexPositionsDescription()?;
        let double = match vertex_description.Format {
            Printing3DBufferFormat::R32G32B32Float => false,
            Printing3DBufferFormat::Printing3DDouble => true,
            _ => return Err(failed("Unsupported Windows vertex format")),
        };
        let vertex_stride = if double { 24 } else { 12 };
        // RepairAsync returns Printing3DDouble with a component stride of 3,
        // even when its input used the older vector-format descriptions.
        if vertex_description.Stride != 3 && vertex_description.Stride != vertex_stride {
            return Err(failed("Unsupported Windows vertex stride"));
        }
        let vertex_count = mesh.VertexCount()?;
        let input = reader(&mesh.GetVertexPositions()?, vertex_count, vertex_stride)?;
        let mut vertices = Vec::with_capacity(vertex_count as usize);
        for _ in 0..vertex_count {
            let mut point = [0.0; 3];
            for coordinate in &mut point {
                *coordinate = if double {
                    input.ReadDouble()? as f32
                } else {
                    input.ReadSingle()?
                };
            }
            vertices.push(point);
        }
        let description = mesh.TriangleIndicesDescription()?;
        if !matches!(
            description.Format,
            Printing3DBufferFormat::Printing3DUInt | Printing3DBufferFormat::R32G32B32UInt
        ) || !matches!(description.Stride, 3 | 12)
        {
            return Err(failed("Unsupported Windows index format or stride"));
        }
        let face_count = mesh.IndexCount()?;
        let input = reader(&mesh.GetTriangleIndices()?, face_count, 12)?;
        let mut faces = Vec::with_capacity(face_count as usize);
        for _ in 0..face_count {
            faces.push([
                input.ReadUInt32()?,
                input.ReadUInt32()?,
                input.ReadUInt32()?,
            ]);
        }
        Mesh::new(vertices, faces).map_err(|error| failed(&error.to_string()))
    }
}

#[cfg(all(test, not(windows)))]
mod tests {
    use super::*;

    #[test]
    fn unsupported_hosts_return_unavailable() {
        let mesh = crate::make_box([10.0; 3]).expect("box");
        assert!(matches!(
            run_windows_repair(&mesh, Duration::from_secs(30), &|| false, &mut |_| {}),
            Err(WindowsRepairError::Unavailable)
        ));
    }
}
