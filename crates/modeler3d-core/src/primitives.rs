use std::collections::HashMap;
use std::f32::consts::PI;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::{CoreError, Mesh};

/// Millimetres used by the reference primitive catalog.
pub const DEFAULT_PRIMITIVE_SIZE_MM: f32 = 40.0;
/// Radius used by the reference primitive catalog.
pub const DEFAULT_PRIMITIVE_RADIUS_MM: f32 = DEFAULT_PRIMITIVE_SIZE_MM / 2.0;
const DEFAULT_CONE_SEGMENTS: usize = 192;
const DEFAULT_CYLINDER_SEGMENTS: usize = 119;
/// Minimum editable torus major radius.
pub const TORUS_RADIUS_MIN_MM: f32 = 0.5;
/// Maximum editable torus major radius.
pub const TORUS_RADIUS_MAX_MM: f32 = 1000.0;
/// Minimum editable torus tube radius.
pub const TORUS_TUBE_MIN_MM: f32 = 0.1;
/// Maximum editable torus tube radius.
pub const TORUS_TUBE_MAX_MM: f32 = 500.0;

/// The stable primitive names exposed by the core command surface.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PrimitiveKind {
    Cube,
    Cylinder,
    Cone,
    Sphere,
    Pyramid,
    Torus,
    Hexagon,
    Wedge,
    Tetrahedron,
}

impl PrimitiveKind {
    /// Parses the case-insensitive catalog spelling.
    pub fn parse(name: &str) -> Result<Self, CoreError> {
        match name.trim().to_ascii_lowercase().as_str() {
            "cube" => Ok(Self::Cube),
            "cylinder" => Ok(Self::Cylinder),
            "cone" => Ok(Self::Cone),
            "sphere" => Ok(Self::Sphere),
            "pyramid" => Ok(Self::Pyramid),
            "torus" => Ok(Self::Torus),
            "hexagon" => Ok(Self::Hexagon),
            "wedge" => Ok(Self::Wedge),
            "tetrahedron" => Ok(Self::Tetrahedron),
            other => Err(CoreError::InvalidPrimitive(format!(
                "unknown primitive: {other}"
            ))),
        }
    }

    /// Returns the stable lower-case catalog spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Cube => "cube",
            Self::Cylinder => "cylinder",
            Self::Cone => "cone",
            Self::Sphere => "sphere",
            Self::Pyramid => "pyramid",
            Self::Torus => "torus",
            Self::Hexagon => "hexagon",
            Self::Wedge => "wedge",
            Self::Tetrahedron => "tetrahedron",
        }
    }
}

/// Shared RGBA defaults for newly created catalog primitives.
#[must_use]
pub const fn primitive_default_color(kind: PrimitiveKind) -> [f32; 4] {
    match kind {
        PrimitiveKind::Cube => [0.25, 0.55, 0.95, 1.0],
        PrimitiveKind::Cylinder => [0.95, 0.55, 0.20, 1.0],
        PrimitiveKind::Cone => [0.95, 0.75, 0.20, 1.0],
        PrimitiveKind::Sphere => [0.65, 0.35, 0.85, 1.0],
        PrimitiveKind::Pyramid => [0.85, 0.35, 0.55, 1.0],
        PrimitiveKind::Torus => [0.15, 0.70, 0.65, 1.0],
        PrimitiveKind::Hexagon => [0.25, 0.75, 0.40, 1.0],
        PrimitiveKind::Wedge => [0.90, 0.35, 0.30, 1.0],
        PrimitiveKind::Tetrahedron => [0.55, 0.40, 0.90, 1.0],
    }
}

/// Build one primitive, optionally applying the UI's width/depth/height
/// dimensions.  All dimensions and radii are in millimetres.
pub fn make_primitive(
    kind: PrimitiveKind,
    dimensions_mm: Option<[f32; 3]>,
) -> Result<Mesh, CoreError> {
    if let Some(dimensions) = dimensions_mm {
        validate_dimensions(dimensions)?;
        let [width, depth, height] = dimensions;
        match kind {
            PrimitiveKind::Cube => make_box(dimensions),
            PrimitiveKind::Wedge => make_wedge(width, depth, height),
            PrimitiveKind::Cylinder => {
                make_cylinder(width.min(depth) / 2.0, height, DEFAULT_CYLINDER_SEGMENTS)
            }
            PrimitiveKind::Cone => make_cone(width.min(depth) / 2.0, height, DEFAULT_CONE_SEGMENTS),
            PrimitiveKind::Hexagon => make_hexagon(width.min(depth) / 2.0, height),
            PrimitiveKind::Pyramid => make_pyramid(width.min(depth) / 2.0, height),
            PrimitiveKind::Sphere => make_sphere(width.min(depth).min(height) / 2.0, 4),
            PrimitiveKind::Torus => make_torus(width.min(depth) / 2.0, height / 4.0, 100, 100),
            PrimitiveKind::Tetrahedron => make_tetrahedron(width.min(depth).min(height) / 2.0),
        }
    } else {
        match kind {
            PrimitiveKind::Cube => make_box([DEFAULT_PRIMITIVE_SIZE_MM; 3]),
            PrimitiveKind::Cylinder => make_cylinder(
                DEFAULT_PRIMITIVE_RADIUS_MM,
                DEFAULT_PRIMITIVE_SIZE_MM,
                DEFAULT_CYLINDER_SEGMENTS,
            ),
            PrimitiveKind::Cone => make_cone(
                DEFAULT_PRIMITIVE_RADIUS_MM,
                DEFAULT_PRIMITIVE_SIZE_MM,
                DEFAULT_CONE_SEGMENTS,
            ),
            PrimitiveKind::Sphere => make_sphere(DEFAULT_PRIMITIVE_RADIUS_MM, 4),
            PrimitiveKind::Pyramid => {
                make_pyramid(DEFAULT_PRIMITIVE_RADIUS_MM, DEFAULT_PRIMITIVE_SIZE_MM)
            }
            PrimitiveKind::Torus => make_torus(25.0, 10.0, 100, 100),
            PrimitiveKind::Hexagon => make_hexagon(28.7935, 15.48893),
            PrimitiveKind::Wedge => make_wedge(40.002, 40.002, 19.998),
            PrimitiveKind::Tetrahedron => make_tetrahedron(DEFAULT_PRIMITIVE_RADIUS_MM),
        }
    }
}

/// Build a centered box with width, depth, and height in millimetres.
pub fn make_box(extents_mm: [f32; 3]) -> Result<Mesh, CoreError> {
    validate_dimensions(extents_mm)?;
    let [width, depth, height] = extents_mm;
    let [x, y, z] = [width / 2.0, depth / 2.0, height / 2.0];
    let vertices = vec![
        [-x, -y, -z],
        [-x, -y, z],
        [-x, y, -z],
        [-x, y, z],
        [x, -y, -z],
        [x, -y, z],
        [x, y, -z],
        [x, y, z],
    ];
    // This ordering matches trimesh's box face order and therefore its binary
    // STL triangle order for ordinary project round trips.
    let faces = vec![
        [1, 3, 0],
        [4, 1, 0],
        [0, 3, 2],
        [2, 4, 0],
        [1, 7, 3],
        [5, 1, 4],
        [5, 7, 1],
        [3, 7, 2],
        [6, 4, 2],
        [2, 7, 6],
        [6, 5, 4],
        [7, 5, 6],
    ];
    Mesh::new(vertices, faces)
}

/// Build an icosphere with the same 20·4^n triangle growth as the reference.
pub fn make_sphere(radius_mm: f32, subdivisions: u32) -> Result<Mesh, CoreError> {
    validate_positive(radius_mm, "sphere radius")?;
    if subdivisions > 6 {
        return Err(CoreError::InvalidPrimitive(
            "sphere subdivisions must be at most 6".to_owned(),
        ));
    }
    let golden = (1.0 + 5.0_f32.sqrt()) / 2.0;
    let raw_vertices = vec![
        [-1.0, golden, 0.0],
        [1.0, golden, 0.0],
        [-1.0, -golden, 0.0],
        [1.0, -golden, 0.0],
        [0.0, -1.0, golden],
        [0.0, 1.0, golden],
        [0.0, -1.0, -golden],
        [0.0, 1.0, -golden],
        [golden, 0.0, -1.0],
        [golden, 0.0, 1.0],
        [-golden, 0.0, -1.0],
        [-golden, 0.0, 1.0],
    ];
    let mut vertices = raw_vertices
        .into_iter()
        .map(|vertex| normalize(vertex, radius_mm))
        .collect::<Vec<_>>();
    let mut faces = vec![
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ];
    for _ in 0..subdivisions {
        let mut midpoint_cache = HashMap::new();
        let mut next_faces = Vec::with_capacity(faces.len().saturating_mul(4));
        for face in faces {
            let ab = midpoint(
                &mut vertices,
                &mut midpoint_cache,
                face[0],
                face[1],
                radius_mm,
            )?;
            let bc = midpoint(
                &mut vertices,
                &mut midpoint_cache,
                face[1],
                face[2],
                radius_mm,
            )?;
            let ca = midpoint(
                &mut vertices,
                &mut midpoint_cache,
                face[2],
                face[0],
                radius_mm,
            )?;
            next_faces.extend_from_slice(&[
                [face[0], ab, ca],
                [face[1], bc, ab],
                [face[2], ca, bc],
                [ab, bc, ca],
            ]);
        }
        faces = next_faces;
    }
    Mesh::new(vertices, faces)
}

/// Build a centered cylinder with a configurable radial section count.
pub fn make_cylinder(radius_mm: f32, height_mm: f32, segments: usize) -> Result<Mesh, CoreError> {
    validate_positive(radius_mm, "cylinder radius")?;
    validate_positive(height_mm, "cylinder height")?;
    validate_segments(segments)?;
    let mut vertices = ring_vertices(radius_mm, height_mm, segments, true);
    let bottom_center = u32::try_from(vertices.len())
        .map_err(|_| CoreError::InvalidPrimitive("cylinder is too large".to_owned()))?;
    vertices.push([0.0, -height_mm / 2.0, 0.0]);
    let top_center = u32::try_from(vertices.len())
        .map_err(|_| CoreError::InvalidPrimitive("cylinder is too large".to_owned()))?;
    vertices.push([0.0, height_mm / 2.0, 0.0]);
    let mut faces = Vec::with_capacity(segments.saturating_mul(4));
    for index in 0..segments {
        let next = (index + 1) % segments;
        let bottom = index as u32;
        let bottom_next = next as u32;
        let top = (segments + index) as u32;
        let top_next = (segments + next) as u32;
        faces.extend_from_slice(&[
            [bottom, top_next, bottom_next],
            [bottom, top, top_next],
            [bottom_center, bottom, bottom_next],
            [top_center, top_next, top],
        ]);
    }
    Mesh::new(vertices, faces)
}

/// Build a cone whose base is at y=0 and apex is at y=height, matching the
/// reference's non-centered cone placement.
pub fn make_cone(radius_mm: f32, height_mm: f32, segments: usize) -> Result<Mesh, CoreError> {
    validate_positive(radius_mm, "cone radius")?;
    validate_positive(height_mm, "cone height")?;
    validate_segments(segments)?;
    let mut vertices = ring_vertices_at_y(radius_mm, 0.0, segments);
    let apex = u32::try_from(vertices.len())
        .map_err(|_| CoreError::InvalidPrimitive("cone is too large".to_owned()))?;
    vertices.push([0.0, height_mm, 0.0]);
    let center = u32::try_from(vertices.len())
        .map_err(|_| CoreError::InvalidPrimitive("cone is too large".to_owned()))?;
    vertices.push([0.0, 0.0, 0.0]);
    let mut faces = Vec::with_capacity(segments.saturating_mul(2));
    for index in 0..segments {
        let next = (index + 1) % segments;
        let current = index as u32;
        let next_vertex = next as u32;
        faces.push([current, apex, next_vertex]);
        faces.push([center, current, next_vertex]);
    }
    Mesh::new(vertices, faces)
}

/// Build the four-sided pyramid used by the reference catalog.
pub fn make_pyramid(radius_mm: f32, height_mm: f32) -> Result<Mesh, CoreError> {
    validate_positive(radius_mm, "pyramid radius")?;
    validate_positive(height_mm, "pyramid height")?;
    let vertices = vec![
        [-radius_mm, -height_mm / 2.0, -radius_mm],
        [radius_mm, -height_mm / 2.0, -radius_mm],
        [radius_mm, -height_mm / 2.0, radius_mm],
        [-radius_mm, -height_mm / 2.0, radius_mm],
        [0.0, height_mm / 2.0, 0.0],
    ];
    let faces = vec![
        [0, 1, 2],
        [0, 2, 3],
        [0, 4, 1],
        [1, 4, 2],
        [2, 4, 3],
        [3, 4, 0],
    ];
    Mesh::new(vertices, faces)
}

/// Build a torus with major and minor sections.
pub fn make_torus(
    major_radius_mm: f32,
    minor_radius_mm: f32,
    major_sections: usize,
    minor_sections: usize,
) -> Result<Mesh, CoreError> {
    validate_positive(major_radius_mm, "torus major radius")?;
    validate_positive(minor_radius_mm, "torus minor radius")?;
    if minor_radius_mm >= major_radius_mm {
        return Err(CoreError::InvalidPrimitive(
            "torus minor radius must be smaller than major radius".to_owned(),
        ));
    }
    validate_segments(major_sections)?;
    validate_segments(minor_sections)?;
    let vertex_count = major_sections
        .checked_mul(minor_sections)
        .ok_or_else(|| CoreError::InvalidPrimitive("torus vertex count overflow".to_owned()))?;
    let mut vertices = Vec::with_capacity(vertex_count);
    for major in 0..major_sections {
        let u = 2.0 * PI * major as f32 / major_sections as f32;
        let (u_sine, u_cosine) = u.sin_cos();
        for minor in 0..minor_sections {
            let v = 2.0 * PI * minor as f32 / minor_sections as f32;
            let radial = major_radius_mm + minor_radius_mm * v.cos();
            vertices.push([
                radial * u_cosine,
                minor_radius_mm * v.sin(),
                radial * u_sine,
            ]);
        }
    }
    let face_count = major_sections
        .checked_mul(minor_sections)
        .and_then(|count| count.checked_mul(2))
        .ok_or_else(|| CoreError::InvalidPrimitive("torus face count overflow".to_owned()))?;
    let mut faces = Vec::with_capacity(face_count);
    for major in 0..major_sections {
        let next_major = (major + 1) % major_sections;
        for minor in 0..minor_sections {
            let next_minor = (minor + 1) % minor_sections;
            let current = (major * minor_sections + minor) as u32;
            let next_minor_vertex = (major * minor_sections + next_minor) as u32;
            let next_major_vertex = (next_major * minor_sections + minor) as u32;
            let diagonal = (next_major * minor_sections + next_minor) as u32;
            faces.extend_from_slice(&[
                [current, diagonal, next_major_vertex],
                [current, next_minor_vertex, diagonal],
            ]);
        }
    }
    Mesh::new(vertices, faces)
}

/// Build the six-sided prism used by the catalog.
pub fn make_hexagon(radius_mm: f32, height_mm: f32) -> Result<Mesh, CoreError> {
    validate_positive(radius_mm, "hexagon radius")?;
    validate_positive(height_mm, "hexagon height")?;
    let z_radius = 50.0 / 3.0_f32.sqrt();
    let mut vertices = Vec::with_capacity(12);
    for i in 0..6 {
        let angle = i as f32 * PI / 3.0;
        vertices.push([
            radius_mm * angle.cos(),
            -height_mm / 2.0,
            z_radius * angle.sin(),
        ]);
    }
    for i in 0..6 {
        let angle = i as f32 * PI / 3.0;
        vertices.push([
            radius_mm * angle.cos(),
            height_mm / 2.0,
            z_radius * angle.sin(),
        ]);
    }
    let mut faces = Vec::with_capacity(20);
    for i in 1..5 {
        faces.extend_from_slice(&[
            [0, i as u32, (i + 1) as u32],
            [6, (6 + i + 1) as u32, (6 + i) as u32],
        ]);
    }
    for i in 0..6 {
        let next = (i + 1) % 6;
        faces.extend_from_slice(&[
            [i as u32, (6 + next) as u32, next as u32],
            [i as u32, (6 + i) as u32, (6 + next) as u32],
        ]);
    }
    Mesh::new(vertices, faces)
}

/// Build the centered triangular wedge used by the catalog.
pub fn make_wedge(width_mm: f32, depth_mm: f32, height_mm: f32) -> Result<Mesh, CoreError> {
    validate_dimensions([width_mm, depth_mm, height_mm])?;
    let [width, depth, height] = [width_mm / 2.0, depth_mm / 2.0, height_mm / 2.0];
    let vertices = vec![
        [-width, -height, -depth],
        [width, -height, -depth],
        [-width, height, -depth],
        [-width, -height, depth],
        [width, -height, depth],
        [-width, height, depth],
    ];
    // The faces are the reference's outward-wound, inverted ordering.
    let faces = vec![
        [2, 1, 0],
        [4, 5, 3],
        [4, 3, 0],
        [1, 4, 0],
        [5, 4, 1],
        [2, 5, 1],
        [5, 2, 0],
        [3, 5, 0],
    ];
    Mesh::new(vertices, faces)
}

/// Build the four-face tetrahedron used by the catalog.
pub fn make_tetrahedron(radius_mm: f32) -> Result<Mesh, CoreError> {
    validate_positive(radius_mm, "tetrahedron radius")?;
    let scale = radius_mm / DEFAULT_PRIMITIVE_RADIUS_MM;
    let width = 42.859117 * scale;
    let depth = 49.486747 * scale;
    let height = 40.000183 * scale;
    let vertices = vec![
        [-width / 2.0, -height / 2.0, -depth / 2.0],
        [-width / 2.0, -height / 2.0, depth / 2.0],
        [width / 2.0, -height / 2.0, 0.0],
        [0.0, height / 2.0, 0.0],
    ];
    Mesh::new(vertices, vec![[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]])
}

/// Bounded editable torus parameters retained in object metadata.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TorusParameters {
    /// Solid or hole mode retained for the future boolean-aware editor.
    pub mode: String,
    /// Major radius in millimetres.
    pub radius_mm: f32,
    /// Tube radius in millimetres.
    pub tube_mm: f32,
    /// Major section count.
    pub sides: usize,
    /// Minor section count.
    pub steps: usize,
}

impl Default for TorusParameters {
    fn default() -> Self {
        Self {
            mode: "solid".to_owned(),
            radius_mm: 7.5,
            tube_mm: 2.5,
            sides: 24,
            steps: 48,
        }
    }
}

/// Validates and canonicalizes bounded torus metadata.
pub fn normalize_torus_parameters(
    mut parameters: TorusParameters,
) -> Result<TorusParameters, CoreError> {
    parameters.mode = parameters.mode.trim().to_ascii_lowercase();
    if parameters.mode != "solid" && parameters.mode != "hole" {
        return Err(CoreError::InvalidPrimitive(
            "torus mode must be solid or hole".to_owned(),
        ));
    }
    if !parameters.radius_mm.is_finite()
        || !(TORUS_RADIUS_MIN_MM..=TORUS_RADIUS_MAX_MM).contains(&parameters.radius_mm)
    {
        return Err(CoreError::InvalidPrimitive(format!(
            "torus radius must be between {TORUS_RADIUS_MIN_MM} and {TORUS_RADIUS_MAX_MM} mm"
        )));
    }
    if !parameters.tube_mm.is_finite()
        || !(TORUS_TUBE_MIN_MM..=TORUS_TUBE_MAX_MM).contains(&parameters.tube_mm)
    {
        return Err(CoreError::InvalidPrimitive(format!(
            "torus tube must be between {TORUS_TUBE_MIN_MM} and {TORUS_TUBE_MAX_MM} mm"
        )));
    }
    if parameters.tube_mm >= parameters.radius_mm {
        return Err(CoreError::InvalidPrimitive(
            "torus tube must be smaller than radius".to_owned(),
        ));
    }
    validate_segments(parameters.sides)?;
    validate_segments(parameters.steps)?;
    Ok(parameters)
}

/// Build the bounded editable torus and return its canonical parameters.
pub fn build_parametric_torus(
    parameters: TorusParameters,
) -> Result<(Mesh, TorusParameters), CoreError> {
    let parameters = normalize_torus_parameters(parameters)?;
    let mesh = make_torus(
        parameters.radius_mm,
        parameters.tube_mm,
        parameters.sides,
        parameters.steps,
    )?;
    Ok((mesh, parameters))
}

/// Add durable torus metadata to an object metadata object.
pub fn torus_metadata(
    parameters: TorusParameters,
    base: Option<Value>,
) -> Result<Value, CoreError> {
    let parameters = normalize_torus_parameters(parameters)?;
    let mut metadata = match base {
        Some(Value::Object(value)) => value,
        Some(_) => {
            return Err(CoreError::InvalidObject(
                "object metadata must be a JSON object".to_owned(),
            ));
        }
        None => Map::new(),
    };
    metadata.insert("primitive".to_owned(), Value::String("torus".to_owned()));
    metadata.insert(
        "torus_parameters".to_owned(),
        serde_json::to_value(parameters).map_err(CoreError::json)?,
    );
    Ok(Value::Object(metadata))
}

/// Read and validate editable torus metadata, if present.
pub fn torus_parameters_from_metadata(
    metadata: &Value,
) -> Result<Option<TorusParameters>, CoreError> {
    let Some(parameters) = metadata.get("torus_parameters") else {
        return Ok(None);
    };
    let parsed: TorusParameters =
        serde_json::from_value(parameters.clone()).map_err(CoreError::json)?;
    normalize_torus_parameters(parsed).map(Some)
}

fn midpoint(
    vertices: &mut Vec<[f32; 3]>,
    cache: &mut HashMap<(u32, u32), u32>,
    first: u32,
    second: u32,
    radius: f32,
) -> Result<u32, CoreError> {
    let key = if first < second {
        (first, second)
    } else {
        (second, first)
    };
    if let Some(index) = cache.get(&key) {
        return Ok(*index);
    }
    let first_vertex = *vertices.get(first as usize).ok_or_else(|| {
        CoreError::InvalidPrimitive("sphere subdivision referenced a missing vertex".to_owned())
    })?;
    let second_vertex = *vertices.get(second as usize).ok_or_else(|| {
        CoreError::InvalidPrimitive("sphere subdivision referenced a missing vertex".to_owned())
    })?;
    let index = u32::try_from(vertices.len())
        .map_err(|_| CoreError::InvalidPrimitive("sphere has too many vertices".to_owned()))?;
    vertices.push(normalize(
        [
            (first_vertex[0] + second_vertex[0]) / 2.0,
            (first_vertex[1] + second_vertex[1]) / 2.0,
            (first_vertex[2] + second_vertex[2]) / 2.0,
        ],
        radius,
    ));
    cache.insert(key, index);
    Ok(index)
}

fn normalize(vector: [f32; 3], radius: f32) -> [f32; 3] {
    let length = (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt();
    [
        vector[0] * radius / length,
        vector[1] * radius / length,
        vector[2] * radius / length,
    ]
}

fn ring_vertices(radius: f32, height: f32, segments: usize, centered: bool) -> Vec<[f32; 3]> {
    let bottom_y = if centered { -height / 2.0 } else { 0.0 };
    let top_y = if centered { height / 2.0 } else { height };
    let mut vertices = ring_vertices_at_y(radius, bottom_y, segments);
    vertices.extend(ring_vertices_at_y(radius, top_y, segments));
    fit_radial_bounds(&mut vertices, radius);
    vertices
}

fn ring_vertices_at_y(radius: f32, y: f32, segments: usize) -> Vec<[f32; 3]> {
    (0..segments)
        .map(|index| {
            let angle = 2.0 * PI * index as f32 / segments as f32;
            [radius * angle.cos(), y, radius * angle.sin()]
        })
        .collect()
}

fn fit_radial_bounds(vertices: &mut [[f32; 3]], radius: f32) {
    for axis in [0_usize, 2_usize] {
        let mut minimum = f32::INFINITY;
        let mut maximum = f32::NEG_INFINITY;
        for vertex in vertices.iter() {
            minimum = minimum.min(vertex[axis]);
            maximum = maximum.max(vertex[axis]);
        }
        let center = (minimum + maximum) / 2.0;
        let span = maximum - minimum;
        if span > 0.0 {
            for vertex in vertices.iter_mut() {
                vertex[axis] = (vertex[axis] - center) * (2.0 * radius / span);
            }
        }
    }
}

fn validate_dimensions(dimensions: [f32; 3]) -> Result<(), CoreError> {
    if dimensions
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(CoreError::InvalidPrimitive(
            "dimensions_mm must contain three positive finite numbers".to_owned(),
        ));
    }
    Ok(())
}

fn validate_positive(value: f32, label: &str) -> Result<(), CoreError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(CoreError::InvalidPrimitive(format!(
            "{label} must be positive and finite"
        )));
    }
    Ok(())
}

fn validate_segments(segments: usize) -> Result<(), CoreError> {
    if !(3..=256).contains(&segments) {
        return Err(CoreError::InvalidPrimitive(
            "segment counts must be between 3 and 256".to_owned(),
        ));
    }
    Ok(())
}

/// Serializable helper for callers that want the catalog's default metadata.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrimitiveMetadata {
    /// The stable primitive name.
    pub primitive: String,
}

impl PrimitiveMetadata {
    /// Creates metadata for one primitive kind.
    #[must_use]
    pub fn for_kind(kind: PrimitiveKind) -> Self {
        Self {
            primitive: kind.name().to_owned(),
        }
    }

    /// Converts the helper to a JSON metadata object.
    #[must_use]
    pub fn to_value(&self) -> Value {
        json!({ "primitive": self.primitive })
    }
}
