//! Text relief and constant-thickness shell operations.
//!
//! These operations deliberately keep their raster and voxel paths bounded.
//! The Python reference uses the same distinction: text is rasterized into a
//! closed relief, convex shells use an exact parallel-plane offset, and
//! concave shells use a conservative voxel offset.

use std::{collections::HashMap, fs, path::Path};

use fontdue::{Font, FontSettings};
use manifold_csg::Manifold;

use crate::mesh_ops::{manifold_from_mesh, mesh_from_manifold};
use crate::{CoreError, Mesh, MeshOperationLimits, inspect_mesh};

/// The smallest accepted raster font size for text embossing.
pub const EMBOSS_FONT_SIZE_MIN: u32 = 1;
/// The largest accepted raster font size for text embossing.
pub const EMBOSS_FONT_SIZE_MAX: u32 = 200;
/// The largest accepted text length, measured in Unicode scalar values.
pub const EMBOSS_MAX_TEXT_LENGTH: usize = 256;
/// The largest accepted relief depth in millimetres.
pub const EMBOSS_DEPTH_MAX_MM: f32 = 50.0;
/// The largest absolute text placement offset in millimetres.
pub const EMBOSS_OFFSET_MAX_MM: f32 = 10_000.0;
/// The maximum voxel cells along one source-model axis for concave shells.
pub const HOLLOW_MAX_AXIS_CELLS: usize = 160;
/// The maximum number of voxels allocated for a concave shell.
pub const HOLLOW_MAX_VOXELS: usize = 2_500_000;

const EMBOSS_CELL_MM: f64 = 0.5;
const EMBOSS_OVERLAP_MAX_MM: f64 = 0.05;
const OP_EMBOSS: &str = "emboss_text";
const OP_HOLLOW: &str = "hollow";

/// The supported named text fonts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbossFont {
    /// Use the portable built-in default glyph set.
    Default,
    /// Use Arial or a portable sans-serif substitute.
    Sans,
    /// Use Georgia or a portable serif substitute.
    Serif,
    /// Use Consolas or a portable monospace substitute.
    Mono,
}

impl EmbossFont {
    /// Parses the stable MCP/UI font name.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "default" => Some(Self::Default),
            "sans" => Some(Self::Sans),
            "serif" => Some(Self::Serif),
            "mono" => Some(Self::Mono),
            _ => None,
        }
    }

    /// Returns the stable serialized font name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Sans => "sans",
            Self::Serif => "serif",
            Self::Mono => "mono",
        }
    }
}

/// Validated text relief parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct TextEmbossOptions {
    /// Text to rasterize.
    pub text: String,
    /// Raster font size in pixels.
    pub font_size: u32,
    /// Relief height in millimetres.
    pub depth_mm: f32,
    /// Whether the relief is added to the top surface instead of engraved.
    pub raised: bool,
    /// Named font selection.
    pub font: EmbossFont,
    /// Horizontal placement offset in millimetres.
    pub offset_x_mm: f32,
    /// Placement offset along the target's local Z axis in millimetres.
    pub offset_z_mm: f32,
    /// Isolated operation resource policy.
    pub limits: MeshOperationLimits,
}

impl Default for TextEmbossOptions {
    fn default() -> Self {
        Self {
            text: String::new(),
            font_size: 10,
            depth_mm: 1.0,
            raised: true,
            font: EmbossFont::Default,
            offset_x_mm: 0.0,
            offset_z_mm: 0.0,
            limits: MeshOperationLimits::default(),
        }
    }
}

/// Validated constant-thickness shell parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HollowOptions {
    /// Desired wall thickness in millimetres.
    pub wall_thickness_mm: f32,
    /// Isolated operation resource policy.
    pub limits: MeshOperationLimits,
}

impl Default for HollowOptions {
    fn default() -> Self {
        Self {
            // OF-61: 2 mm, the wall the owner found works on normal models.
            wall_thickness_mm: 2.0,
            limits: MeshOperationLimits::default(),
        }
    }
}

/// Details of a completed text relief.
#[derive(Clone, Debug, PartialEq)]
pub struct TextEmbossReport {
    /// Source vertex count.
    pub source_vertices: usize,
    /// Source triangle count.
    pub source_faces: usize,
    /// Result vertex count.
    pub result_vertices: usize,
    /// Result triangle count.
    pub result_faces: usize,
    /// Absolute source volume in cubic millimetres.
    pub source_volume_mm3: f64,
    /// Absolute result volume in cubic millimetres.
    pub result_volume_mm3: f64,
    /// Serialized font selection.
    pub font: String,
    /// Raster width in cells.
    pub raster_width: usize,
    /// Raster height in cells.
    pub raster_height: usize,
    /// Number of non-zero raster cells used as relief boxes.
    pub occupied_cells: usize,
    /// Applied relief depth.
    pub depth_mm: f32,
    /// Whether material was added rather than removed.
    pub raised: bool,
}

/// Mesh and report returned by text embossing.
#[derive(Clone, Debug)]
pub struct TextEmbossResult {
    /// Replaced mesh.
    pub mesh: Mesh,
    /// Operation report.
    pub report: TextEmbossReport,
}

/// Details of a completed shell operation.
#[derive(Clone, Debug, PartialEq)]
pub struct HollowReport {
    /// Source vertex count.
    pub source_vertices: usize,
    /// Source triangle count.
    pub source_faces: usize,
    /// Result vertex count.
    pub result_vertices: usize,
    /// Result triangle count.
    pub result_faces: usize,
    /// Absolute source volume in cubic millimetres.
    pub source_volume_mm3: f64,
    /// Absolute result volume in cubic millimetres.
    pub result_volume_mm3: f64,
    /// Whether the exact convex or bounded voxel path was used.
    pub branch: String,
    /// Requested wall thickness.
    pub wall_thickness_mm: f32,
    /// Absolute volume of the removed inner solid.
    pub inner_volume_mm3: f64,
}

/// Mesh and report returned by hollowing.
#[derive(Clone, Debug)]
pub struct HollowResult {
    /// Replaced mesh.
    pub mesh: Mesh,
    /// Operation report.
    pub report: HollowReport,
}

struct Raster {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

#[derive(Clone, Copy)]
struct Plane {
    normal: [f64; 3],
    offset: f64,
}

struct Triangle {
    a: [f64; 3],
    b: [f64; 3],
    c: [f64; 3],
}

/// Applies a bounded rasterized text relief to a closed source solid.
pub fn text_emboss(mesh: &Mesh, options: TextEmbossOptions) -> Result<TextEmbossResult, CoreError> {
    text_emboss_with_hooks(mesh, options, None)
}

/// Applies text embossing while allowing an adapter to cancel between bounded
/// geometry phases.
pub fn text_emboss_with_hooks(
    mesh: &Mesh,
    options: TextEmbossOptions,
    mut cancel: Option<&mut dyn FnMut() -> Result<(), CoreError>>,
) -> Result<TextEmbossResult, CoreError> {
    check_cancel(&mut cancel, OP_EMBOSS)?;
    let (minimum, maximum) = checked_solid_input(mesh, OP_EMBOSS, &options.limits)?;
    validate_emboss_options(&options)?;

    let raster = render_text(&options)?;
    check_cancel(&mut cancel, OP_EMBOSS)?;
    let cell_count = raster.width.checked_mul(raster.height).ok_or_else(|| {
        CoreError::OperationResourceLimit {
            operation: OP_EMBOSS.to_owned(),
            reason: "text raster dimensions overflowed".to_owned(),
        }
    })?;
    let occupied_cells = raster.pixels.iter().filter(|value| **value > 0).count();
    if occupied_cells == 0 {
        return Err(CoreError::MeshOperation(
            "text raster contains no visible glyph pixels".to_owned(),
        ));
    }
    let box_work = u64::try_from(occupied_cells)
        .ok()
        .and_then(|count| count.checked_mul(12))
        .ok_or_else(|| CoreError::OperationResourceLimit {
            operation: OP_EMBOSS.to_owned(),
            reason: "text relief work count overflowed".to_owned(),
        })?;
    options.limits.check_work(OP_EMBOSS, box_work)?;
    let max_boxes = options.limits.max_faces / 12;
    if occupied_cells > max_boxes {
        return Err(CoreError::OperationResourceLimit {
            operation: OP_EMBOSS.to_owned(),
            reason: format!(
                "text relief needs {occupied_cells} boxes, above the {max_boxes} box policy"
            ),
        });
    }
    let raster_bytes =
        u64::try_from(cell_count).map_err(|_| CoreError::OperationResourceLimit {
            operation: OP_EMBOSS.to_owned(),
            reason: "text raster size exceeds the accounting range".to_owned(),
        })?;
    if raster_bytes > options.limits.max_memory_bytes {
        return Err(CoreError::OperationResourceLimit {
            operation: OP_EMBOSS.to_owned(),
            reason: format!(
                "text raster needs {raster_bytes} bytes, above the {} byte policy",
                options.limits.max_memory_bytes
            ),
        });
    }

    let target = manifold_from_mesh(mesh, 0)?;
    target.status().map_err(|error| {
        CoreError::MeshOperation(format!("text target is not a valid solid: {error}"))
    })?;

    let target_center = [
        f64::from(minimum[0] + maximum[0]) * 0.5,
        f64::from(minimum[1] + maximum[1]) * 0.5,
        f64::from(minimum[2] + maximum[2]) * 0.5,
    ];
    let raster_width_mm = raster.width as f64 * EMBOSS_CELL_MM;
    let raster_height_mm = raster.height as f64 * EMBOSS_CELL_MM;
    let depth = f64::from(options.depth_mm);
    let overlap = EMBOSS_OVERLAP_MAX_MM.min(depth * 0.1);
    let surface_y = f64::from(maximum[1]);
    let base_y = if options.raised {
        surface_y - overlap
    } else {
        surface_y + overlap - depth
    };
    let mut boxes = Vec::with_capacity(occupied_cells);
    for row in 0..raster.height {
        check_cancel(&mut cancel, OP_EMBOSS)?;
        for column in 0..raster.width {
            let Some(alpha) = raster.pixels.get(row * raster.width + column).copied() else {
                return Err(CoreError::MeshOperation(
                    "text raster indexing failed".to_owned(),
                ));
            };
            if alpha == 0 {
                continue;
            }
            let box_height = depth * f64::from(alpha) / 255.0;
            if !box_height.is_finite() || box_height <= f64::EPSILON {
                continue;
            }
            let local_x = (column as f64 + 0.5) * EMBOSS_CELL_MM - raster_width_mm * 0.5;
            let local_z = -((row as f64 + 0.5) * EMBOSS_CELL_MM - raster_height_mm * 0.5);
            let center = [
                target_center[0] + local_x + f64::from(options.offset_x_mm),
                if options.raised {
                    base_y + box_height * 0.5
                } else {
                    surface_y + overlap - box_height * 0.5
                },
                target_center[2] + local_z + f64::from(options.offset_z_mm),
            ];
            boxes.push(
                Manifold::cube(EMBOSS_CELL_MM, box_height, EMBOSS_CELL_MM, true)
                    .translate(center[0], center[1], center[2]),
            );
        }
    }
    if boxes.is_empty() {
        return Err(CoreError::MeshOperation(
            "text relief did not produce any finite boxes".to_owned(),
        ));
    }
    check_cancel(&mut cancel, OP_EMBOSS)?;
    let relief = Manifold::batch_union(&boxes);
    relief.status().map_err(|error| {
        CoreError::MeshOperation(format!("text relief boolean preparation failed: {error}"))
    })?;
    let result_manifold = if options.raised {
        target.union(&relief)
    } else {
        target.difference(&relief)
    };
    check_cancel(&mut cancel, OP_EMBOSS)?;
    result_manifold.status().map_err(|error| {
        CoreError::MeshOperation(format!("text relief boolean failed: {error}"))
    })?;
    let (result_mesh, _) = mesh_from_manifold(&result_manifold)?;
    let result_health = validate_result(&result_mesh, OP_EMBOSS, &options.limits)?;
    let report = TextEmbossReport {
        source_vertices: mesh.vertex_count(),
        source_faces: mesh.face_count(),
        result_vertices: result_mesh.vertex_count(),
        result_faces: result_mesh.face_count(),
        source_volume_mm3: signed_volume_mm3(mesh).abs(),
        result_volume_mm3: signed_volume_mm3(&result_mesh).abs(),
        font: options.font.as_str().to_owned(),
        raster_width: raster.width,
        raster_height: raster.height,
        occupied_cells,
        depth_mm: options.depth_mm,
        raised: options.raised,
    };
    if !is_closed_volume(&result_health) {
        return Err(CoreError::MeshOperation(format!(
            "text relief result is not a closed solid: {}",
            result_health.issues.join("; ")
        )));
    }
    Ok(TextEmbossResult {
        mesh: result_mesh,
        report,
    })
}

/// Hollows a closed source solid with an exact convex or bounded voxel inner
/// offset, then subtracts that inner solid from the source.
pub fn hollow_mesh(mesh: &Mesh, options: HollowOptions) -> Result<HollowResult, CoreError> {
    hollow_mesh_with_hooks(mesh, options, None)
}

/// Hollows a solid while allowing an adapter to cancel between bounded
/// offset, voxel, and boolean phases.
pub fn hollow_mesh_with_hooks(
    mesh: &Mesh,
    options: HollowOptions,
    cancel: Option<&mut dyn FnMut() -> Result<(), CoreError>>,
) -> Result<HollowResult, CoreError> {
    // OF-54: a model too big or detailed for the bounded hollow says so in
    // plain words, on the desktop and in MCP alike.
    hollow_mesh_bounded(mesh, options, cancel).map_err(|error| match error {
        CoreError::OperationResourceLimit { .. } => CoreError::MeshOperation(
            "this model is too detailed to hollow; try Simplify first, or a thicker wall"
                .to_owned(),
        ),
        other => other,
    })
}

fn hollow_mesh_bounded(
    mesh: &Mesh,
    options: HollowOptions,
    mut cancel: Option<&mut dyn FnMut() -> Result<(), CoreError>>,
) -> Result<HollowResult, CoreError> {
    check_cancel(&mut cancel, OP_HOLLOW)?;
    let (minimum, maximum) = checked_solid_input(mesh, OP_HOLLOW, &options.limits)?;
    let wall = f64::from(options.wall_thickness_mm);
    if !wall.is_finite() || wall <= 0.0 {
        return Err(CoreError::MeshOperation(
            "wall_thickness_mm must be finite and greater than zero".to_owned(),
        ));
    }
    let extents = [
        f64::from(maximum[0] - minimum[0]),
        f64::from(maximum[1] - minimum[1]),
        f64::from(maximum[2] - minimum[2]),
    ];
    let minimum_extent = extents.iter().copied().fold(f64::INFINITY, f64::min);
    if !minimum_extent.is_finite() || minimum_extent <= 0.0 || wall * 2.0 >= minimum_extent {
        return Err(CoreError::MeshOperation(format!(
            "wall thickness {wall} must be less than half of the smallest model extent {minimum_extent}"
        )));
    }

    // The exact convex inside is tried first; a convex model with many
    // faces (the stock sphere: 5,120) is over its budget, so it takes the
    // grid path every other shape uses instead of failing (OF-61).
    let convex = match convex_planes(mesh, wall)? {
        Some(planes) => match inner_from_convex_planes(&planes, &options.limits, &mut cancel) {
            Ok(inner) => Some(inner),
            Err(CoreError::OperationResourceLimit { .. }) => None,
            Err(other) => return Err(other),
        },
        None => None,
    };
    let (inner_mesh, branch) = match convex {
        Some(inner) => (inner, "convex-exact"),
        None => (
            voxel_inner_mesh(mesh, minimum, maximum, wall, &options.limits, &mut cancel)?,
            "concave-voxel",
        ),
    };
    check_cancel(&mut cancel, OP_HOLLOW)?;
    let inner_health = validate_result(&inner_mesh, OP_HOLLOW, &options.limits)?;
    if !is_closed_volume(&inner_health) {
        return Err(CoreError::MeshOperation(
            "hollow inner offset is not a closed solid".to_owned(),
        ));
    }
    let source_manifold = manifold_from_mesh(mesh, 0)?;
    let inner_manifold = manifold_from_mesh(&inner_mesh, 1)?;
    source_manifold.status().map_err(|error| {
        CoreError::MeshOperation(format!("hollow source is not a valid solid: {error}"))
    })?;
    inner_manifold.status().map_err(|error| {
        CoreError::MeshOperation(format!("hollow inner offset is not a valid solid: {error}"))
    })?;
    let result_manifold = source_manifold.difference(&inner_manifold);
    check_cancel(&mut cancel, OP_HOLLOW)?;
    result_manifold
        .status()
        .map_err(|error| CoreError::MeshOperation(format!("hollow boolean failed: {error}")))?;
    let (result_mesh, _) = mesh_from_manifold(&result_manifold)?;
    let result_health = validate_result(&result_mesh, OP_HOLLOW, &options.limits)?;
    if !is_closed_volume(&result_health) {
        return Err(CoreError::MeshOperation(format!(
            "hollow result is not a closed solid (bodies {}, volume {:.6}): {}",
            result_health.body_count,
            signed_volume_mm3(&result_mesh).abs(),
            result_health.issues.join("; ")
        )));
    }
    Ok(HollowResult {
        report: HollowReport {
            source_vertices: mesh.vertex_count(),
            source_faces: mesh.face_count(),
            result_vertices: result_mesh.vertex_count(),
            result_faces: result_mesh.face_count(),
            source_volume_mm3: signed_volume_mm3(mesh).abs(),
            result_volume_mm3: signed_volume_mm3(&result_mesh).abs(),
            branch: branch.to_owned(),
            wall_thickness_mm: options.wall_thickness_mm,
            inner_volume_mm3: signed_volume_mm3(&inner_mesh).abs(),
        },
        mesh: result_mesh,
    })
}

fn checked_solid_input(
    mesh: &Mesh,
    operation: &str,
    limits: &MeshOperationLimits,
) -> Result<([f32; 3], [f32; 3]), CoreError> {
    limits.validate(operation)?;
    mesh.validate_non_empty()?;
    limits.check_mesh(operation, mesh)?;
    let health = inspect_mesh(mesh)?;
    if !is_closed_volume(&health) {
        return Err(CoreError::MeshOperation(format!(
            "{operation} requires a watertight closed volume: {}",
            health.issues.join("; ")
        )));
    }
    let (minimum, maximum) = mesh
        .bounds()
        .ok_or_else(|| CoreError::InvalidMesh(format!("{operation} source has no bounds")))?;
    if [minimum, maximum]
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(CoreError::InvalidMesh(format!(
            "{operation} source bounds are non-finite"
        )));
    }
    Ok((minimum, maximum))
}

fn validate_emboss_options(options: &TextEmbossOptions) -> Result<(), CoreError> {
    if options.text.is_empty() {
        return Err(CoreError::MeshOperation(
            "text must not be empty".to_owned(),
        ));
    }
    if options.text.chars().count() > EMBOSS_MAX_TEXT_LENGTH {
        return Err(CoreError::MeshOperation(format!(
            "text must be at most {EMBOSS_MAX_TEXT_LENGTH} characters"
        )));
    }
    if !(EMBOSS_FONT_SIZE_MIN..=EMBOSS_FONT_SIZE_MAX).contains(&options.font_size) {
        return Err(CoreError::MeshOperation(format!(
            "font_size must be between {EMBOSS_FONT_SIZE_MIN} and {EMBOSS_FONT_SIZE_MAX}"
        )));
    }
    if !options.depth_mm.is_finite() || options.depth_mm <= 0.0 {
        return Err(CoreError::MeshOperation(
            "depth_mm must be finite and greater than zero".to_owned(),
        ));
    }
    if options.depth_mm > EMBOSS_DEPTH_MAX_MM {
        return Err(CoreError::MeshOperation(format!(
            "depth_mm must be at most {EMBOSS_DEPTH_MAX_MM}"
        )));
    }
    for (name, value) in [
        ("offset_x_mm", options.offset_x_mm),
        ("offset_z_mm", options.offset_z_mm),
    ] {
        if !value.is_finite() || value.abs() > EMBOSS_OFFSET_MAX_MM {
            return Err(CoreError::MeshOperation(format!(
                "{name} must be finite and have absolute value at most {EMBOSS_OFFSET_MAX_MM}"
            )));
        }
    }
    Ok(())
}

fn render_text(options: &TextEmbossOptions) -> Result<Raster, CoreError> {
    let font = load_font(options.font)?;
    if let Some(font) = font {
        render_font_text(&font, &options.text, options.font_size)
    } else {
        render_builtin_text(&options.text, options.font_size)
    }
}

fn font_candidates(font: EmbossFont) -> &'static [&'static str] {
    const SANS: &[&str] = &[
        r"C:\Windows\Fonts\arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/System/Library/Fonts/Supplemental/Arial.ttf",
    ];
    const SERIF: &[&str] = &[
        r"C:\Windows\Fonts\georgia.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
        "/System/Library/Fonts/Supplemental/Georgia.ttf",
    ];
    const MONO: &[&str] = &[
        r"C:\Windows\Fonts\consola.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
    ];
    match font {
        EmbossFont::Default => &[],
        EmbossFont::Sans => SANS,
        EmbossFont::Serif => SERIF,
        EmbossFont::Mono => MONO,
    }
}

fn load_font(font: EmbossFont) -> Result<Option<Font>, CoreError> {
    if font == EmbossFont::Default {
        return Ok(None);
    }
    let candidates = font_candidates(font);
    for candidate in candidates {
        let path = Path::new(candidate);
        if !path.is_file() {
            continue;
        }
        let bytes = fs::read(path).map_err(|error| {
            CoreError::MeshOperation(format!("could not read {} font: {error}", font.as_str()))
        })?;
        let parsed = Font::from_bytes(bytes, FontSettings::default()).map_err(|error| {
            CoreError::MeshOperation(format!("could not parse {} font: {error:?}", font.as_str()))
        })?;
        return Ok(Some(parsed));
    }
    Err(CoreError::MeshOperation(format!(
        "{} font is not installed in a supported system path",
        font.as_str()
    )))
}

fn render_font_text(font: &Font, text: &str, font_size: u32) -> Result<Raster, CoreError> {
    let px = font_size as f32;
    let mut glyphs = Vec::with_capacity(text.chars().count());
    let mut pen_x = 0.0_f32;
    let mut min_x = 0.0_f32;
    let mut max_x = 0.0_f32;
    let mut min_y = 0_i32;
    let mut max_y = 0_i32;
    for character in text.chars() {
        let metrics = font.metrics(character, px);
        if !metrics.advance_width.is_finite() || metrics.advance_width < 0.0 {
            return Err(CoreError::MeshOperation(
                "font returned a non-finite glyph advance".to_owned(),
            ));
        }
        let left = pen_x + metrics.xmin as f32;
        let right = left + metrics.width as f32;
        min_x = min_x.min(left);
        max_x = max_x.max(right).max(pen_x + metrics.advance_width);
        min_y = min_y.min(metrics.ymin);
        max_y = max_y.max(metrics.ymin.saturating_add(
            i32::try_from(metrics.height).map_err(|_| {
                CoreError::MeshOperation("font glyph height is too large".to_owned())
            })?,
        ));
        glyphs.push((character, pen_x, metrics));
        pen_x += metrics.advance_width;
    }
    let width = raster_dimension(f64::from(max_x - min_x), "font raster width")?;
    let height_span = f64::from(max_y.saturating_sub(min_y));
    let height = raster_dimension(height_span, "font raster height")?;
    let pixels_len =
        width
            .checked_mul(height)
            .ok_or_else(|| CoreError::OperationResourceLimit {
                operation: OP_EMBOSS.to_owned(),
                reason: "font raster allocation overflowed".to_owned(),
            })?;
    let mut pixels = vec![0_u8; pixels_len];
    let width_i64 = i64::try_from(width)
        .map_err(|_| CoreError::MeshOperation("font raster width is too large".to_owned()))?;
    let height_i64 = i64::try_from(height)
        .map_err(|_| CoreError::MeshOperation("font raster height is too large".to_owned()))?;
    for (character, glyph_x, expected_metrics) in glyphs {
        let (metrics, bitmap) = font.rasterize(character, px);
        if metrics.width != expected_metrics.width || metrics.height != expected_metrics.height {
            return Err(CoreError::MeshOperation(
                "font raster metrics changed between layout and rasterization".to_owned(),
            ));
        }
        let x_base = (2.0 + glyph_x + metrics.xmin as f32 - min_x).round() as i64;
        let glyph_top = 2_i64
            .checked_add(i64::from(max_y))
            .and_then(|value| value.checked_sub(i64::from(metrics.ymin)))
            .and_then(|value| value.checked_sub(i64::try_from(metrics.height).ok()?))
            .ok_or_else(|| {
                CoreError::MeshOperation("font glyph placement overflowed".to_owned())
            })?;
        for row in 0..metrics.height {
            for column in 0..metrics.width {
                let source_index = row
                    .checked_mul(metrics.width)
                    .and_then(|value| value.checked_add(column))
                    .ok_or_else(|| {
                        CoreError::MeshOperation("font bitmap indexing overflowed".to_owned())
                    })?;
                let Some(&alpha) = bitmap.get(source_index) else {
                    return Err(CoreError::MeshOperation(
                        "font bitmap is shorter than its metrics".to_owned(),
                    ));
                };
                let target_x = x_base
                    + i64::try_from(column).map_err(|_| {
                        CoreError::MeshOperation("font glyph column is too large".to_owned())
                    })?;
                let target_y = glyph_top
                    + i64::try_from(row).map_err(|_| {
                        CoreError::MeshOperation("font glyph row is too large".to_owned())
                    })?;
                if alpha == 0
                    || target_x < 0
                    || target_x >= width_i64
                    || target_y < 0
                    || target_y >= height_i64
                {
                    continue;
                }
                let x = usize::try_from(target_x).map_err(|_| {
                    CoreError::MeshOperation("font raster X coordinate is invalid".to_owned())
                })?;
                let y = usize::try_from(target_y).map_err(|_| {
                    CoreError::MeshOperation("font raster Y coordinate is invalid".to_owned())
                })?;
                let target_index = y
                    .checked_mul(width)
                    .and_then(|value| value.checked_add(x))
                    .ok_or_else(|| {
                        CoreError::MeshOperation("font raster indexing overflowed".to_owned())
                    })?;
                if let Some(pixel) = pixels.get_mut(target_index) {
                    *pixel = (*pixel).max(alpha);
                }
            }
        }
    }
    Ok(Raster {
        width,
        height,
        pixels,
    })
}

fn raster_dimension(span: f64, label: &str) -> Result<usize, CoreError> {
    if !span.is_finite() || span < 0.0 || span > (usize::MAX.saturating_sub(4)) as f64 {
        return Err(CoreError::OperationResourceLimit {
            operation: OP_EMBOSS.to_owned(),
            reason: format!("{label} is outside the supported range"),
        });
    }
    let rounded = span.ceil() as usize;
    rounded
        .checked_add(4)
        .ok_or_else(|| CoreError::OperationResourceLimit {
            operation: OP_EMBOSS.to_owned(),
            reason: format!("{label} overflowed"),
        })
}

fn render_builtin_text(text: &str, font_size: u32) -> Result<Raster, CoreError> {
    let scale = usize::try_from((font_size.saturating_add(6)) / 7).map_err(|_| {
        CoreError::OperationResourceLimit {
            operation: OP_EMBOSS.to_owned(),
            reason: "built-in font scale overflowed".to_owned(),
        }
    })?;
    let advance = scale
        .checked_mul(6)
        .ok_or_else(|| CoreError::OperationResourceLimit {
            operation: OP_EMBOSS.to_owned(),
            reason: "built-in font width overflowed".to_owned(),
        })?;
    let width = text
        .chars()
        .count()
        .checked_mul(advance)
        .and_then(|value| value.checked_add(4))
        .ok_or_else(|| CoreError::OperationResourceLimit {
            operation: OP_EMBOSS.to_owned(),
            reason: "built-in font width overflowed".to_owned(),
        })?;
    let height = scale
        .checked_mul(7)
        .and_then(|value| value.checked_add(4))
        .ok_or_else(|| CoreError::OperationResourceLimit {
            operation: OP_EMBOSS.to_owned(),
            reason: "built-in font height overflowed".to_owned(),
        })?;
    let pixels_len =
        width
            .checked_mul(height)
            .ok_or_else(|| CoreError::OperationResourceLimit {
                operation: OP_EMBOSS.to_owned(),
                reason: "built-in font raster allocation overflowed".to_owned(),
            })?;
    let mut pixels = vec![0_u8; pixels_len];
    for (character_index, character) in text.chars().enumerate() {
        let pattern = builtin_glyph(character);
        let x_origin = 2 + character_index * advance;
        for (row, bits) in pattern.iter().copied().enumerate() {
            for column in 0..5_usize {
                if bits & (1_u8 << (4 - column)) == 0 {
                    continue;
                }
                for y_scale in 0..scale {
                    for x_scale in 0..scale {
                        let x = x_origin + column * scale + x_scale;
                        let y = 2 + row * scale + y_scale;
                        let index = y * width + x;
                        if let Some(pixel) = pixels.get_mut(index) {
                            *pixel = 255;
                        }
                    }
                }
            }
        }
    }
    Ok(Raster {
        width,
        height,
        pixels,
    })
}

fn builtin_glyph(character: char) -> [u8; 7] {
    let upper = character.to_ascii_uppercase();
    match upper {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
        'X' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
        'Y' => [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
        ],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 0b11111],
        '.' => [0, 0, 0, 0, 0, 0, 0b00100],
        ':' => [0, 0b00100, 0, 0, 0b00100, 0, 0],
        '!' => [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0, 0b00100],
        '?' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0, 0b00100],
        ' ' => [0; 7],
        _ => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0, 0b00100],
    }
}

fn convex_planes(mesh: &Mesh, wall: f64) -> Result<Option<Vec<Plane>>, CoreError> {
    let signed_volume = signed_volume_mm3(mesh);
    if !signed_volume.is_finite() || signed_volume.abs() <= 1.0e-12 {
        return Err(CoreError::MeshOperation(
            "hollow source has no usable oriented volume".to_owned(),
        ));
    }
    let outward_sign = if signed_volume >= 0.0 { 1.0 } else { -1.0 };
    let (minimum, maximum) = mesh
        .bounds()
        .ok_or_else(|| CoreError::MeshOperation("hollow source has no bounds".to_owned()))?;
    let scale = f64::from(
        (maximum[0] - minimum[0])
            .max(maximum[1] - minimum[1])
            .max(maximum[2] - minimum[2]),
    )
    .max(1.0);
    let tolerance = scale * 1.0e-5;
    let mut planes = Vec::new();
    for face in &mesh.faces {
        let Some(point) = mesh.vertices.get(face[0] as usize).copied() else {
            return Err(CoreError::InvalidMesh(
                "hollow face index is outside the vertex buffer".to_owned(),
            ));
        };
        let raw = mesh.face_normal(*face);
        let mut normal = [
            f64::from(raw[0]) * outward_sign,
            f64::from(raw[1]) * outward_sign,
            f64::from(raw[2]) * outward_sign,
        ];
        let length = dot3(normal, normal).sqrt();
        if !length.is_finite() || length <= f64::EPSILON {
            return Err(CoreError::MeshOperation(
                "hollow source contains a degenerate face normal".to_owned(),
            ));
        }
        normal = scale3(normal, 1.0 / length);
        let source_offset = dot3(
            normal,
            [
                f64::from(point[0]),
                f64::from(point[1]),
                f64::from(point[2]),
            ],
        );
        for vertex in &mesh.vertices {
            let value = dot3(
                normal,
                [
                    f64::from(vertex[0]),
                    f64::from(vertex[1]),
                    f64::from(vertex[2]),
                ],
            );
            if value > source_offset + tolerance {
                return Ok(None);
            }
        }
        let offset = source_offset - wall;
        if !planes.iter().any(|existing: &Plane| {
            dot3(existing.normal, normal) > 1.0 - 1.0e-7
                && (existing.offset - offset).abs() <= tolerance * 2.0
        }) {
            planes.push(Plane { normal, offset });
        }
    }
    if planes.len() < 4 {
        return Err(CoreError::MeshOperation(
            "hollow convex source has fewer than four usable planes".to_owned(),
        ));
    }
    Ok(Some(planes))
}

fn inner_from_convex_planes(
    planes: &[Plane],
    limits: &MeshOperationLimits,
    cancel: &mut Option<&mut dyn FnMut() -> Result<(), CoreError>>,
) -> Result<Mesh, CoreError> {
    let plane_count =
        u64::try_from(planes.len()).map_err(|_| CoreError::OperationResourceLimit {
            operation: OP_HOLLOW.to_owned(),
            reason: "convex plane count overflowed".to_owned(),
        })?;
    let work = plane_count
        .checked_mul(plane_count.saturating_sub(1))
        .and_then(|value| value.checked_mul(plane_count.saturating_sub(2)))
        .map(|value| value / 6)
        .ok_or_else(|| CoreError::OperationResourceLimit {
            operation: OP_HOLLOW.to_owned(),
            reason: "convex plane work count overflowed".to_owned(),
        })?;
    limits.check_work(OP_HOLLOW, work)?;
    let mut points = Vec::new();
    for first in 0..planes.len() {
        check_cancel(cancel, OP_HOLLOW)?;
        for second in (first + 1)..planes.len() {
            for third in (second + 1)..planes.len() {
                let Some(point) = plane_intersection(planes[first], planes[second], planes[third])
                else {
                    continue;
                };
                if planes
                    .iter()
                    .all(|plane| dot3(plane.normal, point) <= plane.offset + 1.0e-5)
                    && !points.iter().any(|existing: &[f64; 3]| {
                        dot3(sub3(*existing, point), sub3(*existing, point)) <= 1.0e-8
                    })
                {
                    points.push(point);
                    if points.len() > limits.max_vertices {
                        return Err(CoreError::OperationResourceLimit {
                            operation: OP_HOLLOW.to_owned(),
                            reason: "convex inner offset has too many vertices".to_owned(),
                        });
                    }
                }
            }
        }
    }
    if points.len() < 4 {
        return Err(CoreError::MeshOperation(
            "convex inner offset is empty or degenerate".to_owned(),
        ));
    }
    let inner = Manifold::hull_pts(&points);
    inner
        .status()
        .map_err(|error| CoreError::MeshOperation(format!("convex inner hull failed: {error}")))?;
    if inner.is_empty() || !inner.volume().is_finite() || inner.volume() <= 0.0 {
        return Err(CoreError::MeshOperation(
            "convex inner offset has no volume".to_owned(),
        ));
    }
    let (mesh, _) = mesh_from_manifold(&inner)?;
    Ok(mesh)
}

/// Inner offset for a concave solid (OF-54). The inside is filled one grid
/// row at a time from the triangles that row crosses, the distance to the
/// surface comes from a grid distance transform, and the blocky inner
/// boundary is then reduced where it is flat. The work is about the number
/// of grid cells plus a few per triangle, instead of cells times triangles.
fn voxel_inner_mesh(
    mesh: &Mesh,
    minimum: [f32; 3],
    maximum: [f32; 3],
    wall: f64,
    limits: &MeshOperationLimits,
    cancel: &mut Option<&mut dyn FnMut() -> Result<(), CoreError>>,
) -> Result<Mesh, CoreError> {
    let extents = [
        f64::from(maximum[0] - minimum[0]),
        f64::from(maximum[1] - minimum[1]),
        f64::from(maximum[2] - minimum[2]),
    ];
    let maximum_extent = extents.iter().copied().fold(0.0_f64, f64::max);
    let mut pitch = (wall * 0.25).max(maximum_extent / HOLLOW_MAX_AXIS_CELLS as f64);
    let mut dimensions;
    loop {
        dimensions = [
            voxel_dimension(extents[0], pitch)?,
            voxel_dimension(extents[1], pitch)?,
            voxel_dimension(extents[2], pitch)?,
        ];
        let cells = dimensions[0]
            .saturating_mul(dimensions[1])
            .saturating_mul(dimensions[2]);
        if cells <= HOLLOW_MAX_VOXELS {
            break;
        }
        pitch *= 1.1;
    }
    if !pitch.is_finite() || pitch <= 0.0 || pitch > wall * 0.75 {
        return Err(CoreError::MeshOperation(format!(
            "this model is too large for a {wall:.1} mm wall; try a thicker wall (at least {:.1} mm)",
            pitch / 0.75
        )));
    }
    let cells = dimensions[0] * dimensions[1] * dimensions[2];
    let grid_bytes = u64::try_from(cells)
        .ok()
        .and_then(|cells| cells.checked_mul(5))
        .ok_or_else(|| CoreError::OperationResourceLimit {
            operation: OP_HOLLOW.to_owned(),
            reason: "concave voxel grid size overflowed".to_owned(),
        })?;
    if grid_bytes > limits.max_memory_bytes {
        return Err(CoreError::OperationResourceLimit {
            operation: OP_HOLLOW.to_owned(),
            reason: format!("concave voxel grid needs {grid_bytes} bytes"),
        });
    }
    let triangles = mesh_triangles(mesh)?;
    let origin = [
        f64::from(minimum[0]) - pitch,
        f64::from(minimum[1]) - pitch,
        f64::from(minimum[2]) - pitch,
    ];
    let centre = |axis: usize, index: usize| origin[axis] + (index as f64 + 0.5) * pitch;
    let cell_range = |axis: usize, low: f64, high: f64| -> Option<(usize, usize)> {
        let first = ((low - origin[axis]) / pitch - 0.5).ceil().max(0.0);
        let last = ((high - origin[axis]) / pitch - 0.5)
            .floor()
            .min(dimensions[axis] as f64 - 1.0);
        (first <= last).then_some((first as usize, last as usize))
    };

    // Rows along x: each triangle is listed in the rows its y/z box crosses.
    let row_count = dimensions[1] * dimensions[2];
    let mut rows: Vec<Vec<u32>> = vec![Vec::new(); row_count];
    let mut registrations = 0_u64;
    for (index, triangle) in triangles.iter().enumerate() {
        let low_y = triangle.a[1].min(triangle.b[1]).min(triangle.c[1]);
        let high_y = triangle.a[1].max(triangle.b[1]).max(triangle.c[1]);
        let low_z = triangle.a[2].min(triangle.b[2]).min(triangle.c[2]);
        let high_z = triangle.a[2].max(triangle.b[2]).max(triangle.c[2]);
        let (Some((y0, y1)), Some((z0, z1))) =
            (cell_range(1, low_y, high_y), cell_range(2, low_z, high_z))
        else {
            continue;
        };
        for z in z0..=z1 {
            for y in y0..=y1 {
                rows[z * dimensions[1] + y].push(index as u32);
                registrations += 1;
            }
        }
    }
    let work =
        registrations.saturating_add(u64::try_from(cells).unwrap_or(u64::MAX).saturating_mul(4));
    limits.check_work(OP_HOLLOW, work)?;

    // Inside cells by crossing parity along each row.
    let mut inside = vec![false; cells];
    let mut crossings = Vec::new();
    for z in 0..dimensions[2] {
        check_cancel(cancel, OP_HOLLOW)?;
        for y in 0..dimensions[1] {
            // A tiny offset keeps the ray off shared edges and vertices.
            let ray_y = centre(1, y) + pitch * 1.0e-4;
            let ray_z = centre(2, z) + pitch * 1.7e-4;
            crossings.clear();
            for triangle in &rows[z * dimensions[1] + y] {
                if let Some(x) = row_crossing(&triangles[*triangle as usize], ray_y, ray_z) {
                    crossings.push(x);
                }
            }
            if crossings.len() < 2 {
                continue;
            }
            crossings.sort_by(f64::total_cmp);
            for pair in crossings.as_chunks::<2>().0 {
                if let Some((x0, x1)) = cell_range(0, pair[0], pair[1]) {
                    for x in x0..=x1 {
                        inside[grid_index(x, y, z, dimensions)] = true;
                    }
                }
            }
        }
    }

    // Squared distance (in cells) from each inside cell to the nearest
    // outside cell, by the separable exact distance transform.
    let mut distance: Vec<f32> = inside
        .iter()
        .map(|inside| if *inside { f32::INFINITY } else { 0.0 })
        .collect();
    for axis in 0..3 {
        check_cancel(cancel, OP_HOLLOW)?;
        distance_transform_axis(&mut distance, dimensions, axis);
    }
    // The surface lies between an inside and an outside cell centre, and the
    // inner boundary is drawn on the cell faces, so require the wall plus a
    // cell's half-diagonal: the wall is never thinner than asked.
    let threshold = wall / pitch + 0.5 + 0.87;
    let threshold_squared = (threshold * threshold) as f32;
    let mut occupied: Vec<u8> = inside
        .iter()
        .zip(&distance)
        .map(|(inside, squared)| u8::from(*inside && *squared >= threshold_squared))
        .collect();
    clean_voxel_contacts(&mut occupied, dimensions, cancel)?;
    let occupied_count = occupied.iter().filter(|cell| **cell == 1).count();
    if occupied_count == 0 {
        return Err(CoreError::MeshOperation(format!(
            "a {wall:.1} mm wall fills this whole model; try a thinner wall"
        )));
    }
    let expected_faces =
        occupied_count
            .checked_mul(12)
            .ok_or_else(|| CoreError::OperationResourceLimit {
                operation: OP_HOLLOW.to_owned(),
                reason: "concave voxel boundary face count overflowed".to_owned(),
            })?;
    if expected_faces > limits.max_faces {
        return Err(CoreError::OperationResourceLimit {
            operation: OP_HOLLOW.to_owned(),
            reason: format!("concave inner boundary may need {expected_faces} faces"),
        });
    }
    let blocky = voxel_boundary_mesh(&occupied, dimensions, origin, pitch)?;
    check_cancel(cancel, OP_HOLLOW)?;
    // Merge the flat parts of the staircase; a collapse may move the surface
    // by at most a quarter cell, so the wall stays as thick as asked.
    Ok(crate::simplify::simplify_within_tolerance(&blocky, pitch * 0.25, limits).unwrap_or(blocky))
}

/// Whether the cells around one grid corner (an 8-bit occupancy mask, bit
/// `dx | dy << 1 | dz << 2`) give a manifold boundary there: the occupied
/// cells and the empty cells must each be one face-connected group.
fn corner_is_manifold(mask: u8) -> bool {
    fn groups(set: u8) -> usize {
        let mut seen = 0_u8;
        let mut count = 0;
        for start in 0..8_u8 {
            if set & (1 << start) == 0 || seen & (1 << start) != 0 {
                continue;
            }
            count += 1;
            let mut stack = vec![start];
            seen |= 1 << start;
            while let Some(cell) = stack.pop() {
                for bit in [1_u8, 2, 4] {
                    let next = cell ^ bit;
                    if set & (1 << next) != 0 && seen & (1 << next) == 0 {
                        seen |= 1 << next;
                        stack.push(next);
                    }
                }
            }
        }
        count
    }
    groups(mask) <= 1 && groups(!mask) <= 1
}

/// Removes inner cells until every grid corner is manifold (OF-54), so the
/// blocky inner boundary is a closed surface with no edge- or corner-only
/// contacts. Only removes cells, so the wall can only get thicker.
fn clean_voxel_contacts(
    occupied: &mut [u8],
    dimensions: [usize; 3],
    cancel: &mut Option<&mut dyn FnMut() -> Result<(), CoreError>>,
) -> Result<(), CoreError> {
    let manifold: Vec<bool> = (0..=255_u8).map(corner_is_manifold).collect();
    for _ in 0..32 {
        check_cancel(cancel, OP_HOLLOW)?;
        let mut changed = false;
        for vz in 1..dimensions[2] {
            for vy in 1..dimensions[1] {
                for vx in 1..dimensions[0] {
                    let cell = |bit: u8| {
                        grid_index(
                            vx - 1 + usize::from(bit & 1),
                            vy - 1 + usize::from((bit >> 1) & 1),
                            vz - 1 + usize::from((bit >> 2) & 1),
                            dimensions,
                        )
                    };
                    let mut mask = 0_u8;
                    for bit in 0..8_u8 {
                        if occupied[cell(bit)] == 1 {
                            mask |= 1 << bit;
                        }
                    }
                    if manifold[usize::from(mask)] {
                        continue;
                    }
                    // Prefer the single removal that fixes this corner.
                    let fix = (0..8_u8)
                        .filter(|bit| mask & (1 << bit) != 0)
                        .find(|bit| manifold[usize::from(mask & !(1 << bit))])
                        .or_else(|| (0..8_u8).find(|bit| mask & (1 << bit) != 0));
                    if let Some(bit) = fix {
                        occupied[cell(bit)] = 0;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            return Ok(());
        }
    }
    Err(CoreError::MeshOperation(
        "could not make a clean inner surface for this model; try a thicker wall".to_owned(),
    ))
}

/// Where the row line (y, z) crosses a triangle along x, if it does.
fn row_crossing(triangle: &Triangle, y: f64, z: f64) -> Option<f64> {
    let (a, b, c) = (triangle.a, triangle.b, triangle.c);
    let area = (b[1] - a[1]) * (c[2] - a[2]) - (c[1] - a[1]) * (b[2] - a[2]);
    if area.abs() <= 1.0e-14 {
        return None;
    }
    let u = ((y - a[1]) * (c[2] - a[2]) - (c[1] - a[1]) * (z - a[2])) / area;
    let v = ((b[1] - a[1]) * (z - a[2]) - (y - a[1]) * (b[2] - a[2])) / area;
    if u < 0.0 || v < 0.0 || u + v > 1.0 {
        return None;
    }
    Some(a[0] + u * (b[0] - a[0]) + v * (c[0] - a[0]))
}

/// One axis of the Felzenszwalb-Huttenlocher squared distance transform.
fn distance_transform_axis(grid: &mut [f32], dimensions: [usize; 3], axis: usize) {
    let length = dimensions[axis];
    let (other_a, other_b) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let mut line = vec![0.0_f32; length];
    let mut output = vec![0.0_f32; length];
    let mut hull = vec![0_usize; length];
    let mut bounds = vec![0.0_f32; length + 1];
    for second in 0..dimensions[other_b] {
        for first in 0..dimensions[other_a] {
            let index_of = |position: usize| {
                let mut coordinate = [0_usize; 3];
                coordinate[axis] = position;
                coordinate[other_a] = first;
                coordinate[other_b] = second;
                grid_index(coordinate[0], coordinate[1], coordinate[2], dimensions)
            };
            for (position, value) in line.iter_mut().enumerate() {
                *value = grid[index_of(position)];
            }
            if line.iter().all(|value| value.is_infinite()) {
                continue;
            }
            let mut count = 0_usize;
            for position in 0..length {
                if line[position].is_infinite() {
                    continue;
                }
                loop {
                    if count == 0 {
                        hull[0] = position;
                        bounds[0] = f32::NEG_INFINITY;
                        bounds[1] = f32::INFINITY;
                        count = 1;
                        break;
                    }
                    let previous = hull[count - 1];
                    let crossing = ((line[position] + (position * position) as f32)
                        - (line[previous] + (previous * previous) as f32))
                        / (2.0 * (position as f32 - previous as f32));
                    if crossing <= bounds[count - 1] {
                        count -= 1;
                        continue;
                    }
                    hull[count] = position;
                    bounds[count] = crossing;
                    bounds[count + 1] = f32::INFINITY;
                    count += 1;
                    break;
                }
            }
            let mut segment = 0_usize;
            for (position, value) in output.iter_mut().enumerate() {
                while bounds[segment + 1] < position as f32 {
                    segment += 1;
                }
                let nearest = hull[segment];
                let offset = position as f32 - nearest as f32;
                *value = offset * offset + line[nearest];
            }
            for (position, value) in output.iter().enumerate() {
                grid[index_of(position)] = *value;
            }
        }
    }
}

fn voxel_dimension(extent: f64, pitch: f64) -> Result<usize, CoreError> {
    if !extent.is_finite() || extent <= 0.0 || !pitch.is_finite() || pitch <= 0.0 {
        return Err(CoreError::MeshOperation(
            "concave voxel source has invalid bounds".to_owned(),
        ));
    }
    let span = (extent / pitch).ceil();
    if !span.is_finite() || span > (usize::MAX.saturating_sub(2)) as f64 {
        return Err(CoreError::OperationResourceLimit {
            operation: OP_HOLLOW.to_owned(),
            reason: "concave voxel axis dimension exceeds the accounting range".to_owned(),
        });
    }
    let cells = span as usize;
    cells
        .checked_add(2)
        .ok_or_else(|| CoreError::OperationResourceLimit {
            operation: OP_HOLLOW.to_owned(),
            reason: "concave voxel axis dimension overflowed".to_owned(),
        })
}

fn mesh_triangles(mesh: &Mesh) -> Result<Vec<Triangle>, CoreError> {
    let mut triangles = Vec::with_capacity(mesh.face_count());
    for face in &mesh.faces {
        let Some(a) = mesh.vertices.get(face[0] as usize).copied() else {
            return Err(CoreError::InvalidMesh(
                "hollow triangle index is outside the vertex buffer".to_owned(),
            ));
        };
        let Some(b) = mesh.vertices.get(face[1] as usize).copied() else {
            return Err(CoreError::InvalidMesh(
                "hollow triangle index is outside the vertex buffer".to_owned(),
            ));
        };
        let Some(c) = mesh.vertices.get(face[2] as usize).copied() else {
            return Err(CoreError::InvalidMesh(
                "hollow triangle index is outside the vertex buffer".to_owned(),
            ));
        };
        triangles.push(Triangle {
            a: [f64::from(a[0]), f64::from(a[1]), f64::from(a[2])],
            b: [f64::from(b[0]), f64::from(b[1]), f64::from(b[2])],
            c: [f64::from(c[0]), f64::from(c[1]), f64::from(c[2])],
        });
    }
    Ok(triangles)
}

fn voxel_boundary_mesh(
    occupied: &[u8],
    dimensions: [usize; 3],
    origin: [f64; 3],
    pitch: f64,
) -> Result<Mesh, CoreError> {
    let mut vertices = Vec::new();
    let mut faces = Vec::new();
    let mut vertex_map: HashMap<(usize, usize, usize), u32> = HashMap::new();
    let directions: [([i32; 3], [[usize; 3]; 4]); 6] = [
        ([1, 0, 0], [[1, 0, 0], [1, 1, 0], [1, 1, 1], [1, 0, 1]]),
        ([-1, 0, 0], [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]]),
        ([0, 1, 0], [[0, 1, 0], [0, 1, 1], [1, 1, 1], [1, 1, 0]]),
        ([0, -1, 0], [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]]),
        ([0, 0, 1], [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]]),
        ([0, 0, -1], [[0, 0, 0], [0, 1, 0], [1, 1, 0], [1, 0, 0]]),
    ];
    for z in 0..dimensions[2] {
        for y in 0..dimensions[1] {
            for x in 0..dimensions[0] {
                if occupied.get(grid_index(x, y, z, dimensions)) != Some(&1) {
                    continue;
                }
                for (direction, corners) in directions {
                    let neighbor = [
                        x as i32 + direction[0],
                        y as i32 + direction[1],
                        z as i32 + direction[2],
                    ];
                    let neighbor_is_occupied = neighbor[0] >= 0
                        && neighbor[1] >= 0
                        && neighbor[2] >= 0
                        && (neighbor[0] as usize) < dimensions[0]
                        && (neighbor[1] as usize) < dimensions[1]
                        && (neighbor[2] as usize) < dimensions[2]
                        && occupied.get(grid_index(
                            neighbor[0] as usize,
                            neighbor[1] as usize,
                            neighbor[2] as usize,
                            dimensions,
                        )) == Some(&1);
                    if neighbor_is_occupied {
                        continue;
                    }
                    let mut quad = [0_u32; 4];
                    for (slot, corner) in quad.iter_mut().zip(corners) {
                        let key = (x + corner[0], y + corner[1], z + corner[2]);
                        if let Some(index) = vertex_map.get(&key).copied() {
                            *slot = index;
                            continue;
                        }
                        let index = u32::try_from(vertices.len()).map_err(|_| {
                            CoreError::OperationResourceLimit {
                                operation: OP_HOLLOW.to_owned(),
                                reason: "concave boundary has too many vertices".to_owned(),
                            }
                        })?;
                        vertices.push([
                            (origin[0] + key.0 as f64 * pitch) as f32,
                            (origin[1] + key.1 as f64 * pitch) as f32,
                            (origin[2] + key.2 as f64 * pitch) as f32,
                        ]);
                        vertex_map.insert(key, index);
                        *slot = index;
                    }
                    faces.push([quad[0], quad[1], quad[2]]);
                    faces.push([quad[0], quad[2], quad[3]]);
                }
            }
        }
    }
    Mesh::new(vertices, faces)
}

fn validate_result(
    mesh: &Mesh,
    operation: &str,
    limits: &MeshOperationLimits,
) -> Result<crate::MeshHealthReport, CoreError> {
    mesh.validate_non_empty()?;
    limits.check_mesh(operation, mesh)?;
    inspect_mesh(mesh)
}

fn check_cancel(
    cancel: &mut Option<&mut dyn FnMut() -> Result<(), CoreError>>,
    operation: &str,
) -> Result<(), CoreError> {
    if let Some(callback) = cancel.as_deref_mut() {
        callback().map_err(|error| match error {
            CoreError::OperationCancelled(_) => error,
            other => {
                CoreError::MeshOperation(format!("{operation} cancellation hook failed: {other}"))
            }
        })?;
    }
    Ok(())
}

fn is_closed_volume(health: &crate::MeshHealthReport) -> bool {
    health.watertight
        && health.winding_consistent
        && health.degenerate_faces == 0
        && health.non_manifold_edges == 0
        && health.duplicate_faces == 0
        && health.is_volume
}

fn plane_intersection(first: Plane, second: Plane, third: Plane) -> Option<[f64; 3]> {
    let cross_second_third = cross3(second.normal, third.normal);
    let determinant = dot3(first.normal, cross_second_third);
    if determinant.abs() <= 1.0e-10 {
        return None;
    }
    let point = add3(
        add3(
            scale3(cross_second_third, first.offset),
            scale3(cross3(third.normal, first.normal), second.offset),
        ),
        scale3(cross3(first.normal, second.normal), third.offset),
    );
    let result = scale3(point, 1.0 / determinant);
    if result.iter().all(|value| value.is_finite()) {
        Some(result)
    } else {
        None
    }
}

fn grid_index(x: usize, y: usize, z: usize, dimensions: [usize; 3]) -> usize {
    x + dimensions[0] * (y + dimensions[1] * z)
}

fn signed_volume_mm3(mesh: &Mesh) -> f64 {
    mesh.faces
        .iter()
        .filter_map(|face| {
            let a = mesh.vertices.get(face[0] as usize)?;
            let b = mesh.vertices.get(face[1] as usize)?;
            let c = mesh.vertices.get(face[2] as usize)?;
            let a = [f64::from(a[0]), f64::from(a[1]), f64::from(a[2])];
            let b = [f64::from(b[0]), f64::from(b[1]), f64::from(b[2])];
            let c = [f64::from(c[0]), f64::from(c[1]), f64::from(c[2])];
            Some(dot3(a, cross3(b, c)) / 6.0)
        })
        .sum()
}

fn dot3(first: [f64; 3], second: [f64; 3]) -> f64 {
    first[0] * second[0] + first[1] * second[1] + first[2] * second[2]
}

fn cross3(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn add3(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[0] + second[0],
        first[1] + second[1],
        first[2] + second[2],
    ]
}

fn sub3(first: [f64; 3], second: [f64; 3]) -> [f64; 3] {
    [
        first[0] - second[0],
        first[1] - second[1],
        first[2] - second[2],
    ]
}

fn scale3(value: [f64; 3], factor: f64) -> [f64; 3] {
    [value[0] * factor, value[1] * factor, value[2] * factor]
}
