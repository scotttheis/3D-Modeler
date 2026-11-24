use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::{Cursor, Read, Write as IoWrite};
use std::path::Path;

use quick_xml::Writer;
use quick_xml::XmlVersion;
use quick_xml::escape::unescape;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::reader::Reader;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::{CoreError, DEFAULT_MESH_COLOR, Mesh, Transform};

const CORE_NS: &str = "http://schemas.microsoft.com/3dmanufacturing/core/2015/02";
const MATERIAL_NS: &str = "http://schemas.microsoft.com/3dmanufacturing/material/2015/02";
const RELATIONSHIPS_NS: &str = "http://schemas.openxmlformats.org/package/2006/relationships";
const CONTENT_TYPES_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";
const MODEL_RELATIONSHIP_TYPE: &str =
    "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel";
const THUMBNAIL_RELATIONSHIP_TYPE: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships/metadata/thumbnail";
const TEXTURE_RELATIONSHIP_TYPE: &str =
    "http://schemas.microsoft.com/3dmanufacturing/2013/01/3dtexture";

/// Maximum bytes accepted for a complete 3MF ZIP archive.
pub const MAX_3MF_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
/// Maximum ZIP members accepted in a 3MF package.
pub const MAX_3MF_ARCHIVE_ENTRIES: usize = 10_000;
/// Maximum expanded bytes across all 3MF ZIP members.
pub const MAX_3MF_UNCOMPRESSED_BYTES: u64 = 1024 * 1024 * 1024;
/// Maximum expanded bytes in one 3MF ZIP member.
pub const MAX_3MF_MEMBER_BYTES: u64 = 256 * 1024 * 1024;
/// The default model XML limit is intentionally open; the member limit remains
/// the package boundary, matching the Python reference behavior.
pub const MAX_3MF_MODEL_XML_BYTES: Option<u64> = None;
/// Maximum bytes read from one relationships XML part.
pub const MAX_3MF_RELATIONSHIPS_XML_BYTES: u64 = 8 * 1024 * 1024;
/// Maximum bytes retained for a package thumbnail.
pub const MAX_3MF_THUMBNAIL_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum UTF-8 bytes used by metadata names and values in one model part.
pub const MAX_3MF_METADATA_BYTES: u64 = 8 * 1024 * 1024;
/// Maximum relationships in one package, including external model parts.
pub const MAX_3MF_RELATIONSHIPS: usize = 10_000;
/// Maximum material colors in one model part.
pub const MAX_3MF_MATERIAL_COLORS: usize = 100_000;
/// Minimum member size at which compression-ratio checks apply.
pub const MAX_3MF_COMPRESSION_CHECK_BYTES: u64 = 1024 * 1024;
/// Maximum allowed expanded-to-compressed ratio for a checked member.
pub const MAX_3MF_COMPRESSION_RATIO: f64 = 1000.0;
/// Maximum model object resources in one package.
pub const MAX_3MF_OBJECTS: usize = 10_000;
/// Maximum build items in one model part.
pub const MAX_3MF_BUILD_ITEMS: usize = 10_000;
/// Maximum component nesting depth before expansion is rejected.
pub const MAX_3MF_COMPONENT_DEPTH: usize = 32;
/// Maximum final mesh instances produced by one package.
pub const MAX_3MF_INSTANCES: usize = 10_000;
/// No default per-mesh vertex ceiling is imposed, matching the Python seam.
pub const MAX_3MF_MESH_VERTICES: Option<usize> = None;
/// No default per-mesh face ceiling is imposed, matching the Python seam.
pub const MAX_3MF_MESH_FACES: Option<usize> = None;
/// No default aggregate vertex ceiling is imposed, matching the Python seam.
pub const MAX_3MF_TOTAL_MESH_VERTICES: Option<usize> = None;
/// No default aggregate face ceiling is imposed, matching the Python seam.
pub const MAX_3MF_TOTAL_MESH_FACES: Option<usize> = None;

/// Resource limits applied while reading or writing a 3MF package.
///
/// The defaults are the same bounded package policy used by the Python
/// reference. Applications can provide smaller limits for previews, tests, or
/// a stricter deployment profile without changing the parser's behavior.
#[derive(Clone, Debug, PartialEq)]
pub struct ThreeMfLimits {
    /// Maximum compressed package bytes.
    pub archive_bytes: u64,
    /// Maximum number of ZIP members.
    pub archive_entries: usize,
    /// Maximum expanded bytes across all members.
    pub uncompressed_bytes: u64,
    /// Maximum expanded bytes in one member.
    pub member_bytes: u64,
    /// Optional separate model XML ceiling.
    pub model_xml_bytes: Option<u64>,
    /// Maximum relationships-part bytes.
    pub relationships_xml_bytes: u64,
    /// Maximum thumbnail bytes.
    pub thumbnail_bytes: u64,
    /// Maximum metadata name/value bytes per model part.
    pub metadata_bytes: u64,
    /// Maximum relationships across the package.
    pub relationships: usize,
    /// Maximum material colors per model part.
    pub material_colors: usize,
    /// Minimum member size checked for compression bombs.
    pub compression_check_bytes: u64,
    /// Maximum expanded-to-compressed ratio.
    pub compression_ratio: f64,
    /// Maximum object resources per model part.
    pub objects: usize,
    /// Maximum build items per model part.
    pub build_items: usize,
    /// Maximum nested component depth.
    pub component_depth: usize,
    /// Maximum final scene instances.
    pub instances: usize,
    /// Optional per-object vertex ceiling.
    pub mesh_vertices: Option<usize>,
    /// Optional per-object face ceiling.
    pub mesh_faces: Option<usize>,
    /// Optional aggregate vertex ceiling.
    pub total_mesh_vertices: Option<usize>,
    /// Optional aggregate face ceiling.
    pub total_mesh_faces: Option<usize>,
}

impl Default for ThreeMfLimits {
    fn default() -> Self {
        Self {
            archive_bytes: MAX_3MF_ARCHIVE_BYTES,
            archive_entries: MAX_3MF_ARCHIVE_ENTRIES,
            uncompressed_bytes: MAX_3MF_UNCOMPRESSED_BYTES,
            member_bytes: MAX_3MF_MEMBER_BYTES,
            model_xml_bytes: MAX_3MF_MODEL_XML_BYTES,
            relationships_xml_bytes: MAX_3MF_RELATIONSHIPS_XML_BYTES,
            thumbnail_bytes: MAX_3MF_THUMBNAIL_BYTES,
            metadata_bytes: MAX_3MF_METADATA_BYTES,
            relationships: MAX_3MF_RELATIONSHIPS,
            material_colors: MAX_3MF_MATERIAL_COLORS,
            compression_check_bytes: MAX_3MF_COMPRESSION_CHECK_BYTES,
            compression_ratio: MAX_3MF_COMPRESSION_RATIO,
            objects: MAX_3MF_OBJECTS,
            build_items: MAX_3MF_BUILD_ITEMS,
            component_depth: MAX_3MF_COMPONENT_DEPTH,
            instances: MAX_3MF_INSTANCES,
            mesh_vertices: MAX_3MF_MESH_VERTICES,
            mesh_faces: MAX_3MF_MESH_FACES,
            total_mesh_vertices: MAX_3MF_TOTAL_MESH_VERTICES,
            total_mesh_faces: MAX_3MF_TOTAL_MESH_FACES,
        }
    }
}

impl ThreeMfLimits {
    fn validate(&self) -> Result<(), CoreError> {
        if !self.compression_ratio.is_finite() || self.compression_ratio < 0.0 {
            return Err(CoreError::three_mf(
                "compression ratio limit must be finite and non-negative",
            ));
        }
        Ok(())
    }
}

/// Canonical 3MF model units and their millimetre conversion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThreeMfUnit {
    /// One thousandth of a millimetre.
    Micron,
    /// One millimetre.
    Millimeter,
    /// Ten millimetres.
    Centimeter,
    /// Twenty-five point four millimetres.
    Inch,
    /// Three hundred and four point eight millimetres.
    Foot,
    /// One thousand millimetres.
    Meter,
}

impl ThreeMfUnit {
    fn parse(raw: &str) -> Result<Self, CoreError> {
        let value = raw.trim().to_ascii_lowercase();
        let unit = match value.as_str() {
            "micron" | "microns" | "um" => Self::Micron,
            "millimeter" | "millimeters" | "mm" => Self::Millimeter,
            "centimeter" | "centimeters" | "cm" => Self::Centimeter,
            "inch" | "inches" | "in" => Self::Inch,
            "foot" | "feet" | "ft" => Self::Foot,
            "meter" | "meters" | "m" => Self::Meter,
            _ => {
                return Err(CoreError::three_mf(format!(
                    "unsupported model unit '{value}'; expected micron, millimeter, centimeter, inch, foot, or meter"
                )));
            }
        };
        Ok(unit)
    }

    /// Returns the canonical spelling used by the model contract.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Micron => "micron",
            Self::Millimeter => "millimeter",
            Self::Centimeter => "centimeter",
            Self::Inch => "inch",
            Self::Foot => "foot",
            Self::Meter => "meter",
        }
    }

    /// Returns this model unit's scale to millimetres.
    #[must_use]
    pub const fn scale_mm(self) -> f64 {
        match self {
            Self::Micron => 0.001,
            Self::Millimeter => 1.0,
            Self::Centimeter => 10.0,
            Self::Inch => 25.4,
            Self::Foot => 304.8,
            Self::Meter => 1000.0,
        }
    }
}

/// Model metadata, preserving repeated metadata elements in document order.
pub type ThreeMfMetadata = BTreeMap<String, Vec<String>>;

/// One relationship discovered in the package or one of its model parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThreeMfRelationship {
    /// The source part; `None` is the package root relationships part.
    pub source: Option<String>,
    /// The OPC relationship type URI.
    pub relationship_type: String,
    /// The encoded target from the relationship XML.
    pub target: String,
    /// The package-normalized target, or `None` for an external target.
    pub resolved_target: Option<String>,
    /// The optional relationship ID.
    pub id: Option<String>,
    /// Whether the target was marked external.
    pub external: bool,
}

/// One imported or exportable 3MF scene instance.
#[derive(Clone, Debug, PartialEq)]
pub struct ThreeMfItem {
    /// The local indexed mesh, already converted to millimetres on import.
    pub mesh: Mesh,
    /// The scene transform, with translation in millimetres.
    pub transform: Transform,
    /// The build or object display name.
    pub name: String,
    /// The representative RGBA display color in the 0..=1 range.
    pub color: [f32; 4],
    /// The model object ID that produced this instance.
    pub object_id: String,
    /// Root model metadata associated with this item.
    pub metadata: ThreeMfMetadata,
    /// Resolved per-face material colors when the model supplied them.
    pub face_colors: Option<Vec<[u8; 4]>>,
    /// Material texture (OF-46) exported as a 3MF texture2d with box-mapped
    /// coordinates on every face still in the item's own colour. Not read on
    /// import.
    pub texture: Option<crate::ObjectTexture>,
}

impl ThreeMfItem {
    /// Creates a validated scene item for 3MF export.
    pub fn new(
        mesh: Mesh,
        transform: Transform,
        name: String,
        color: [f32; 4],
    ) -> Result<Self, CoreError> {
        mesh.validate_non_empty()?;
        validate_export_color(color)?;
        Ok(Self {
            mesh,
            transform,
            name,
            color,
            object_id: String::new(),
            metadata: ThreeMfMetadata::new(),
            face_colors: None,
            texture: None,
        })
    }
}

/// The distinct texture images the items refer to, in first-use order,
/// with the package path each is written to.
fn export_textures(items: &[ThreeMfItem]) -> Vec<(std::sync::Arc<crate::TextureImage>, String)> {
    let mut textures: Vec<(std::sync::Arc<crate::TextureImage>, String)> = Vec::new();
    for item in items {
        let Some(reference) = &item.texture else {
            continue;
        };
        let Some(image) = crate::texture_by_id(&reference.id) else {
            continue;
        };
        if textures.iter().any(|(known, _)| known.id() == image.id()) {
            continue;
        }
        let path = format!(
            "/3D/Textures/texture_{}.{}",
            &image.id()[..16],
            image.encoding().extension()
        );
        textures.push((image, path));
    }
    textures
}

/// Box-mapped texture coordinates for each face of a local mesh, three per
/// face, using the same projection as the viewport (one axis per face).
fn face_texture_coordinates(mesh: &Mesh, tile_mm: f32) -> Vec<[[f32; 2]; 3]> {
    mesh.faces
        .iter()
        .map(|face| {
            let corner = |index: u32| {
                mesh.vertices
                    .get(index as usize)
                    .copied()
                    .unwrap_or([0.0; 3])
            };
            let [a, b, c] = [corner(face[0]), corner(face[1]), corner(face[2])];
            let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let normal = [
                ab[1] * ac[2] - ab[2] * ac[1],
                ab[2] * ac[0] - ab[0] * ac[2],
                ab[0] * ac[1] - ab[1] * ac[0],
            ];
            let axis = crate::box_axis(normal);
            [a, b, c].map(|point| crate::box_uv(point, normal, axis, tile_mm))
        })
        .collect()
}

/// Parsed package data and its millimetre scene instances.
#[derive(Clone, Debug, PartialEq)]
pub struct ThreeMfImport {
    /// Imported mesh instances in build order.
    pub items: Vec<ThreeMfItem>,
    /// The canonical unit declared by the root model part.
    pub unit: ThreeMfUnit,
    /// The root model's conversion to millimetres.
    pub unit_scale_mm: f64,
    /// Root model metadata.
    pub metadata: ThreeMfMetadata,
    /// The selected thumbnail package part, if present.
    pub thumbnail_path: Option<String>,
    /// Thumbnail bytes, if present and within the package limit.
    pub thumbnail_data: Option<Vec<u8>>,
    /// The selected root model package part.
    pub model_path: String,
    /// All normalized package part names.
    pub package_parts: Vec<String>,
    /// Root, model, and external-part relationships in read order.
    pub relationships: Vec<ThreeMfRelationship>,
}

#[derive(Clone, Copy, Debug)]
struct Matrix4 {
    rows: [[f64; 4]; 4],
}

impl Matrix4 {
    const fn identity() -> Self {
        Self {
            rows: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    fn multiply(self, other: Self) -> Result<Self, CoreError> {
        let mut rows = [[0.0_f64; 4]; 4];
        for (row_index, row) in rows.iter_mut().enumerate() {
            for (column_index, value) in row.iter_mut().enumerate() {
                *value = (0..4)
                    .map(|index| self.rows[row_index][index] * other.rows[index][column_index])
                    .sum();
                if !value.is_finite() {
                    return Err(CoreError::three_mf(
                        "3MF transform multiplication produced a non-finite value",
                    ));
                }
            }
        }
        Ok(Self { rows })
    }

    fn scale_translation(mut self, factor: f64) -> Result<Self, CoreError> {
        if !factor.is_finite() {
            return Err(CoreError::three_mf("3MF transform scale must be finite"));
        }
        for row in &mut self.rows[..3] {
            row[3] *= factor;
            if !row[3].is_finite() {
                return Err(CoreError::three_mf(
                    "3MF transform translation became non-finite",
                ));
            }
        }
        Ok(self)
    }

    fn to_transform(self) -> Result<Transform, CoreError> {
        Transform::from_f64_rows(self.rows)
            .map_err(|error| CoreError::three_mf(format!("invalid 3MF transform: {error}")))
    }
}

#[derive(Clone, Debug)]
struct ComponentDefinition {
    target_id: String,
    target_key: String,
    transform: Matrix4,
    path: Option<String>,
}

#[derive(Clone, Debug)]
struct TriangleDefinition {
    face: [u32; 3],
    material_id: Option<String>,
    property_index: Option<usize>,
}

#[derive(Clone, Debug)]
struct ObjectBuilder {
    object_id: String,
    name: String,
    material_id: Option<String>,
    property_index: Option<usize>,
    mesh_seen: bool,
    vertices_seen: bool,
    triangles_seen: bool,
    vertices: Vec<[f32; 3]>,
    triangles: Vec<TriangleDefinition>,
    components: Vec<ComponentDefinition>,
}

#[derive(Clone, Debug)]
struct BuildBuilder {
    object_id: String,
    transform: Matrix4,
    name: Option<String>,
    material_id: Option<String>,
    property_index: Option<usize>,
    path: Option<String>,
}

#[derive(Clone, Debug)]
struct MaterialBuilder {
    id: String,
    colors: Vec<[u8; 4]>,
}

#[derive(Clone, Debug)]
struct MetadataCapture {
    name: String,
    text: String,
}

#[derive(Clone, Debug)]
struct ObjectDefinition {
    key: String,
    object_id: String,
    name: String,
    mesh: Option<Mesh>,
    color: Option<[f32; 4]>,
    face_colors: Option<Vec<[u8; 4]>>,
    components: Vec<ComponentDefinition>,
}

#[derive(Clone, Debug)]
struct BuildDefinition {
    object_id: String,
    object_key: String,
    transform: Matrix4,
    name: Option<String>,
    color: Option<[f32; 4]>,
    path: Option<String>,
}

#[derive(Clone, Debug)]
struct ParsedModel {
    unit: ThreeMfUnit,
    metadata: ThreeMfMetadata,
    objects: Vec<ObjectDefinition>,
    build: Option<Vec<BuildDefinition>>,
}

#[derive(Default)]
struct MeshBudget {
    vertices: usize,
    faces: usize,
}

#[derive(Clone, Debug)]
struct PackageEntry {
    canonical: String,
    index: usize,
    size: u64,
}

struct Package {
    bytes: Vec<u8>,
    entries: BTreeMap<String, PackageEntry>,
    parts: Vec<String>,
    limits: ThreeMfLimits,
}

impl Package {
    fn open(bytes: Vec<u8>, limits: &ThreeMfLimits) -> Result<Self, CoreError> {
        limits.validate()?;
        let byte_count = u64::try_from(bytes.len())
            .map_err(|_| CoreError::three_mf("package size does not fit in u64"))?;
        if byte_count > limits.archive_bytes {
            return Err(CoreError::three_mf(format!(
                "archive exceeds the maximum size of {} bytes",
                limits.archive_bytes
            )));
        }
        let mut archive = ZipArchive::new(Cursor::new(bytes.as_slice())).map_err(|error| {
            CoreError::three_mf(format!("package is not a readable ZIP: {error}"))
        })?;
        if archive.len() > limits.archive_entries {
            return Err(CoreError::three_mf(format!(
                "archive contains more than {} entries",
                limits.archive_entries
            )));
        }

        let mut entries = BTreeMap::new();
        let mut parts = Vec::with_capacity(archive.len());
        let mut total_uncompressed = 0_u64;
        for index in 0..archive.len() {
            let file = archive.by_index(index).map_err(|error| {
                CoreError::three_mf(format!("could not inspect ZIP entry {index}: {error}"))
            })?;
            let raw_name = file
                .name()
                .map_err(|error| CoreError::three_mf(format!("invalid ZIP entry name: {error}")))?
                .into_owned();
            let canonical = normalize_zip_name(&raw_name)?;
            let key = canonical.to_ascii_lowercase();
            if entries.contains_key(&key) {
                return Err(CoreError::three_mf(format!(
                    "package contains a duplicate path: {canonical}"
                )));
            }
            let size = file.size();
            if size > limits.member_bytes {
                return Err(CoreError::three_mf(format!(
                    "ZIP entry exceeds the maximum size of {} bytes: {}",
                    limits.member_bytes, raw_name
                )));
            }
            total_uncompressed = total_uncompressed
                .checked_add(size)
                .ok_or_else(|| CoreError::three_mf("archive expanded-size accounting overflow"))?;
            if total_uncompressed > limits.uncompressed_bytes {
                return Err(CoreError::three_mf(format!(
                    "archive expands beyond {} bytes",
                    limits.uncompressed_bytes
                )));
            }
            let compressed_size = file.compressed_size();
            let ratio = if size > 0 && compressed_size == 0 {
                f64::INFINITY
            } else {
                size as f64 / compressed_size.max(1) as f64
            };
            if size >= limits.compression_check_bytes && ratio > limits.compression_ratio {
                return Err(CoreError::three_mf(format!(
                    "ZIP entry has a suspicious compression ratio: {}",
                    raw_name
                )));
            }
            entries.insert(
                key,
                PackageEntry {
                    canonical: canonical.clone(),
                    index,
                    size,
                },
            );
            parts.push(canonical);
        }
        parts.sort();
        Ok(Self {
            bytes,
            entries,
            parts,
            limits: limits.clone(),
        })
    }

    fn contains(&self, part: &str) -> Result<bool, CoreError> {
        Ok(self
            .entries
            .contains_key(&normalize_zip_name(part)?.to_ascii_lowercase()))
    }

    fn canonical(&self, part: &str) -> Result<Option<String>, CoreError> {
        Ok(self
            .entries
            .get(&normalize_zip_name(part)?.to_ascii_lowercase())
            .map(|entry| entry.canonical.clone()))
    }

    fn read_part(&self, part: &str) -> Result<Vec<u8>, CoreError> {
        let normalized = normalize_zip_name(part)?;
        let key = normalized.to_ascii_lowercase();
        let entry = self.entries.get(&key).ok_or_else(|| {
            CoreError::three_mf(format!("package is missing required part: {normalized}"))
        })?;
        if key.ends_with(".model")
            && let Some(limit) = self.limits.model_xml_bytes
            && entry.size > limit
        {
            return Err(CoreError::three_mf(format!(
                "model XML exceeds the maximum size of {limit} bytes"
            )));
        }
        if key.ends_with(".rels") && entry.size > self.limits.relationships_xml_bytes {
            return Err(CoreError::three_mf(format!(
                "relationships XML exceeds the maximum size of {} bytes",
                self.limits.relationships_xml_bytes
            )));
        }
        let mut archive = ZipArchive::new(Cursor::new(self.bytes.as_slice())).map_err(|error| {
            CoreError::three_mf(format!("could not reopen ZIP package: {error}"))
        })?;
        let mut file = archive.by_index(entry.index).map_err(|error| {
            CoreError::three_mf(format!("could not read ZIP entry {normalized}: {error}"))
        })?;
        let capacity = usize::try_from(entry.size).map_err(|_| {
            CoreError::three_mf(format!("ZIP entry is too large to load: {normalized}"))
        })?;
        let read_limit = entry
            .size
            .checked_add(1)
            .ok_or_else(|| CoreError::three_mf("ZIP read limit overflow"))?;
        let mut limited = (&mut file).take(read_limit);
        let mut bytes = Vec::with_capacity(capacity);
        limited.read_to_end(&mut bytes).map_err(|error| {
            CoreError::three_mf(format!("could not read ZIP entry {normalized}: {error}"))
        })?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > entry.size {
            return Err(CoreError::three_mf(format!(
                "ZIP entry expanded beyond its declared size: {normalized}"
            )));
        }
        Ok(bytes)
    }
}

fn local_name(start: &BytesStart<'_>) -> String {
    start.local_name().as_ref().to_owned()
}

fn local_end_name(end: &quick_xml::events::BytesEnd<'_>) -> String {
    end.local_name().as_ref().to_owned()
}

fn attribute(start: &BytesStart<'_>, name: &str) -> Result<Option<String>, CoreError> {
    for attribute in start.attributes().with_checks(true) {
        let attribute = attribute
            .map_err(|error| CoreError::three_mf(format!("invalid XML attribute: {error}")))?;
        if attribute.key.local_name().as_ref() == name {
            let value = attribute
                .normalized_value(XmlVersion::default())
                .map_err(|error| {
                    CoreError::three_mf(format!("invalid XML attribute value: {error}"))
                })?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}

fn required_attribute(start: &BytesStart<'_>, name: &str) -> Result<String, CoreError> {
    attribute(start, name)?
        .ok_or_else(|| CoreError::three_mf(format!("3MF {name} attribute is missing")))
}

fn property_index(start: &BytesStart<'_>) -> Result<Option<usize>, CoreError> {
    for name in ["p1", "pindex", "p2", "p3"] {
        if let Some(value) = attribute(start, name)? {
            return Ok(value.parse::<usize>().ok());
        }
    }
    Ok(None)
}

fn parse_color(value: Option<String>, context: &str) -> Result<Option<[u8; 4]>, CoreError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let raw = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if raw.len() != 6 && raw.len() != 8 {
        return Err(CoreError::three_mf(format!(
            "invalid color in {context}: {value:?}"
        )));
    }
    let mut channels = [0_u8; 4];
    for (index, channel) in channels.iter_mut().enumerate().take(raw.len() / 2) {
        let start = index * 2;
        *channel = u8::from_str_radix(&raw[start..start + 2], 16)
            .map_err(|_| CoreError::three_mf(format!("invalid color in {context}: {value:?}")))?;
    }
    if raw.len() == 6 {
        channels[3] = 255;
    }
    Ok(Some(channels))
}

fn parse_transform(start: &BytesStart<'_>) -> Result<Matrix4, CoreError> {
    let Some(raw) = attribute(start, "transform")? else {
        return Ok(Matrix4::identity());
    };
    let values = raw
        .split_whitespace()
        .map(|value| {
            value
                .parse::<f64>()
                .map_err(|_| CoreError::three_mf(format!("invalid 3MF transform: {raw:?}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() != 12 || values.iter().any(|value| !value.is_finite()) {
        return Err(CoreError::three_mf(format!(
            "3MF transforms require 12 finite values, got {}",
            values.len()
        )));
    }
    let mut rows = Matrix4::identity().rows;
    for row in 0..3 {
        for column in 0..4 {
            rows[row][column] = values[column * 3 + row];
        }
    }
    Ok(Matrix4 { rows })
}

fn parse_unit(value: Option<String>) -> Result<ThreeMfUnit, CoreError> {
    ThreeMfUnit::parse(value.as_deref().unwrap_or("millimeter"))
}

fn parse_f32(value: Option<String>, label: &str) -> Result<f32, CoreError> {
    let raw = value.ok_or_else(|| CoreError::three_mf(format!("3MF {label} is missing")))?;
    let parsed = raw
        .parse::<f64>()
        .map_err(|_| CoreError::three_mf(format!("invalid 3MF {label}: {raw:?}")))?;
    if !parsed.is_finite() || parsed < f64::from(f32::MIN) || parsed > f64::from(f32::MAX) {
        return Err(CoreError::three_mf(format!(
            "3MF {label} must be a finite float32"
        )));
    }
    Ok(parsed as f32)
}

fn parse_u32(value: Option<String>, label: &str) -> Result<u32, CoreError> {
    let raw = value.ok_or_else(|| CoreError::three_mf(format!("3MF {label} is missing")))?;
    raw.parse::<u32>()
        .map_err(|_| CoreError::three_mf(format!("invalid 3MF {label}: {raw:?}")))
}

struct ModelParser {
    root_seen: bool,
    root_closed: bool,
    unit: ThreeMfUnit,
    context: Vec<String>,
    metadata: ThreeMfMetadata,
    metadata_capture: Option<MetadataCapture>,
    metadata_bytes: u64,
    materials: HashMap<String, Vec<[u8; 4]>>,
    current_material: Option<MaterialBuilder>,
    material_color_count: usize,
    objects: Vec<ObjectBuilder>,
    object_ids: HashSet<String>,
    current_object: Option<ObjectBuilder>,
    build_seen: bool,
    builds: Vec<BuildBuilder>,
}

impl ModelParser {
    fn new() -> Self {
        Self {
            root_seen: false,
            root_closed: false,
            unit: ThreeMfUnit::Millimeter,
            context: Vec::new(),
            metadata: ThreeMfMetadata::new(),
            metadata_capture: None,
            metadata_bytes: 0,
            materials: HashMap::new(),
            current_material: None,
            material_color_count: 0,
            objects: Vec::new(),
            object_ids: HashSet::new(),
            current_object: None,
            build_seen: false,
            builds: Vec::new(),
        }
    }

    fn is_parent(&self, name: &str) -> bool {
        self.context.last().is_some_and(|parent| parent == name)
    }

    fn start(
        &mut self,
        start: &BytesStart<'_>,
        empty: bool,
        limits: &ThreeMfLimits,
    ) -> Result<(), CoreError> {
        let name = local_name(start);
        if !self.root_seen && name != "model" {
            return Err(CoreError::three_mf("3MF model root is missing"));
        }
        match name.as_str() {
            "model" => {
                if self.root_seen {
                    return Err(CoreError::three_mf("3MF contains more than one model root"));
                }
                self.root_seen = true;
                self.unit = parse_unit(attribute(start, "unit")?)?;
                if empty {
                    return Err(CoreError::three_mf("3MF model root cannot be empty"));
                }
            }
            "metadata" => {
                if self.metadata_capture.is_some() {
                    return Err(CoreError::three_mf("3MF metadata elements may not nest"));
                }
                if let Some(name) = attribute(start, "name")? {
                    self.metadata_capture = Some(MetadataCapture {
                        name,
                        text: String::new(),
                    });
                }
                if empty {
                    self.finish_metadata(limits)?;
                }
            }
            "basematerials" | "colorgroup" => {
                let id = required_attribute(start, "id")?;
                if self.current_material.is_some() {
                    return Err(CoreError::three_mf("3MF material resources may not nest"));
                }
                self.current_material = Some(MaterialBuilder {
                    id,
                    colors: Vec::new(),
                });
                if empty {
                    self.finish_material();
                }
            }
            "base" | "color" => {
                if self.current_material.is_some() {
                    let raw = if name == "base" {
                        attribute(start, "displaycolor")?
                    } else {
                        attribute(start, "color")?
                    };
                    if let Some(color) = parse_color(raw, "material resource")? {
                        self.material_color_count =
                            self.material_color_count.checked_add(1).ok_or_else(|| {
                                CoreError::three_mf("3MF material color count overflow")
                            })?;
                        if self.material_color_count > limits.material_colors {
                            return Err(CoreError::three_mf(format!(
                                "resources contain more than {} material colors",
                                limits.material_colors
                            )));
                        }
                        if let Some(material) = &mut self.current_material {
                            material.colors.push(color);
                        }
                    }
                }
            }
            "object" => {
                if self.current_object.is_some() {
                    return Err(CoreError::three_mf("3MF objects may not nest"));
                }
                if self.objects.len() >= limits.objects {
                    return Err(CoreError::three_mf(format!(
                        "contains more than {} object resources",
                        limits.objects
                    )));
                }
                let object_id = required_attribute(start, "id")?;
                if !self.object_ids.insert(object_id.clone()) {
                    return Err(CoreError::three_mf(format!(
                        "contains duplicate object id {object_id}"
                    )));
                }
                let object = ObjectBuilder {
                    name: attribute(start, "name")?.unwrap_or_else(|| object_id.clone()),
                    material_id: attribute(start, "pid")?,
                    property_index: property_index(start)?,
                    object_id,
                    mesh_seen: false,
                    vertices_seen: false,
                    triangles_seen: false,
                    vertices: Vec::new(),
                    triangles: Vec::new(),
                    components: Vec::new(),
                };
                self.current_object = Some(object);
                if empty {
                    self.finish_object();
                }
            }
            "mesh" => {
                if let Some(object) = &mut self.current_object {
                    object.mesh_seen = true;
                }
            }
            "vertices" => {
                if let Some(object) = &mut self.current_object {
                    object.vertices_seen = true;
                }
            }
            "triangles" => {
                if let Some(object) = &mut self.current_object {
                    object.triangles_seen = true;
                }
            }
            "vertex" if self.is_parent("vertices") => {
                if let Some(object) = &mut self.current_object {
                    object.vertices.push([
                        parse_f32(attribute(start, "x")?, "vertex x")?,
                        parse_f32(attribute(start, "y")?, "vertex y")?,
                        parse_f32(attribute(start, "z")?, "vertex z")?,
                    ]);
                    if let Some(limit) = limits.mesh_vertices
                        && object.vertices.len() > limit
                    {
                        return Err(CoreError::three_mf(format!(
                            "object {} contains more than {limit} vertices",
                            object.object_id
                        )));
                    }
                }
            }
            "triangle" if self.is_parent("triangles") => {
                if let Some(object) = &mut self.current_object {
                    object.triangles.push(TriangleDefinition {
                        face: [
                            parse_u32(attribute(start, "v1")?, "triangle v1")?,
                            parse_u32(attribute(start, "v2")?, "triangle v2")?,
                            parse_u32(attribute(start, "v3")?, "triangle v3")?,
                        ],
                        material_id: attribute(start, "pid")?,
                        property_index: property_index(start)?,
                    });
                    if let Some(limit) = limits.mesh_faces
                        && object.triangles.len() > limit
                    {
                        return Err(CoreError::three_mf(format!(
                            "object {} contains more than {limit} faces",
                            object.object_id
                        )));
                    }
                }
            }
            "component" if self.is_parent("components") => {
                if let Some(object) = &mut self.current_object {
                    let target_id = required_attribute(start, "objectid")?;
                    object.components.push(ComponentDefinition {
                        target_key: target_id.clone(),
                        target_id,
                        transform: parse_transform(start)?,
                        path: attribute(start, "path")?,
                    });
                }
            }
            "components" => {}
            "build" => {
                self.build_seen = true;
            }
            "item" if self.is_parent("build") => {
                if self.builds.len() >= limits.build_items {
                    return Err(CoreError::three_mf(format!(
                        "build contains more than {} items",
                        limits.build_items
                    )));
                }
                self.builds.push(BuildBuilder {
                    object_id: required_attribute(start, "objectid")?,
                    transform: parse_transform(start)?,
                    name: attribute(start, "partnumber")?,
                    material_id: attribute(start, "pid")?,
                    property_index: property_index(start)?,
                    path: attribute(start, "path")?,
                });
            }
            _ => {}
        }
        if !empty {
            self.context.push(name);
        }
        Ok(())
    }

    fn finish_metadata(&mut self, limits: &ThreeMfLimits) -> Result<(), CoreError> {
        let Some(capture) = self.metadata_capture.take() else {
            return Ok(());
        };
        let name = capture.name;
        if name.is_empty() {
            return Ok(());
        }
        let value = capture.text.trim().to_owned();
        let bytes = u64::try_from(name.len())
            .and_then(|name_bytes| {
                u64::try_from(value.len()).map(|value_bytes| name_bytes + value_bytes)
            })
            .map_err(|_| CoreError::three_mf("metadata size does not fit in u64"))?;
        self.metadata_bytes = self
            .metadata_bytes
            .checked_add(bytes)
            .ok_or_else(|| CoreError::three_mf("metadata size accounting overflow"))?;
        if self.metadata_bytes > limits.metadata_bytes {
            return Err(CoreError::three_mf(format!(
                "metadata exceeds the maximum size of {} bytes",
                limits.metadata_bytes
            )));
        }
        self.metadata.entry(name).or_default().push(value);
        Ok(())
    }

    fn finish_material(&mut self) {
        if let Some(material) = self.current_material.take()
            && !material.colors.is_empty()
        {
            self.materials.insert(material.id, material.colors);
        }
    }

    fn finish_object(&mut self) {
        if let Some(object) = self.current_object.take() {
            self.objects.push(object);
        }
    }

    fn end(
        &mut self,
        end: &quick_xml::events::BytesEnd<'_>,
        limits: &ThreeMfLimits,
    ) -> Result<(), CoreError> {
        let name = local_end_name(end);
        match name.as_str() {
            "metadata" => self.finish_metadata(limits)?,
            "basematerials" | "colorgroup" => self.finish_material(),
            "object" => self.finish_object(),
            "model" => self.root_closed = true,
            _ => {}
        }
        let Some(open) = self.context.pop() else {
            return Err(CoreError::three_mf(format!(
                "unexpected 3MF closing tag {name}"
            )));
        };
        if open != name {
            return Err(CoreError::three_mf(format!(
                "3MF closing tag {name} does not match {open}"
            )));
        }
        Ok(())
    }

    fn text(&mut self, value: &str) {
        if let Some(capture) = &mut self.metadata_capture {
            capture.text.push_str(value);
        }
    }

    fn finish(
        mut self,
        limits: &ThreeMfLimits,
        mesh_budget: &mut MeshBudget,
    ) -> Result<ParsedModel, CoreError> {
        if !self.root_seen || !self.root_closed || !self.context.is_empty() {
            return Err(CoreError::three_mf("3MF model XML is incomplete"));
        }
        self.finish_material();
        let materials = self.materials;
        let mut objects = Vec::with_capacity(self.objects.len());
        for builder in self.objects {
            objects.push(finalize_object(builder, &materials, limits, mesh_budget)?);
        }
        let build = if self.build_seen {
            let mut build = Vec::with_capacity(self.builds.len());
            for item in self.builds {
                build.push(BuildDefinition {
                    object_key: item.object_id.clone(),
                    object_id: item.object_id,
                    transform: item.transform,
                    name: item.name,
                    color: material_color(
                        &materials,
                        item.material_id.as_deref(),
                        item.property_index,
                    )
                    .map(color_to_float),
                    path: item.path,
                });
            }
            Some(build)
        } else {
            None
        };
        Ok(ParsedModel {
            unit: self.unit,
            metadata: self.metadata,
            objects,
            build,
        })
    }
}

fn color_to_float(color: [u8; 4]) -> [f32; 4] {
    [
        f32::from(color[0]) / 255.0,
        f32::from(color[1]) / 255.0,
        f32::from(color[2]) / 255.0,
        f32::from(color[3]) / 255.0,
    ]
}

fn material_color(
    materials: &HashMap<String, Vec<[u8; 4]>>,
    material_id: Option<&str>,
    property_index: Option<usize>,
) -> Option<[u8; 4]> {
    let colors = materials.get(material_id?)?;
    match property_index {
        Some(index) => colors
            .get(index)
            .copied()
            .or_else(|| colors.first().copied()),
        None => colors.first().copied(),
    }
}

fn resolve_face_colors(
    triangles: &[TriangleDefinition],
    object_material_id: Option<&str>,
    object_property_index: Option<usize>,
    materials: &HashMap<String, Vec<[u8; 4]>>,
) -> (Option<[f32; 4]>, Option<Vec<[u8; 4]>>) {
    let mut colors = vec![[0_u8; 4]; triangles.len()];
    let mut resolved = vec![false; triangles.len()];
    let mut counts: HashMap<[u8; 4], (usize, usize)> = HashMap::new();
    for (index, triangle) in triangles.iter().enumerate() {
        let material_id = triangle.material_id.as_deref().or(object_material_id);
        let property_index = triangle.property_index.or(object_property_index);
        if let Some(color) = material_color(materials, material_id, property_index) {
            colors[index] = color;
            resolved[index] = true;
            let next_order = counts.len();
            if let Some((count, _)) = counts.get_mut(&color) {
                *count += 1;
            } else {
                counts.insert(color, (1, next_order));
            }
        }
    }
    let representative = if counts.is_empty() {
        material_color(materials, object_material_id, object_property_index)
    } else {
        counts
            .iter()
            .max_by(|(_, left), (_, right)| left.0.cmp(&right.0).then_with(|| right.1.cmp(&left.1)))
            .map(|(color, _)| *color)
    };
    let Some(representative) = representative else {
        return (None, None);
    };
    for (index, was_resolved) in resolved.iter().enumerate() {
        if !was_resolved {
            colors[index] = representative;
        }
    }
    (Some(color_to_float(representative)), Some(colors))
}

fn finalize_object(
    builder: ObjectBuilder,
    materials: &HashMap<String, Vec<[u8; 4]>>,
    limits: &ThreeMfLimits,
    mesh_budget: &mut MeshBudget,
) -> Result<ObjectDefinition, CoreError> {
    let mesh = if builder.mesh_seen {
        if !builder.vertices_seen || !builder.triangles_seen {
            return Err(CoreError::three_mf(format!(
                "object {} has an incomplete mesh",
                builder.object_id
            )));
        }
        if builder.vertices.is_empty() || builder.triangles.is_empty() {
            return Err(CoreError::three_mf(format!(
                "object {} contains an empty mesh",
                builder.object_id
            )));
        }
        mesh_budget.vertices = mesh_budget
            .vertices
            .checked_add(builder.vertices.len())
            .ok_or_else(|| CoreError::three_mf("3MF vertex count overflow"))?;
        mesh_budget.faces = mesh_budget
            .faces
            .checked_add(builder.triangles.len())
            .ok_or_else(|| CoreError::three_mf("3MF face count overflow"))?;
        if let Some(limit) = limits.total_mesh_vertices
            && mesh_budget.vertices > limit
        {
            return Err(CoreError::three_mf(format!(
                "meshes contain more than {limit} total vertices"
            )));
        }
        if let Some(limit) = limits.total_mesh_faces
            && mesh_budget.faces > limit
        {
            return Err(CoreError::three_mf(format!(
                "meshes contain more than {limit} total faces"
            )));
        }
        let faces = builder
            .triangles
            .iter()
            .map(|triangle| triangle.face)
            .collect::<Vec<_>>();
        Some(Mesh::new(builder.vertices, faces).map_err(|error| {
            CoreError::three_mf(format!(
                "object {} has an invalid mesh: {error}",
                builder.object_id
            ))
        })?)
    } else {
        None
    };
    let (color, face_colors) = if mesh.is_some() {
        resolve_face_colors(
            &builder.triangles,
            builder.material_id.as_deref(),
            builder.property_index,
            materials,
        )
    } else {
        (None, None)
    };
    Ok(ObjectDefinition {
        key: builder.object_id.clone(),
        object_id: builder.object_id,
        name: builder.name,
        mesh,
        color,
        face_colors,
        components: builder.components,
    })
}

fn parse_model_part(
    raw: &[u8],
    limits: &ThreeMfLimits,
    mesh_budget: &mut MeshBudget,
) -> Result<ParsedModel, CoreError> {
    if contains_xml_marker(raw, b"<!DOCTYPE") || contains_xml_marker(raw, b"<!ENTITY") {
        return Err(CoreError::three_mf("Invalid 3MF model XML"));
    }
    let mut reader = Reader::from_reader(raw);
    reader.config_mut().trim_text(false);
    let mut parser = ModelParser::new();
    let mut buffer = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(start)) => parser.start(&start, false, limits)?,
            Ok(Event::Empty(start)) => parser.start(&start, true, limits)?,
            Ok(Event::End(end)) => parser.end(&end, limits)?,
            Ok(Event::Text(text)) => {
                let value = unescape(text.as_ref())
                    .map_err(|error| CoreError::three_mf(format!("invalid 3MF text: {error}")))?;
                parser.text(value.as_ref());
            }
            Ok(Event::CData(text)) => parser.text(text.as_ref()),
            Ok(Event::GeneralRef(_)) | Ok(Event::DocType(_)) => {
                return Err(CoreError::three_mf("Invalid 3MF model XML"));
            }
            Ok(Event::Decl(_) | Event::Comment(_) | Event::PI(_)) => {}
            Ok(Event::Eof) => break,
            Err(error) => {
                return Err(CoreError::three_mf(format!(
                    "Invalid 3MF model XML: {error}"
                )));
            }
        }
        buffer.clear();
    }
    parser.finish(limits, mesh_budget)
}

fn contains_xml_marker(raw: &[u8], marker: &[u8]) -> bool {
    raw.windows(marker.len())
        .any(|window| window.eq_ignore_ascii_case(marker))
}

fn normalize_zip_name(name: &str) -> Result<String, CoreError> {
    let decoded = percent_decode(name).replace('\\', "/");
    let without_root = decoded.trim_start_matches('/');
    let mut components = Vec::new();
    for component in without_root.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if components.pop().is_none() {
                    return Err(CoreError::three_mf(format!(
                        "invalid path in package: {name:?}"
                    )));
                }
            }
            component => components.push(component),
        }
    }
    if components.is_empty() {
        Ok(".".to_owned())
    } else {
        Ok(components.join("/"))
    }
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn resolve_target(target: &str, base_dir: &str) -> Result<String, CoreError> {
    let decoded = percent_decode(target).replace('\\', "/");
    let candidate = if decoded.starts_with('/') {
        decoded.trim_start_matches('/').to_owned()
    } else if base_dir.is_empty() {
        decoded
    } else {
        format!("{base_dir}/{decoded}")
    };
    normalize_zip_name(&candidate)
}

fn relationship_part(source_part: Option<&str>) -> String {
    let Some(source_part) = source_part else {
        return "_rels/.rels".to_owned();
    };
    let directory = source_part
        .rsplit_once('/')
        .map(|(dir, _)| dir)
        .unwrap_or("");
    let filename = source_part.rsplit('/').next().unwrap_or(source_part);
    if directory.is_empty() {
        format!("_rels/{filename}.rels")
    } else {
        format!("{directory}/_rels/{filename}.rels")
    }
}

fn read_relationships(
    package: &Package,
    source_part: Option<&str>,
    limits: &ThreeMfLimits,
) -> Result<Vec<ThreeMfRelationship>, CoreError> {
    let relationship_path = relationship_part(source_part);
    if !package.contains(&relationship_path)? {
        return Ok(Vec::new());
    }
    let raw = package.read_part(&relationship_path)?;
    if contains_xml_marker(&raw, b"<!DOCTYPE") || contains_xml_marker(&raw, b"<!ENTITY") {
        return Err(CoreError::three_mf(format!(
            "invalid relationships part: {relationship_path}"
        )));
    }
    let base_dir = source_part
        .and_then(|part| part.rsplit_once('/').map(|(directory, _)| directory))
        .unwrap_or("");
    let mut reader = Reader::from_reader(raw.as_slice());
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut relationships = Vec::new();
    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(start)) | Ok(Event::Empty(start))
                if local_name(&start) == "Relationship" =>
            {
                if relationships.len() >= limits.relationships {
                    return Err(CoreError::three_mf(format!(
                        "relationships contain more than {} entries",
                        limits.relationships
                    )));
                }
                let target = attribute(&start, "Target")?;
                let Some(target) = target else {
                    buffer.clear();
                    continue;
                };
                let external = attribute(&start, "TargetMode")?
                    .is_some_and(|mode| mode.eq_ignore_ascii_case("external"));
                let resolved_target = if external {
                    None
                } else {
                    Some(resolve_target(&target, base_dir)?)
                };
                relationships.push(ThreeMfRelationship {
                    source: source_part.map(str::to_owned),
                    relationship_type: attribute(&start, "Type")?.unwrap_or_default(),
                    target,
                    resolved_target,
                    id: attribute(&start, "Id")?,
                    external,
                });
            }
            Ok(Event::GeneralRef(_)) | Ok(Event::DocType(_)) => {
                return Err(CoreError::three_mf(format!(
                    "invalid relationships part: {relationship_path}"
                )));
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(CoreError::three_mf(format!(
                    "invalid relationships part: {relationship_path}: {error}"
                )));
            }
        }
        buffer.clear();
    }
    Ok(relationships)
}

fn find_model_part(
    package: &Package,
    limits: &ThreeMfLimits,
) -> Result<(String, Vec<ThreeMfRelationship>), CoreError> {
    let root_relationships = read_relationships(package, None, limits)?;
    for relationship in &root_relationships {
        if relationship
            .relationship_type
            .to_ascii_lowercase()
            .ends_with("/3dmodel")
            && let Some(target) = &relationship.resolved_target
            && let Some(canonical) = package.canonical(target)?
        {
            return Ok((canonical, root_relationships));
        }
    }
    let fallback = "3D/3dmodel.model";
    if let Some(canonical) = package.canonical(fallback)? {
        return Ok((canonical, root_relationships));
    }
    Err(CoreError::three_mf("package has no 3D model part"))
}

fn find_thumbnail(
    package: &Package,
    relationships: &[ThreeMfRelationship],
    limits: &ThreeMfLimits,
) -> Result<(Option<String>, Option<Vec<u8>>), CoreError> {
    let mut candidates = Vec::new();
    for relationship in relationships {
        if relationship
            .relationship_type
            .to_ascii_lowercase()
            .contains("thumbnail")
            && let Some(target) = &relationship.resolved_target
            && let Some(canonical) = package.canonical(target)?
        {
            candidates.push(canonical);
        }
    }
    for part in &package.parts {
        let filename = part.rsplit('/').next().unwrap_or(part).to_ascii_lowercase();
        if filename.starts_with("thumbnail.") {
            candidates.push(part.clone());
        }
    }
    for candidate in candidates {
        let Some(entry) = package.entries.get(&candidate.to_ascii_lowercase()) else {
            continue;
        };
        if entry.size > limits.thumbnail_bytes {
            return Err(CoreError::three_mf(format!(
                "thumbnail exceeds the maximum size of {} bytes",
                limits.thumbnail_bytes
            )));
        }
        return Ok((Some(candidate), Some(package.read_part(&entry.canonical)?)));
    }
    Ok((None, None))
}

struct ExternalResolver<'a> {
    package: &'a Package,
    limits: &'a ThreeMfLimits,
    root_model_path: String,
    root_unit_scale: f64,
    allowed_paths: HashSet<String>,
    objects: &'a mut Vec<ObjectDefinition>,
    object_indices: &'a mut HashMap<String, usize>,
    mesh_budget: &'a mut MeshBudget,
    relationships: &'a mut Vec<ThreeMfRelationship>,
    loaded: HashMap<String, HashMap<String, String>>,
}

impl ExternalResolver<'_> {
    fn resolve(&mut self, path: &str, object_id: &str) -> Result<String, CoreError> {
        let root_dir = self
            .root_model_path
            .rsplit_once('/')
            .map(|(directory, _)| directory)
            .unwrap_or("");
        let target_path = resolve_target(path, root_dir)?;
        if !self
            .allowed_paths
            .contains(&target_path.to_ascii_lowercase())
        {
            return Err(CoreError::three_mf(format!(
                "external model part is not declared by the root model relationships: {target_path}"
            )));
        }
        let target_key = target_path.to_ascii_lowercase();
        if !self.loaded.contains_key(&target_key) {
            self.load_part(&target_path)?;
        }
        self.loaded
            .get(&target_key)
            .and_then(|objects| objects.get(object_id))
            .cloned()
            .ok_or_else(|| {
                CoreError::three_mf(format!(
                    "external component references unknown object {object_id} in {target_path}"
                ))
            })
    }

    fn load_part(&mut self, target_path: &str) -> Result<(), CoreError> {
        let canonical = self.package.canonical(target_path)?.ok_or_else(|| {
            CoreError::three_mf(format!("external component part is missing: {target_path}"))
        })?;
        let canonical_key = canonical.to_ascii_lowercase();
        if self.loaded.contains_key(&canonical_key) {
            return Ok(());
        }
        let raw = self.package.read_part(&canonical)?;
        let part_relationships = read_relationships(self.package, Some(&canonical), self.limits)?;
        if self
            .relationships
            .len()
            .checked_add(part_relationships.len())
            .is_none_or(|count| count > self.limits.relationships)
        {
            return Err(CoreError::three_mf(format!(
                "relationships contain more than {} entries",
                self.limits.relationships
            )));
        }
        self.relationships.extend(part_relationships);
        let parsed = parse_model_part(&raw, self.limits, self.mesh_budget)?;
        let unit_factor = parsed.unit.scale_mm() / self.root_unit_scale;
        let mut object_map = HashMap::new();
        for object in &parsed.objects {
            object_map.insert(
                object.object_id.clone(),
                format!("{}::{}", canonical, object.object_id),
            );
        }
        let mut normalized = Vec::with_capacity(parsed.objects.len());
        for mut object in parsed.objects {
            let internal_key = object_map
                .get(&object.object_id)
                .cloned()
                .ok_or_else(|| CoreError::three_mf("external object map is incomplete"))?;
            if let Some(mesh) = object.mesh.take() {
                object.mesh = Some(scale_mesh(mesh, unit_factor)?);
            }
            for component in &mut object.components {
                if component.path.is_some() {
                    return Err(CoreError::three_mf(
                        "external component paths are only valid in the root model part",
                    ));
                }
                component.target_key =
                    object_map
                        .get(&component.target_id)
                        .cloned()
                        .ok_or_else(|| {
                            CoreError::three_mf(format!(
                                "external component references unknown object {} in {canonical}",
                                component.target_id
                            ))
                        })?;
                component.transform = component.transform.scale_translation(unit_factor)?;
            }
            object.key = internal_key.clone();
            if self.objects.len() >= self.limits.objects {
                return Err(CoreError::three_mf(format!(
                    "contains more than {} object resources",
                    self.limits.objects
                )));
            }
            let index = self.objects.len();
            self.objects.push(object);
            self.object_indices.insert(internal_key, index);
            normalized.push(index);
        }
        let _ = normalized;
        self.loaded.insert(canonical_key, object_map);
        Ok(())
    }
}

fn scale_mesh(mesh: Mesh, factor: f64) -> Result<Mesh, CoreError> {
    if !factor.is_finite() {
        return Err(CoreError::three_mf("3MF unit scale must be finite"));
    }
    let mut vertices = Vec::with_capacity(mesh.vertices.len());
    for vertex in mesh.vertices {
        let mut scaled = [0.0_f32; 3];
        for (output, input) in scaled.iter_mut().zip(vertex) {
            let value = f64::from(input) * factor;
            if !value.is_finite() || value < f64::from(f32::MIN) || value > f64::from(f32::MAX) {
                return Err(CoreError::three_mf(
                    "3MF scaled vertex is not a finite float32",
                ));
            }
            *output = value as f32;
        }
        vertices.push(scaled);
    }
    Mesh::new(vertices, mesh.faces)
        .map_err(|error| CoreError::three_mf(format!("3MF scaled mesh is invalid: {error}")))
}

fn unique_name(name: &str, used: &mut HashSet<String>) -> String {
    let base = if name.is_empty() { "Imported" } else { name };
    if used.insert(base.to_owned()) {
        return base.to_owned();
    }
    let mut suffix = 2;
    loop {
        let candidate = format!("{base} ({suffix})");
        if used.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

struct PendingItem {
    mesh: Mesh,
    transform: Matrix4,
    name: String,
    color: [f32; 4],
    object_id: String,
    face_colors: Option<Vec<[u8; 4]>>,
}

struct ExpansionContext<'a> {
    objects: &'a [ObjectDefinition],
    object_indices: &'a HashMap<String, usize>,
    pending: &'a mut Vec<PendingItem>,
    limits: &'a ThreeMfLimits,
    stack: Vec<String>,
}

impl ExpansionContext<'_> {
    fn expand(
        &mut self,
        object_key: &str,
        transform: Matrix4,
        name: String,
        inherited_color: Option<[f32; 4]>,
    ) -> Result<(), CoreError> {
        if self.stack.iter().any(|key| key == object_key) {
            let mut cycle = self.stack.join(" -> ");
            if !cycle.is_empty() {
                cycle.push_str(" -> ");
            }
            cycle.push_str(object_key);
            return Err(CoreError::three_mf(format!(
                "component cycle detected: {cycle}"
            )));
        }
        if self.stack.len() > self.limits.component_depth {
            return Err(CoreError::three_mf(format!(
                "component nesting exceeds {} levels",
                self.limits.component_depth
            )));
        }
        let index = self
            .object_indices
            .get(object_key)
            .copied()
            .ok_or_else(|| {
                CoreError::three_mf(format!("references unknown object {object_key}"))
            })?;
        let object = self
            .objects
            .get(index)
            .ok_or_else(|| CoreError::three_mf("object index map is inconsistent"))?;
        let color = object.color.or(inherited_color);
        let mesh = object.mesh.clone();
        let object_name = object.name.clone();
        let object_id = object.object_id.clone();
        let face_colors = object.face_colors.clone();
        let components = object.components.clone();
        if let Some(mesh) = mesh {
            if self.pending.len() >= self.limits.instances {
                return Err(CoreError::three_mf(format!(
                    "expands to more than {} mesh instances",
                    self.limits.instances
                )));
            }
            self.pending.push(PendingItem {
                mesh,
                transform,
                name: if name.is_empty() { object_name } else { name },
                color: color.unwrap_or(DEFAULT_MESH_COLOR),
                object_id,
                face_colors,
            });
            return Ok(());
        }
        if components.is_empty() {
            return Ok(());
        }
        self.stack.push(object_key.to_owned());
        let component_count = components.len();
        for component in components {
            if component.path.is_some() {
                return Err(CoreError::three_mf(format!(
                    "external component paths are unresolved in object {object_key}"
                )));
            }
            let child = self
                .object_indices
                .get(&component.target_key)
                .and_then(|child_index| self.objects.get(*child_index))
                .ok_or_else(|| {
                    CoreError::three_mf(format!(
                        "references unknown component object {}",
                        component.target_id
                    ))
                })?;
            let child_name = if !name.is_empty() && component_count > 1 {
                format!("{name}/{}", child.name)
            } else if name.is_empty() {
                child.name.clone()
            } else {
                name.clone()
            };
            let child_transform = transform.multiply(component.transform)?;
            self.expand(&component.target_key, child_transform, child_name, color)?;
        }
        let _ = self.stack.pop();
        Ok(())
    }
}

/// Loads a 3MF package from a path using the default resource limits.
pub fn load_3mf(path: impl AsRef<Path>) -> Result<ThreeMfImport, CoreError> {
    load_3mf_with_limits(path, &ThreeMfLimits::default())
}

/// Loads a 3MF package from a path with an explicit resource policy.
pub fn load_3mf_with_limits(
    path: impl AsRef<Path>,
    limits: &ThreeMfLimits,
) -> Result<ThreeMfImport, CoreError> {
    let path = path.as_ref();
    let metadata = fs::metadata(path).map_err(|error| {
        CoreError::three_mf(format!(
            "could not inspect package {}: {error}",
            path.display()
        ))
    })?;
    if metadata.len() > limits.archive_bytes {
        return Err(CoreError::three_mf(format!(
            "archive exceeds the maximum size of {} bytes",
            limits.archive_bytes
        )));
    }
    let bytes = fs::read(path).map_err(|error| {
        CoreError::three_mf(format!(
            "could not read package {}: {error}",
            path.display()
        ))
    })?;
    load_3mf_bytes_with_limits(&bytes, limits)
}

/// Loads a 3MF package from bytes using the default resource limits.
pub fn load_3mf_bytes(bytes: &[u8]) -> Result<ThreeMfImport, CoreError> {
    load_3mf_bytes_with_limits(bytes, &ThreeMfLimits::default())
}

/// Loads a 3MF package from bytes with an explicit resource policy.
pub fn load_3mf_bytes_with_limits(
    bytes: &[u8],
    limits: &ThreeMfLimits,
) -> Result<ThreeMfImport, CoreError> {
    let package = Package::open(bytes.to_vec(), limits)?;
    let (model_path, root_relationships) = find_model_part(&package, limits)?;
    let model_relationships = read_relationships(&package, Some(&model_path), limits)?;
    let relationship_count = root_relationships
        .len()
        .checked_add(model_relationships.len())
        .ok_or_else(|| CoreError::three_mf("relationship count overflow"))?;
    if relationship_count > limits.relationships {
        return Err(CoreError::three_mf(format!(
            "relationships contain more than {} entries",
            limits.relationships
        )));
    }
    let mut relationships = root_relationships;
    relationships.extend(model_relationships.clone());
    let (thumbnail_path, thumbnail_data) = find_thumbnail(&package, &relationships, limits)?;
    let model_raw = package.read_part(&model_path)?;
    let mut mesh_budget = MeshBudget::default();
    let parsed = parse_model_part(&model_raw, limits, &mut mesh_budget)?;
    let unit_scale = parsed.unit.scale_mm();
    let mut objects = parsed.objects;
    let mut object_indices = HashMap::new();
    for (index, object) in objects.iter().enumerate() {
        object_indices.insert(object.key.clone(), index);
    }
    let allowed_paths = model_relationships
        .iter()
        .filter(|relationship| {
            relationship
                .relationship_type
                .to_ascii_lowercase()
                .ends_with("/3dmodel")
        })
        .filter_map(|relationship| relationship.resolved_target.as_ref())
        .map(|path| path.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut build = parsed.build;
    {
        let mut resolver = ExternalResolver {
            package: &package,
            limits,
            root_model_path: model_path.clone(),
            root_unit_scale: unit_scale,
            allowed_paths,
            objects: &mut objects,
            object_indices: &mut object_indices,
            mesh_budget: &mut mesh_budget,
            relationships: &mut relationships,
            loaded: HashMap::new(),
        };
        let root_count = resolver.objects.len();
        for object_index in 0..root_count {
            let components = resolver
                .objects
                .get(object_index)
                .map(|object| object.components.clone())
                .unwrap_or_default();
            for (component_index, component) in components.into_iter().enumerate() {
                if let Some(path) = component.path {
                    let target_key = resolver.resolve(&path, &component.target_id)?;
                    if let Some(object) = resolver.objects.get_mut(object_index)
                        && let Some(component) = object.components.get_mut(component_index)
                    {
                        component.target_key = target_key;
                        component.path = None;
                    }
                }
            }
        }
        if let Some(build_items) = &mut build {
            for item in build_items {
                if let Some(path) = item.path.take() {
                    item.object_key = resolver.resolve(&path, &item.object_id)?;
                }
            }
        }
    }

    let mut pending = Vec::new();
    let mut expansion = ExpansionContext {
        objects: &objects,
        object_indices: &object_indices,
        pending: &mut pending,
        limits,
        stack: Vec::new(),
    };
    if let Some(build_items) = &build {
        for item in build_items {
            let object = object_indices
                .get(&item.object_key)
                .and_then(|index| objects.get(*index))
                .ok_or_else(|| {
                    CoreError::three_mf(format!(
                        "build item references unknown object {}",
                        item.object_id
                    ))
                })?;
            let name = item.name.clone().unwrap_or_else(|| object.name.clone());
            let before = expansion.pending.len();
            expansion.expand(&item.object_key, item.transform, name, item.color)?;
            if expansion.pending.len() == before {
                return Err(CoreError::three_mf(format!(
                    "build item {} contains no mesh geometry",
                    item.object_id
                )));
            }
        }
    } else {
        let referenced = objects
            .iter()
            .flat_map(|object| {
                object
                    .components
                    .iter()
                    .map(|component| component.target_key.clone())
            })
            .collect::<HashSet<_>>();
        let roots = objects
            .iter()
            .filter(|object| !referenced.contains(&object.key))
            .map(|object| (object.key.clone(), object.name.clone()))
            .collect::<Vec<_>>();
        for (object_key, name) in roots {
            expansion.expand(&object_key, Matrix4::identity(), name, None)?;
        }
    }

    let mut used_names = HashSet::new();
    let mut items = Vec::with_capacity(pending.len());
    for pending in pending {
        let mesh = scale_mesh(pending.mesh, unit_scale)?;
        let transform = Transform::z_up_file_to_scene().multiply(
            &pending
                .transform
                .scale_translation(unit_scale)?
                .to_transform()?,
        )?;
        let mut item = ThreeMfItem::new(
            mesh,
            transform,
            unique_name(&pending.name, &mut used_names),
            pending.color,
        )?;
        item.object_id = pending.object_id;
        item.metadata = parsed.metadata.clone();
        item.face_colors = pending.face_colors;
        items.push(item);
    }
    Ok(ThreeMfImport {
        items,
        unit: parsed.unit,
        unit_scale_mm: unit_scale,
        metadata: parsed.metadata,
        thumbnail_path,
        thumbnail_data,
        model_path,
        package_parts: package.parts,
        relationships,
    })
}

/// Reads only the thumbnail of a 3MF package from a path.
pub fn load_3mf_thumbnail(path: impl AsRef<Path>) -> Result<Option<Vec<u8>>, CoreError> {
    load_3mf_thumbnail_with_limits(path, &ThreeMfLimits::default())
}

/// Reads only the thumbnail of a 3MF package from a path with explicit limits.
pub fn load_3mf_thumbnail_with_limits(
    path: impl AsRef<Path>,
    limits: &ThreeMfLimits,
) -> Result<Option<Vec<u8>>, CoreError> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|error| {
        CoreError::three_mf(format!(
            "could not read package {}: {error}",
            path.display()
        ))
    })?;
    load_3mf_thumbnail_bytes_with_limits(&bytes, limits)
}

/// Reads only the thumbnail of a 3MF package from bytes.
pub fn load_3mf_thumbnail_bytes(bytes: &[u8]) -> Result<Option<Vec<u8>>, CoreError> {
    load_3mf_thumbnail_bytes_with_limits(bytes, &ThreeMfLimits::default())
}

/// Reads only the thumbnail of a 3MF package from bytes with explicit limits.
pub fn load_3mf_thumbnail_bytes_with_limits(
    bytes: &[u8],
    limits: &ThreeMfLimits,
) -> Result<Option<Vec<u8>>, CoreError> {
    let package = Package::open(bytes.to_vec(), limits)?;
    let (model_path, root_relationships) = find_model_part(&package, limits)?;
    let model_relationships = read_relationships(&package, Some(&model_path), limits)?;
    let relationship_count = root_relationships
        .len()
        .checked_add(model_relationships.len())
        .ok_or_else(|| CoreError::three_mf("relationship count overflow"))?;
    if relationship_count > limits.relationships {
        return Err(CoreError::three_mf(format!(
            "relationships contain more than {} entries",
            limits.relationships
        )));
    }
    let mut relationships = root_relationships;
    relationships.extend(model_relationships);
    Ok(find_thumbnail(&package, &relationships, limits)?.1)
}

fn write_error(error: std::io::Error) -> CoreError {
    CoreError::three_mf(format!("3MF XML write failed: {error}"))
}

fn write_start(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    attributes: &[(&str, String)],
) -> Result<(), CoreError> {
    let mut element = BytesStart::new(name);
    for (key, value) in attributes {
        element.push_attribute((*key, value.as_str()));
    }
    writer
        .write_event(Event::Start(element))
        .map_err(write_error)
}

fn write_empty(
    writer: &mut Writer<Vec<u8>>,
    name: &str,
    attributes: &[(&str, String)],
) -> Result<(), CoreError> {
    let mut element = BytesStart::new(name);
    for (key, value) in attributes {
        element.push_attribute((*key, value.as_str()));
    }
    writer
        .write_event(Event::Empty(element))
        .map_err(write_error)
}

fn write_end(writer: &mut Writer<Vec<u8>>, name: &str) -> Result<(), CoreError> {
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .map_err(write_error)
}

fn write_text(writer: &mut Writer<Vec<u8>>, text: &str) -> Result<(), CoreError> {
    writer
        .write_event(Event::Text(BytesText::new(text)))
        .map_err(write_error)
}

fn format_number(value: f64) -> Result<String, CoreError> {
    if !value.is_finite() {
        return Err(CoreError::three_mf("3MF cannot encode a non-finite number"));
    }
    let value = if value.abs() < 1.0e-12 { 0.0 } else { value };
    let mut formatted = format!("{value:.9}");
    if let Some(dot) = formatted.find('.') {
        while formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.len() == dot + 1 {
            formatted.pop();
        }
    }
    if formatted == "-0" {
        formatted = "0".to_owned();
    }
    Ok(formatted)
}

fn transform_text(transform: Transform) -> Result<String, CoreError> {
    let rows = transform.f64_rows();
    let columns = [
        [rows[0][0], rows[1][0], rows[2][0]],
        [rows[0][1], rows[1][1], rows[2][1]],
        [rows[0][2], rows[1][2], rows[2][2]],
        [rows[0][3], rows[1][3], rows[2][3]],
    ];
    let mut values = Vec::with_capacity(12);
    for column in columns {
        for value in column {
            values.push(format_number(value)?);
        }
    }
    Ok(values.join(" "))
}

fn validate_export_color(color: [f32; 4]) -> Result<(), CoreError> {
    if color
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0 || *value > 255.0)
    {
        return Err(CoreError::three_mf(
            "export color must use finite values between 0 and 1, or 0 and 255",
        ));
    }
    Ok(())
}

fn export_color_bytes(color: [f32; 4]) -> Result<[u8; 4], CoreError> {
    validate_export_color(color)?;
    let scale = if color.iter().copied().fold(0.0_f32, f32::max) <= 1.0 {
        255.0
    } else {
        1.0
    };
    Ok(color.map(|value| (value * scale).round() as u8))
}

fn export_color_bytes_text(color: [u8; 4]) -> String {
    format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        color[0], color[1], color[2], color[3]
    )
}

fn intern_export_material(
    color: [u8; 4],
    colors: &mut Vec<[u8; 4]>,
    indices: &mut HashMap<[u8; 4], usize>,
    limit: usize,
) -> Result<usize, CoreError> {
    if let Some(index) = indices.get(&color) {
        return Ok(*index);
    }
    if colors.len() >= limit {
        return Err(CoreError::three_mf(format!(
            "export requires more than {limit} material colors"
        )));
    }
    let index = colors.len();
    colors.push(color);
    indices.insert(color, index);
    Ok(index)
}

fn merge_metadata(items: &[ThreeMfItem], metadata: Option<&ThreeMfMetadata>) -> ThreeMfMetadata {
    if let Some(metadata) = metadata {
        return metadata.clone();
    }
    let mut merged = ThreeMfMetadata::new();
    for item in items {
        for (key, values) in &item.metadata {
            let entry = merged.entry(key.clone()).or_default();
            for value in values {
                if !entry.contains(value) {
                    entry.push(value.clone());
                }
            }
        }
    }
    merged
}

fn model_xml(
    items: &[ThreeMfItem],
    metadata: &ThreeMfMetadata,
    limits: &ThreeMfLimits,
) -> Result<Vec<u8>, CoreError> {
    let mut material_colors = Vec::new();
    let mut material_indices = HashMap::new();
    let mut object_material_indices = Vec::with_capacity(items.len());
    for item in items {
        object_material_indices.push(intern_export_material(
            export_color_bytes(item.color)?,
            &mut material_colors,
            &mut material_indices,
            limits.material_colors,
        )?);
        if let Some(face_colors) = &item.face_colors {
            for color in face_colors {
                intern_export_material(
                    *color,
                    &mut material_colors,
                    &mut material_indices,
                    limits.material_colors,
                )?;
            }
        }
    }
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(write_error)?;
    write_start(
        &mut writer,
        "model",
        &[
            ("xmlns", CORE_NS.to_owned()),
            ("xmlns:m", MATERIAL_NS.to_owned()),
            ("unit", "millimeter".to_owned()),
            ("xml:lang", "en-US".to_owned()),
        ],
    )?;
    for (key, values) in metadata {
        let name = key.trim();
        if name.is_empty() {
            continue;
        }
        for value in values {
            write_start(&mut writer, "metadata", &[("name", name.to_owned())])?;
            write_text(&mut writer, value)?;
            write_end(&mut writer, "metadata")?;
        }
    }
    write_start(&mut writer, "resources", &[])?;
    write_start(&mut writer, "m:basematerials", &[("id", "1".to_owned())])?;
    for (index, color) in material_colors.iter().enumerate() {
        write_empty(
            &mut writer,
            "m:base",
            &[
                ("name", format!("Material {}", index + 1)),
                ("displaycolor", export_color_bytes_text(*color)),
            ],
        )?;
    }
    write_end(&mut writer, "m:basematerials")?;

    // Textures (OF-46): one texture2d per image, one texture2dgroup of
    // per-corner coordinates per textured item. Ids follow the objects'.
    let textures = export_textures(items);
    let mut next_resource_id = items.len() + 2;
    let mut texture_resource_ids = HashMap::new();
    for (image, path) in &textures {
        write_empty(
            &mut writer,
            "m:texture2d",
            &[
                ("id", next_resource_id.to_string()),
                ("path", path.clone()),
                ("contenttype", image.encoding().mime().to_owned()),
                ("tilestyleu", "wrap".to_owned()),
                ("tilestylev", "wrap".to_owned()),
            ],
        )?;
        texture_resource_ids.insert(image.id().to_owned(), next_resource_id);
        next_resource_id += 1;
    }
    let mut item_texture_groups: Vec<Option<(usize, Vec<bool>)>> = Vec::with_capacity(items.len());
    for item in items {
        let resource = item.texture.as_ref().and_then(|reference| {
            texture_resource_ids
                .get(&reference.id)
                .map(|texture_id| (*texture_id, reference.tile_mm))
        });
        let Some((texture_id, tile_mm)) = resource else {
            item_texture_groups.push(None);
            continue;
        };
        let base = export_color_bytes(item.color)?;
        let textured_faces = (0..item.mesh.face_count())
            .map(|face| {
                item.face_colors
                    .as_ref()
                    .and_then(|colors| colors.get(face))
                    .is_none_or(|color| *color == base)
            })
            .collect::<Vec<_>>();
        let group_id = next_resource_id;
        next_resource_id += 1;
        write_start(
            &mut writer,
            "m:texture2dgroup",
            &[
                ("id", group_id.to_string()),
                ("texid", texture_id.to_string()),
            ],
        )?;
        for (face, coordinates) in face_texture_coordinates(&item.mesh, tile_mm)
            .into_iter()
            .enumerate()
        {
            if !textured_faces.get(face).copied().unwrap_or(false) {
                continue;
            }
            for [u, v] in coordinates {
                write_empty(
                    &mut writer,
                    "m:tex2coord",
                    &[
                        ("u", format_number(f64::from(u))?),
                        ("v", format_number(f64::from(v))?),
                    ],
                )?;
            }
        }
        write_end(&mut writer, "m:texture2dgroup")?;
        item_texture_groups.push(Some((group_id, textured_faces)));
    }

    for (index, item) in items.iter().enumerate() {
        item.mesh.validate_non_empty()?;
        let name = if item.name.is_empty() {
            format!("Object {}", index + 1)
        } else {
            item.name.clone()
        };
        let object_id = (index + 2).to_string();
        write_start(
            &mut writer,
            "object",
            &[
                ("id", object_id),
                ("type", "model".to_owned()),
                ("name", name),
                ("pid", "1".to_owned()),
                (
                    "pindex",
                    object_material_indices
                        .get(index)
                        .ok_or_else(|| CoreError::three_mf("object material index is missing"))?
                        .to_string(),
                ),
            ],
        )?;
        write_start(&mut writer, "mesh", &[])?;
        write_start(&mut writer, "vertices", &[])?;
        for vertex in &item.mesh.vertices {
            write_empty(
                &mut writer,
                "vertex",
                &[
                    ("x", format_number(f64::from(vertex[0]))?),
                    ("y", format_number(f64::from(vertex[1]))?),
                    ("z", format_number(f64::from(vertex[2]))?),
                ],
            )?;
        }
        write_end(&mut writer, "vertices")?;
        write_start(&mut writer, "triangles", &[])?;
        let texture_group = item_texture_groups.get(index).and_then(Option::as_ref);
        let mut next_coordinate = 0_usize;
        for (face_index, face) in item.mesh.faces.iter().enumerate() {
            if let Some((group_id, textured_faces)) = texture_group
                && textured_faces.get(face_index).copied().unwrap_or(false)
            {
                write_empty(
                    &mut writer,
                    "triangle",
                    &[
                        ("v1", face[0].to_string()),
                        ("v2", face[1].to_string()),
                        ("v3", face[2].to_string()),
                        ("pid", group_id.to_string()),
                        ("p1", next_coordinate.to_string()),
                        ("p2", (next_coordinate + 1).to_string()),
                        ("p3", (next_coordinate + 2).to_string()),
                    ],
                )?;
                next_coordinate += 3;
            } else if let Some(face_colors) = &item.face_colors {
                let color = face_colors
                    .get(face_index)
                    .ok_or_else(|| CoreError::three_mf("export face-color entry is unavailable"))?;
                let property_index = material_indices.get(color).ok_or_else(|| {
                    CoreError::three_mf("export face color has no material resource")
                })?;
                write_empty(
                    &mut writer,
                    "triangle",
                    &[
                        ("v1", face[0].to_string()),
                        ("v2", face[1].to_string()),
                        ("v3", face[2].to_string()),
                        ("pid", "1".to_owned()),
                        ("p1", property_index.to_string()),
                    ],
                )?;
            } else {
                write_empty(
                    &mut writer,
                    "triangle",
                    &[
                        ("v1", face[0].to_string()),
                        ("v2", face[1].to_string()),
                        ("v3", face[2].to_string()),
                    ],
                )?;
            }
        }
        write_end(&mut writer, "triangles")?;
        write_end(&mut writer, "mesh")?;
        write_end(&mut writer, "object")?;
    }
    write_end(&mut writer, "resources")?;
    write_start(&mut writer, "build", &[])?;
    for (index, item) in items.iter().enumerate() {
        let name = if item.name.is_empty() {
            format!("Object {}", index + 1)
        } else {
            item.name.clone()
        };
        write_empty(
            &mut writer,
            "item",
            &[
                ("objectid", (index + 2).to_string()),
                ("partnumber", name),
                (
                    "transform",
                    transform_text(Transform::scene_to_z_up_file().multiply(&item.transform)?)?,
                ),
            ],
        )?;
    }
    write_end(&mut writer, "build")?;
    write_end(&mut writer, "model")?;
    let bytes = writer.into_inner();
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.member_bytes {
        return Err(CoreError::three_mf(format!(
            "model XML exceeds the maximum size of {} bytes",
            limits.member_bytes
        )));
    }
    Ok(bytes)
}

fn content_types_xml(include_png: bool, include_jpeg: bool) -> Result<Vec<u8>, CoreError> {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(write_error)?;
    write_start(
        &mut writer,
        "Types",
        &[("xmlns", CONTENT_TYPES_NS.to_owned())],
    )?;
    write_empty(
        &mut writer,
        "Default",
        &[
            ("Extension", "rels".to_owned()),
            (
                "ContentType",
                "application/vnd.openxmlformats-package.relationships+xml".to_owned(),
            ),
        ],
    )?;
    write_empty(
        &mut writer,
        "Default",
        &[
            ("Extension", "model".to_owned()),
            (
                "ContentType",
                "application/vnd.ms-package.3dmanufacturing-3dmodel+xml".to_owned(),
            ),
        ],
    )?;
    if include_png {
        write_empty(
            &mut writer,
            "Default",
            &[
                ("Extension", "png".to_owned()),
                ("ContentType", "image/png".to_owned()),
            ],
        )?;
    }
    if include_jpeg {
        write_empty(
            &mut writer,
            "Default",
            &[
                ("Extension", "jpg".to_owned()),
                ("ContentType", "image/jpeg".to_owned()),
            ],
        )?;
    }
    write_end(&mut writer, "Types")?;
    Ok(writer.into_inner())
}

fn relationships_xml(relationships: &[(&str, &str, &str)]) -> Result<Vec<u8>, CoreError> {
    let mut writer = Writer::new(Vec::new());
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(write_error)?;
    write_start(
        &mut writer,
        "Relationships",
        &[("xmlns", RELATIONSHIPS_NS.to_owned())],
    )?;
    for (id, relationship_type, target) in relationships {
        write_empty(
            &mut writer,
            "Relationship",
            &[
                ("Id", (*id).to_owned()),
                ("Type", (*relationship_type).to_owned()),
                ("Target", (*target).to_owned()),
            ],
        )?;
    }
    write_end(&mut writer, "Relationships")?;
    Ok(writer.into_inner())
}

/// Exports scene instances to a Python-compatible 3MF package in memory.
pub fn export_3mf(
    items: &[ThreeMfItem],
    metadata: Option<&ThreeMfMetadata>,
    thumbnail_data: Option<&[u8]>,
) -> Result<Vec<u8>, CoreError> {
    let limits = ThreeMfLimits::default();
    if items.is_empty() {
        return Err(CoreError::three_mf("there are no objects to export to 3MF"));
    }
    if items.len() > limits.objects || items.len() > limits.build_items {
        return Err(CoreError::three_mf(format!(
            "3MF export contains more than {} objects",
            limits.objects
        )));
    }
    if let Some(thumbnail) = thumbnail_data
        && u64::try_from(thumbnail.len()).unwrap_or(u64::MAX) > limits.thumbnail_bytes
    {
        return Err(CoreError::three_mf(format!(
            "thumbnail exceeds the maximum size of {} bytes",
            limits.thumbnail_bytes
        )));
    }
    for item in items {
        item.mesh.validate_non_empty()?;
        validate_export_color(item.color)?;
        if let Some(face_colors) = &item.face_colors
            && face_colors.len() != item.mesh.face_count()
        {
            return Err(CoreError::three_mf(
                "export face-color count does not match face count",
            ));
        }
    }
    let model = model_xml(items, &merge_metadata(items, metadata), &limits)?;
    let textures = export_textures(items);
    let has_encoding = |encoding: crate::TextureEncoding| {
        textures
            .iter()
            .any(|(image, _)| image.encoding() == encoding)
    };
    let content_types = content_types_xml(
        thumbnail_data.is_some() || has_encoding(crate::TextureEncoding::Png),
        has_encoding(crate::TextureEncoding::Jpeg),
    )?;
    let root_relationships =
        relationships_xml(&[("rModel", MODEL_RELATIONSHIP_TYPE, "/3D/3dmodel.model")])?;
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    writer
        .start_file("[Content_Types].xml", options)
        .map_err(|error| {
            CoreError::three_mf(format!("could not start content-types part: {error}"))
        })?;
    writer.write_all(&content_types).map_err(|error| {
        CoreError::three_mf(format!("could not write content-types part: {error}"))
    })?;
    writer.start_file("_rels/.rels", options).map_err(|error| {
        CoreError::three_mf(format!("could not start root relationships: {error}"))
    })?;
    writer.write_all(&root_relationships).map_err(|error| {
        CoreError::three_mf(format!("could not write root relationships: {error}"))
    })?;
    writer
        .start_file("3D/3dmodel.model", options)
        .map_err(|error| CoreError::three_mf(format!("could not start model part: {error}")))?;
    writer
        .write_all(&model)
        .map_err(|error| CoreError::three_mf(format!("could not write model part: {error}")))?;
    let texture_relationship_ids = (0..textures.len())
        .map(|index| format!("rTexture{}", index + 1))
        .collect::<Vec<_>>();
    let mut model_relationship_entries = Vec::new();
    if thumbnail_data.is_some() {
        model_relationship_entries.push((
            "rThumbnail",
            THUMBNAIL_RELATIONSHIP_TYPE,
            "../Metadata/thumbnail.png",
        ));
    }
    for ((_, path), id) in textures.iter().zip(&texture_relationship_ids) {
        model_relationship_entries.push((id.as_str(), TEXTURE_RELATIONSHIP_TYPE, path.as_str()));
    }
    if !model_relationship_entries.is_empty() {
        let model_relationships = relationships_xml(&model_relationship_entries)?;
        writer
            .start_file("3D/_rels/3dmodel.model.rels", options)
            .map_err(|error| {
                CoreError::three_mf(format!("could not start model relationships: {error}"))
            })?;
        writer.write_all(&model_relationships).map_err(|error| {
            CoreError::three_mf(format!("could not write model relationships: {error}"))
        })?;
    }
    for (image, path) in &textures {
        writer
            .start_file(path.trim_start_matches('/'), options)
            .map_err(|error| CoreError::three_mf(format!("could not start texture: {error}")))?;
        writer
            .write_all(image.encoded())
            .map_err(|error| CoreError::three_mf(format!("could not write texture: {error}")))?;
    }
    if let Some(thumbnail) = thumbnail_data {
        writer
            .start_file("Metadata/thumbnail.png", options)
            .map_err(|error| CoreError::three_mf(format!("could not start thumbnail: {error}")))?;
        writer
            .write_all(thumbnail)
            .map_err(|error| CoreError::three_mf(format!("could not write thumbnail: {error}")))?;
    }
    let output = writer
        .finish()
        .map_err(|error| CoreError::three_mf(format!("could not finish 3MF package: {error}")))?
        .into_inner();
    if u64::try_from(output.len()).unwrap_or(u64::MAX) > limits.archive_bytes {
        return Err(CoreError::three_mf(format!(
            "exported archive exceeds the maximum size of {} bytes",
            limits.archive_bytes
        )));
    }
    Ok(output)
}

/// Exports scene instances to a 3MF path.
pub fn save_3mf(
    items: &[ThreeMfItem],
    path: impl AsRef<Path>,
    metadata: Option<&ThreeMfMetadata>,
    thumbnail_data: Option<&[u8]>,
) -> Result<(), CoreError> {
    let path = path.as_ref();
    let bytes = export_3mf(items, metadata, thumbnail_data)?;
    fs::write(path, bytes).map_err(|error| {
        CoreError::three_mf(format!(
            "could not write package {}: {error}",
            path.display()
        ))
    })
}
