use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;

use crate::{
    CoreError, GltfItem, Mesh, ThreeMfItem, Transform, WrlItem, export_3mf, export_glb,
    export_gltf, export_wrl, load_3mf_bytes, load_gltf, load_gltf_bytes,
};

/// Maximum bytes accepted by one imported mesh file.
pub const MAX_MESH_FILE_BYTES: u64 = 512 * 1024 * 1024;
/// Maximum vertices accepted by one imported mesh file.
pub const MAX_MESH_VERTICES: usize = 5_000_000;
/// Maximum triangular faces accepted by one imported mesh file.
pub const MAX_MESH_FACES: usize = 5_000_000;
const MAX_PLY_HEADER_BYTES: usize = 1024 * 1024;
const MAX_MESH_LINE_BYTES: usize = 16 * 1024 * 1024;

/// Mesh formats supported by the portable core file boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeshFormat {
    /// STL, written as binary and read as binary or ASCII.
    Stl,
    /// Wavefront OBJ with optional per-vertex RGB(A) values.
    Obj,
    /// PLY, written as binary little-endian and read as ASCII or binary.
    Ply,
    /// 3MF, read as a single mesh through this boundary; use [`crate::load_3mf`]
    /// for the full multi-object scene representation.
    ThreeMf,
    /// glTF JSON, read as a single transformed mesh through this boundary.
    Gltf,
    /// Binary glTF, read as a single transformed mesh through this boundary.
    Glb,
    /// VRML 2.0/WRL export; import is not supported by the portable boundary.
    Wrl,
}

impl MeshFormat {
    /// Selects a format from a path extension without applying any unit scale.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .ok_or_else(|| CoreError::UnsupportedMeshFormat(path.display().to_string()))?;
        match extension.as_str() {
            "stl" => Ok(Self::Stl),
            "obj" => Ok(Self::Obj),
            "ply" => Ok(Self::Ply),
            "3mf" => Ok(Self::ThreeMf),
            "gltf" => Ok(Self::Gltf),
            "glb" => Ok(Self::Glb),
            "wrl" => Ok(Self::Wrl),
            _ => Err(CoreError::UnsupportedMeshFormat(extension)),
        }
    }
}

/// A mesh plus optional per-vertex RGBA data read from a file format.
///
/// Geometry remains authoritative in Mesh. Colors are kept at the file
/// boundary so a format with vertex colors does not force render or scene
/// concerns into the portable geometry type.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshAsset {
    mesh: Mesh,
    vertex_colors: Option<Vec<[u8; 4]>>,
}

impl MeshAsset {
    /// Creates a validated non-empty asset and checks color cardinality.
    pub fn new(mesh: Mesh, vertex_colors: Option<Vec<[u8; 4]>>) -> Result<Self, CoreError> {
        mesh.validate_non_empty()?;
        if let Some(colors) = &vertex_colors
            && colors.len() != mesh.vertex_count()
        {
            return Err(CoreError::MeshFile(format!(
                "vertex color count {} does not match vertex count {}",
                colors.len(),
                mesh.vertex_count()
            )));
        }
        Ok(Self {
            mesh,
            vertex_colors,
        })
    }

    /// Returns the authoritative geometry.
    #[must_use]
    pub const fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    /// Returns optional per-vertex RGBA colors in 8-bit channels.
    #[must_use]
    pub fn vertex_colors(&self) -> Option<&[[u8; 4]]> {
        self.vertex_colors.as_deref()
    }

    /// Returns the representative color used by the Python import boundary.
    #[must_use]
    pub fn display_color(&self) -> Option<[f32; 4]> {
        let colors = self.vertex_colors.as_deref()?;
        if colors.is_empty() {
            return None;
        }
        let mut totals = [0_u64; 4];
        for color in colors {
            for channel in 0..4 {
                totals[channel] = totals[channel].saturating_add(u64::from(color[channel]));
            }
        }
        let count = colors.len() as f32;
        Some([
            totals[0] as f32 / count / 255.0,
            totals[1] as f32 / count / 255.0,
            totals[2] as f32 / count / 255.0,
            totals[3] as f32 / count / 255.0,
        ])
    }
}

/// Loads a mesh from a supported path and discards optional file colors.
pub fn load_mesh(path: impl AsRef<Path>) -> Result<Mesh, CoreError> {
    Ok(load_mesh_asset(path)?.mesh)
}

/// Loads a mesh and preserves optional file colors at the import boundary.
pub fn load_mesh_asset(path: impl AsRef<Path>) -> Result<MeshAsset, CoreError> {
    let path = path.as_ref();
    let format = MeshFormat::from_path(path)?;
    if matches!(format, MeshFormat::Gltf | MeshFormat::Glb) {
        return gltf_asset(load_gltf(path)?);
    }
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_MESH_FILE_BYTES {
        return Err(CoreError::MeshFile(format!(
            "file exceeds the maximum size of {MAX_MESH_FILE_BYTES} bytes"
        )));
    }
    let bytes = fs::read(path)?;
    if u64::try_from(bytes.len()).is_ok_and(|size| size > MAX_MESH_FILE_BYTES) {
        return Err(CoreError::MeshFile(format!(
            "file exceeds the maximum size of {MAX_MESH_FILE_BYTES} bytes"
        )));
    }
    load_mesh_asset_bytes(format, &bytes)
}

/// Loads a mesh from format-tagged bytes without applying a unit scale.
pub fn load_mesh_bytes(format: MeshFormat, bytes: &[u8]) -> Result<Mesh, CoreError> {
    Ok(load_mesh_asset_bytes(format, bytes)?.mesh)
}

/// Loads a colored or uncolored mesh from format-tagged bytes.
pub fn load_mesh_asset_bytes(format: MeshFormat, bytes: &[u8]) -> Result<MeshAsset, CoreError> {
    let size = u64::try_from(bytes.len())
        .map_err(|_| CoreError::MeshFile("mesh file size does not fit in u64".to_owned()))?;
    if size > MAX_MESH_FILE_BYTES {
        return Err(CoreError::MeshFile(format!(
            "file exceeds the maximum size of {MAX_MESH_FILE_BYTES} bytes"
        )));
    }
    if bytes.is_empty() {
        return Err(CoreError::MeshFile("file is empty".to_owned()));
    }
    match format {
        MeshFormat::Stl => z_up_file_to_scene(decode_stl(bytes)?),
        MeshFormat::Obj => z_up_file_to_scene(decode_obj(bytes)?),
        MeshFormat::Ply => z_up_file_to_scene(decode_ply(bytes)?),
        MeshFormat::ThreeMf => decode_3mf(bytes),
        MeshFormat::Gltf | MeshFormat::Glb => decode_gltf(bytes),
        MeshFormat::Wrl => {
            let items = crate::load_wrl_bytes(bytes)?;
            let meshes = items
                .iter()
                .map(|item| item.mesh.transformed(&item.transform))
                .collect::<Result<Vec<_>, _>>()?;
            let colors = items
                .iter()
                .flat_map(|item| {
                    let rgba = item
                        .color
                        .unwrap_or(crate::DEFAULT_MESH_COLOR)
                        .map(|v| (v * 255.).round() as u8);
                    std::iter::repeat_n(rgba, item.mesh.vertex_count())
                })
                .collect();
            let mesh = Mesh::concatenate(&meshes)?;
            MeshAsset::new(mesh, Some(colors))
        }
    }
}

/// Saves an uncolored mesh using the format selected by the path extension.
pub fn save_mesh(mesh: &Mesh, path: impl AsRef<Path>) -> Result<(), CoreError> {
    let asset = MeshAsset::new(mesh.clone(), None)?;
    save_mesh_asset(&asset, path)
}

/// Saves a mesh asset using the format selected by the path extension.
pub fn save_mesh_asset(asset: &MeshAsset, path: impl AsRef<Path>) -> Result<(), CoreError> {
    let bytes = export_mesh_asset(asset, MeshFormat::from_path(path.as_ref())?)?;
    fs::write(path, bytes)?;
    Ok(())
}

/// Exports an uncolored mesh to format-tagged bytes.
pub fn export_mesh(mesh: &Mesh, format: MeshFormat) -> Result<Vec<u8>, CoreError> {
    let asset = MeshAsset::new(mesh.clone(), None)?;
    export_mesh_asset(&asset, format)
}

/// Exports a mesh asset to format-tagged bytes.
pub fn export_mesh_asset(asset: &MeshAsset, format: MeshFormat) -> Result<Vec<u8>, CoreError> {
    asset.mesh.validate_non_empty()?;
    validate_colors(asset)?;
    match format {
        MeshFormat::Stl => scene_to_z_up_file(asset)?.mesh.to_binary_stl(),
        MeshFormat::Obj => encode_obj(&scene_to_z_up_file(asset)?),
        MeshFormat::Ply => encode_binary_ply(&scene_to_z_up_file(asset)?),
        MeshFormat::ThreeMf => encode_3mf(asset),
        MeshFormat::Gltf => encode_gltf(asset),
        MeshFormat::Glb => encode_glb(asset),
        MeshFormat::Wrl => encode_wrl(asset),
    }
}

fn decode_gltf(bytes: &[u8]) -> Result<MeshAsset, CoreError> {
    gltf_asset(load_gltf_bytes(bytes)?)
}

fn gltf_asset(items: Vec<GltfItem>) -> Result<MeshAsset, CoreError> {
    if items.len() != 1 {
        return Err(CoreError::MeshFile(format!(
            "GLTF contains {} mesh instances; use load_gltf for scene import",
            items.len()
        )));
    }
    let item = items
        .first()
        .ok_or_else(|| CoreError::MeshFile("GLTF contains no mesh instances".to_owned()))?;
    let mesh = item.mesh.transformed(&item.transform)?;
    let colors = item.color.map(|color| {
        let channels = [
            (color[0] * 255.0).round() as u8,
            (color[1] * 255.0).round() as u8,
            (color[2] * 255.0).round() as u8,
            (color[3] * 255.0).round() as u8,
        ];
        vec![channels; mesh.vertex_count()]
    });
    MeshAsset::new(mesh, colors)
}

fn decode_3mf(bytes: &[u8]) -> Result<MeshAsset, CoreError> {
    let imported = load_3mf_bytes(bytes)?;
    if imported.items.len() != 1 {
        return Err(CoreError::MeshFile(format!(
            "3MF contains {} mesh instances; use load_3mf for scene import",
            imported.items.len()
        )));
    }
    let item = imported
        .items
        .first()
        .ok_or_else(|| CoreError::MeshFile("3MF contains no mesh instances".to_owned()))?;
    // The item transform carries the 3MF placement and the Z-up to Y-up turn.
    MeshAsset::new(item.mesh.transformed(&item.transform)?, None)
}

/// Turns a Z-up STL/OBJ/PLY mesh into the Y-up scene; colors are per vertex
/// and unaffected.
fn z_up_file_to_scene(asset: MeshAsset) -> Result<MeshAsset, CoreError> {
    let mesh = asset.mesh.transformed(&Transform::z_up_file_to_scene())?;
    MeshAsset::new(mesh, asset.vertex_colors)
}

/// Turns a Y-up scene mesh into Z-up file coordinates for STL/OBJ/PLY.
fn scene_to_z_up_file(asset: &MeshAsset) -> Result<MeshAsset, CoreError> {
    let mesh = asset.mesh.transformed(&Transform::scene_to_z_up_file())?;
    MeshAsset::new(mesh, asset.vertex_colors.clone())
}

fn encode_3mf(asset: &MeshAsset) -> Result<Vec<u8>, CoreError> {
    let color = asset.display_color().unwrap_or(crate::DEFAULT_MESH_COLOR);
    let item = ThreeMfItem::new(
        asset.mesh.clone(),
        Transform::identity(),
        "Object 1".to_owned(),
        color,
    )?;
    export_3mf(std::slice::from_ref(&item), None, None)
}

fn encode_gltf(asset: &MeshAsset) -> Result<Vec<u8>, CoreError> {
    let color = asset.display_color();
    let item = GltfItem::new(
        asset.mesh.clone(),
        Transform::identity(),
        "Object 1".to_owned(),
        color,
    )?;
    export_gltf(std::slice::from_ref(&item))
}

fn encode_glb(asset: &MeshAsset) -> Result<Vec<u8>, CoreError> {
    let color = asset.display_color();
    let item = GltfItem::new(
        asset.mesh.clone(),
        Transform::identity(),
        "Object 1".to_owned(),
        color,
    )?;
    export_glb(std::slice::from_ref(&item))
}

fn encode_wrl(asset: &MeshAsset) -> Result<Vec<u8>, CoreError> {
    let item = WrlItem::new(
        asset.mesh.clone(),
        Transform::identity(),
        "Object 1".to_owned(),
        asset.display_color(),
    )?;
    export_wrl(std::slice::from_ref(&item))
}

fn validate_colors(asset: &MeshAsset) -> Result<(), CoreError> {
    if let Some(colors) = &asset.vertex_colors
        && colors.len() != asset.mesh.vertex_count()
    {
        return Err(CoreError::MeshFile(format!(
            "vertex color count {} does not match vertex count {}",
            colors.len(),
            asset.mesh.vertex_count()
        )));
    }
    Ok(())
}

fn decode_stl(bytes: &[u8]) -> Result<MeshAsset, CoreError> {
    if bytes.len() >= 84 {
        let count_bytes = bytes
            .get(80..84)
            .ok_or_else(|| CoreError::MeshFile("binary STL count is truncated".to_owned()))?;
        let count = u32::from_le_bytes(
            count_bytes
                .try_into()
                .map_err(|_| CoreError::MeshFile("binary STL count is truncated".to_owned()))?,
        );
        let expected = usize::try_from(count)
            .ok()
            .and_then(|faces| faces.checked_mul(50))
            .and_then(|payload| payload.checked_add(84));
        if expected == Some(bytes.len()) && count > 0 {
            let face_count = usize::try_from(count).map_err(|_| {
                CoreError::MeshFile("binary STL face count is too large".to_owned())
            })?;
            if face_count > MAX_MESH_FACES {
                return Err(CoreError::MeshFile(format!(
                    "binary STL contains more than {MAX_MESH_FACES} faces"
                )));
            }
            return Mesh::from_binary_stl(bytes).and_then(|mesh| MeshAsset::new(mesh, None));
        }
    }
    decode_ascii_stl(bytes)
}

fn decode_ascii_stl(bytes: &[u8]) -> Result<MeshAsset, CoreError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| CoreError::MeshFile("ASCII STL is not valid UTF-8".to_owned()))?;
    let mut triangle_vertices = Vec::new();
    for (line_number, line) in text.lines().enumerate() {
        if line.len() > MAX_MESH_LINE_BYTES {
            return Err(CoreError::MeshFile(format!(
                "STL line {} exceeds the maximum length",
                line_number + 1
            )));
        }
        let mut fields = line.split_whitespace();
        if fields.next() != Some("vertex") {
            continue;
        }
        let values = [
            parse_f32(fields.next(), "STL vertex x")?,
            parse_f32(fields.next(), "STL vertex y")?,
            parse_f32(fields.next(), "STL vertex z")?,
        ];
        if fields.next().is_some() {
            return Err(CoreError::MeshFile(format!(
                "STL vertex on line {} has extra fields",
                line_number + 1
            )));
        }
        triangle_vertices.push(values);
        if triangle_vertices.len() > MAX_MESH_FACES.saturating_mul(3) {
            return Err(CoreError::MeshFile(format!(
                "ASCII STL contains more than {MAX_MESH_FACES} faces"
            )));
        }
    }
    if triangle_vertices.is_empty() || triangle_vertices.len() % 3 != 0 {
        return Err(CoreError::MeshFile(
            "ASCII STL does not contain complete triangular facets".to_owned(),
        ));
    }
    let mut vertices = Vec::with_capacity(triangle_vertices.len());
    let mut faces = Vec::with_capacity(triangle_vertices.len() / 3);
    let mut vertex_map = HashMap::new();
    let (triangles, _) = triangle_vertices.as_chunks::<3>();
    for triangle in triangles {
        let mut face = [0_u32; 3];
        for (corner, point) in triangle.iter().enumerate() {
            let key = [point[0].to_bits(), point[1].to_bits(), point[2].to_bits()];
            let index = if let Some(index) = vertex_map.get(&key) {
                *index
            } else {
                let index = u32::try_from(vertices.len())
                    .map_err(|_| CoreError::MeshFile("STL has too many vertices".to_owned()))?;
                vertices.push(*point);
                vertex_map.insert(key, index);
                index
            };
            face[corner] = index;
        }
        faces.push(face);
    }
    let mesh = Mesh::new(vertices, faces)?;
    MeshAsset::new(mesh, None)
}

fn decode_obj(bytes: &[u8]) -> Result<MeshAsset, CoreError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| CoreError::MeshFile("OBJ is not valid UTF-8".to_owned()))?;
    let mut vertices = Vec::new();
    let mut colors = Vec::new();
    let mut faces = Vec::new();
    for (line_number, line) in text.lines().enumerate() {
        if line.len() > MAX_MESH_LINE_BYTES {
            return Err(CoreError::MeshFile(format!(
                "OBJ line {} exceeds the maximum length",
                line_number + 1
            )));
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut fields = trimmed.split_whitespace();
        let Some(kind) = fields.next() else {
            continue;
        };
        match kind {
            "v" => {
                let point = [
                    parse_f32(fields.next(), "OBJ vertex x")?,
                    parse_f32(fields.next(), "OBJ vertex y")?,
                    parse_f32(fields.next(), "OBJ vertex z")?,
                ];
                let extra = fields.collect::<Vec<_>>();
                let color = match extra.len() {
                    0 => None,
                    3 | 4 => Some([
                        parse_obj_color(extra[0])?,
                        parse_obj_color(extra[1])?,
                        parse_obj_color(extra[2])?,
                        if extra.len() == 4 {
                            parse_obj_color(extra[3])?
                        } else {
                            255
                        },
                    ]),
                    _ => {
                        return Err(CoreError::MeshFile(format!(
                            "OBJ vertex on line {} has an unsupported color field count",
                            line_number + 1
                        )));
                    }
                };
                vertices.push(point);
                colors.push(color);
                if vertices.len() > MAX_MESH_VERTICES {
                    return Err(CoreError::MeshFile(format!(
                        "OBJ contains more than {MAX_MESH_VERTICES} vertices"
                    )));
                }
            }
            "f" => {
                let references = fields.collect::<Vec<_>>();
                if references.len() < 3 {
                    return Err(CoreError::MeshFile(format!(
                        "OBJ face on line {} has fewer than three vertices",
                        line_number + 1
                    )));
                }
                let first = parse_obj_index(references[0], vertices.len())?;
                for pair in references[1..].windows(2) {
                    let second = parse_obj_index(pair[0], vertices.len())?;
                    let third = parse_obj_index(pair[1], vertices.len())?;
                    if faces.len() >= MAX_MESH_FACES {
                        return Err(CoreError::MeshFile(format!(
                            "OBJ contains more than {MAX_MESH_FACES} faces"
                        )));
                    }
                    faces.push([first, second, third]);
                }
            }
            _ => {}
        }
    }
    let vertex_colors = if colors.iter().any(Option::is_some) {
        if colors.iter().any(Option::is_none) {
            return Err(CoreError::MeshFile(
                "OBJ contains colors on only some vertices".to_owned(),
            ));
        }
        Some(colors.into_iter().flatten().collect())
    } else {
        None
    };
    let mesh = Mesh::new(vertices, faces)?;
    MeshAsset::new(mesh, vertex_colors)
}

fn parse_obj_index(reference: &str, vertex_count: usize) -> Result<u32, CoreError> {
    let raw = reference
        .split('/')
        .next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| CoreError::MeshFile("OBJ face has an empty vertex index".to_owned()))?;
    let index = raw
        .parse::<i64>()
        .map_err(|_| CoreError::MeshFile(format!("invalid OBJ vertex index '{raw}'")))?;
    if index == 0 {
        return Err(CoreError::MeshFile(
            "OBJ vertex indices are one-based and cannot be zero".to_owned(),
        ));
    }
    let vertex_count = i64::try_from(vertex_count)
        .map_err(|_| CoreError::MeshFile("OBJ vertex count is too large".to_owned()))?;
    let resolved = if index > 0 {
        index - 1
    } else {
        vertex_count
            .checked_add(index)
            .ok_or_else(|| CoreError::MeshFile("OBJ negative index overflow".to_owned()))?
    };
    if resolved < 0 || resolved >= vertex_count {
        return Err(CoreError::MeshFile(format!(
            "OBJ vertex index '{index}' is out of range"
        )));
    }
    u32::try_from(resolved).map_err(|_| CoreError::MeshFile("OBJ index is too large".to_owned()))
}

fn parse_obj_color(value: &str) -> Result<u8, CoreError> {
    let value = value
        .parse::<f32>()
        .map_err(|_| CoreError::MeshFile(format!("invalid OBJ color '{value}'")))?;
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(CoreError::MeshFile(
            "OBJ colors must be finite values between 0 and 1".to_owned(),
        ));
    }
    Ok((value * 255.0).round() as u8)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlyEncoding {
    Ascii,
    BinaryLittle,
    BinaryBig,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PlyScalar {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}

impl PlyScalar {
    fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "char" | "int8" => Some(Self::I8),
            "uchar" | "uint8" => Some(Self::U8),
            "short" | "int16" => Some(Self::I16),
            "ushort" | "uint16" => Some(Self::U16),
            "int" | "int32" => Some(Self::I32),
            "uint" | "uint32" => Some(Self::U32),
            "float" | "float32" => Some(Self::F32),
            "double" | "float64" => Some(Self::F64),
            _ => None,
        }
    }

    const fn width(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }

    const fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }
}

#[derive(Clone, Debug)]
enum PlyProperty {
    Scalar {
        ty: PlyScalar,
        name: String,
    },
    List {
        count_ty: PlyScalar,
        item_ty: PlyScalar,
        name: String,
    },
}

#[derive(Clone, Debug)]
struct PlyElement {
    name: String,
    count: usize,
    properties: Vec<PlyProperty>,
}

#[derive(Clone, Debug)]
struct PlyHeader {
    encoding: PlyEncoding,
    elements: Vec<PlyElement>,
    data_start: usize,
}

fn decode_ply(bytes: &[u8]) -> Result<MeshAsset, CoreError> {
    let header = parse_ply_header(bytes)?;
    match header.encoding {
        PlyEncoding::Ascii => decode_ascii_ply(bytes, &header),
        PlyEncoding::BinaryLittle | PlyEncoding::BinaryBig => decode_binary_ply(bytes, &header),
    }
}

fn parse_ply_header(bytes: &[u8]) -> Result<PlyHeader, CoreError> {
    let search_limit = bytes.len().min(MAX_PLY_HEADER_BYTES);
    let marker = b"end_header";
    let mut marker_start = None;
    for index in 0..search_limit.saturating_sub(marker.len()).saturating_add(1) {
        if bytes.get(index..index + marker.len()) == Some(marker)
            && (index == 0 || bytes[index - 1] == b'\n')
        {
            marker_start = Some(index);
            break;
        }
    }
    let marker_start = marker_start
        .ok_or_else(|| CoreError::MeshFile("PLY header has no end_header line".to_owned()))?;
    let marker_end = marker_start + marker.len();
    let data_start = if bytes.get(marker_end..marker_end + 2) == Some(b"\r\n") {
        marker_end + 2
    } else if bytes.get(marker_end..marker_end + 1) == Some(b"\n") {
        marker_end + 1
    } else {
        return Err(CoreError::MeshFile(
            "PLY end_header line is not newline terminated".to_owned(),
        ));
    };
    if data_start > MAX_PLY_HEADER_BYTES {
        return Err(CoreError::MeshFile("PLY header is too large".to_owned()));
    }
    let text = std::str::from_utf8(&bytes[..data_start])
        .map_err(|_| CoreError::MeshFile("PLY header is not valid UTF-8".to_owned()))?;
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some("ply") {
        return Err(CoreError::MeshFile(
            "PLY header does not start with ply".to_owned(),
        ));
    }
    let mut encoding = None;
    let mut elements = Vec::new();
    let mut ended = false;
    for line in lines {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.is_empty() || fields[0] == "comment" || fields[0] == "obj_info" {
            continue;
        }
        match fields[0] {
            "format" if fields.len() >= 2 => {
                encoding = match fields[1] {
                    "ascii" => Some(PlyEncoding::Ascii),
                    "binary_little_endian" => Some(PlyEncoding::BinaryLittle),
                    "binary_big_endian" => Some(PlyEncoding::BinaryBig),
                    other => {
                        return Err(CoreError::MeshFile(format!(
                            "unsupported PLY encoding '{other}'"
                        )));
                    }
                };
            }
            "element" if fields.len() == 3 => {
                let count = fields[2].parse::<usize>().map_err(|_| {
                    CoreError::MeshFile(format!("invalid PLY element count '{}'", fields[2]))
                })?;
                if count > MAX_MESH_VERTICES {
                    return Err(CoreError::MeshFile(format!(
                        "PLY element '{}' exceeds the supported count",
                        fields[1]
                    )));
                }
                elements.push(PlyElement {
                    name: fields[1].to_ascii_lowercase(),
                    count,
                    properties: Vec::new(),
                });
            }
            "property" => {
                let element = elements.last_mut().ok_or_else(|| {
                    CoreError::MeshFile("PLY property appears before an element".to_owned())
                })?;
                let property = match fields.as_slice() {
                    ["property", ty, name] => PlyProperty::Scalar {
                        ty: PlyScalar::parse(ty).ok_or_else(|| {
                            CoreError::MeshFile(format!("unsupported PLY scalar type '{ty}'"))
                        })?,
                        name: name.to_ascii_lowercase(),
                    },
                    ["property", "list", count_ty, item_ty, name] => PlyProperty::List {
                        count_ty: PlyScalar::parse(count_ty).ok_or_else(|| {
                            CoreError::MeshFile(format!(
                                "unsupported PLY list count type '{count_ty}'"
                            ))
                        })?,
                        item_ty: PlyScalar::parse(item_ty).ok_or_else(|| {
                            CoreError::MeshFile(format!(
                                "unsupported PLY list item type '{item_ty}'"
                            ))
                        })?,
                        name: name.to_ascii_lowercase(),
                    },
                    _ => {
                        return Err(CoreError::MeshFile(
                            "PLY property declaration is malformed".to_owned(),
                        ));
                    }
                };
                element.properties.push(property);
            }
            "end_header" => {
                ended = true;
                break;
            }
            other => {
                return Err(CoreError::MeshFile(format!(
                    "unsupported PLY header directive '{other}'"
                )));
            }
        }
    }
    if !ended {
        return Err(CoreError::MeshFile("PLY header is incomplete".to_owned()));
    }
    let encoding =
        encoding.ok_or_else(|| CoreError::MeshFile("PLY format is missing".to_owned()))?;
    if !elements.iter().any(|element| element.name == "vertex") {
        return Err(CoreError::MeshFile("PLY has no vertex element".to_owned()));
    }
    if !elements.iter().any(|element| element.name == "face") {
        return Err(CoreError::MeshFile("PLY has no face element".to_owned()));
    }
    Ok(PlyHeader {
        encoding,
        elements,
        data_start,
    })
}

fn decode_ascii_ply(bytes: &[u8], header: &PlyHeader) -> Result<MeshAsset, CoreError> {
    let text = std::str::from_utf8(&bytes[header.data_start..])
        .map_err(|_| CoreError::MeshFile("ASCII PLY data is not valid UTF-8".to_owned()))?;
    let mut tokens = text.split_whitespace();
    let vertex_element = header
        .elements
        .iter()
        .find(|element| element.name == "vertex")
        .ok_or_else(|| CoreError::MeshFile("PLY has no vertex element".to_owned()))?;
    let has_colors = ply_has_rgb(vertex_element);
    let mut geometry = PlyGeometry {
        vertices: Vec::with_capacity(vertex_element.count),
        faces: Vec::new(),
        colors: has_colors.then(|| Vec::with_capacity(vertex_element.count)),
    };

    for element in &header.elements {
        for _ in 0..element.count {
            let mut point = [None; 3];
            let mut color = [255_u8; 4];
            let mut seen_color = [false; 4];
            let mut indices = None;
            for property in &element.properties {
                match property {
                    PlyProperty::Scalar { ty, name } => {
                        let token = tokens.next().ok_or_else(|| {
                            CoreError::MeshFile("ASCII PLY data ends in a scalar".to_owned())
                        })?;
                        let value = parse_ascii_scalar(token, *ty)?;
                        if element.name == "vertex" {
                            assign_ply_vertex_value(
                                name,
                                value,
                                *ty,
                                &mut point,
                                &mut color,
                                &mut seen_color,
                            )?;
                        }
                    }
                    PlyProperty::List {
                        count_ty,
                        item_ty,
                        name,
                    } => {
                        let count_token = tokens.next().ok_or_else(|| {
                            CoreError::MeshFile("ASCII PLY data ends in a list count".to_owned())
                        })?;
                        let count = ply_count(parse_ascii_scalar(count_token, *count_ty)?, "list")?;
                        if count > MAX_MESH_VERTICES {
                            return Err(CoreError::MeshFile(
                                "PLY list contains too many entries".to_owned(),
                            ));
                        }
                        let capture =
                            element.name == "face" && indices.is_none() && ply_is_index_list(name);
                        let mut list = capture.then(|| Vec::with_capacity(count));
                        for _ in 0..count {
                            let token = tokens.next().ok_or_else(|| {
                                CoreError::MeshFile("ASCII PLY data ends in a list".to_owned())
                            })?;
                            let value = parse_ascii_scalar(token, *item_ty)?;
                            if let Some(list) = list.as_mut() {
                                list.push(ply_index(value)?);
                            }
                        }
                        if let Some(list) = list {
                            indices = Some(list);
                        }
                    }
                }
            }
            finish_ply_row(element, point, color, seen_color, indices, &mut geometry)?;
        }
    }
    geometry.into_asset()
}

fn decode_binary_ply(bytes: &[u8], header: &PlyHeader) -> Result<MeshAsset, CoreError> {
    let vertex_element = header
        .elements
        .iter()
        .find(|element| element.name == "vertex")
        .ok_or_else(|| CoreError::MeshFile("PLY has no vertex element".to_owned()))?;
    let has_colors = ply_has_rgb(vertex_element);
    let mut geometry = PlyGeometry {
        vertices: Vec::with_capacity(vertex_element.count),
        faces: Vec::new(),
        colors: has_colors.then(|| Vec::with_capacity(vertex_element.count)),
    };
    let mut cursor = header.data_start;
    let little_endian = header.encoding == PlyEncoding::BinaryLittle;
    for element in &header.elements {
        for _ in 0..element.count {
            let mut point = [None; 3];
            let mut color = [255_u8; 4];
            let mut seen_color = [false; 4];
            let mut indices = None;
            for property in &element.properties {
                match property {
                    PlyProperty::Scalar { ty, name } => {
                        let value = read_binary_scalar(bytes, &mut cursor, *ty, little_endian)?;
                        if element.name == "vertex" {
                            assign_ply_vertex_value(
                                name,
                                value,
                                *ty,
                                &mut point,
                                &mut color,
                                &mut seen_color,
                            )?;
                        }
                    }
                    PlyProperty::List {
                        count_ty,
                        item_ty,
                        name,
                    } => {
                        let count = ply_count(
                            read_binary_scalar(bytes, &mut cursor, *count_ty, little_endian)?,
                            "list",
                        )?;
                        if count > MAX_MESH_VERTICES {
                            return Err(CoreError::MeshFile(
                                "PLY list contains too many entries".to_owned(),
                            ));
                        }
                        let capture =
                            element.name == "face" && indices.is_none() && ply_is_index_list(name);
                        let mut list = capture.then(|| Vec::with_capacity(count));
                        for _ in 0..count {
                            let value =
                                read_binary_scalar(bytes, &mut cursor, *item_ty, little_endian)?;
                            if let Some(list) = list.as_mut() {
                                list.push(ply_index(value)?);
                            }
                        }
                        if let Some(list) = list {
                            indices = Some(list);
                        }
                    }
                }
            }
            finish_ply_row(element, point, color, seen_color, indices, &mut geometry)?;
        }
    }
    geometry.into_asset()
}

#[derive(Default)]
struct PlyGeometry {
    vertices: Vec<[f32; 3]>,
    faces: Vec<[u32; 3]>,
    colors: Option<Vec<[u8; 4]>>,
}

impl PlyGeometry {
    fn into_asset(self) -> Result<MeshAsset, CoreError> {
        let mesh = Mesh::new(self.vertices, self.faces)?;
        MeshAsset::new(mesh, self.colors)
    }
}

fn finish_ply_row(
    element: &PlyElement,
    point: [Option<f32>; 3],
    color: [u8; 4],
    seen_color: [bool; 4],
    indices: Option<Vec<u32>>,
    geometry: &mut PlyGeometry,
) -> Result<(), CoreError> {
    if element.name == "vertex" {
        let point = [
            point[0]
                .ok_or_else(|| CoreError::MeshFile("PLY vertex has no x property".to_owned()))?,
            point[1]
                .ok_or_else(|| CoreError::MeshFile("PLY vertex has no y property".to_owned()))?,
            point[2]
                .ok_or_else(|| CoreError::MeshFile("PLY vertex has no z property".to_owned()))?,
        ];
        geometry.vertices.push(point);
        if let Some(colors) = geometry.colors.as_mut() {
            if !seen_color[..3].iter().all(|seen| *seen) {
                return Err(CoreError::MeshFile(
                    "PLY vertex colors must define red, green, and blue".to_owned(),
                ));
            }
            colors.push(color);
        }
    } else if element.name == "face" {
        let indices = indices
            .ok_or_else(|| CoreError::MeshFile("PLY face has no vertex index list".to_owned()))?;
        if indices.len() < 3 {
            return Err(CoreError::MeshFile(
                "PLY face has fewer than three vertices".to_owned(),
            ));
        }
        let triangle_count = indices.len() - 2;
        if geometry.faces.len().saturating_add(triangle_count) > MAX_MESH_FACES {
            return Err(CoreError::MeshFile(format!(
                "PLY contains more than {MAX_MESH_FACES} faces"
            )));
        }
        for pair in indices[1..].windows(2) {
            geometry.faces.push([indices[0], pair[0], pair[1]]);
        }
    }
    Ok(())
}

fn ply_has_rgb(element: &PlyElement) -> bool {
    let mut channels = [false; 3];
    for property in &element.properties {
        if let PlyProperty::Scalar { name, .. } = property
            && let Some(channel) = ply_color_channel(name)
            && channel < 3
        {
            channels[channel] = true;
        }
    }
    channels.iter().all(|present| *present)
}

fn ply_color_channel(name: &str) -> Option<usize> {
    match name {
        "red" | "r" => Some(0),
        "green" | "g" => Some(1),
        "blue" | "b" => Some(2),
        "alpha" | "a" => Some(3),
        _ => None,
    }
}

fn assign_ply_vertex_value(
    name: &str,
    value: f64,
    ty: PlyScalar,
    point: &mut [Option<f32>; 3],
    color: &mut [u8; 4],
    seen_color: &mut [bool; 4],
) -> Result<(), CoreError> {
    match name {
        "x" | "y" | "z" => {
            let axis = match name {
                "x" => 0,
                "y" => 1,
                _ => 2,
            };
            let value = value as f32;
            if !value.is_finite() {
                return Err(CoreError::MeshFile(
                    "PLY vertex coordinates must be finite".to_owned(),
                ));
            }
            point[axis] = Some(value);
        }
        _ => {
            if let Some(channel) = ply_color_channel(name) {
                color[channel] = ply_color(value, ty)?;
                seen_color[channel] = true;
            }
        }
    }
    Ok(())
}

fn ply_color(value: f64, ty: PlyScalar) -> Result<u8, CoreError> {
    if !value.is_finite() {
        return Err(CoreError::MeshFile("PLY colors must be finite".to_owned()));
    }
    let normalized = if ty.is_float() && (0.0..=1.0).contains(&value) {
        value * 255.0
    } else {
        value
    };
    if !(0.0..=255.0).contains(&normalized) {
        return Err(CoreError::MeshFile(
            "PLY colors must be between 0 and 255".to_owned(),
        ));
    }
    Ok(normalized.round() as u8)
}

fn ply_is_index_list(name: &str) -> bool {
    matches!(name, "vertex_indices" | "vertex_index" | "vertex_ids")
}

fn ply_count(value: f64, label: &str) -> Result<usize, CoreError> {
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 {
        return Err(CoreError::MeshFile(format!(
            "PLY {label} is not a non-negative integer"
        )));
    }
    let value = value as u128;
    usize::try_from(value).map_err(|_| CoreError::MeshFile(format!("PLY {label} is too large")))
}

fn ply_index(value: f64) -> Result<u32, CoreError> {
    let value = ply_count(value, "face index")?;
    u32::try_from(value).map_err(|_| CoreError::MeshFile("PLY face index is too large".to_owned()))
}

fn parse_ascii_scalar(token: &str, ty: PlyScalar) -> Result<f64, CoreError> {
    let value = match ty {
        PlyScalar::I8 => token.parse::<i8>().map(f64::from).map_err(|_| ()),
        PlyScalar::U8 => token.parse::<u8>().map(f64::from).map_err(|_| ()),
        PlyScalar::I16 => token.parse::<i16>().map(f64::from).map_err(|_| ()),
        PlyScalar::U16 => token.parse::<u16>().map(f64::from).map_err(|_| ()),
        PlyScalar::I32 => token.parse::<i32>().map(f64::from).map_err(|_| ()),
        PlyScalar::U32 => token.parse::<u32>().map(f64::from).map_err(|_| ()),
        PlyScalar::F32 => token.parse::<f32>().map(f64::from).map_err(|_| ()),
        PlyScalar::F64 => token.parse::<f64>().map_err(|_| ()),
    }
    .map_err(|_| CoreError::MeshFile(format!("invalid PLY value '{token}'")))?;
    if !value.is_finite() {
        return Err(CoreError::MeshFile("PLY values must be finite".to_owned()));
    }
    Ok(value)
}

fn read_binary_scalar(
    bytes: &[u8],
    cursor: &mut usize,
    ty: PlyScalar,
    little_endian: bool,
) -> Result<f64, CoreError> {
    let end = cursor
        .checked_add(ty.width())
        .ok_or_else(|| CoreError::MeshFile("PLY cursor overflow".to_owned()))?;
    let raw = bytes
        .get(*cursor..end)
        .ok_or_else(|| CoreError::MeshFile("PLY binary data is truncated".to_owned()))?;
    *cursor = end;
    let value = match ty {
        PlyScalar::I8 => i8::from_ne_bytes([raw[0]]) as f64,
        PlyScalar::U8 => f64::from(raw[0]),
        PlyScalar::I16 => {
            let bytes = raw
                .try_into()
                .map_err(|_| CoreError::MeshFile("PLY scalar is truncated".to_owned()))?;
            if little_endian {
                i16::from_le_bytes(bytes) as f64
            } else {
                i16::from_be_bytes(bytes) as f64
            }
        }
        PlyScalar::U16 => {
            let bytes = raw
                .try_into()
                .map_err(|_| CoreError::MeshFile("PLY scalar is truncated".to_owned()))?;
            if little_endian {
                u16::from_le_bytes(bytes) as f64
            } else {
                u16::from_be_bytes(bytes) as f64
            }
        }
        PlyScalar::I32 => {
            let bytes = raw
                .try_into()
                .map_err(|_| CoreError::MeshFile("PLY scalar is truncated".to_owned()))?;
            if little_endian {
                i32::from_le_bytes(bytes) as f64
            } else {
                i32::from_be_bytes(bytes) as f64
            }
        }
        PlyScalar::U32 => {
            let bytes = raw
                .try_into()
                .map_err(|_| CoreError::MeshFile("PLY scalar is truncated".to_owned()))?;
            if little_endian {
                u32::from_le_bytes(bytes) as f64
            } else {
                u32::from_be_bytes(bytes) as f64
            }
        }
        PlyScalar::F32 => {
            let bytes = raw
                .try_into()
                .map_err(|_| CoreError::MeshFile("PLY scalar is truncated".to_owned()))?;
            if little_endian {
                f32::from_le_bytes(bytes) as f64
            } else {
                f32::from_be_bytes(bytes) as f64
            }
        }
        PlyScalar::F64 => {
            let bytes = raw
                .try_into()
                .map_err(|_| CoreError::MeshFile("PLY scalar is truncated".to_owned()))?;
            if little_endian {
                f64::from_le_bytes(bytes)
            } else {
                f64::from_be_bytes(bytes)
            }
        }
    };
    if !value.is_finite() {
        return Err(CoreError::MeshFile("PLY values must be finite".to_owned()));
    }
    Ok(value)
}

fn encode_obj(asset: &MeshAsset) -> Result<Vec<u8>, CoreError> {
    let colors = asset.vertex_colors.as_deref();
    let mut output = String::from("# https://github.com/mikedh/trimesh\n");
    for (index, vertex) in asset.mesh.vertices.iter().enumerate() {
        write!(
            output,
            "v {:.8} {:.8} {:.8}",
            vertex[0], vertex[1], vertex[2]
        )
        .map_err(CoreError::mesh_file)?;
        if let Some(colors) = colors {
            let color = colors[index];
            write!(
                output,
                " {:.8} {:.8} {:.8}",
                f32::from(color[0]) / 255.0,
                f32::from(color[1]) / 255.0,
                f32::from(color[2]) / 255.0
            )
            .map_err(CoreError::mesh_file)?;
        }
        output.push('\n');
    }
    for face in &asset.mesh.faces {
        let first = face[0]
            .checked_add(1)
            .ok_or_else(|| CoreError::MeshFile("OBJ face index overflow".to_owned()))?;
        let second = face[1]
            .checked_add(1)
            .ok_or_else(|| CoreError::MeshFile("OBJ face index overflow".to_owned()))?;
        let third = face[2]
            .checked_add(1)
            .ok_or_else(|| CoreError::MeshFile("OBJ face index overflow".to_owned()))?;
        writeln!(output, "f {first} {second} {third}").map_err(CoreError::mesh_file)?;
    }
    output.push('\n');
    Ok(output.into_bytes())
}

fn encode_binary_ply(asset: &MeshAsset) -> Result<Vec<u8>, CoreError> {
    let colors = asset.vertex_colors.as_deref();
    let color_header = if colors.is_some() {
        "property uchar red\nproperty uchar green\nproperty uchar blue\nproperty uchar alpha\n"
    } else {
        ""
    };
    let header = format!(
        "ply\nformat binary_little_endian 1.0\ncomment https://github.com/mikedh/trimesh\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\n{}element face {}\nproperty list uchar int vertex_indices\nend_header\n",
        asset.mesh.vertex_count(),
        color_header,
        asset.mesh.face_count()
    );
    let vertex_stride = 12_usize
        .checked_add(colors.map_or(0, |_| 4))
        .ok_or_else(|| CoreError::MeshFile("PLY size overflow".to_owned()))?;
    let size = header
        .len()
        .checked_add(
            asset
                .mesh
                .vertex_count()
                .checked_mul(vertex_stride)
                .ok_or_else(|| CoreError::MeshFile("PLY size overflow".to_owned()))?,
        )
        .and_then(|size| {
            asset
                .mesh
                .face_count()
                .checked_mul(13)
                .and_then(|faces| size.checked_add(faces))
        })
        .ok_or_else(|| CoreError::MeshFile("PLY size overflow".to_owned()))?;
    let mut output = Vec::with_capacity(size);
    output.extend_from_slice(header.as_bytes());
    for (index, vertex) in asset.mesh.vertices.iter().enumerate() {
        for value in vertex {
            output.extend_from_slice(&value.to_le_bytes());
        }
        if let Some(colors) = colors {
            output.extend_from_slice(&colors[index]);
        }
    }
    for face in &asset.mesh.faces {
        output.push(3);
        for index in face {
            let index = i32::try_from(*index).map_err(|_| {
                CoreError::MeshFile("PLY face index exceeds signed 32-bit range".to_owned())
            })?;
            output.extend_from_slice(&index.to_le_bytes());
        }
    }
    Ok(output)
}

fn parse_f32(value: Option<&str>, label: &str) -> Result<f32, CoreError> {
    let value = value.ok_or_else(|| CoreError::MeshFile(format!("{label} is missing")))?;
    let value = value
        .parse::<f32>()
        .map_err(|_| CoreError::MeshFile(format!("{label} is invalid")))?;
    if !value.is_finite() {
        return Err(CoreError::MeshFile(format!("{label} must be finite")));
    }
    Ok(value)
}
