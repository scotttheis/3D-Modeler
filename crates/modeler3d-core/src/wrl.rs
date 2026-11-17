//! Bounded VRML 2.0/WRL scene export.
//!
//! VRML has no physical unit declaration.  3D Modeler writes model coordinates
//! in millimetres and keeps each scene item in its own Transform/Shape pair so
//! placement, names, display colors, and transparency survive export.  WRL
//! import is provided by the bounded static scene reader.

use std::collections::HashSet;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{CoreError, DEFAULT_MESH_COLOR, Mesh, Transform};

/// The supported WRL path suffix.
pub const WRL_SUFFIX: &str = ".wrl";
/// Maximum scene objects written to one WRL document.
pub const MAX_WRL_OBJECTS: usize = 10_000;
/// Maximum vertices in one WRL object.
pub const MAX_WRL_VERTICES: usize = 5_000_000;
/// Maximum triangular faces in one WRL object.
pub const MAX_WRL_FACES: usize = 10_000_000;
/// Maximum encoded WRL document bytes.
pub const MAX_WRL_FILE_BYTES: u64 = 256 * 1024 * 1024;

/// Resource limits applied while writing a WRL document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrlLimits {
    /// Maximum object count.
    pub objects: usize,
    /// Maximum vertices per object.
    pub vertices: usize,
    /// Maximum faces per object.
    pub faces: usize,
    /// Maximum UTF-8 document bytes.
    pub file_bytes: u64,
}

impl Default for WrlLimits {
    fn default() -> Self {
        Self {
            objects: MAX_WRL_OBJECTS,
            vertices: MAX_WRL_VERTICES,
            faces: MAX_WRL_FACES,
            file_bytes: MAX_WRL_FILE_BYTES,
        }
    }
}

impl WrlLimits {
    fn validate(&self) -> Result<(), CoreError> {
        if self.objects == 0 || self.vertices == 0 || self.faces == 0 || self.file_bytes == 0 {
            return Err(CoreError::wrl("WRL resource limits must be positive"));
        }
        Ok(())
    }
}

/// One scene item accepted by the WRL exporter.
#[derive(Clone, Debug, PartialEq)]
pub struct WrlItem {
    /// Local indexed geometry in 3D Modeler millimetres.
    pub mesh: Mesh,
    /// Affine scene transform with millimetre translation.
    pub transform: Transform,
    /// Human-readable object name retained in a VRML comment and DEF name.
    pub name: String,
    /// Optional display RGBA color; absent uses the 3D Modeler default green.
    pub color: Option<[f32; 4]>,
}

impl WrlItem {
    /// Creates a validated WRL scene item.
    pub fn new(
        mesh: Mesh,
        transform: Transform,
        name: String,
        color: Option<[f32; 4]>,
    ) -> Result<Self, CoreError> {
        mesh.validate_non_empty()?;
        if name.trim().is_empty() {
            return Err(CoreError::wrl("scene item name must not be empty"));
        }
        if let Some(color) = color
            && color
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(CoreError::InvalidColor(
                "WRL RGBA values must be finite and between 0 and 1".to_owned(),
            ));
        }
        Transform::from_rows(transform.rows())
            .map_err(|error| CoreError::wrl(format!("invalid WRL transform: {error}")))?;
        Ok(Self {
            mesh,
            transform,
            name,
            color,
        })
    }
}

/// Result of a path-based WRL export.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WrlExportReport {
    /// The requested output path.
    pub path: String,
    /// Number of scene items written.
    pub objects_exported: usize,
}

struct TransformParts {
    translation: [f64; 3],
    axis: [f64; 3],
    angle: f64,
    scale: [f64; 3],
}

/// Exports scene items to a bounded VRML 2.0 UTF-8 document.
pub fn export_wrl(items: &[WrlItem]) -> Result<Vec<u8>, CoreError> {
    export_wrl_with_limits(items, &WrlLimits::default())
}

/// Exports scene items with explicit object, mesh, and byte limits.
pub fn export_wrl_with_limits(items: &[WrlItem], limits: &WrlLimits) -> Result<Vec<u8>, CoreError> {
    limits.validate()?;
    if items.is_empty() {
        return Err(CoreError::wrl("no visible objects to export"));
    }
    if items.len() > limits.objects {
        return Err(CoreError::wrl(format!(
            "WRL export exceeds the object limit of {}",
            limits.objects
        )));
    }
    let mut lines = Vec::new();
    lines
        .try_reserve(items.len().saturating_mul(40))
        .map_err(|_| CoreError::wrl("WRL output exceeds available memory"))?;
    lines.push("#VRML V2.0 utf8".to_owned());
    lines.push("# 3D Modeler coordinates are millimetres.".to_owned());
    lines.push("WorldInfo { title \"3D Modeler export\" }".to_owned());
    let mut used_names = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        validate_item(item, index, limits)?;
        let TransformParts {
            translation,
            axis,
            angle,
            scale,
        } = decompose_transform(
            item.transform,
            &format!("WRL object {}", index.saturating_add(1)),
        )?;
        let color = rounded_color(item.color);
        let def_name = safe_def_name(&item.name, index, &mut used_names);
        lines.push(format!("# Object name: {}", vrml_string(&item.name)));
        lines.push(format!("DEF {def_name} Transform {{"));
        lines.push(format!(
            "  translation {}",
            vector(&translation, "translation")?
        ));
        lines.push(format!(
            "  rotation {} {}",
            vector(&axis, "rotation axis")?,
            number(angle, "rotation angle")?
        ));
        lines.push(format!("  scale {}", vector(&scale, "scale")?));
        lines.extend([
            "  children [".to_owned(),
            "    Shape {".to_owned(),
            "      appearance Appearance {".to_owned(),
            "        material Material {".to_owned(),
            format!(
                "          diffuseColor {}",
                vector(&color[..3], "diffuse color")?
            ),
            format!(
                "          transparency {}",
                number(1.0 - color[3], "transparency")?
            ),
            "        }".to_owned(),
            "      }".to_owned(),
            "      geometry IndexedFaceSet {".to_owned(),
            "        solid TRUE".to_owned(),
            "        creaseAngle 0.785398163".to_owned(),
            "        coord Coordinate {".to_owned(),
            "          point [".to_owned(),
        ]);
        for vertex in &item.mesh.vertices {
            let values = [
                f64::from(vertex[0]),
                f64::from(vertex[1]),
                f64::from(vertex[2]),
            ];
            lines.push(format!("            {},", vector(&values, "vertex")?));
        }
        lines.extend([
            "          ]".to_owned(),
            "        }".to_owned(),
            "        coordIndex [".to_owned(),
        ]);
        for face in &item.mesh.faces {
            lines.push(format!(
                "          {}, {}, {}, -1,",
                face[0], face[1], face[2]
            ));
        }
        lines.extend([
            "        ]".to_owned(),
            "      }".to_owned(),
            "    }".to_owned(),
            "  ]".to_owned(),
            "}".to_owned(),
        ]);
    }
    let text = format!("{}\n", lines.join("\n"));
    let bytes = text.into_bytes();
    if u64::try_from(bytes.len()).is_ok_and(|size| size > limits.file_bytes) {
        return Err(CoreError::wrl(format!(
            "WRL export exceeds the maximum size of {} bytes",
            limits.file_bytes
        )));
    }
    Ok(bytes)
}

/// Writes a WRL document through a completed temporary file and replacement.
pub fn save_wrl(items: &[WrlItem], path: impl AsRef<Path>) -> Result<WrlExportReport, CoreError> {
    let path = path.as_ref();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| CoreError::UnsupportedMeshFormat(path.display().to_string()))?;
    if extension != WRL_SUFFIX.trim_start_matches('.') {
        return Err(CoreError::wrl("WRL export path must end in .wrl"));
    }
    let bytes = export_wrl(items)?;
    write_atomic(path, &bytes)?;
    Ok(WrlExportReport {
        path: path.display().to_string(),
        objects_exported: items.len(),
    })
}

/// Exports one mesh as a single identity-transformed WRL object.
pub fn export_wrl_mesh(mesh: &Mesh) -> Result<Vec<u8>, CoreError> {
    let item = WrlItem::new(
        mesh.clone(),
        Transform::identity(),
        "Exported".to_owned(),
        None,
    )?;
    export_wrl(std::slice::from_ref(&item))
}

/// Writes one mesh through the legacy single-object WRL helper boundary.
pub fn save_wrl_mesh(mesh: &Mesh, path: impl AsRef<Path>) -> Result<WrlExportReport, CoreError> {
    let item = WrlItem::new(
        mesh.clone(),
        Transform::identity(),
        "Exported".to_owned(),
        None,
    )?;
    save_wrl(std::slice::from_ref(&item), path)
}

fn validate_item(item: &WrlItem, index: usize, limits: &WrlLimits) -> Result<(), CoreError> {
    item.mesh.validate_non_empty().map_err(|error| {
        CoreError::wrl(format!("WRL object {} mesh is invalid: {error}", index + 1))
    })?;
    if item.mesh.vertex_count() > limits.vertices || item.mesh.face_count() > limits.faces {
        return Err(CoreError::wrl(format!(
            "WRL object {} exceeds the mesh complexity limit",
            index + 1
        )));
    }
    if item
        .mesh
        .vertices
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(CoreError::wrl(format!(
            "WRL object {} contains non-finite geometry",
            index + 1
        )));
    }
    if let Some(color) = item.color
        && color
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err(CoreError::wrl(format!(
            "WRL object {} colors must be between 0 and 1",
            index + 1
        )));
    }
    Ok(())
}

fn rounded_color(color: Option<[f32; 4]>) -> [f64; 4] {
    let source = match color {
        Some(color) => color,
        None => DEFAULT_MESH_COLOR,
    };
    source.map(|value| (f64::from(value) * 1_000_000.0).round() / 1_000_000.0)
}

fn safe_def_name(name: &str, index: usize, used: &mut HashSet<String>) -> String {
    let mut base = String::new();
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            base.push(character);
        } else {
            base.push('_');
        }
    }
    let base = base.trim_matches('_');
    let mut base = if base.is_empty() {
        format!("Object_{}", index.saturating_add(1))
    } else {
        base.to_owned()
    };
    if base
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        base = format!("Object_{base}");
    }
    let mut candidate = base.clone();
    let mut suffix = 2_usize;
    while used.contains(&candidate) {
        candidate = format!("{base}_{suffix}");
        suffix = suffix.saturating_add(1);
    }
    used.insert(candidate.clone());
    candidate
}

fn vrml_string(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\u{0a}' => output.push_str("\\n"),
            '\u{0d}' => output.push_str("\\r"),
            '\u{09}' => output.push_str("\\t"),
            character if character.is_ascii() && character.is_control() => {
                let _ = write!(output, "\\u{:04x}", character as u32);
            }
            character if character.is_ascii() => output.push(character),
            character => {
                let mut units = [0_u16; 2];
                for unit in character.encode_utf16(&mut units) {
                    let _ = write!(output, "\\u{:04x}", u32::from(*unit));
                }
            }
        }
    }
    output.push('"');
    output
}

fn vector(values: &[f64], context: &str) -> Result<String, CoreError> {
    let mut output = Vec::new();
    output
        .try_reserve(values.len())
        .map_err(|_| CoreError::wrl(format!("{context} exceeds available memory")))?;
    for value in values {
        output.push(number(*value, context)?);
    }
    Ok(output.join(" "))
}

fn number(value: f64, context: &str) -> Result<String, CoreError> {
    if !value.is_finite() {
        return Err(CoreError::wrl(format!(
            "WRL {context} contains a non-finite number"
        )));
    }
    if value.abs() < 1.0e-12 {
        return Ok("0".to_owned());
    }
    let absolute = value.abs();
    let mut normalized = absolute;
    let mut exponent = 0_i32;
    if normalized >= 10.0 {
        while normalized >= 10.0 {
            normalized /= 10.0;
            exponent = exponent.saturating_add(1);
        }
    } else if normalized < 1.0 {
        while normalized < 1.0 {
            normalized *= 10.0;
            exponent = exponent.saturating_sub(1);
        }
    }
    let mut output = if (-4..9).contains(&exponent) {
        let precision = usize::try_from((8_i32 - exponent).max(0))
            .map_err(|_| CoreError::wrl("WRL fixed-point precision is invalid"))?;
        format!("{value:.precision$}")
    } else {
        let scientific = format!("{value:.8e}");
        let (mantissa, raw_exponent) = scientific
            .split_once('e')
            .ok_or_else(|| CoreError::wrl("WRL scientific formatting failed"))?;
        let mantissa = trim_decimal(mantissa);
        let raw_exponent = raw_exponent
            .parse::<i32>()
            .map_err(|_| CoreError::wrl("WRL scientific exponent is invalid"))?;
        let exponent = if raw_exponent >= 0 {
            format!("+{raw_exponent:02}")
        } else {
            format!("-{:02}", raw_exponent.unsigned_abs())
        };
        format!("{mantissa}e{exponent}")
    };
    output = trim_decimal(&output);
    if output == "-0" {
        output = "0".to_owned();
    }
    Ok(output)
}

fn trim_decimal(value: &str) -> String {
    if let Some((mantissa, exponent)) = value.split_once('e') {
        return format!("{}e{exponent}", trim_decimal(mantissa));
    }
    let mut output = value.to_owned();
    while output.contains('.') && output.ends_with('0') {
        output.pop();
    }
    if output.ends_with('.') {
        output.pop();
    }
    output
}

fn decompose_transform(transform: Transform, context: &str) -> Result<TransformParts, CoreError> {
    let rows = transform.rows();
    let mut linear = [[0.0_f64; 3]; 3];
    for (row, values) in linear.iter_mut().enumerate() {
        for (column, value) in values.iter_mut().enumerate() {
            *value = f64::from(rows[row][column]);
        }
    }
    let mut scale = [0.0_f64; 3];
    for (column, scale_value) in scale.iter_mut().enumerate() {
        *scale_value = snap_integer(column_norm(&linear, column));
        if !scale_value.is_finite() || *scale_value <= 1.0e-12 {
            return Err(CoreError::wrl(format!(
                "{context} has a singular transform"
            )));
        }
    }
    let mut rotation = [[0.0_f64; 3]; 3];
    for (row, values) in rotation.iter_mut().enumerate() {
        for (column, value) in values.iter_mut().enumerate() {
            *value = snap_rotation_component(linear[row][column] / scale[column]);
        }
    }
    for left in 0..3 {
        for right in 0..3 {
            let dot = (0..3)
                .map(|row| rotation[row][left] * rotation[row][right])
                .sum::<f64>();
            let expected = if left == right { 1.0 } else { 0.0 };
            if !close(dot, expected, 2.0e-5, 2.0e-5) {
                return Err(CoreError::wrl(format!(
                    "{context} contains unsupported shear"
                )));
            }
        }
    }
    let mut determinant = determinant3(&rotation);
    if determinant < 0.0 {
        scale[0] *= -1.0;
        for row in &mut rotation {
            row[0] *= -1.0;
        }
        determinant = determinant3(&rotation);
    }
    for value in &mut scale {
        *value = snap_integer(*value);
    }
    if !close(determinant, 1.0, 2.0e-5, 2.0e-5) {
        return Err(CoreError::wrl(format!(
            "{context} contains an invalid rotation"
        )));
    }
    let (axis, angle) = axis_angle(rotation, context)?;
    let translation = [
        f64::from(rows[0][3]),
        f64::from(rows[1][3]),
        f64::from(rows[2][3]),
    ];
    Ok(TransformParts {
        translation,
        axis,
        angle,
        scale,
    })
}

fn column_norm(matrix: &[[f64; 3]; 3], column: usize) -> f64 {
    matrix
        .iter()
        .map(|row| row[column] * row[column])
        .sum::<f64>()
        .sqrt()
}

fn snap_integer(value: f64) -> f64 {
    let nearest = value.round();
    if (value - nearest).abs() <= 1.0e-6 {
        nearest
    } else {
        value
    }
}

fn snap_rotation_component(value: f64) -> f64 {
    if value.abs() <= 1.0e-7 {
        0.0
    } else if (value.abs() - 1.0).abs() <= 1.0e-7 {
        value.signum()
    } else {
        value
    }
}

fn determinant3(matrix: &[[f64; 3]; 3]) -> f64 {
    matrix[0][0] * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0])
}

fn close(actual: f64, expected: f64, absolute: f64, relative: f64) -> bool {
    (actual - expected).abs() <= absolute + relative * expected.abs()
}

fn axis_angle(rotation: [[f64; 3]; 3], context: &str) -> Result<([f64; 3], f64), CoreError> {
    let trace = rotation[0][0] + rotation[1][1] + rotation[2][2];
    let mut quaternion = [0.0_f64; 4];
    if trace > 0.0 {
        let scale = (trace + 1.0).sqrt() * 2.0;
        quaternion[0] = 0.25 * scale;
        quaternion[1] = (rotation[2][1] - rotation[1][2]) / scale;
        quaternion[2] = (rotation[0][2] - rotation[2][0]) / scale;
        quaternion[3] = (rotation[1][0] - rotation[0][1]) / scale;
    } else {
        let diagonal = [rotation[0][0], rotation[1][1], rotation[2][2]];
        let largest = if diagonal[0] >= diagonal[1] && diagonal[0] >= diagonal[2] {
            0
        } else if diagonal[1] >= diagonal[2] {
            1
        } else {
            2
        };
        match largest {
            0 => {
                let scale = (1.0 + rotation[0][0] - rotation[1][1] - rotation[2][2])
                    .max(0.0)
                    .sqrt()
                    * 2.0;
                quaternion[0] = (rotation[2][1] - rotation[1][2]) / scale;
                quaternion[1] = 0.25 * scale;
                quaternion[2] = (rotation[0][1] + rotation[1][0]) / scale;
                quaternion[3] = (rotation[0][2] + rotation[2][0]) / scale;
            }
            1 => {
                let scale = (1.0 - rotation[0][0] + rotation[1][1] - rotation[2][2])
                    .max(0.0)
                    .sqrt()
                    * 2.0;
                quaternion[0] = (rotation[0][2] - rotation[2][0]) / scale;
                quaternion[1] = (rotation[0][1] + rotation[1][0]) / scale;
                quaternion[2] = 0.25 * scale;
                quaternion[3] = (rotation[1][2] + rotation[2][1]) / scale;
            }
            _ => {
                let scale = (1.0 - rotation[0][0] - rotation[1][1] + rotation[2][2])
                    .max(0.0)
                    .sqrt()
                    * 2.0;
                quaternion[0] = (rotation[1][0] - rotation[0][1]) / scale;
                quaternion[1] = (rotation[0][2] + rotation[2][0]) / scale;
                quaternion[2] = (rotation[1][2] + rotation[2][1]) / scale;
                quaternion[3] = 0.25 * scale;
            }
        }
    }
    let norm = quaternion
        .iter()
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt();
    if !norm.is_finite() || norm <= 1.0e-12 {
        return Err(CoreError::wrl(format!("{context} has no usable rotation")));
    }
    for value in &mut quaternion {
        *value /= norm;
    }
    if quaternion[0] < 0.0 {
        for value in &mut quaternion {
            *value *= -1.0;
        }
    }
    let scalar = quaternion[0].clamp(-1.0, 1.0);
    let angle = 2.0 * scalar.acos();
    let sine = (angle * 0.5).sin();
    if sine.abs() <= 1.0e-8 {
        return Ok(([0.0, 0.0, 1.0], 0.0));
    }
    let axis = [
        quaternion[1] / sine,
        quaternion[2] / sine,
        quaternion[3] / sine,
    ];
    if axis.iter().any(|value| !value.is_finite()) {
        return Err(CoreError::wrl(format!("{context} has no usable rotation")));
    }
    Ok((axis, angle))
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
    let parent = path.parent().map_or(Path::new("."), |parent| parent);
    fs::create_dir_all(parent).map_err(|error| {
        CoreError::wrl(format!("could not create {}: {error}", parent.display()))
    })?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| CoreError::wrl("WRL output path has no valid file name"))?;
    let seed = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary =
        PathBuf::from(parent).join(format!(".{name}.{}.{}.tmp", std::process::id(), seed));
    let result = (|| -> Result<(), CoreError> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| {
                CoreError::wrl(format!("could not create temporary output: {error}"))
            })?;
        file.write_all(bytes).map_err(|error| {
            CoreError::wrl(format!("could not write temporary output: {error}"))
        })?;
        file.sync_all().map_err(|error| {
            CoreError::wrl(format!("could not flush temporary output: {error}"))
        })?;
        drop(file);
        match fs::rename(&temporary, path) {
            Ok(()) => Ok(()),
            Err(_error) if path.exists() => {
                fs::remove_file(path).map_err(|error| {
                    CoreError::wrl(format!("could not replace existing output: {error}"))
                })?;
                fs::rename(&temporary, path)
                    .map_err(|error| CoreError::wrl(format!("could not finalize output: {error}")))
            }
            Err(error) => Err(CoreError::wrl(format!(
                "could not finalize output: {error}"
            ))),
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
