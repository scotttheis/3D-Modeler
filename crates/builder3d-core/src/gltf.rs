//! Bounded glTF 2.0 and GLB scene import and export.
//!
//! The application model uses millimetres while glTF stores distances in
//! metres.  This module performs that conversion at the file boundary and
//! keeps node transforms, mesh data, and representative PBR colors together
//! as scene items.  It deliberately uses the workspace's existing JSON
//! dependency instead of a second format crate so that package limits and
//! external-resource policy remain explicit and testable.

use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Map, Value};

use crate::{CoreError, DEFAULT_MESH_COLOR, Mesh, Transform};

/// Maximum bytes accepted by one glTF JSON or GLB file.
pub const MAX_GLTF_FILE_BYTES: u64 = 256 * 1024 * 1024;
/// Maximum bytes retained for a glTF JSON document.
pub const MAX_GLTF_JSON_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum bytes in one glTF buffer after its declared byte length is applied.
pub const MAX_GLTF_BUFFER_BYTES: u64 = 256 * 1024 * 1024;
/// Maximum bytes across all buffers, including external buffer resources.
pub const MAX_GLTF_TOTAL_BUFFER_BYTES: u64 = 512 * 1024 * 1024;
/// Maximum number of buffer resources in one document.
pub const MAX_GLTF_BUFFERS: usize = 1_024;
/// Maximum number of buffer views in one document.
pub const MAX_GLTF_BUFFER_VIEWS: usize = 100_000;
/// Maximum number of accessors in one document.
pub const MAX_GLTF_ACCESSORS: usize = 100_000;
/// Maximum number of materials in one document.
pub const MAX_GLTF_MATERIALS: usize = 100_000;
/// Maximum number of meshes in one document.
pub const MAX_GLTF_MESHES: usize = 100_000;
/// Maximum number of nodes in one document.
pub const MAX_GLTF_NODES: usize = 100_000;
/// Maximum number of scene items produced by one document.
pub const MAX_GLTF_ITEMS: usize = 10_000;
/// Maximum vertices accepted by one imported glTF mesh.
pub const MAX_GLTF_VERTICES: usize = 5_000_000;
/// Maximum triangular faces accepted by one imported glTF mesh.
pub const MAX_GLTF_FACES: usize = 10_000_000;
/// Maximum node nesting depth accepted by the scene walker.
pub const MAX_GLTF_NODE_DEPTH: usize = 256;
/// Maximum number of GLB chunks inspected in one container.
pub const MAX_GLTF_CHUNKS: usize = 1_024;

/// Resource limits applied while reading a glTF or GLB document.
#[derive(Clone, Debug, PartialEq)]
pub struct GltfLimits {
    /// Maximum compressed or standalone source file bytes.
    pub file_bytes: u64,
    /// Maximum JSON chunk/document bytes.
    pub json_bytes: u64,
    /// Maximum bytes in one decoded buffer.
    pub buffer_bytes: u64,
    /// Maximum bytes across all decoded buffers.
    pub total_buffer_bytes: u64,
    /// Maximum number of buffer resources.
    pub buffers: usize,
    /// Maximum number of buffer views.
    pub buffer_views: usize,
    /// Maximum number of accessors.
    pub accessors: usize,
    /// Maximum number of materials.
    pub materials: usize,
    /// Maximum number of meshes.
    pub meshes: usize,
    /// Maximum number of nodes.
    pub nodes: usize,
    /// Maximum number of imported scene items.
    pub items: usize,
    /// Maximum vertices in one mesh resource.
    pub vertices: usize,
    /// Maximum faces in one mesh resource.
    pub faces: usize,
    /// Maximum node hierarchy depth.
    pub node_depth: usize,
}

impl Default for GltfLimits {
    fn default() -> Self {
        Self {
            file_bytes: MAX_GLTF_FILE_BYTES,
            json_bytes: MAX_GLTF_JSON_BYTES,
            buffer_bytes: MAX_GLTF_BUFFER_BYTES,
            total_buffer_bytes: MAX_GLTF_TOTAL_BUFFER_BYTES,
            buffers: MAX_GLTF_BUFFERS,
            buffer_views: MAX_GLTF_BUFFER_VIEWS,
            accessors: MAX_GLTF_ACCESSORS,
            materials: MAX_GLTF_MATERIALS,
            meshes: MAX_GLTF_MESHES,
            nodes: MAX_GLTF_NODES,
            items: MAX_GLTF_ITEMS,
            vertices: MAX_GLTF_VERTICES,
            faces: MAX_GLTF_FACES,
            node_depth: MAX_GLTF_NODE_DEPTH,
        }
    }
}

impl GltfLimits {
    fn validate(&self) -> Result<(), CoreError> {
        let positive = [
            self.file_bytes,
            self.json_bytes,
            self.buffer_bytes,
            self.total_buffer_bytes,
        ];
        if positive.contains(&0)
            || self.buffers == 0
            || self.buffer_views == 0
            || self.accessors == 0
            || self.materials == 0
            || self.meshes == 0
            || self.nodes == 0
            || self.items == 0
            || self.vertices == 0
            || self.faces == 0
            || self.node_depth == 0
        {
            return Err(CoreError::gltf("glTF resource limits must be positive"));
        }
        if self.json_bytes > self.file_bytes {
            return Err(CoreError::gltf(
                "glTF JSON limit cannot exceed the file limit",
            ));
        }
        Ok(())
    }
}

/// One imported or exportable glTF scene item.
#[derive(Clone, Debug, PartialEq)]
pub struct GltfItem {
    /// Local indexed geometry, converted from metres to millimetres on import.
    pub mesh: Mesh,
    /// Node transform, with its translation converted to millimetres.
    pub transform: Transform,
    /// Node, mesh, or generated display name.
    pub name: String,
    /// Representative RGBA material or vertex color, if the source defines it.
    pub color: Option<[f32; 4]>,
}

impl GltfItem {
    /// Creates a validated scene item at the millimetre application boundary.
    pub fn new(
        mesh: Mesh,
        transform: Transform,
        name: String,
        color: Option<[f32; 4]>,
    ) -> Result<Self, CoreError> {
        mesh.validate_non_empty()?;
        if name.trim().is_empty() {
            return Err(CoreError::gltf("scene item name must not be empty"));
        }
        validate_color(color)?;
        Transform::from_rows(transform.rows())
            .map_err(|error| CoreError::gltf(format!("invalid scene item transform: {error}")))?;
        Ok(Self {
            mesh,
            transform,
            name,
            color,
        })
    }
}

/// Result of a path-based glTF export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GltfExportReport {
    /// The requested main document path.
    pub path: String,
    /// Number of scene items written.
    pub objects_exported: usize,
    /// External binary sidecars written for a `.gltf` document.
    pub sidecars: Vec<String>,
}

/// Loads a glTF or GLB scene from a path, resolving safe external buffers next
/// to the document.  All returned geometry and translations are millimetres.
pub fn load_gltf(path: impl AsRef<Path>) -> Result<Vec<GltfItem>, CoreError> {
    load_gltf_with_limits(path, &GltfLimits::default())
}

/// Loads a glTF or GLB scene from a path with an explicit resource policy.
pub fn load_gltf_with_limits(
    path: impl AsRef<Path>,
    limits: &GltfLimits,
) -> Result<Vec<GltfItem>, CoreError> {
    limits.validate()?;
    let path = path.as_ref();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| CoreError::UnsupportedMeshFormat(path.display().to_string()))?;
    if extension != "gltf" && extension != "glb" {
        return Err(CoreError::UnsupportedMeshFormat(extension));
    }
    let metadata = fs::metadata(path)
        .map_err(|error| CoreError::gltf(format!("could not stat {}: {error}", path.display())))?;
    if metadata.len() > limits.file_bytes {
        return Err(CoreError::gltf(format!(
            "file exceeds the maximum size of {} bytes",
            limits.file_bytes
        )));
    }
    let bytes = fs::read(path)
        .map_err(|error| CoreError::gltf(format!("could not read {}: {error}", path.display())))?;
    let parent = path.parent().map_or(Path::new("."), |parent| parent);
    decode_document(&bytes, Some(parent), limits)
}

/// Loads an embedded-resource glTF or GLB byte stream.
pub fn load_gltf_bytes(bytes: &[u8]) -> Result<Vec<GltfItem>, CoreError> {
    load_gltf_bytes_with_limits(bytes, &GltfLimits::default())
}

/// Loads a glTF or GLB byte stream with an explicit resource policy.
///
/// External URI buffers are rejected because a byte stream has no implicit
/// package directory.  Use [`load_gltf_bytes_from_directory`] when a caller
/// has an approved directory for external resources.
pub fn load_gltf_bytes_with_limits(
    bytes: &[u8],
    limits: &GltfLimits,
) -> Result<Vec<GltfItem>, CoreError> {
    decode_document(bytes, None, limits)
}

/// Loads a glTF or GLB byte stream and resolves external buffers below one
/// approved directory.  The directory itself must already exist.
pub fn load_gltf_bytes_from_directory(
    bytes: &[u8],
    base_dir: impl AsRef<Path>,
    limits: &GltfLimits,
) -> Result<Vec<GltfItem>, CoreError> {
    let base_dir = base_dir.as_ref();
    if !base_dir.is_dir() {
        return Err(CoreError::gltf(format!(
            "external buffer directory is not a directory: {}",
            base_dir.display()
        )));
    }
    decode_document(bytes, Some(base_dir), limits)
}

/// Exports a self-contained glTF JSON document with a base64 data URI buffer.
pub fn export_gltf(items: &[GltfItem]) -> Result<Vec<u8>, CoreError> {
    let package = build_export(items, Some("data"))?;
    if u64::try_from(package.json.len()).is_ok_and(|size| size > MAX_GLTF_FILE_BYTES) {
        return Err(CoreError::gltf(
            "exported glTF document exceeds the file limit",
        ));
    }
    Ok(package.json)
}

/// Exports a binary GLB document containing the scene and its buffer.
pub fn export_glb(items: &[GltfItem]) -> Result<Vec<u8>, CoreError> {
    let package = build_export(items, None)?;
    let json = pad_glb_json(&package.json);
    let binary = pad_glb_binary(&package.binary);
    let total = 12_usize
        .checked_add(8)
        .and_then(|value| value.checked_add(json.len()))
        .and_then(|value| value.checked_add(8))
        .and_then(|value| value.checked_add(binary.len()))
        .ok_or_else(|| CoreError::gltf("GLB size overflow"))?;
    if u64::try_from(total).is_ok_and(|size| size > MAX_GLTF_FILE_BYTES) {
        return Err(CoreError::gltf("exported GLB exceeds the file limit"));
    }
    let total_u32 = u32::try_from(total)
        .map_err(|_| CoreError::gltf("exported GLB exceeds the container limit"))?;
    let json_len =
        u32::try_from(json.len()).map_err(|_| CoreError::gltf("GLB JSON chunk is too large"))?;
    let binary_len =
        u32::try_from(binary.len()).map_err(|_| CoreError::gltf("GLB BIN chunk is too large"))?;
    let mut output = Vec::with_capacity(total);
    output.extend_from_slice(b"glTF");
    output.extend_from_slice(&2_u32.to_le_bytes());
    output.extend_from_slice(&total_u32.to_le_bytes());
    output.extend_from_slice(&json_len.to_le_bytes());
    output.extend_from_slice(b"JSON");
    output.extend_from_slice(&json);
    output.extend_from_slice(&binary_len.to_le_bytes());
    output.extend_from_slice(b"BIN\0");
    output.extend_from_slice(&binary);
    Ok(output)
}

/// Writes a `.gltf` or `.glb` scene using completed temporary files.
pub fn save_gltf(
    items: &[GltfItem],
    path: impl AsRef<Path>,
) -> Result<GltfExportReport, CoreError> {
    let path = path.as_ref();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| CoreError::UnsupportedMeshFormat(path.display().to_string()))?;
    match extension.as_str() {
        "glb" => {
            let bytes = export_glb(items)?;
            write_atomic(path, &bytes)?;
            Ok(GltfExportReport {
                path: path.display().to_string(),
                objects_exported: items.len(),
                sidecars: Vec::new(),
            })
        }
        "gltf" => {
            let sidecar_name = format!(
                "{}_gltf_buffer_0.bin",
                path.file_stem()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| CoreError::gltf("GLTF path has no valid file stem"))?
            );
            let package = build_export(items, Some(&sidecar_name))?;
            let sidecar_path = path.with_file_name(&sidecar_name);
            write_atomic(&sidecar_path, &package.binary)?;
            write_atomic(path, &package.json)?;
            Ok(GltfExportReport {
                path: path.display().to_string(),
                objects_exported: items.len(),
                sidecars: vec![sidecar_path.display().to_string()],
            })
        }
        _ => Err(CoreError::UnsupportedMeshFormat(extension)),
    }
}

/// Writes a binary GLB scene.  This is a named convenience alias for callers
/// that want the extension-specific operation to be explicit.
pub fn save_glb(items: &[GltfItem], path: impl AsRef<Path>) -> Result<GltfExportReport, CoreError> {
    let path = path.as_ref();
    if path
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("glb"))
    {
        return Err(CoreError::UnsupportedMeshFormat(path.display().to_string()));
    }
    save_gltf(items, path)
}

struct ExportPackage {
    json: Vec<u8>,
    binary: Vec<u8>,
}

fn decode_document(
    bytes: &[u8],
    base_dir: Option<&Path>,
    limits: &GltfLimits,
) -> Result<Vec<GltfItem>, CoreError> {
    limits.validate()?;
    let size = u64::try_from(bytes.len())
        .map_err(|_| CoreError::gltf("source size does not fit in u64"))?;
    if size > limits.file_bytes {
        return Err(CoreError::gltf(format!(
            "file exceeds the maximum size of {} bytes",
            limits.file_bytes
        )));
    }
    if bytes.is_empty() {
        return Err(CoreError::gltf("file is empty"));
    }
    let (json_bytes, glb_binary) = parse_container(bytes, limits)?;
    let json = serde_json::from_slice::<Value>(&json_bytes)
        .map_err(|error| CoreError::gltf(format!("JSON document is invalid: {error}")))?;
    let buffers = load_buffers(&json, glb_binary.as_deref(), base_dir, limits)?;
    let decoder = Decoder::new(json, buffers, limits.clone())?;
    decoder.decode_scene()
}

fn parse_container(
    bytes: &[u8],
    limits: &GltfLimits,
) -> Result<(Vec<u8>, Option<Vec<u8>>), CoreError> {
    if bytes.get(0..4) != Some(b"glTF") {
        if u64::try_from(bytes.len()).is_ok_and(|size| size > limits.json_bytes) {
            return Err(CoreError::gltf("glTF JSON document exceeds its limit"));
        }
        return Ok((bytes.to_vec(), None));
    }
    if bytes.len() < 12 {
        return Err(CoreError::gltf("GLB header is truncated"));
    }
    let version = read_u32(bytes, 4, "GLB version")?;
    if version != 2 {
        return Err(CoreError::gltf(format!(
            "unsupported GLB version {version}; expected 2"
        )));
    }
    let declared_length = usize::try_from(read_u32(bytes, 8, "GLB length")?)
        .map_err(|_| CoreError::gltf("GLB length does not fit in usize"))?;
    if declared_length != bytes.len() {
        return Err(CoreError::gltf(format!(
            "GLB length {declared_length} does not match file length {}",
            bytes.len()
        )));
    }
    let mut offset = 12_usize;
    let mut chunk_count = 0_usize;
    let mut json = None;
    let mut binary = None;
    while offset < bytes.len() {
        chunk_count = chunk_count
            .checked_add(1)
            .ok_or_else(|| CoreError::gltf("GLB chunk count overflow"))?;
        if chunk_count > MAX_GLTF_CHUNKS {
            return Err(CoreError::gltf("GLB contains too many chunks"));
        }
        let chunk_length = usize::try_from(read_u32(bytes, offset, "GLB chunk length")?)
            .map_err(|_| CoreError::gltf("GLB chunk length does not fit in usize"))?;
        let chunk_type = bytes
            .get(offset + 4..offset + 8)
            .ok_or_else(|| CoreError::gltf("GLB chunk header is truncated"))?;
        let data_start = offset
            .checked_add(8)
            .ok_or_else(|| CoreError::gltf("GLB chunk offset overflow"))?;
        let data_end = data_start
            .checked_add(chunk_length)
            .ok_or_else(|| CoreError::gltf("GLB chunk length overflow"))?;
        if data_end > bytes.len() {
            return Err(CoreError::gltf("GLB chunk extends beyond the file"));
        }
        let data = bytes
            .get(data_start..data_end)
            .ok_or_else(|| CoreError::gltf("GLB chunk data is truncated"))?;
        if chunk_type == b"JSON" {
            if json.is_some() {
                return Err(CoreError::gltf("GLB contains more than one JSON chunk"));
            }
            if u64::try_from(data.len()).is_ok_and(|size| size > limits.json_bytes) {
                return Err(CoreError::gltf("GLB JSON chunk exceeds its limit"));
            }
            json = Some(data.to_vec());
        } else if chunk_type == b"BIN\0" {
            if binary.is_some() {
                return Err(CoreError::gltf("GLB contains more than one BIN chunk"));
            }
            if u64::try_from(data.len()).is_ok_and(|size| size > limits.buffer_bytes) {
                return Err(CoreError::gltf("GLB BIN chunk exceeds its buffer limit"));
            }
            binary = Some(data.to_vec());
        }
        offset = data_end;
    }
    let json = json.ok_or_else(|| CoreError::gltf("GLB has no JSON chunk"))?;
    Ok((json, binary))
}

fn load_buffers(
    root: &Value,
    glb_binary: Option<&[u8]>,
    base_dir: Option<&Path>,
    limits: &GltfLimits,
) -> Result<Vec<Vec<u8>>, CoreError> {
    let buffers = required_array(root, "buffers")?;
    if buffers.len() > limits.buffers {
        return Err(CoreError::gltf(format!(
            "document contains more than {} buffers",
            limits.buffers
        )));
    }
    let mut result = Vec::new();
    try_reserve(&mut result, buffers.len(), "buffer list")?;
    let mut total = 0_u64;
    for (index, value) in buffers.iter().enumerate() {
        let object = value
            .as_object()
            .ok_or_else(|| CoreError::gltf(format!("buffer {index} is not an object")))?;
        let declared = required_u64(
            object.get("byteLength"),
            &format!("buffer {index} byteLength"),
        )?;
        if declared > limits.buffer_bytes {
            return Err(CoreError::gltf(format!(
                "buffer {index} exceeds the maximum size of {} bytes",
                limits.buffer_bytes
            )));
        }
        total = total
            .checked_add(declared)
            .ok_or_else(|| CoreError::gltf("total buffer size overflow"))?;
        if total > limits.total_buffer_bytes {
            return Err(CoreError::gltf(format!(
                "buffers exceed the maximum total size of {} bytes",
                limits.total_buffer_bytes
            )));
        }
        let data = if let Some(uri_value) = object.get("uri") {
            let uri = uri_value
                .as_str()
                .ok_or_else(|| CoreError::gltf(format!("buffer {index} URI is not a string")))?;
            if uri.starts_with("data:") {
                decode_data_uri(uri, declared, index, limits)?
            } else {
                read_external_buffer(uri, base_dir, declared, index, limits)?
            }
        } else if index == 0 {
            glb_binary
                .ok_or_else(|| {
                    CoreError::gltf(format!("buffer {index} has no URI or GLB BIN data"))
                })?
                .to_vec()
        } else {
            return Err(CoreError::gltf(format!(
                "buffer {index} has no URI or GLB BIN data"
            )));
        };
        if u64::try_from(data.len()).is_ok_and(|size| size < declared) {
            return Err(CoreError::gltf(format!(
                "buffer {index} contains fewer bytes than its declared byteLength"
            )));
        }
        let declared_len = usize::try_from(declared)
            .map_err(|_| CoreError::gltf(format!("buffer {index} is too large for this host")))?;
        let retained = data
            .get(..declared_len)
            .ok_or_else(|| CoreError::gltf(format!("buffer {index} is truncated")))?;
        result.push(retained.to_vec());
    }
    Ok(result)
}

fn decode_data_uri(
    uri: &str,
    declared: u64,
    index: usize,
    limits: &GltfLimits,
) -> Result<Vec<u8>, CoreError> {
    let (header, payload) = uri
        .split_once(',')
        .ok_or_else(|| CoreError::gltf(format!("buffer {index} data URI has no payload")))?;
    let base64 = header
        .split(';')
        .any(|part| part.eq_ignore_ascii_case("base64"));
    let decoded = if base64 {
        decode_base64(
            payload,
            limits.buffer_bytes,
            &format!("buffer {index} data URI"),
        )?
    } else {
        percent_decode(payload.as_bytes(), &format!("buffer {index} data URI"))?
    };
    if u64::try_from(decoded.len()).is_ok_and(|size| size > limits.buffer_bytes) {
        return Err(CoreError::gltf(format!(
            "buffer {index} data URI exceeds the maximum size of {} bytes",
            limits.buffer_bytes
        )));
    }
    if u64::try_from(decoded.len()).is_ok_and(|size| size < declared) {
        return Err(CoreError::gltf(format!(
            "buffer {index} data URI is shorter than its declared byteLength"
        )));
    }
    Ok(decoded)
}

fn read_external_buffer(
    uri: &str,
    base_dir: Option<&Path>,
    declared: u64,
    index: usize,
    limits: &GltfLimits,
) -> Result<Vec<u8>, CoreError> {
    let base_dir = base_dir.ok_or_else(|| {
        CoreError::gltf(format!(
            "buffer {index} uses an external URI but no approved directory was supplied"
        ))
    })?;
    let relative = safe_relative_uri(uri, index)?;
    let candidate = base_dir.join(&relative);
    let canonical_base = fs::canonicalize(base_dir).map_err(|error| {
        CoreError::gltf(format!(
            "could not resolve external buffer directory {}: {error}",
            base_dir.display()
        ))
    })?;
    let canonical_candidate = fs::canonicalize(&candidate).map_err(|error| {
        CoreError::gltf(format!(
            "could not resolve external buffer {}: {error}",
            candidate.display()
        ))
    })?;
    if !canonical_candidate.starts_with(&canonical_base) {
        return Err(CoreError::gltf(format!(
            "external buffer {uri:?} escapes its approved directory"
        )));
    }
    let metadata = fs::metadata(&canonical_candidate).map_err(|error| {
        CoreError::gltf(format!(
            "could not stat external buffer {}: {error}",
            canonical_candidate.display()
        ))
    })?;
    if metadata.len() > limits.buffer_bytes || metadata.len() > limits.file_bytes {
        return Err(CoreError::gltf(format!(
            "external buffer {index} exceeds the configured size limit"
        )));
    }
    if metadata.len() < declared {
        return Err(CoreError::gltf(format!(
            "external buffer {index} is shorter than its declared byteLength"
        )));
    }
    fs::read(&canonical_candidate).map_err(|error| {
        CoreError::gltf(format!(
            "could not read external buffer {}: {error}",
            canonical_candidate.display()
        ))
    })
}

fn safe_relative_uri(uri: &str, index: usize) -> Result<PathBuf, CoreError> {
    if uri.is_empty() || uri.contains('#') || uri.contains('?') || uri.contains('\\') {
        return Err(CoreError::gltf(format!(
            "external buffer {index} has an unsafe URI"
        )));
    }
    let decoded = percent_decode(uri.as_bytes(), &format!("buffer {index} URI"))?;
    let decoded = String::from_utf8(decoded)
        .map_err(|_| CoreError::gltf(format!("external buffer {index} URI is not UTF-8")))?;
    let path = Path::new(&decoded);
    if path.is_absolute() || path.has_root() {
        return Err(CoreError::gltf(format!(
            "external buffer {index} URI must be relative"
        )));
    }
    for component in path.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(CoreError::gltf(format!(
                "external buffer {index} URI contains a parent or root path"
            )));
        }
    }
    if path.as_os_str().is_empty() {
        return Err(CoreError::gltf(format!(
            "external buffer {index} URI is empty"
        )));
    }
    Ok(path.to_path_buf())
}

fn percent_decode(input: &[u8], context: &str) -> Result<Vec<u8>, CoreError> {
    let mut output = Vec::new();
    try_reserve(&mut output, input.len(), context)?;
    let mut index = 0_usize;
    while index < input.len() {
        let byte = *input
            .get(index)
            .ok_or_else(|| CoreError::gltf(format!("{context} decoding failed")))?;
        if byte == b'%' {
            let high = *input
                .get(index + 1)
                .ok_or_else(|| CoreError::gltf(format!("{context} has an incomplete escape")))?;
            let low = *input
                .get(index + 2)
                .ok_or_else(|| CoreError::gltf(format!("{context} has an incomplete escape")))?;
            let high = hex_value(high)
                .ok_or_else(|| CoreError::gltf(format!("{context} has an invalid escape")))?;
            let low = hex_value(low)
                .ok_or_else(|| CoreError::gltf(format!("{context} has an invalid escape")))?;
            output.push((high << 4) | low);
            index = index.saturating_add(3);
        } else {
            output.push(byte);
            index = index.saturating_add(1);
        }
    }
    Ok(output)
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn decode_base64(input: &str, max_bytes: u64, context: &str) -> Result<Vec<u8>, CoreError> {
    let mut output = Vec::new();
    let mut accumulator = 0_u32;
    let mut bits = 0_u8;
    let mut padding = false;
    for byte in input.bytes() {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if byte == b'=' {
            padding = true;
            continue;
        }
        if padding {
            return Err(CoreError::gltf(format!("{context} has data after padding")));
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => {
                return Err(CoreError::gltf(format!(
                    "{context} has an invalid base64 byte"
                )));
            }
        };
        accumulator = (accumulator << 6) | u32::from(value);
        bits = bits.saturating_add(6);
        if bits >= 8 {
            bits -= 8;
            output.push(((accumulator >> bits) & 0xff) as u8);
            if bits == 0 {
                accumulator = 0;
            } else {
                accumulator &= (1_u32 << bits) - 1;
            }
            if u64::try_from(output.len()).is_ok_and(|size| size > max_bytes) {
                return Err(CoreError::gltf(format!(
                    "{context} exceeds the configured size limit"
                )));
            }
        }
    }
    if bits == 1 || bits == 3 || bits == 5 {
        return Err(CoreError::gltf(format!(
            "{context} has invalid base64 padding"
        )));
    }
    Ok(output)
}

fn required_array<'a>(root: &'a Value, name: &str) -> Result<&'a [Value], CoreError> {
    root.get(name)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| CoreError::gltf(format!("glTF document is missing array '{name}'")))
}

fn optional_array<'a>(root: &'a Value, name: &str) -> Result<Option<&'a [Value]>, CoreError> {
    match root.get(name) {
        None => Ok(None),
        Some(value) => value
            .as_array()
            .map(|array| Some(array.as_slice()))
            .ok_or_else(|| CoreError::gltf(format!("glTF field '{name}' is not an array"))),
    }
}

fn optional_array_in_object<'a>(
    root: &'a Map<String, Value>,
    name: &str,
) -> Result<Option<&'a [Value]>, CoreError> {
    match root.get(name) {
        None => Ok(None),
        Some(value) => value
            .as_array()
            .map(|array| Some(array.as_slice()))
            .ok_or_else(|| CoreError::gltf(format!("glTF field '{name}' is not an array"))),
    }
}

fn required_u64(value: Option<&Value>, label: &str) -> Result<u64, CoreError> {
    value
        .and_then(Value::as_u64)
        .ok_or_else(|| CoreError::gltf(format!("{label} must be a non-negative integer")))
}

fn optional_u64(value: Option<&Value>, default: u64, label: &str) -> Result<u64, CoreError> {
    match value {
        None => Ok(default),
        Some(value) => required_u64(Some(value), label),
    }
}

fn number_f64(value: Option<&Value>, label: &str) -> Result<f64, CoreError> {
    let value = value
        .and_then(Value::as_f64)
        .ok_or_else(|| CoreError::gltf(format!("{label} must be a finite number")))?;
    if !value.is_finite() {
        return Err(CoreError::gltf(format!("{label} must be finite")));
    }
    Ok(value)
}

fn read_u32(bytes: &[u8], offset: usize, label: &str) -> Result<u32, CoreError> {
    let raw = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| CoreError::gltf(format!("{label} is truncated")))?;
    let raw: [u8; 4] = raw
        .try_into()
        .map_err(|_| CoreError::gltf(format!("{label} is truncated")))?;
    Ok(u32::from_le_bytes(raw))
}

fn try_reserve<T>(items: &mut Vec<T>, count: usize, context: &str) -> Result<(), CoreError> {
    items
        .try_reserve(count)
        .map_err(|_| CoreError::gltf(format!("{context} exceeds available memory")))
}

fn validate_color(color: Option<[f32; 4]>) -> Result<(), CoreError> {
    if let Some(color) = color
        && color
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(CoreError::InvalidColor(
            "glTF RGBA values must be finite and between 0 and 1".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComponentType {
    I8,
    U8,
    I16,
    U16,
    U32,
    F32,
}

impl ComponentType {
    fn parse(value: Option<&Value>, label: &str) -> Result<Self, CoreError> {
        match required_u64(value, label)? {
            5120 => Ok(Self::I8),
            5121 => Ok(Self::U8),
            5122 => Ok(Self::I16),
            5123 => Ok(Self::U16),
            5125 => Ok(Self::U32),
            5126 => Ok(Self::F32),
            other => Err(CoreError::gltf(format!(
                "{label} uses unsupported component type {other}"
            ))),
        }
    }

    const fn width(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::U32 | Self::F32 => 4,
        }
    }

    const fn is_unsigned_index(self) -> bool {
        matches!(self, Self::U8 | Self::U16 | Self::U32)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AccessorType {
    Scalar,
    Vec2,
    Vec3,
    Vec4,
    Mat2,
    Mat3,
    Mat4,
}

impl AccessorType {
    fn parse(value: Option<&Value>, label: &str) -> Result<Self, CoreError> {
        let value = value
            .and_then(Value::as_str)
            .ok_or_else(|| CoreError::gltf(format!("{label} must be a string")))?;
        match value {
            "SCALAR" => Ok(Self::Scalar),
            "VEC2" => Ok(Self::Vec2),
            "VEC3" => Ok(Self::Vec3),
            "VEC4" => Ok(Self::Vec4),
            "MAT2" => Ok(Self::Mat2),
            "MAT3" => Ok(Self::Mat3),
            "MAT4" => Ok(Self::Mat4),
            other => Err(CoreError::gltf(format!(
                "{label} uses unsupported accessor type {other:?}"
            ))),
        }
    }

    const fn components(self) -> usize {
        match self {
            Self::Scalar => 1,
            Self::Vec2 => 2,
            Self::Vec3 => 3,
            Self::Vec4 => 4,
            Self::Mat2 => 4,
            Self::Mat3 => 9,
            Self::Mat4 => 16,
        }
    }
}

#[derive(Clone, Debug)]
struct BufferView {
    buffer: usize,
    offset: usize,
    length: usize,
    stride: Option<usize>,
}

#[derive(Clone, Debug)]
struct SparseAccessor {
    count: usize,
    indices_view: usize,
    indices_offset: usize,
    indices_type: ComponentType,
    values_view: usize,
    values_offset: usize,
}

#[derive(Clone, Debug)]
struct Accessor {
    view: Option<usize>,
    offset: usize,
    count: usize,
    component: ComponentType,
    kind: AccessorType,
    normalized: bool,
    sparse: Option<SparseAccessor>,
}

#[derive(Clone, Debug)]
struct MeshData {
    mesh: Mesh,
    color: Option<[f32; 4]>,
}

struct Decoder {
    root: Value,
    buffers: Vec<Vec<u8>>,
    views: Vec<BufferView>,
    accessors: Vec<Accessor>,
    materials: Vec<Option<[f32; 4]>>,
    limits: GltfLimits,
}

impl Decoder {
    fn new(root: Value, buffers: Vec<Vec<u8>>, limits: GltfLimits) -> Result<Self, CoreError> {
        let object = root
            .as_object()
            .ok_or_else(|| CoreError::gltf("glTF root must be a JSON object"))?;
        let views = parse_buffer_views(object, &buffers, &limits)?;
        let accessors = parse_accessors(object, &limits)?;
        let materials = parse_materials(object, &limits)?;
        Ok(Self {
            root,
            buffers,
            views,
            accessors,
            materials,
            limits,
        })
    }

    fn decode_scene(&self) -> Result<Vec<GltfItem>, CoreError> {
        let object = self
            .root
            .as_object()
            .ok_or_else(|| CoreError::gltf("glTF root must be a JSON object"))?;
        let nodes = required_array(&self.root, "nodes")?;
        if nodes.len() > self.limits.nodes {
            return Err(CoreError::gltf(format!(
                "document contains more than {} nodes",
                self.limits.nodes
            )));
        }
        let mut mesh_cache =
            vec![None; optional_array(&self.root, "meshes")?.map_or(0, |items| items.len())];
        if mesh_cache.len() > self.limits.meshes {
            return Err(CoreError::gltf(format!(
                "document contains more than {} meshes",
                self.limits.meshes
            )));
        }
        let roots = scene_roots(object, nodes)?;
        if roots.is_empty() {
            return Err(CoreError::gltf("glTF scene contains no root nodes"));
        }
        let mut items = Vec::new();
        try_reserve(&mut items, roots.len(), "scene items")?;
        let mut stack = HashSet::new();
        for root in roots {
            self.visit_node(
                root,
                Transform::identity(),
                nodes,
                &mut mesh_cache,
                &mut items,
                &mut stack,
                0,
            )?;
        }
        if items.is_empty() {
            return Err(CoreError::gltf("GLTF file contained no mesh nodes"));
        }
        Ok(items)
    }

    #[allow(clippy::too_many_arguments)]
    fn visit_node(
        &self,
        index: usize,
        parent: Transform,
        nodes: &[Value],
        mesh_cache: &mut [Option<MeshData>],
        items: &mut Vec<GltfItem>,
        stack: &mut HashSet<usize>,
        depth: usize,
    ) -> Result<(), CoreError> {
        if depth > self.limits.node_depth {
            return Err(CoreError::gltf(format!(
                "node hierarchy exceeds depth {}",
                self.limits.node_depth
            )));
        }
        if !stack.insert(index) {
            return Err(CoreError::gltf("glTF node hierarchy contains a cycle"));
        }
        let node = nodes
            .get(index)
            .and_then(Value::as_object)
            .ok_or_else(|| CoreError::gltf(format!("node {index} is not an object")))?;
        let local = parse_node_transform(node, index)?;
        let world = parent
            .multiply(&local)
            .map_err(|error| CoreError::gltf(format!("node {index} transform failed: {error}")))?;
        if let Some(mesh_value) = node.get("mesh") {
            let mesh_index = usize_from_u64(
                required_u64(Some(mesh_value), &format!("node {index} mesh"))?,
                &format!("node {index} mesh"),
            )?;
            if mesh_index >= mesh_cache.len() {
                return Err(CoreError::gltf(format!(
                    "node {index} references missing mesh {mesh_index}"
                )));
            }
            if mesh_cache.get(mesh_index).is_some_and(Option::is_none) {
                let mesh = self.decode_mesh(mesh_index)?;
                let slot = mesh_cache
                    .get_mut(mesh_index)
                    .ok_or_else(|| CoreError::gltf("mesh cache entry is missing"))?;
                *slot = Some(mesh);
            }
            let mesh_data = mesh_cache
                .get(mesh_index)
                .and_then(Option::as_ref)
                .ok_or_else(|| CoreError::gltf("mesh cache entry was not created"))?;
            if items.len() >= self.limits.items {
                return Err(CoreError::gltf(format!(
                    "scene contains more than {} mesh items",
                    self.limits.items
                )));
            }
            let name = node_name(node, mesh_index, index, &self.root)?;
            items.push(GltfItem::new(
                mesh_data.mesh.clone(),
                world,
                name,
                mesh_data.color,
            )?);
        }
        if let Some(children) = node.get("children") {
            let children = children
                .as_array()
                .ok_or_else(|| CoreError::gltf(format!("node {index} children is not an array")))?;
            for (child_position, child) in children.iter().enumerate() {
                let child_index = usize_from_u64(
                    required_u64(Some(child), &format!("node {index} child {child_position}"))?,
                    &format!("node {index} child {child_position}"),
                )?;
                if child_index >= nodes.len() {
                    return Err(CoreError::gltf(format!(
                        "node {index} references missing child {child_index}"
                    )));
                }
                self.visit_node(
                    child_index,
                    world,
                    nodes,
                    mesh_cache,
                    items,
                    stack,
                    depth.saturating_add(1),
                )?;
            }
        }
        stack.remove(&index);
        Ok(())
    }

    fn decode_mesh(&self, index: usize) -> Result<MeshData, CoreError> {
        let meshes = required_array(&self.root, "meshes")?;
        let mesh_object = meshes
            .get(index)
            .and_then(Value::as_object)
            .ok_or_else(|| CoreError::gltf(format!("mesh {index} is not an object")))?;
        let primitives = mesh_object
            .get("primitives")
            .and_then(Value::as_array)
            .ok_or_else(|| CoreError::gltf(format!("mesh {index} has no primitives")))?;
        if primitives.is_empty() {
            return Err(CoreError::gltf(format!("mesh {index} has no primitives")));
        }
        let mut vertices = Vec::new();
        let mut faces = Vec::new();
        let mut color_sum = [0.0_f64; 4];
        let mut color_count = 0_u64;
        for (primitive_index, primitive_value) in primitives.iter().enumerate() {
            let primitive = primitive_value.as_object().ok_or_else(|| {
                CoreError::gltf(format!(
                    "mesh {index} primitive {primitive_index} is not an object"
                ))
            })?;
            let attributes = primitive
                .get("attributes")
                .and_then(Value::as_object)
                .ok_or_else(|| {
                    CoreError::gltf(format!(
                        "mesh {index} primitive {primitive_index} has no attributes"
                    ))
                })?;
            let position_index = usize_from_u64(
                required_u64(
                    attributes.get("POSITION"),
                    &format!("mesh {index} primitive {primitive_index} POSITION"),
                )?,
                "POSITION accessor",
            )?;
            let position_accessor = self.accessor(position_index)?;
            if position_accessor.kind != AccessorType::Vec3 || position_accessor.normalized {
                return Err(CoreError::gltf(format!(
                    "mesh {index} primitive {primitive_index} POSITION must be an unnormalized VEC3"
                )));
            }
            let positions = self.read_values(position_index)?;
            if positions.is_empty() {
                return Err(CoreError::gltf(format!(
                    "mesh {index} primitive {primitive_index} has no vertices"
                )));
            }
            let vertex_start = vertices.len();
            let new_vertex_count = vertex_start
                .checked_add(positions.len())
                .ok_or_else(|| CoreError::gltf("vertex count overflow"))?;
            if new_vertex_count > self.limits.vertices {
                return Err(CoreError::gltf(format!(
                    "mesh {index} exceeds the vertex limit of {}",
                    self.limits.vertices
                )));
            }
            try_reserve(&mut vertices, positions.len(), "mesh vertices")?;
            for (position_index, position) in positions.iter().enumerate() {
                let x = millimetres(position.first().copied(), "POSITION x")?;
                let y = millimetres(position.get(1).copied(), "POSITION y")?;
                let z = millimetres(position.get(2).copied(), "POSITION z")?;
                if ![x, y, z].iter().all(|value| value.is_finite()) {
                    return Err(CoreError::gltf(format!(
                        "mesh {index} vertex {position_index} is non-finite"
                    )));
                }
                vertices.push([x, y, z]);
            }
            if let Some(normal_value) = attributes.get("NORMAL") {
                let normal_index = usize_from_u64(
                    required_u64(Some(normal_value), "NORMAL accessor")?,
                    "NORMAL accessor",
                )?;
                let normal_accessor = self.accessor(normal_index)?;
                if normal_accessor.kind != AccessorType::Vec3
                    || normal_accessor.count != positions.len()
                {
                    return Err(CoreError::gltf(format!(
                        "mesh {index} primitive {primitive_index} NORMAL does not match POSITION"
                    )));
                }
                let normals = self.read_values(normal_index)?;
                if normals.iter().flatten().any(|value| !value.is_finite()) {
                    return Err(CoreError::gltf(format!(
                        "mesh {index} primitive {primitive_index} contains non-finite normals"
                    )));
                }
            }
            let primitive_color = self.primitive_color(primitive, positions.len())?;
            if let Some(color) = primitive_color {
                for channel in 0..4 {
                    color_sum[channel] += f64::from(color[channel]) * positions.len() as f64;
                }
                color_count =
                    color_count
                        .checked_add(u64::try_from(positions.len()).map_err(|_| {
                            CoreError::gltf("color sample count does not fit in u64")
                        })?)
                        .ok_or_else(|| CoreError::gltf("color sample count overflow"))?;
            }
            let indices = if let Some(index_value) = primitive.get("indices") {
                let accessor_index = usize_from_u64(
                    required_u64(Some(index_value), "indices accessor")?,
                    "indices accessor",
                )?;
                self.read_indices(accessor_index)?
            } else {
                let mut generated = Vec::new();
                try_reserve(&mut generated, positions.len(), "generated indices")?;
                for value in 0..positions.len() {
                    generated.push(
                        u32::try_from(value)
                            .map_err(|_| CoreError::gltf("generated index exceeds u32"))?,
                    );
                }
                generated
            };
            let mode = match primitive.get("mode") {
                Some(value) => required_u64(Some(value), "primitive mode")?,
                None => 4,
            };
            let potential_faces = match mode {
                4 => indices
                    .len()
                    .checked_add(2)
                    .map_or(usize::MAX, |count| count / 3),
                5 | 6 => indices.len().saturating_sub(2),
                _ => 0,
            };
            if potential_faces > self.limits.faces {
                return Err(CoreError::gltf(format!(
                    "mesh {index} primitive {primitive_index} exceeds the face limit of {}",
                    self.limits.faces
                )));
            }
            let primitive_faces = triangulate_indices(&indices, mode, index, primitive_index)?;
            let new_face_count = faces
                .len()
                .checked_add(primitive_faces.len())
                .ok_or_else(|| CoreError::gltf("face count overflow"))?;
            if new_face_count > self.limits.faces {
                return Err(CoreError::gltf(format!(
                    "mesh {index} exceeds the face limit of {}",
                    self.limits.faces
                )));
            }
            try_reserve(&mut faces, primitive_faces.len(), "mesh faces")?;
            let offset = u32::try_from(vertex_start)
                .map_err(|_| CoreError::gltf("mesh vertex offset exceeds u32"))?;
            for face in primitive_faces {
                let shifted = [
                    face[0]
                        .checked_add(offset)
                        .ok_or_else(|| CoreError::gltf("face index overflow"))?,
                    face[1]
                        .checked_add(offset)
                        .ok_or_else(|| CoreError::gltf("face index overflow"))?,
                    face[2]
                        .checked_add(offset)
                        .ok_or_else(|| CoreError::gltf("face index overflow"))?,
                ];
                faces.push(shifted);
            }
        }
        let mesh = Mesh::new(vertices, faces)
            .map_err(|error| CoreError::gltf(format!("mesh {index} is invalid: {error}")))?;
        let color = if color_count == 0 {
            None
        } else {
            let count = color_count as f64;
            Some([
                (color_sum[0] / count) as f32,
                (color_sum[1] / count) as f32,
                (color_sum[2] / count) as f32,
                (color_sum[3] / count) as f32,
            ])
        };
        Ok(MeshData { mesh, color })
    }

    fn primitive_color(
        &self,
        primitive: &Map<String, Value>,
        vertex_count: usize,
    ) -> Result<Option<[f32; 4]>, CoreError> {
        let vertex_color =
            if let Some(attributes) = primitive.get("attributes").and_then(Value::as_object)
                && let Some(value) = attributes.get("COLOR_0")
            {
                let accessor_index = usize_from_u64(
                    required_u64(Some(value), "COLOR_0 accessor")?,
                    "COLOR_0 accessor",
                )?;
                let accessor = self.accessor(accessor_index)?;
                if !matches!(accessor.kind, AccessorType::Vec3 | AccessorType::Vec4)
                    || accessor.count != vertex_count
                {
                    return Err(CoreError::gltf(
                        "COLOR_0 must be a VEC3/VEC4 matching POSITION",
                    ));
                }
                let values = self.read_values(accessor_index)?;
                let mut sums = [0.0_f64; 4];
                for value in values {
                    for (channel, sum) in sums.iter_mut().take(3).enumerate() {
                        *sum += f64::from(*value.get(channel).ok_or_else(|| {
                            CoreError::gltf("COLOR_0 has a missing color channel")
                        })?);
                    }
                    sums[3] += match value.get(3).copied() {
                        Some(alpha) => f64::from(alpha),
                        None => 1.0,
                    };
                }
                let count = vertex_count as f64;
                let mut color = [
                    (sums[0] / count) as f32,
                    (sums[1] / count) as f32,
                    (sums[2] / count) as f32,
                    (sums[3] / count) as f32,
                ];
                if color.iter().any(|value| *value > 1.0) {
                    for value in &mut color {
                        *value /= 255.0;
                    }
                }
                Some(color)
            } else {
                None
            };
        if vertex_color.is_some() {
            return Ok(vertex_color);
        }
        let material_index = primitive
            .get("material")
            .map(|value| required_u64(Some(value), "primitive material"))
            .transpose()?;
        match material_index {
            None => Ok(None),
            Some(index) => {
                let index = usize_from_u64(index, "primitive material")?;
                self.materials
                    .get(index)
                    .copied()
                    .ok_or_else(|| CoreError::gltf(format!("missing material {index}")))
            }
        }
    }

    fn accessor(&self, index: usize) -> Result<&Accessor, CoreError> {
        self.accessors
            .get(index)
            .ok_or_else(|| CoreError::gltf(format!("missing accessor {index}")))
    }

    fn read_values(&self, index: usize) -> Result<Vec<Vec<f32>>, CoreError> {
        let accessor = self.accessor(index)?;
        let components = accessor.kind.components();
        let count = accessor.count;
        if accessor.view.is_none() && accessor.sparse.is_none() {
            return Err(CoreError::gltf(format!(
                "accessor {index} has no bufferView and is not sparse"
            )));
        }
        let maximum = self
            .limits
            .vertices
            .saturating_mul(components.max(1))
            .max(self.limits.faces.saturating_mul(3));
        if count > maximum {
            return Err(CoreError::gltf(format!(
                "accessor {index} contains too many values"
            )));
        }
        let mut result = Vec::new();
        try_reserve(&mut result, count, "accessor values")?;
        for element in 0..count {
            let mut row = Vec::new();
            try_reserve(&mut row, components, "accessor components")?;
            for component in 0..components {
                let value = match accessor.view {
                    Some(view) => self.read_component(
                        view,
                        accessor.offset,
                        element,
                        component,
                        accessor.component,
                        accessor.normalized,
                        components,
                    )?,
                    None => 0.0,
                };
                row.push(value);
            }
            result.push(row);
        }
        if let Some(sparse) = &accessor.sparse {
            for sparse_index in 0..sparse.count {
                let destination = self.read_sparse_index(sparse, sparse_index)?;
                if destination >= count {
                    return Err(CoreError::gltf(format!(
                        "accessor {index} sparse index is out of range"
                    )));
                }
                let mut row = Vec::new();
                try_reserve(&mut row, components, "sparse accessor components")?;
                for component in 0..components {
                    row.push(self.read_component(
                        sparse.values_view,
                        sparse.values_offset,
                        sparse_index,
                        component,
                        accessor.component,
                        accessor.normalized,
                        components,
                    )?);
                }
                let target = result
                    .get_mut(destination)
                    .ok_or_else(|| CoreError::gltf("sparse accessor destination is missing"))?;
                *target = row;
            }
        }
        Ok(result)
    }

    fn read_indices(&self, index: usize) -> Result<Vec<u32>, CoreError> {
        let accessor = self.accessor(index)?;
        if accessor.kind != AccessorType::Scalar || !accessor.component.is_unsigned_index() {
            return Err(CoreError::gltf(format!(
                "indices accessor {index} must be an unsigned SCALAR"
            )));
        }
        let values = self.read_values(index)?;
        let mut result = Vec::new();
        try_reserve(&mut result, values.len(), "mesh indices")?;
        for value in values {
            let value = value
                .first()
                .copied()
                .ok_or_else(|| CoreError::gltf("index accessor value is empty"))?;
            if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > u32::MAX as f32
            {
                return Err(CoreError::gltf("index accessor contains an invalid value"));
            }
            result.push(value as u32);
        }
        Ok(result)
    }

    fn read_sparse_index(&self, sparse: &SparseAccessor, index: usize) -> Result<usize, CoreError> {
        let value = self.read_component(
            sparse.indices_view,
            sparse.indices_offset,
            index,
            0,
            sparse.indices_type,
            false,
            1,
        )?;
        if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
            return Err(CoreError::gltf("sparse accessor index is invalid"));
        }
        usize::try_from(value as u64)
            .map_err(|_| CoreError::gltf("sparse accessor index is too large"))
    }

    #[allow(clippy::too_many_arguments)]
    fn read_component(
        &self,
        view_index: usize,
        accessor_offset: usize,
        element: usize,
        component: usize,
        component_type: ComponentType,
        normalized: bool,
        components: usize,
    ) -> Result<f32, CoreError> {
        let view = self
            .views
            .get(view_index)
            .ok_or_else(|| CoreError::gltf(format!("missing buffer view {view_index}")))?;
        let component_width = component_type.width();
        let element_width = component_width
            .checked_mul(components)
            .ok_or_else(|| CoreError::gltf("accessor element width overflow"))?;
        let stride = match view.stride {
            Some(stride) => stride,
            None => element_width,
        };
        if stride < element_width {
            return Err(CoreError::gltf(
                "buffer view byteStride is smaller than the accessor",
            ));
        }
        let relative = accessor_offset
            .checked_add(
                element
                    .checked_mul(stride)
                    .ok_or_else(|| CoreError::gltf("accessor offset overflow"))?,
            )
            .and_then(|value| value.checked_add(component.checked_mul(component_width)?))
            .ok_or_else(|| CoreError::gltf("accessor offset overflow"))?;
        let end = relative
            .checked_add(component_width)
            .ok_or_else(|| CoreError::gltf("accessor component offset overflow"))?;
        if end > view.length {
            return Err(CoreError::gltf("accessor extends beyond its buffer view"));
        }
        let absolute = view
            .offset
            .checked_add(relative)
            .ok_or_else(|| CoreError::gltf("buffer offset overflow"))?;
        let buffer = self
            .buffers
            .get(view.buffer)
            .ok_or_else(|| CoreError::gltf(format!("missing buffer {}", view.buffer)))?;
        let absolute_end = absolute
            .checked_add(component_width)
            .ok_or_else(|| CoreError::gltf("buffer component offset overflow"))?;
        let raw = buffer
            .get(absolute..absolute_end)
            .ok_or_else(|| CoreError::gltf("accessor extends beyond its buffer"))?;
        decode_component(raw, component_type, normalized)
    }
}

fn decode_component(
    raw: &[u8],
    component_type: ComponentType,
    normalized: bool,
) -> Result<f32, CoreError> {
    let value = match component_type {
        ComponentType::I8 => {
            let byte = *raw
                .first()
                .ok_or_else(|| CoreError::gltf("component is truncated"))?;
            let value = i8::from_ne_bytes([byte]);
            if normalized {
                f32::from(value).max(-1.0) / 127.0
            } else {
                f32::from(value)
            }
        }
        ComponentType::U8 => {
            let value = *raw
                .first()
                .ok_or_else(|| CoreError::gltf("component is truncated"))?;
            if normalized {
                f32::from(value) / 255.0
            } else {
                f32::from(value)
            }
        }
        ComponentType::I16 => {
            let bytes: [u8; 2] = raw
                .get(..2)
                .ok_or_else(|| CoreError::gltf("component is truncated"))?
                .try_into()
                .map_err(|_| CoreError::gltf("component is truncated"))?;
            let value = i16::from_le_bytes(bytes);
            if normalized {
                f32::from(value).max(-1.0) / 32_767.0
            } else {
                f32::from(value)
            }
        }
        ComponentType::U16 => {
            let bytes: [u8; 2] = raw
                .get(..2)
                .ok_or_else(|| CoreError::gltf("component is truncated"))?
                .try_into()
                .map_err(|_| CoreError::gltf("component is truncated"))?;
            let value = u16::from_le_bytes(bytes);
            if normalized {
                f32::from(value) / 65_535.0
            } else {
                f32::from(value)
            }
        }
        ComponentType::U32 => {
            let bytes: [u8; 4] = raw
                .get(..4)
                .ok_or_else(|| CoreError::gltf("component is truncated"))?
                .try_into()
                .map_err(|_| CoreError::gltf("component is truncated"))?;
            let value = u32::from_le_bytes(bytes);
            value as f32
        }
        ComponentType::F32 => {
            let bytes: [u8; 4] = raw
                .get(..4)
                .ok_or_else(|| CoreError::gltf("component is truncated"))?
                .try_into()
                .map_err(|_| CoreError::gltf("component is truncated"))?;
            f32::from_le_bytes(bytes)
        }
    };
    if !value.is_finite() {
        return Err(CoreError::gltf("accessor contains a non-finite value"));
    }
    Ok(value)
}

fn parse_buffer_views(
    root: &Map<String, Value>,
    buffers: &[Vec<u8>],
    limits: &GltfLimits,
) -> Result<Vec<BufferView>, CoreError> {
    let Some(values) = optional_array_in_object(root, "bufferViews")? else {
        return Ok(Vec::new());
    };
    if values.len() > limits.buffer_views {
        return Err(CoreError::gltf(format!(
            "document contains more than {} buffer views",
            limits.buffer_views
        )));
    }
    let mut result = Vec::new();
    try_reserve(&mut result, values.len(), "buffer views")?;
    for (index, value) in values.iter().enumerate() {
        let object = value
            .as_object()
            .ok_or_else(|| CoreError::gltf(format!("buffer view {index} is not an object")))?;
        let buffer = usize_from_u64(
            required_u64(object.get("buffer"), &format!("buffer view {index} buffer"))?,
            &format!("buffer view {index} buffer"),
        )?;
        let offset = usize_from_u64(
            optional_u64(
                object.get("byteOffset"),
                0,
                &format!("buffer view {index} byteOffset"),
            )?,
            &format!("buffer view {index} byteOffset"),
        )?;
        let length = usize_from_u64(
            required_u64(
                object.get("byteLength"),
                &format!("buffer view {index} byteLength"),
            )?,
            &format!("buffer view {index} byteLength"),
        )?;
        let stride = object
            .get("byteStride")
            .map(|value| {
                usize_from_u64(
                    required_u64(Some(value), &format!("buffer view {index} byteStride"))?,
                    &format!("buffer view {index} byteStride"),
                )
            })
            .transpose()?;
        if let Some(stride) = stride
            && !(4..=252).contains(&stride)
        {
            return Err(CoreError::gltf(format!(
                "buffer view {index} byteStride is outside the glTF range"
            )));
        }
        let buffer_data = buffers.get(buffer).ok_or_else(|| {
            CoreError::gltf(format!(
                "buffer view {index} references missing buffer {buffer}"
            ))
        })?;
        let end = offset
            .checked_add(length)
            .ok_or_else(|| CoreError::gltf(format!("buffer view {index} range overflows")))?;
        if end > buffer_data.len() {
            return Err(CoreError::gltf(format!(
                "buffer view {index} extends beyond its buffer"
            )));
        }
        result.push(BufferView {
            buffer,
            offset,
            length,
            stride,
        });
    }
    Ok(result)
}

fn parse_accessors(
    root: &Map<String, Value>,
    limits: &GltfLimits,
) -> Result<Vec<Accessor>, CoreError> {
    let Some(values) = optional_array_in_object(root, "accessors")? else {
        return Ok(Vec::new());
    };
    if values.len() > limits.accessors {
        return Err(CoreError::gltf(format!(
            "document contains more than {} accessors",
            limits.accessors
        )));
    }
    let mut result = Vec::new();
    try_reserve(&mut result, values.len(), "accessors")?;
    for (index, value) in values.iter().enumerate() {
        let object = value
            .as_object()
            .ok_or_else(|| CoreError::gltf(format!("accessor {index} is not an object")))?;
        let count = usize_from_u64(
            required_u64(object.get("count"), &format!("accessor {index} count"))?,
            &format!("accessor {index} count"),
        )?;
        if count > limits.vertices.saturating_mul(4) {
            return Err(CoreError::gltf(format!(
                "accessor {index} count is too large"
            )));
        }
        let component = ComponentType::parse(
            object.get("componentType"),
            &format!("accessor {index} componentType"),
        )?;
        let kind = AccessorType::parse(object.get("type"), &format!("accessor {index} type"))?;
        let view = object
            .get("bufferView")
            .map(|value| {
                usize_from_u64(
                    required_u64(Some(value), &format!("accessor {index} bufferView"))?,
                    &format!("accessor {index} bufferView"),
                )
            })
            .transpose()?;
        let offset = usize_from_u64(
            optional_u64(
                object.get("byteOffset"),
                0,
                &format!("accessor {index} byteOffset"),
            )?,
            &format!("accessor {index} byteOffset"),
        )?;
        let normalized = match object.get("normalized") {
            Some(value) => value.as_bool().ok_or_else(|| {
                CoreError::gltf(format!("accessor {index} normalized is not boolean"))
            })?,
            None => false,
        };
        let sparse = parse_sparse(object.get("sparse"), index, count)?;
        result.push(Accessor {
            view,
            offset,
            count,
            component,
            kind,
            normalized,
            sparse,
        });
    }
    Ok(result)
}

fn parse_sparse(
    value: Option<&Value>,
    accessor_index: usize,
    accessor_count: usize,
) -> Result<Option<SparseAccessor>, CoreError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let object = value.as_object().ok_or_else(|| {
        CoreError::gltf(format!("accessor {accessor_index} sparse is not an object"))
    })?;
    let count = usize_from_u64(
        required_u64(
            object.get("count"),
            &format!("accessor {accessor_index} sparse count"),
        )?,
        &format!("accessor {accessor_index} sparse count"),
    )?;
    if count > accessor_count {
        return Err(CoreError::gltf(format!(
            "accessor {accessor_index} sparse count exceeds accessor count"
        )));
    }
    let indices = object
        .get("indices")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            CoreError::gltf(format!(
                "accessor {accessor_index} sparse indices is missing"
            ))
        })?;
    let indices_type = ComponentType::parse(
        indices.get("componentType"),
        &format!("accessor {accessor_index} sparse index componentType"),
    )?;
    if !indices_type.is_unsigned_index() {
        return Err(CoreError::gltf(format!(
            "accessor {accessor_index} sparse indices must be unsigned"
        )));
    }
    let indices_view = usize_from_u64(
        required_u64(
            indices.get("bufferView"),
            &format!("accessor {accessor_index} sparse index bufferView"),
        )?,
        "sparse index bufferView",
    )?;
    let indices_offset = usize_from_u64(
        optional_u64(
            indices.get("byteOffset"),
            0,
            &format!("accessor {accessor_index} sparse index byteOffset"),
        )?,
        "sparse index byteOffset",
    )?;
    let values = object
        .get("values")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            CoreError::gltf(format!(
                "accessor {accessor_index} sparse values is missing"
            ))
        })?;
    let values_view = usize_from_u64(
        required_u64(
            values.get("bufferView"),
            &format!("accessor {accessor_index} sparse values bufferView"),
        )?,
        "sparse values bufferView",
    )?;
    let values_offset = usize_from_u64(
        optional_u64(
            values.get("byteOffset"),
            0,
            &format!("accessor {accessor_index} sparse values byteOffset"),
        )?,
        "sparse values byteOffset",
    )?;
    Ok(Some(SparseAccessor {
        count,
        indices_view,
        indices_offset,
        indices_type,
        values_view,
        values_offset,
    }))
}

fn parse_materials(
    root: &Map<String, Value>,
    limits: &GltfLimits,
) -> Result<Vec<Option<[f32; 4]>>, CoreError> {
    let Some(values) = optional_array_in_object(root, "materials")? else {
        return Ok(Vec::new());
    };
    if values.len() > limits.materials {
        return Err(CoreError::gltf(format!(
            "document contains more than {} materials",
            limits.materials
        )));
    }
    let mut materials = Vec::new();
    try_reserve(&mut materials, values.len(), "materials")?;
    for (index, value) in values.iter().enumerate() {
        let object = value
            .as_object()
            .ok_or_else(|| CoreError::gltf(format!("material {index} is not an object")))?;
        let Some(pbr) = object.get("pbrMetallicRoughness") else {
            materials.push(None);
            continue;
        };
        let pbr = pbr.as_object().ok_or_else(|| {
            CoreError::gltf(format!("material {index} PBR field is not an object"))
        })?;
        let values = match pbr.get("baseColorFactor") {
            None => [1.0, 1.0, 1.0, 1.0],
            Some(value) => {
                let values = value.as_array().ok_or_else(|| {
                    CoreError::gltf(format!("material {index} baseColorFactor is not an array"))
                })?;
                if values.len() != 4 {
                    return Err(CoreError::gltf(format!(
                        "material {index} baseColorFactor must have four values"
                    )));
                }
                [
                    number_f64(values.first(), "material red")? as f32,
                    number_f64(values.get(1), "material green")? as f32,
                    number_f64(values.get(2), "material blue")? as f32,
                    number_f64(values.get(3), "material alpha")? as f32,
                ]
            }
        };
        if values
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(CoreError::gltf(format!(
                "material {index} baseColorFactor is outside 0..=1"
            )));
        }
        materials.push(Some(values));
    }
    Ok(materials)
}

fn usize_from_u64(value: u64, label: &str) -> Result<usize, CoreError> {
    usize::try_from(value).map_err(|_| CoreError::gltf(format!("{label} is too large")))
}

fn millimetres(value: Option<f32>, label: &str) -> Result<f32, CoreError> {
    let value = value.ok_or_else(|| CoreError::gltf(format!("{label} is missing")))?;
    let value = value * 1_000.0;
    if !value.is_finite() {
        return Err(CoreError::gltf(format!("{label} is outside float32 range")));
    }
    Ok(value)
}

fn triangulate_indices(
    indices: &[u32],
    mode: u64,
    mesh_index: usize,
    primitive_index: usize,
) -> Result<Vec<[u32; 3]>, CoreError> {
    let mut faces = Vec::new();
    match mode {
        4 => {
            if !indices.len().is_multiple_of(3) {
                return Err(CoreError::gltf(format!(
                    "mesh {mesh_index} primitive {primitive_index} triangle index count is not divisible by three"
                )));
            }
            try_reserve(&mut faces, indices.len() / 3, "triangle faces")?;
            let (triangles, remainder) = indices.as_chunks::<3>();
            if !remainder.is_empty() {
                return Err(CoreError::gltf(format!(
                    "mesh {mesh_index} primitive {primitive_index} triangle index count is not divisible by three"
                )));
            }
            for triangle in triangles {
                let Some(first) = triangle.first() else {
                    return Err(CoreError::gltf("triangle chunk is empty"));
                };
                let Some(second) = triangle.get(1) else {
                    return Err(CoreError::gltf("triangle chunk is incomplete"));
                };
                let Some(third) = triangle.get(2) else {
                    return Err(CoreError::gltf("triangle chunk is incomplete"));
                };
                faces.push([*first, *second, *third]);
            }
        }
        5 => {
            if indices.len() >= 3 {
                try_reserve(
                    &mut faces,
                    indices.len().saturating_sub(2),
                    "triangle strip faces",
                )?;
                for position in 0..indices.len().saturating_sub(2) {
                    let a = *indices
                        .get(position)
                        .ok_or_else(|| CoreError::gltf("triangle strip index is missing"))?;
                    let b = *indices
                        .get(position + 1)
                        .ok_or_else(|| CoreError::gltf("triangle strip index is missing"))?;
                    let c = *indices
                        .get(position + 2)
                        .ok_or_else(|| CoreError::gltf("triangle strip index is missing"))?;
                    if position % 2 == 0 {
                        faces.push([a, b, c]);
                    } else {
                        faces.push([b, a, c]);
                    }
                }
            }
        }
        6 => {
            if indices.len() >= 3 {
                let first = *indices
                    .first()
                    .ok_or_else(|| CoreError::gltf("triangle fan has no first index"))?;
                try_reserve(
                    &mut faces,
                    indices.len().saturating_sub(2),
                    "triangle fan faces",
                )?;
                for position in 1..indices.len().saturating_sub(1) {
                    let second = *indices
                        .get(position)
                        .ok_or_else(|| CoreError::gltf("triangle fan index is missing"))?;
                    let third = *indices
                        .get(position + 1)
                        .ok_or_else(|| CoreError::gltf("triangle fan index is missing"))?;
                    faces.push([first, second, third]);
                }
            }
        }
        other => {
            return Err(CoreError::gltf(format!(
                "mesh {mesh_index} primitive {primitive_index} uses unsupported mode {other}"
            )));
        }
    }
    Ok(faces)
}

fn scene_roots(root: &Map<String, Value>, nodes: &[Value]) -> Result<Vec<usize>, CoreError> {
    if let Some(scenes) = root.get("scenes") {
        let scenes = scenes
            .as_array()
            .ok_or_else(|| CoreError::gltf("glTF scenes is not an array"))?;
        if scenes.is_empty() {
            return Err(CoreError::gltf("glTF scenes array is empty"));
        }
        let scene_index_value = match root.get("scene") {
            Some(value) => required_u64(Some(value), "default scene")?,
            None => 0,
        };
        let scene_index = usize_from_u64(scene_index_value, "default scene")?;
        let scene = scenes
            .get(scene_index)
            .and_then(Value::as_object)
            .ok_or_else(|| CoreError::gltf("default scene is missing"))?;
        let roots = scene
            .get("nodes")
            .and_then(Value::as_array)
            .ok_or_else(|| CoreError::gltf("default scene has no nodes array"))?;
        let mut result = Vec::new();
        try_reserve(&mut result, roots.len(), "scene roots")?;
        for (position, value) in roots.iter().enumerate() {
            let index = usize_from_u64(
                required_u64(Some(value), &format!("scene root {position}"))?,
                &format!("scene root {position}"),
            )?;
            if index >= nodes.len() {
                return Err(CoreError::gltf(format!("scene root {index} is missing")));
            }
            result.push(index);
        }
        return Ok(result);
    }
    let mut has_parent = vec![false; nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        let node = node
            .as_object()
            .ok_or_else(|| CoreError::gltf(format!("node {index} is not an object")))?;
        if let Some(children) = node.get("children") {
            let children = children
                .as_array()
                .ok_or_else(|| CoreError::gltf(format!("node {index} children is not an array")))?;
            for child in children {
                let child = usize_from_u64(required_u64(Some(child), "node child")?, "node child")?;
                let slot = has_parent.get_mut(child).ok_or_else(|| {
                    CoreError::gltf(format!("node {index} references missing child {child}"))
                })?;
                *slot = true;
            }
        }
    }
    let mut roots = Vec::new();
    for (index, parent) in has_parent.into_iter().enumerate() {
        if !parent {
            roots.push(index);
        }
    }
    Ok(roots)
}

fn node_name(
    node: &Map<String, Value>,
    mesh_index: usize,
    node_index: usize,
    root: &Value,
) -> Result<String, CoreError> {
    if let Some(name) = node.get("name") {
        let name = name
            .as_str()
            .ok_or_else(|| CoreError::gltf(format!("node {node_index} name is not a string")))?;
        if !name.trim().is_empty() {
            return Ok(name.to_owned());
        }
    }
    if let Some(meshes) = root.get("meshes").and_then(Value::as_array)
        && let Some(name) = meshes
            .get(mesh_index)
            .and_then(Value::as_object)
            .and_then(|mesh| mesh.get("name"))
    {
        let name = name
            .as_str()
            .ok_or_else(|| CoreError::gltf(format!("mesh {mesh_index} name is not a string")))?;
        if !name.trim().is_empty() {
            return Ok(name.to_owned());
        }
    }
    Ok(format!("Object {}", node_index.saturating_add(1)))
}

fn parse_node_transform(node: &Map<String, Value>, index: usize) -> Result<Transform, CoreError> {
    if let Some(matrix) = node.get("matrix") {
        let values = matrix
            .as_array()
            .ok_or_else(|| CoreError::gltf(format!("node {index} matrix is not an array")))?;
        if values.len() != 16 {
            return Err(CoreError::gltf(format!(
                "node {index} matrix must have 16 values"
            )));
        }
        let mut rows = [[0.0_f32; 4]; 4];
        for (row, row_values) in rows.iter_mut().enumerate() {
            for (column, output) in row_values.iter_mut().enumerate() {
                let value = number_f64(values.get(column * 4 + row), "node matrix entry")?;
                if value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
                    return Err(CoreError::gltf(format!(
                        "node {index} matrix entry is too large"
                    )));
                }
                *output = value as f32;
            }
        }
        for row in rows.iter_mut().take(3) {
            row[3] *= 1_000.0;
        }
        return Transform::from_rows(rows)
            .map_err(|error| CoreError::gltf(format!("node {index} matrix is invalid: {error}")));
    }
    let translation = vector3(
        node.get("translation"),
        [0.0, 0.0, 0.0],
        "translation",
        index,
    )?;
    let scale = vector3(node.get("scale"), [1.0, 1.0, 1.0], "scale", index)?;
    let rotation = vector4(
        node.get("rotation"),
        [0.0, 0.0, 0.0, 1.0],
        "rotation",
        index,
    )?;
    let length = (rotation[0] * rotation[0]
        + rotation[1] * rotation[1]
        + rotation[2] * rotation[2]
        + rotation[3] * rotation[3])
        .sqrt();
    if !length.is_finite() || length <= f32::EPSILON {
        return Err(CoreError::gltf(format!("node {index} rotation is invalid")));
    }
    let [x, y, z, w] = [
        rotation[0] / length,
        rotation[1] / length,
        rotation[2] / length,
        rotation[3] / length,
    ];
    let rows = [
        [
            (1.0 - 2.0 * (y * y + z * z)) * scale[0],
            (2.0 * (x * y - z * w)) * scale[1],
            (2.0 * (x * z + y * w)) * scale[2],
            translation[0] * 1_000.0,
        ],
        [
            (2.0 * (x * y + z * w)) * scale[0],
            (1.0 - 2.0 * (x * x + z * z)) * scale[1],
            (2.0 * (y * z - x * w)) * scale[2],
            translation[1] * 1_000.0,
        ],
        [
            (2.0 * (x * z - y * w)) * scale[0],
            (2.0 * (y * z + x * w)) * scale[1],
            (1.0 - 2.0 * (x * x + y * y)) * scale[2],
            translation[2] * 1_000.0,
        ],
        [0.0, 0.0, 0.0, 1.0],
    ];
    if rows.iter().flatten().any(|value| !value.is_finite()) {
        return Err(CoreError::gltf(format!(
            "node {index} transform is non-finite"
        )));
    }
    Transform::from_rows(rows)
        .map_err(|error| CoreError::gltf(format!("node {index} transform is invalid: {error}")))
}

fn vector3(
    value: Option<&Value>,
    default: [f32; 3],
    label: &str,
    index: usize,
) -> Result<[f32; 3], CoreError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let values = value
        .as_array()
        .ok_or_else(|| CoreError::gltf(format!("node {index} {label} is not an array")))?;
    if values.len() != 3 {
        return Err(CoreError::gltf(format!(
            "node {index} {label} must have three values"
        )));
    }
    Ok([
        number_f64(values.first(), &format!("node {index} {label}"))? as f32,
        number_f64(values.get(1), &format!("node {index} {label}"))? as f32,
        number_f64(values.get(2), &format!("node {index} {label}"))? as f32,
    ])
}

fn vector4(
    value: Option<&Value>,
    default: [f32; 4],
    label: &str,
    index: usize,
) -> Result<[f32; 4], CoreError> {
    let Some(value) = value else {
        return Ok(default);
    };
    let values = value
        .as_array()
        .ok_or_else(|| CoreError::gltf(format!("node {index} {label} is not an array")))?;
    if values.len() != 4 {
        return Err(CoreError::gltf(format!(
            "node {index} {label} must have four values"
        )));
    }
    Ok([
        number_f64(values.first(), &format!("node {index} {label}"))? as f32,
        number_f64(values.get(1), &format!("node {index} {label}"))? as f32,
        number_f64(values.get(2), &format!("node {index} {label}"))? as f32,
        number_f64(values.get(3), &format!("node {index} {label}"))? as f32,
    ])
}

fn build_export(items: &[GltfItem], buffer_uri: Option<&str>) -> Result<ExportPackage, CoreError> {
    if items.is_empty() {
        return Err(CoreError::gltf("no visible objects to export"));
    }
    if items.len() > MAX_GLTF_ITEMS {
        return Err(CoreError::gltf(format!(
            "export exceeds the object limit of {MAX_GLTF_ITEMS}"
        )));
    }
    let mut binary = Vec::new();
    let mut accessors = Vec::new();
    let mut views = Vec::new();
    let mut meshes = Vec::new();
    let mut materials = Vec::new();
    let mut nodes = Vec::new();
    let mut scene_nodes = Vec::new();
    let mut names = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        item.mesh.validate_non_empty()?;
        if item.mesh.vertex_count() > MAX_GLTF_VERTICES || item.mesh.face_count() > MAX_GLTF_FACES {
            return Err(CoreError::gltf(format!(
                "object {} exceeds the GLTF mesh complexity limit",
                index.saturating_add(1)
            )));
        }
        validate_color(item.color)?;
        let name = unique_name(&item.name, index, &mut names);
        let position_data = positions_bytes(&item.mesh)?;
        let normal_data = normals_bytes(&item.mesh)?;
        let index_data = indices_bytes(&item.mesh)?;
        let position_view = append_view(&mut binary, &mut views, position_data, 34962)?;
        let normal_view = append_view(&mut binary, &mut views, normal_data, 34962)?;
        let index_view = append_view(&mut binary, &mut views, index_data, 34963)?;
        let position_accessor = append_position_accessor(&mut accessors, &item.mesh)?;
        let normal_accessor = append_normal_accessor(&mut accessors, &item.mesh)?;
        let index_accessor = append_index_accessor(&mut accessors, &item.mesh)?;
        let position_view_index = position_view;
        let normal_view_index = normal_view;
        let index_view_index = index_view;
        set_accessor_view(&mut accessors, position_accessor, position_view_index)?;
        set_accessor_view(&mut accessors, normal_accessor, normal_view_index)?;
        set_accessor_view(&mut accessors, index_accessor, index_view_index)?;
        let color = match item.color {
            Some(color) => color,
            None => DEFAULT_MESH_COLOR,
        };
        let material_index = materials.len();
        materials.push(material_value(color));
        let attributes = Map::from_iter([
            ("POSITION".to_owned(), Value::from(position_accessor)),
            ("NORMAL".to_owned(), Value::from(normal_accessor)),
        ]);
        let primitive = Value::Object(Map::from_iter([
            ("attributes".to_owned(), Value::Object(attributes)),
            ("indices".to_owned(), Value::from(index_accessor)),
            ("mode".to_owned(), Value::from(4_u64)),
            ("material".to_owned(), Value::from(material_index)),
        ]));
        meshes.push(Value::Object(Map::from_iter([
            ("name".to_owned(), Value::String(format!("mesh_{index}"))),
            ("primitives".to_owned(), Value::Array(vec![primitive])),
        ])));
        nodes.push(Value::Object(Map::from_iter([
            ("name".to_owned(), Value::String(name)),
            ("mesh".to_owned(), Value::from(index)),
            (
                "matrix".to_owned(),
                Value::Array(transform_matrix_values(item.transform)?),
            ),
        ])));
        scene_nodes.push(Value::from(index));
    }
    let uri = match buffer_uri {
        Some("data") => Some(format!(
            "data:application/octet-stream;base64,{}",
            encode_base64(&binary)?
        )),
        Some(value) => Some(value.to_owned()),
        None => None,
    };
    let mut buffer = Map::new();
    buffer.insert("byteLength".to_owned(), Value::from(binary.len()));
    if let Some(uri) = uri {
        buffer.insert("uri".to_owned(), Value::String(uri));
    }
    let root = Value::Object(Map::from_iter([
        ("scene".to_owned(), Value::from(0_u64)),
        (
            "scenes".to_owned(),
            Value::Array(vec![Value::Object(Map::from_iter([(
                "nodes".to_owned(),
                Value::Array(scene_nodes),
            )]))]),
        ),
        (
            "asset".to_owned(),
            Value::Object(Map::from_iter([
                ("version".to_owned(), Value::String("2.0".to_owned())),
                (
                    "generator".to_owned(),
                    Value::String("Builder3D Rust".to_owned()),
                ),
            ])),
        ),
        ("accessors".to_owned(), Value::Array(accessors)),
        ("meshes".to_owned(), Value::Array(meshes)),
        ("materials".to_owned(), Value::Array(materials)),
        ("nodes".to_owned(), Value::Array(nodes)),
        (
            "buffers".to_owned(),
            Value::Array(vec![Value::Object(buffer)]),
        ),
        ("bufferViews".to_owned(), Value::Array(views)),
    ]));
    let json = serde_json::to_vec(&root)
        .map_err(|error| CoreError::gltf(format!("could not encode glTF JSON: {error}")))?;
    if u64::try_from(json.len()).is_ok_and(|size| size > MAX_GLTF_JSON_BYTES) {
        return Err(CoreError::gltf("exported glTF JSON exceeds its limit"));
    }
    if u64::try_from(binary.len()).is_ok_and(|size| size > MAX_GLTF_BUFFER_BYTES) {
        return Err(CoreError::gltf("exported glTF buffer exceeds its limit"));
    }
    Ok(ExportPackage { json, binary })
}

fn unique_name(name: &str, index: usize, used: &mut HashSet<String>) -> String {
    let base = if name.trim().is_empty() {
        format!("Object {}", index.saturating_add(1))
    } else {
        name.to_owned()
    };
    let mut candidate = base.clone();
    let mut suffix = 2_usize;
    while used.contains(&candidate) {
        candidate = format!("{base} ({suffix})");
        suffix = suffix.saturating_add(1);
    }
    used.insert(candidate.clone());
    candidate
}

fn append_view(
    binary: &mut Vec<u8>,
    views: &mut Vec<Value>,
    data: Vec<u8>,
    target: u64,
) -> Result<usize, CoreError> {
    binary
        .try_reserve(data.len().saturating_add(3))
        .map_err(|_| CoreError::gltf("GLTF buffer exceeds available memory"))?;
    while !binary.len().is_multiple_of(4) {
        binary.push(0);
    }
    let offset = binary.len();
    let length = data.len();
    binary.extend_from_slice(&data);
    let view_index = views.len();
    views.push(Value::Object(Map::from_iter([
        ("buffer".to_owned(), Value::from(0_u64)),
        ("byteOffset".to_owned(), Value::from(offset)),
        ("byteLength".to_owned(), Value::from(length)),
        ("target".to_owned(), Value::from(target)),
    ])));
    Ok(view_index)
}

fn set_accessor_view(
    accessors: &mut [Value],
    accessor_index: usize,
    view_index: usize,
) -> Result<(), CoreError> {
    let accessor = accessors
        .get_mut(accessor_index)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| CoreError::gltf("export accessor is missing"))?;
    accessor.insert("bufferView".to_owned(), Value::from(view_index));
    Ok(())
}

fn positions_bytes(mesh: &Mesh) -> Result<Vec<u8>, CoreError> {
    let mut output = Vec::new();
    try_reserve(
        &mut output,
        mesh.vertex_count().saturating_mul(12),
        "position buffer",
    )?;
    for vertex in &mesh.vertices {
        for value in vertex {
            let value = *value / 1_000.0;
            if !value.is_finite() {
                return Err(CoreError::gltf("position is not finite in metres"));
            }
            output.extend_from_slice(&value.to_le_bytes());
        }
    }
    Ok(output)
}

fn normals_bytes(mesh: &Mesh) -> Result<Vec<u8>, CoreError> {
    let normals = vertex_normals(mesh)?;
    let mut output = Vec::new();
    try_reserve(
        &mut output,
        normals.len().saturating_mul(12),
        "normal buffer",
    )?;
    for normal in normals {
        for value in normal {
            output.extend_from_slice(&(value.to_le_bytes()));
        }
    }
    Ok(output)
}

fn indices_bytes(mesh: &Mesh) -> Result<Vec<u8>, CoreError> {
    let mut output = Vec::new();
    try_reserve(
        &mut output,
        mesh.face_count().saturating_mul(12),
        "index buffer",
    )?;
    for face in &mesh.faces {
        for index in face {
            output.extend_from_slice(&index.to_le_bytes());
        }
    }
    Ok(output)
}

fn vertex_normals(mesh: &Mesh) -> Result<Vec<[f32; 3]>, CoreError> {
    let mut sums = Vec::new();
    try_reserve(&mut sums, mesh.vertex_count(), "vertex normals")?;
    sums.resize(mesh.vertex_count(), [0.0; 3]);
    for face in &mesh.faces {
        let a = mesh
            .vertices
            .get(face[0] as usize)
            .ok_or_else(|| CoreError::gltf("face references a missing vertex"))?;
        let b = mesh
            .vertices
            .get(face[1] as usize)
            .ok_or_else(|| CoreError::gltf("face references a missing vertex"))?;
        let c = mesh
            .vertices
            .get(face[2] as usize)
            .ok_or_else(|| CoreError::gltf("face references a missing vertex"))?;
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let cross = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        for index in face {
            let sum = sums
                .get_mut(*index as usize)
                .ok_or_else(|| CoreError::gltf("normal vertex is missing"))?;
            for axis in 0..3 {
                sum[axis] += cross[axis];
            }
        }
    }
    for sum in &mut sums {
        let length = (sum[0] * sum[0] + sum[1] * sum[1] + sum[2] * sum[2]).sqrt();
        if !length.is_finite() {
            return Err(CoreError::gltf("vertex normal is non-finite"));
        }
        if length > f32::EPSILON {
            for value in sum {
                *value /= length;
            }
        }
    }
    Ok(sums)
}

fn append_position_accessor(accessors: &mut Vec<Value>, mesh: &Mesh) -> Result<usize, CoreError> {
    let (minimum, maximum) = bounds_m(mesh, false)?;
    let index = accessors.len();
    accessors.push(Value::Object(Map::from_iter([
        ("componentType".to_owned(), Value::from(5126_u64)),
        ("type".to_owned(), Value::String("VEC3".to_owned())),
        ("count".to_owned(), Value::from(mesh.vertex_count())),
        (
            "min".to_owned(),
            Value::Array(minimum.into_iter().map(Value::from).collect()),
        ),
        (
            "max".to_owned(),
            Value::Array(maximum.into_iter().map(Value::from).collect()),
        ),
    ])));
    Ok(index)
}

fn append_normal_accessor(accessors: &mut Vec<Value>, mesh: &Mesh) -> Result<usize, CoreError> {
    let normals = vertex_normals(mesh)?;
    let (minimum, maximum) = bounds_for_points(&normals, true)?;
    let index = accessors.len();
    accessors.push(Value::Object(Map::from_iter([
        ("componentType".to_owned(), Value::from(5126_u64)),
        ("type".to_owned(), Value::String("VEC3".to_owned())),
        ("count".to_owned(), Value::from(mesh.vertex_count())),
        (
            "min".to_owned(),
            Value::Array(minimum.into_iter().map(Value::from).collect()),
        ),
        (
            "max".to_owned(),
            Value::Array(maximum.into_iter().map(Value::from).collect()),
        ),
    ])));
    Ok(index)
}

fn append_index_accessor(accessors: &mut Vec<Value>, mesh: &Mesh) -> Result<usize, CoreError> {
    let (minimum, maximum) = index_bounds(mesh)?;
    let index = accessors.len();
    accessors.push(Value::Object(Map::from_iter([
        ("componentType".to_owned(), Value::from(5125_u64)),
        ("type".to_owned(), Value::String("SCALAR".to_owned())),
        (
            "count".to_owned(),
            Value::from(mesh.face_count().saturating_mul(3)),
        ),
        ("min".to_owned(), Value::Array(vec![Value::from(minimum)])),
        ("max".to_owned(), Value::Array(vec![Value::from(maximum)])),
    ])));
    Ok(index)
}

fn bounds_m(mesh: &Mesh, normals: bool) -> Result<([f32; 3], [f32; 3]), CoreError> {
    if normals {
        let points = vertex_normals(mesh)?;
        bounds_for_points(&points, true)
    } else {
        let points = mesh
            .vertices
            .iter()
            .map(|point| [point[0] / 1_000.0, point[1] / 1_000.0, point[2] / 1_000.0])
            .collect::<Vec<_>>();
        bounds_for_points(&points, false)
    }
}

fn bounds_for_points(
    points: &[[f32; 3]],
    allow_empty: bool,
) -> Result<([f32; 3], [f32; 3]), CoreError> {
    let first = match points.first() {
        Some(point) => *point,
        None if allow_empty => [0.0; 3],
        None => return Err(CoreError::gltf("cannot calculate empty accessor bounds")),
    };
    if points.iter().flatten().any(|value| !value.is_finite()) {
        return Err(CoreError::gltf(
            "accessor bounds contain a non-finite value",
        ));
    }
    let mut minimum = first;
    let mut maximum = first;
    for point in points.iter().skip(1) {
        for axis in 0..3 {
            minimum[axis] = minimum[axis].min(point[axis]);
            maximum[axis] = maximum[axis].max(point[axis]);
        }
    }
    Ok((minimum, maximum))
}

fn index_bounds(mesh: &Mesh) -> Result<(u32, u32), CoreError> {
    let mut minimum = u32::MAX;
    let mut maximum = 0_u32;
    for face in &mesh.faces {
        for index in face {
            minimum = minimum.min(*index);
            maximum = maximum.max(*index);
        }
    }
    if minimum == u32::MAX {
        return Err(CoreError::gltf("cannot calculate empty index bounds"));
    }
    Ok((minimum, maximum))
}

fn material_value(color: [f32; 4]) -> Value {
    let alpha_mode = if color[3] < 1.0 { "BLEND" } else { "OPAQUE" };
    Value::Object(Map::from_iter([
        (
            "pbrMetallicRoughness".to_owned(),
            Value::Object(Map::from_iter([
                (
                    "baseColorFactor".to_owned(),
                    Value::Array(color.into_iter().map(Value::from).collect()),
                ),
                ("roughnessFactor".to_owned(), Value::from(0.8_f32)),
                ("metallicFactor".to_owned(), Value::from(0.0_f32)),
            ])),
        ),
        ("alphaMode".to_owned(), Value::String(alpha_mode.to_owned())),
        ("doubleSided".to_owned(), Value::from(false)),
    ]))
}

fn transform_matrix_values(transform: Transform) -> Result<Vec<Value>, CoreError> {
    let rows = transform.rows();
    let mut values = Vec::new();
    try_reserve(&mut values, 16, "node matrix")?;
    for column in 0..4 {
        for (row, row_values) in rows.iter().enumerate() {
            let mut value = *row_values
                .get(column)
                .ok_or_else(|| CoreError::gltf("node transform matrix entry is missing"))?;
            if column == 3 && row < 3 {
                value /= 1_000.0;
            }
            if !value.is_finite() {
                return Err(CoreError::gltf("node transform is non-finite"));
            }
            values.push(Value::from(value));
        }
    }
    Ok(values)
}

fn encode_base64(input: &[u8]) -> Result<String, CoreError> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    output
        .try_reserve(input.len().saturating_mul(4).div_ceil(3))
        .map_err(|_| CoreError::gltf("base64 output exceeds available memory"))?;
    let (chunks, remainder) = input.as_chunks::<3>();
    for chunk in chunks {
        let a = chunk[0];
        let b = chunk[1];
        let c = chunk[2];
        output.push(TABLE[(a >> 2) as usize] as char);
        output.push(TABLE[((a & 0x03) << 4 | b >> 4) as usize] as char);
        output.push(TABLE[((b & 0x0f) << 2 | c >> 6) as usize] as char);
        output.push(TABLE[(c & 0x3f) as usize] as char);
    }
    match remainder {
        [] => {}
        [a] => {
            output.push(TABLE[(a >> 2) as usize] as char);
            output.push(TABLE[((a & 0x03) << 4) as usize] as char);
            output.push('=');
            output.push('=');
        }
        [a, b] => {
            output.push(TABLE[(a >> 2) as usize] as char);
            output.push(TABLE[((a & 0x03) << 4 | b >> 4) as usize] as char);
            output.push(TABLE[((b & 0x0f) << 2) as usize] as char);
            output.push('=');
        }
        _ => {}
    }
    Ok(output)
}

fn pad_glb_json(json: &[u8]) -> Vec<u8> {
    let mut output = json.to_vec();
    while !output.len().is_multiple_of(4) {
        output.push(b' ');
    }
    output
}

fn pad_glb_binary(binary: &[u8]) -> Vec<u8> {
    let mut output = binary.to_vec();
    while !output.len().is_multiple_of(4) {
        output.push(0);
    }
    output
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let parent = path.parent().map_or(Path::new("."), |parent| parent);
    fs::create_dir_all(parent).map_err(|error| {
        CoreError::gltf(format!("could not create {}: {error}", parent.display()))
    })?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| CoreError::gltf("GLTF output path has no valid file name"))?;
    let seed = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), seed));
    let result = (|| -> Result<(), CoreError> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| {
                CoreError::gltf(format!("could not create temporary output: {error}"))
            })?;
        file.write_all(bytes).map_err(|error| {
            CoreError::gltf(format!("could not write temporary output: {error}"))
        })?;
        file.sync_all().map_err(|error| {
            CoreError::gltf(format!("could not flush temporary output: {error}"))
        })?;
        drop(file);
        match fs::rename(&temporary, path) {
            Ok(()) => Ok(()),
            Err(_error) if path.exists() => {
                fs::remove_file(path).map_err(|error| {
                    CoreError::gltf(format!("could not replace existing output: {error}"))
                })?;
                fs::rename(&temporary, path)
                    .map_err(|error| CoreError::gltf(format!("could not finalize output: {error}")))
            }
            Err(error) => Err(CoreError::gltf(format!(
                "could not finalize output: {error}"
            ))),
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
