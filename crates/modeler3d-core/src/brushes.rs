//! Topology-preserving local brush operations.
//!
//! These operations mirror the Python tools contract: all parameters are
//! bounded, source meshes are never mutated, path inputs are capped, and the
//! final displacement is clipped to a radius-derived safety envelope.

use std::collections::{BTreeMap, BTreeSet};

use crate::{CoreError, Mesh, MeshOperationLimits};

/// Brush falloff choices.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrushFalloff {
    /// Smoothstep with zero slope at the radius.
    Smooth,
    /// Linear influence to the radius.
    Linear,
    /// Full influence strictly inside the radius.
    Constant,
}

impl BrushFalloff {
    /// Parses the Python falloff names.
    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "smooth" => Ok(Self::Smooth),
            "linear" => Ok(Self::Linear),
            "constant" => Ok(Self::Constant),
            other => Err(CoreError::MeshOperation(format!(
                "falloff must be one of: smooth, linear, constant (got {other})"
            ))),
        }
    }

    fn weight(self, normalized_distance: f32) -> f32 {
        let distance = normalized_distance.clamp(0.0, 1.0);
        match self {
            Self::Linear => 1.0 - distance,
            Self::Constant => {
                if distance < 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
            Self::Smooth => 1.0 - distance * distance * (3.0 - 2.0 * distance),
        }
    }
}

/// Inward or outward normal/path direction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrushDirection {
    /// Pull toward a stroke center or push along a surface normal.
    Inward,
    /// Push away from a stroke center or along a surface normal.
    Outward,
}

impl BrushDirection {
    /// Parses the Python direction names.
    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "inward" => Ok(Self::Inward),
            "outward" => Ok(Self::Outward),
            other => Err(CoreError::MeshOperation(format!(
                "direction must be one of: inward, outward (got {other})"
            ))),
        }
    }
}

/// Symmetry planes supported by Elastic Grab.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrushSymmetry {
    /// No mirrored stroke.
    None,
    /// Mirror in the X plane.
    X,
    /// Mirror in the Y plane.
    Y,
    /// Mirror in the Z plane.
    Z,
    /// Mirror in X and Y.
    Xy,
    /// Mirror in X and Z.
    Xz,
    /// Mirror in Y and Z.
    Yz,
    /// Mirror in all three planes.
    Xyz,
}

impl BrushSymmetry {
    /// Parses case-insensitive Python symmetry names.
    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "none" => Ok(Self::None),
            "x" => Ok(Self::X),
            "y" => Ok(Self::Y),
            "z" => Ok(Self::Z),
            "xy" => Ok(Self::Xy),
            "xz" => Ok(Self::Xz),
            "yz" => Ok(Self::Yz),
            "xyz" => Ok(Self::Xyz),
            other => Err(CoreError::MeshOperation(format!(
                "symmetry must be one of: none, x, y, z, xy, xz, yz, xyz (got {other})"
            ))),
        }
    }

    fn axes(self) -> &'static [usize] {
        match self {
            Self::None => &[],
            Self::X => &[0],
            Self::Y => &[1],
            Self::Z => &[2],
            Self::Xy => &[0, 1],
            Self::Xz => &[0, 2],
            Self::Yz => &[1, 2],
            Self::Xyz => &[0, 1, 2],
        }
    }
}

/// Elastic Grab parameter contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElasticGrabOptions {
    /// Influence radius in millimetres.
    pub radius_mm: f32,
    /// Strength in the inclusive 0..=1 range.
    pub strength: f32,
    /// Falloff curve.
    pub falloff: BrushFalloff,
    /// Mirrored stroke planes.
    pub symmetry: BrushSymmetry,
    /// Resource policy.
    pub limits: MeshOperationLimits,
}

impl Default for ElasticGrabOptions {
    fn default() -> Self {
        Self {
            radius_mm: 10.0,
            strength: 0.75,
            falloff: BrushFalloff::Smooth,
            symmetry: BrushSymmetry::None,
            limits: MeshOperationLimits::default(),
        }
    }
}

/// Crease Polish parameter contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreasePolishOptions {
    /// Influence radius in millimetres.
    pub radius_mm: f32,
    /// Strength in the inclusive 0..=1 range.
    pub strength: f32,
    /// Pull direction.
    pub direction: BrushDirection,
    /// Falloff curve.
    pub falloff: BrushFalloff,
    /// How narrow and deep the groove is at the stroke line, in the
    /// inclusive 0..=1 range (OF-59). 0 is the gentle OF-49 crease; 1 puts
    /// the depth in a tight groove at the line and digs up to 60 % deeper.
    /// The pinch toward the line is unchanged, so the surface never folds.
    pub sharpness: f32,
    /// Resource policy.
    pub limits: MeshOperationLimits,
}

impl Default for CreasePolishOptions {
    fn default() -> Self {
        Self {
            radius_mm: 5.0,
            strength: 0.5,
            direction: BrushDirection::Inward,
            falloff: BrushFalloff::Smooth,
            sharpness: 0.0,
            limits: MeshOperationLimits::default(),
        }
    }
}

/// Smooth Brush parameter contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SmoothBrushOptions {
    /// Influence radius in millimetres.
    pub radius_mm: f32,
    /// Strength in the inclusive 0..=1 range.
    pub strength: f32,
    /// Fraction of normal motion retained.
    pub shrinkage: f32,
    /// Falloff curve.
    pub falloff: BrushFalloff,
    /// Resource policy.
    pub limits: MeshOperationLimits,
}

impl Default for SmoothBrushOptions {
    fn default() -> Self {
        Self {
            radius_mm: 8.0,
            strength: 0.5,
            shrinkage: 0.25,
            falloff: BrushFalloff::Smooth,
            limits: MeshOperationLimits::default(),
        }
    }
}

/// Blob parameter contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlobOptions {
    /// Influence radius in millimetres.
    pub radius_mm: f32,
    /// Strength in the inclusive 0..=1 range.
    pub strength: f32,
    /// Normal direction.
    pub direction: BrushDirection,
    /// Falloff curve.
    pub falloff: BrushFalloff,
    /// Resource policy.
    pub limits: MeshOperationLimits,
}

impl Default for BlobOptions {
    fn default() -> Self {
        Self {
            radius_mm: 8.0,
            strength: 0.5,
            direction: BrushDirection::Outward,
            falloff: BrushFalloff::Smooth,
            limits: MeshOperationLimits::default(),
        }
    }
}

/// Shared brush evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct BrushReport {
    /// Stable operation name.
    pub operation: String,
    /// Whether positions changed.
    pub changed: bool,
    /// Number of path centers.
    pub centers: usize,
    /// Number of vertices with non-zero final displacement.
    pub affected_vertices: usize,
    /// Maximum final displacement in millimetres.
    pub maximum_displacement_mm: f32,
    /// Radius used by the operation.
    pub radius_mm: f32,
    /// Strength used by the operation.
    pub strength: f32,
    /// Non-fatal diagnostics.
    pub warnings: Vec<String>,
}

/// Brush mesh and evidence.
#[derive(Clone, Debug, PartialEq)]
pub struct BrushResult {
    /// Topology-preserving result mesh.
    pub mesh: Mesh,
    /// Operation evidence.
    pub report: BrushReport,
}

/// Applies Elastic Grab to one local-space center and offset.
pub fn elastic_grab(
    mesh: &Mesh,
    center_mm: [f32; 3],
    offset_mm: [f32; 3],
    options: ElasticGrabOptions,
) -> Result<BrushResult, CoreError> {
    validate_mesh_and_limits(mesh, &options.limits, "elastic_grab")?;
    validate_radius_strength(options.radius_mm, options.strength, "elastic_grab")?;
    validate_point(center_mm, "center_mm")?;
    validate_point(offset_mm, "offset_mm")?;
    let offset_length = length(offset_mm);
    if offset_length > 2_000.0 {
        return Err(CoreError::MeshOperation(
            "offset_mm magnitude must be at most 2000 mm".to_owned(),
        ));
    }
    let work_units = operation_work(mesh, 1)?;
    options.limits.check_work("elastic_grab", work_units)?;
    let source = mesh.vertices.clone();
    let mut vertices = source.clone();
    if options.strength > 0.0 && offset_length > 0.0 {
        let mut total_weight = vec![0.0_f32; vertices.len()];
        let mut weighted = vec![[0.0_f32; 3]; vertices.len()];
        for (brush_center, brush_offset) in mirrored_strokes(center_mm, offset_mm, options.symmetry)
        {
            for (index, vertex) in vertices.iter().copied().enumerate() {
                let distance = length(subtract(vertex, brush_center));
                let weight = options.falloff.weight(distance / options.radius_mm);
                total_weight[index] += weight;
                for axis in 0..3 {
                    weighted[index][axis] += weight * brush_offset[axis];
                }
            }
        }
        for index in 0..vertices.len() {
            if total_weight[index] <= 1.0e-12 {
                continue;
            }
            let influence = total_weight[index].min(1.0) * options.strength;
            for axis in 0..3 {
                vertices[index][axis] += weighted[index][axis] / total_weight[index] * influence;
            }
        }
    }
    finish_brush(
        mesh,
        vertices,
        BrushFinishOptions {
            operation: "elastic_grab",
            centers: 1,
            radius: options.radius_mm,
            strength: options.strength,
            maximum_displacement: 2_000.0,
            limits: &options.limits,
        },
    )
}

/// Alias matching the Python operation name.
pub fn elastic_grab_mesh(
    mesh: &Mesh,
    center_mm: [f32; 3],
    offset_mm: [f32; 3],
    options: ElasticGrabOptions,
) -> Result<BrushResult, CoreError> {
    elastic_grab(mesh, center_mm, offset_mm, options)
}

/// Crease depth at full strength, as a fraction of the brush radius.
const CREASE_DEPTH_PER_RADIUS: f32 = 0.25;
/// Fraction of the way to the stroke line a vertex at the line's centre is
/// pinched at full strength (at most one half, so nothing crosses the line).
const CREASE_PINCH: f32 = 0.5;
/// Extra depth at full sharpness, as a fraction of the gentle depth.
const CREASE_SHARP_DEPTH_GAIN: f32 = 0.6;
/// Exponent of the depth profile at zero sharpness (cubed falloff).
const CREASE_DEPTH_EXPONENT: f32 = 3.0;
/// Extra depth-profile exponent at full sharpness (narrower groove).
const CREASE_SHARP_EXPONENT_GAIN: f32 = 4.0;

/// Applies Crease Polish to a local-space stroke path.
pub fn crease_polish(
    mesh: &Mesh,
    centers_mm: &[[f32; 3]],
    options: CreasePolishOptions,
) -> Result<BrushResult, CoreError> {
    validate_mesh_and_limits(mesh, &options.limits, "crease_polish")?;
    validate_radius_strength(options.radius_mm, options.strength, "crease_polish")?;
    if !(0.0..=1.0).contains(&options.sharpness) || !options.sharpness.is_finite() {
        return Err(CoreError::MeshOperation(
            "sharpness must be between 0 and 1".to_owned(),
        ));
    }
    validate_centers(centers_mm)?;
    let work_units = operation_work(mesh, centers_mm.len())?;
    options.limits.check_work("crease_polish", work_units)?;
    // OF-49, modelled on Blender's Crease brush: the stroke acts once as a
    // whole (a long stroke does not dig deeper than a short one), pushes the
    // surface gently along the brush-area normal, and pinches the
    // surrounding surface toward the stroke line. The pinch moves a vertex
    // at most half-way to the line, scaled by a falloff whose slope keeps the
    // order of vertices across the line, so the crease cannot fold over
    // itself.
    let source = mesh.vertices.clone();
    let mut vertices = source.clone();
    let sign = if options.direction == BrushDirection::Inward {
        -1.0
    } else {
        1.0
    };
    if options.strength > 0.0 {
        let normals = vertex_normals(&source, &mesh.faces);
        let area_normals: Vec<[f32; 3]> = centers_mm
            .iter()
            .map(|center| {
                let mut sum = [0.0_f32; 3];
                for (vertex, normal) in source.iter().zip(&normals) {
                    let weight = options
                        .falloff
                        .weight(length(subtract(*vertex, *center)) / options.radius_mm);
                    sum = add(sum, scale(*normal, weight));
                }
                let magnitude = length(sum);
                if magnitude > 1.0e-12 && magnitude.is_finite() {
                    scale(sum, 1.0 / magnitude)
                } else {
                    [0.0; 3]
                }
            })
            .collect();
        let depth = options.radius_mm
            * CREASE_DEPTH_PER_RADIUS
            * options.strength
            * (1.0 + CREASE_SHARP_DEPTH_GAIN * options.sharpness);
        let depth_exponent = CREASE_DEPTH_EXPONENT + CREASE_SHARP_EXPONENT_GAIN * options.sharpness;
        let pinch = CREASE_PINCH * options.strength;
        for (index, vertex) in vertices.iter_mut().enumerate() {
            let Some((nearest, segment)) = nearest_on_path(*vertex, centers_mm) else {
                continue;
            };
            let toward = subtract(nearest, *vertex);
            let distance = length(toward);
            if distance >= options.radius_mm {
                continue;
            }
            let weight = options.falloff.weight(distance / options.radius_mm);
            if weight <= 0.0 {
                continue;
            }
            let mut normal = area_normals[segment];
            if length(normal) <= 0.5 {
                normal = normals[index];
            }
            // Pinch along the surface only; the depth comes from the normal push.
            let tangent = subtract(toward, scale(normal, dot(toward, normal)));
            // The depth is concentrated at the line (cubed falloff, narrower
            // as sharpness rises) so it reads as a crease; the pinch uses the
            // full falloff and draws the wider surrounding surface in toward it.
            let moved = add(
                scale(tangent, pinch * weight),
                scale(normal, sign * depth * weight.powf(depth_exponent)),
            );
            *vertex = add(*vertex, moved);
        }
    }
    finish_brush(
        mesh,
        vertices,
        BrushFinishOptions {
            operation: "crease_polish",
            centers: centers_mm.len(),
            radius: options.radius_mm,
            strength: options.strength,
            maximum_displacement: options.radius_mm,
            limits: &options.limits,
        },
    )
}

/// Alias matching the Python operation name.
pub fn crease_polish_mesh(
    mesh: &Mesh,
    centers_mm: &[[f32; 3]],
    options: CreasePolishOptions,
) -> Result<BrushResult, CoreError> {
    crease_polish(mesh, centers_mm, options)
}

/// Applies local Laplacian Smooth Brush displacement.
pub fn smooth_brush(
    mesh: &Mesh,
    centers_mm: &[[f32; 3]],
    options: SmoothBrushOptions,
) -> Result<BrushResult, CoreError> {
    validate_mesh_and_limits(mesh, &options.limits, "smooth_brush")?;
    validate_radius_strength(options.radius_mm, options.strength, "smooth_brush")?;
    if !(0.0..=1.0).contains(&options.shrinkage) || !options.shrinkage.is_finite() {
        return Err(CoreError::MeshOperation(
            "shrinkage must be between 0 and 1".to_owned(),
        ));
    }
    validate_centers(centers_mm)?;
    let work_units = operation_work(mesh, centers_mm.len())?
        .checked_mul(2)
        .ok_or_else(|| CoreError::MeshOperation("smooth_brush work count overflow".to_owned()))?;
    options.limits.check_work("smooth_brush", work_units)?;
    // OF-50 / D9: Blender's sculpt Smooth (a Laplacian toward the neighbour
    // average, weighted by the brush falloff), with its hard-edge rule so
    // sparse stock shapes keep their form instead of melting: a vertex on a
    // sharp edge only slides along that edge, and a corner stays put.
    let adjacency = feature_neighbors(mesh);
    let source = mesh.vertices.clone();
    let mut vertices = source.clone();
    if options.strength > 0.0 {
        for center in centers_mm {
            let normals = vertex_normals(&vertices, &mesh.faces);
            let mut next = vertices.clone();
            for (index, vertex) in vertices.iter().copied().enumerate() {
                if adjacency[index].is_empty() {
                    continue;
                }
                let mut average = [0.0_f32; 3];
                for neighbor in &adjacency[index] {
                    for axis in 0..3 {
                        average[axis] += vertices[*neighbor][axis] / adjacency[index].len() as f32;
                    }
                }
                let delta = subtract(average, vertex);
                let normal = normals[index];
                let normal_amount = dot(delta, normal);
                let normal_component = scale(normal, normal_amount);
                let tangent = subtract(delta, normal_component);
                let adjusted = add(tangent, scale(normal_component, options.shrinkage));
                let weight = options
                    .falloff
                    .weight(length(subtract(vertex, *center)) / options.radius_mm);
                next[index] = add(vertex, scale(adjusted, weight * options.strength));
            }
            vertices = next;
        }
    }
    finish_brush(
        mesh,
        vertices,
        BrushFinishOptions {
            operation: "smooth_brush",
            centers: centers_mm.len(),
            radius: options.radius_mm,
            strength: options.strength,
            maximum_displacement: options.radius_mm * 2.0,
            limits: &options.limits,
        },
    )
}

/// Alias matching the Python operation name.
pub fn smooth_brush_mesh(
    mesh: &Mesh,
    centers_mm: &[[f32; 3]],
    options: SmoothBrushOptions,
) -> Result<BrushResult, CoreError> {
    smooth_brush(mesh, centers_mm, options)
}

/// Applies topology-preserving normal Blob displacement.
pub fn blob(
    mesh: &Mesh,
    centers_mm: &[[f32; 3]],
    options: BlobOptions,
) -> Result<BrushResult, CoreError> {
    validate_mesh_and_limits(mesh, &options.limits, "blob")?;
    validate_radius_strength(options.radius_mm, options.strength, "blob")?;
    validate_centers(centers_mm)?;
    let work_units = operation_work(mesh, centers_mm.len())?
        .checked_mul(2)
        .ok_or_else(|| CoreError::MeshOperation("blob work count overflow".to_owned()))?;
    options.limits.check_work("blob", work_units)?;
    let source = mesh.vertices.clone();
    let mut vertices = source.clone();
    let sign = if options.direction == BrushDirection::Outward {
        1.0
    } else {
        -1.0
    };
    if options.strength > 0.0 {
        for center in centers_mm {
            let normals = vertex_normals(&vertices, &mesh.faces);
            for (index, vertex) in vertices.iter_mut().enumerate() {
                let weight = options
                    .falloff
                    .weight(length(subtract(*vertex, *center)) / options.radius_mm);
                let amount = sign * options.radius_mm * 0.25 * weight * options.strength;
                *vertex = add(*vertex, scale(normals[index], amount));
            }
        }
    }
    finish_brush(
        mesh,
        vertices,
        BrushFinishOptions {
            operation: "blob",
            centers: centers_mm.len(),
            radius: options.radius_mm,
            strength: options.strength,
            maximum_displacement: options.radius_mm * 2.0,
            limits: &options.limits,
        },
    )
}

/// Alias matching the Python operation name.
pub fn blob_mesh(
    mesh: &Mesh,
    centers_mm: &[[f32; 3]],
    options: BlobOptions,
) -> Result<BrushResult, CoreError> {
    blob(mesh, centers_mm, options)
}

fn validate_mesh_and_limits(
    mesh: &Mesh,
    limits: &MeshOperationLimits,
    operation: &str,
) -> Result<(), CoreError> {
    mesh.validate_non_empty()?;
    limits.validate(operation)?;
    limits.check_mesh(operation, mesh)?;
    Ok(())
}

fn validate_radius_strength(radius: f32, strength: f32, operation: &str) -> Result<(), CoreError> {
    if !radius.is_finite() || !(0.1..=1000.0).contains(&radius) {
        return Err(CoreError::MeshOperation(format!(
            "{operation} radius must be between 0.1 and 1000 mm"
        )));
    }
    if !strength.is_finite() || !(0.0..=1.0).contains(&strength) {
        return Err(CoreError::MeshOperation(format!(
            "{operation} strength must be between 0 and 1"
        )));
    }
    Ok(())
}

fn validate_point(point: [f32; 3], name: &str) -> Result<(), CoreError> {
    if point.iter().any(|value| !value.is_finite()) {
        return Err(CoreError::MeshOperation(format!(
            "{name} must contain three finite numbers"
        )));
    }
    Ok(())
}

fn validate_centers(centers: &[[f32; 3]]) -> Result<(), CoreError> {
    if !(1..=256).contains(&centers.len())
        || centers
            .iter()
            .any(|center| center.iter().any(|value| !value.is_finite()))
    {
        return Err(CoreError::MeshOperation(
            "centers_mm must contain between 1 and 256 finite 3D points".to_owned(),
        ));
    }
    Ok(())
}

fn operation_work(mesh: &Mesh, centers: usize) -> Result<u64, CoreError> {
    u64::try_from(mesh.vertex_count())
        .ok()
        .and_then(|vertices| vertices.checked_mul(centers as u64))
        .and_then(|units| units.checked_mul(8))
        .ok_or_else(|| CoreError::MeshOperation("brush work count overflow".to_owned()))
}

struct BrushFinishOptions<'a> {
    operation: &'a str,
    centers: usize,
    radius: f32,
    strength: f32,
    maximum_displacement: f32,
    limits: &'a MeshOperationLimits,
}

fn finish_brush(
    source: &Mesh,
    mut vertices: Vec<[f32; 3]>,
    finish: BrushFinishOptions<'_>,
) -> Result<BrushResult, CoreError> {
    let mut maximum = 0.0_f32;
    let mut affected = 0_usize;
    for (source_vertex, vertex) in source.vertices.iter().zip(&mut vertices) {
        let displacement = subtract(*vertex, *source_vertex);
        let distance = length(displacement);
        if distance > 1.0e-12 {
            affected = affected.saturating_add(1);
        }
        if distance > maximum {
            maximum = distance;
        }
        if distance > finish.maximum_displacement && distance.is_finite() {
            let clipped = scale(displacement, finish.maximum_displacement / distance);
            *vertex = add(*source_vertex, clipped);
        }
    }
    if vertices.iter().flatten().any(|value| !value.is_finite()) {
        return Err(CoreError::MeshOperation(format!(
            "{} produced non-finite vertex positions",
            finish.operation
        )));
    }
    let result = Mesh::new(vertices, source.faces.clone())?;
    finish.limits.check_mesh(finish.operation, &result)?;
    let final_maximum = source
        .vertices
        .iter()
        .zip(&result.vertices)
        .map(|(first, second)| length(subtract(*second, *first)))
        .fold(0.0_f32, f32::max);
    Ok(BrushResult {
        report: BrushReport {
            operation: finish.operation.to_owned(),
            changed: result != *source,
            centers: finish.centers,
            affected_vertices: affected,
            maximum_displacement_mm: final_maximum,
            radius_mm: finish.radius,
            strength: finish.strength,
            warnings: Vec::new(),
        },
        mesh: result,
    })
}

/// Nearest point on the stroke polyline (or the single point) and the index
/// of the stroke centre that segment starts at.
fn nearest_on_path(point: [f32; 3], path: &[[f32; 3]]) -> Option<([f32; 3], usize)> {
    let first = *path.first()?;
    let mut best = (first, 0_usize, length(subtract(point, first)));
    for (index, pair) in path.windows(2).enumerate() {
        let segment = subtract(pair[1], pair[0]);
        let squared = dot(segment, segment);
        let along = if squared > 1.0e-12 {
            (dot(subtract(point, pair[0]), segment) / squared).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let candidate = add(pair[0], scale(segment, along));
        let distance = length(subtract(point, candidate));
        if distance < best.2 {
            let nearer = if along > 0.5 { index + 1 } else { index };
            best = (candidate, nearer, distance);
        }
    }
    Some((best.0, best.1))
}

fn mirrored_strokes(
    center: [f32; 3],
    offset: [f32; 3],
    symmetry: BrushSymmetry,
) -> Vec<([f32; 3], [f32; 3])> {
    let axes = symmetry.axes();
    let mut result = Vec::with_capacity(1_usize << axes.len());
    for mask in 0..(1_usize << axes.len()) {
        let mut mirror = [1.0_f32; 3];
        for (bit, axis) in axes.iter().copied().enumerate() {
            if mask & (1_usize << bit) != 0 {
                mirror[axis] = -1.0;
            }
        }
        result.push((
            [
                center[0] * mirror[0],
                center[1] * mirror[1],
                center[2] * mirror[2],
            ],
            [
                offset[0] * mirror[0],
                offset[1] * mirror[1],
                offset[2] * mirror[2],
            ],
        ));
    }
    result
}

fn vertex_adjacency(mesh: &Mesh) -> Vec<Vec<usize>> {
    let mut adjacency = vec![BTreeSet::<usize>::new(); mesh.vertex_count()];
    for face in &mesh.faces {
        let pairs = [
            (face[0] as usize, face[1] as usize),
            (face[1] as usize, face[0] as usize),
            (face[1] as usize, face[2] as usize),
            (face[2] as usize, face[1] as usize),
            (face[2] as usize, face[0] as usize),
            (face[0] as usize, face[2] as usize),
        ];
        for (first, second) in pairs {
            if first == second {
                continue;
            }
            if let Some(neighbors) = adjacency.get_mut(first) {
                neighbors.insert(second);
            }
        }
    }
    adjacency
        .into_iter()
        .map(|set| set.into_iter().collect())
        .collect()
}

/// Cosine of the angle between face normals above which an edge counts as
/// sharp (60°): stock rims and box edges are 90°, sculpted bumps are softer.
const SHARP_EDGE_COSINE: f32 = 0.5;

/// Cosine of the largest turn a sharp edge line may make at a vertex and
/// still let the vertex slide along it (30°). A sharper turn is a corner and
/// stays put: an open square's corners (90°) and a hexagon's rim (60°) keep
/// their shape, while a stock cylinder's rim (about 3° per segment) slides.
const EDGE_LINE_TURN_COSINE: f64 = 0.866_025_4;

/// Smoothing neighbours that respect sharp features: all neighbours on a
/// smooth surface; the two neighbours along the edge for a vertex on one
/// sharp edge line (including open borders) that turns by at most 30°
/// there; none for a corner, where one or three or more sharp edges meet or
/// the line turns more sharply. Degenerate faces add no edges and no normal.
pub(crate) fn feature_neighbors(mesh: &Mesh) -> Vec<Vec<usize>> {
    let face_normals: Vec<Option<[f64; 3]>> = mesh
        .faces
        .iter()
        .map(|face| unit_face_normal(&mesh.vertices, *face))
        .collect();
    let mut edges = BTreeMap::<(u32, u32), Vec<usize>>::new();
    for (index, face) in mesh.faces.iter().enumerate() {
        if face_normals[index].is_none() {
            continue;
        }
        for (first, second) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            edges
                .entry((first.min(second), first.max(second)))
                .or_default()
                .push(index);
        }
    }
    let mut sharp = vec![Vec::<usize>::new(); mesh.vertex_count()];
    for ((first, second), faces) in &edges {
        let is_sharp = match faces.as_slice() {
            [one, two] => match (face_normals[*one], face_normals[*two]) {
                (Some(a), Some(b)) => dot64(a, b) < f64::from(SHARP_EDGE_COSINE),
                _ => true,
            },
            _ => true,
        };
        if is_sharp {
            if let Some(list) = sharp.get_mut(*first as usize) {
                list.push(*second as usize);
            }
            if let Some(list) = sharp.get_mut(*second as usize) {
                list.push(*first as usize);
            }
        }
    }
    vertex_adjacency(mesh)
        .into_iter()
        .zip(sharp)
        .enumerate()
        .map(|(index, (all, sharp))| match sharp.len() {
            0 => all,
            2 if edge_line_is_straight_enough(&mesh.vertices, sharp[0], index, sharp[1]) => sharp,
            _ => Vec::new(),
        })
        .collect()
}

/// Whether the edge line `before -> at -> after` turns by at most 30° at
/// `at`. Computed in f64 from directions only, so mesh scale does not matter.
fn edge_line_is_straight_enough(
    vertices: &[[f32; 3]],
    before: usize,
    at: usize,
    after: usize,
) -> bool {
    let (Some(before), Some(at), Some(after)) =
        (vertices.get(before), vertices.get(at), vertices.get(after))
    else {
        return false;
    };
    let incoming = unit64(sub64(*at, *before));
    let outgoing = unit64(sub64(*after, *at));
    match (incoming, outgoing) {
        (Some(incoming), Some(outgoing)) => dot64(incoming, outgoing) >= EDGE_LINE_TURN_COSINE,
        _ => false,
    }
}

/// Unit face normal in f64, or `None` for a face with repeated or missing
/// vertices or no area. Relative, not absolute, so tiny meshes keep normals.
fn unit_face_normal(vertices: &[[f32; 3]], face: [u32; 3]) -> Option<[f64; 3]> {
    if face[0] == face[1] || face[1] == face[2] || face[2] == face[0] {
        return None;
    }
    let a = *vertices.get(face[0] as usize)?;
    let b = *vertices.get(face[1] as usize)?;
    let c = *vertices.get(face[2] as usize)?;
    let first = sub64(b, a);
    let second = sub64(c, a);
    let cross = [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ];
    let scale = (dot64(first, first) * dot64(second, second)).sqrt();
    let magnitude = dot64(cross, cross).sqrt();
    if !magnitude.is_finite() || scale <= 0.0 || magnitude <= scale * 1.0e-12 {
        return None;
    }
    Some(cross.map(|value| value / magnitude))
}

fn sub64(first: [f32; 3], second: [f32; 3]) -> [f64; 3] {
    [0, 1, 2].map(|axis| f64::from(first[axis]) - f64::from(second[axis]))
}

fn dot64(first: [f64; 3], second: [f64; 3]) -> f64 {
    first[0] * second[0] + first[1] * second[1] + first[2] * second[2]
}

fn unit64(value: [f64; 3]) -> Option<[f64; 3]> {
    let magnitude = dot64(value, value).sqrt();
    (magnitude.is_finite() && magnitude > 0.0).then(|| value.map(|item| item / magnitude))
}

fn vertex_normals(vertices: &[[f32; 3]], faces: &[[u32; 3]]) -> Vec<[f32; 3]> {
    let mut normals = vec![[0.0_f32; 3]; vertices.len()];
    for face in faces {
        let Some(first) = vertices.get(face[0] as usize) else {
            continue;
        };
        let Some(second) = vertices.get(face[1] as usize) else {
            continue;
        };
        let Some(third) = vertices.get(face[2] as usize) else {
            continue;
        };
        let normal = cross(subtract(*second, *first), subtract(*third, *first));
        for index in face {
            if let Some(value) = normals.get_mut(*index as usize) {
                *value = add(*value, normal);
            }
        }
    }
    for normal in &mut normals {
        let magnitude = length(*normal);
        if magnitude > 1.0e-12 && magnitude.is_finite() {
            *normal = scale(*normal, 1.0 / magnitude);
        } else {
            *normal = [0.0; 3];
        }
    }
    normals
}

fn add(first: [f32; 3], second: [f32; 3]) -> [f32; 3] {
    [
        first[0] + second[0],
        first[1] + second[1],
        first[2] + second[2],
    ]
}

fn subtract(first: [f32; 3], second: [f32; 3]) -> [f32; 3] {
    [
        first[0] - second[0],
        first[1] - second[1],
        first[2] - second[2],
    ]
}

fn scale(value: [f32; 3], factor: f32) -> [f32; 3] {
    [value[0] * factor, value[1] * factor, value[2] * factor]
}

fn dot(first: [f32; 3], second: [f32; 3]) -> f32 {
    first[0] * second[0] + first[1] * second[1] + first[2] * second[2]
}

fn cross(first: [f32; 3], second: [f32; 3]) -> [f32; 3] {
    [
        first[1] * second[2] - first[2] * second[1],
        first[2] * second[0] - first[0] * second[2],
        first[0] * second[1] - first[1] * second[0],
    ]
}

fn length(value: [f32; 3]) -> f32 {
    dot(value, value).sqrt()
}
