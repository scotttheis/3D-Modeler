//! Distance measuring shared by the desktop ruler and MCP `measure` (OF-55).
//!
//! Everything here is world-space millimetres and read-only: measuring never
//! changes the scene and never records history. The desktop ruler picks its
//! two ends on surfaces (snapped to corners and edges in screen space by the
//! shell) and reports them through [`measure_points`]; MCP may also name an
//! object as an end, in which case the end is the closest point on that
//! object's surface ([`closest_point_on_mesh`], [`closest_points_between_meshes`]).

use crate::{CoreError, Mesh};

/// Upper bound on triangle-pair checks for an object-to-object measurement.
/// Pairs are pruned by bounding spheres, so ordinary models stay far below it.
pub const MEASURE_MAX_PAIR_CHECKS: u64 = 400_000_000;

/// One measured distance between two world points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub struct Measurement {
    /// Start of the ruler (millimetres).
    pub from_mm: [f32; 3],
    /// End of the ruler (millimetres).
    pub to_mm: [f32; 3],
    /// `to - from` on each world axis (millimetres, signed).
    pub delta_mm: [f32; 3],
    /// Straight-line length (millimetres).
    pub distance_mm: f32,
}

/// Measures the straight line between two world points.
pub fn measure_points(from_mm: [f32; 3], to_mm: [f32; 3]) -> Result<Measurement, CoreError> {
    if !from_mm
        .iter()
        .chain(to_mm.iter())
        .all(|value| value.is_finite())
    {
        return Err(CoreError::MeshOperation(String::from(
            "measure points must be finite millimetre coordinates",
        )));
    }
    let delta_mm = sub(to_mm, from_mm);
    Ok(Measurement {
        from_mm,
        to_mm,
        delta_mm,
        distance_mm: length(delta_mm),
    })
}

/// The point on `mesh`'s surface nearest to `point_mm`.
pub fn closest_point_on_mesh(mesh: &Mesh, point_mm: [f32; 3]) -> Result<[f32; 3], CoreError> {
    mesh.validate_non_empty()?;
    let mut best: Option<([f32; 3], f32)> = None;
    for face in &mesh.faces {
        let [a, b, c] = triangle(mesh, *face)?;
        let candidate = closest_point_on_triangle(point_mm, a, b, c);
        let distance = length_squared(sub(candidate, point_mm));
        if best.is_none_or(|(_, current)| distance < current) {
            best = Some((candidate, distance));
        }
    }
    best.map(|(point, _)| point)
        .ok_or_else(|| CoreError::MeshOperation(String::from("measure needs an object with faces")))
}

/// The closest pair of surface points between two meshes, first on `first`.
///
/// Touching or crossing surfaces give a distance of zero at a shared point.
/// A mesh fully inside another is measured to the outer one's surface.
pub fn closest_points_between_meshes(
    first: &Mesh,
    second: &Mesh,
) -> Result<([f32; 3], [f32; 3]), CoreError> {
    first.validate_non_empty()?;
    second.validate_non_empty()?;
    let mut first_triangles = bounded_triangles(first)?;
    let mut second_triangles = bounded_triangles(second)?;
    // Check the faces nearest the other mesh first so the best distance
    // shrinks early and the bounding-sphere test prunes most pairs.
    let first_center = mesh_center(first);
    let second_center = mesh_center(second);
    first_triangles.sort_by(|left, right| {
        length_squared(sub(left.center, second_center))
            .total_cmp(&length_squared(sub(right.center, second_center)))
    });
    second_triangles.sort_by(|left, right| {
        length_squared(sub(left.center, first_center))
            .total_cmp(&length_squared(sub(right.center, first_center)))
    });
    let pair_budget = (first_triangles.len() as u64).saturating_mul(second_triangles.len() as u64);
    if pair_budget > MEASURE_MAX_PAIR_CHECKS {
        return Err(CoreError::OperationResourceLimit {
            operation: String::from("measure"),
            reason: format!(
                "{} face pairs are above the {MEASURE_MAX_PAIR_CHECKS} limit; measure between points instead",
                pair_budget
            ),
        });
    }
    let mut best: Option<([f32; 3], [f32; 3], f32)> = None;
    for left in &first_triangles {
        for right in &second_triangles {
            if let Some((_, _, current)) = best {
                let gap = length(sub(left.center, right.center)) - left.radius - right.radius;
                if gap > 0.0 && gap * gap >= current {
                    continue;
                }
            }
            let (on_left, on_right) = closest_points_between_triangles(left.corners, right.corners);
            let distance = length_squared(sub(on_left, on_right));
            if best.is_none_or(|(_, _, current)| distance < current) {
                best = Some((on_left, on_right, distance));
                if distance <= 0.0 {
                    return Ok((on_left, on_right));
                }
            }
        }
    }
    best.map(|(left, right, _)| (left, right))
        .ok_or_else(|| CoreError::MeshOperation(String::from("measure needs objects with faces")))
}

#[derive(Clone, Copy)]
struct BoundedTriangle {
    corners: [[f32; 3]; 3],
    center: [f32; 3],
    radius: f32,
}

fn bounded_triangles(mesh: &Mesh) -> Result<Vec<BoundedTriangle>, CoreError> {
    mesh.faces
        .iter()
        .map(|face| {
            let corners = triangle(mesh, *face)?;
            let center = scale(add(add(corners[0], corners[1]), corners[2]), 1.0 / 3.0);
            let radius = corners
                .iter()
                .map(|corner| length(sub(*corner, center)))
                .fold(0.0_f32, f32::max);
            Ok(BoundedTriangle {
                corners,
                center,
                radius,
            })
        })
        .collect()
}

fn mesh_center(mesh: &Mesh) -> [f32; 3] {
    mesh.bounds().map_or([0.0; 3], |(minimum, maximum)| {
        scale(add(minimum, maximum), 0.5)
    })
}

fn triangle(mesh: &Mesh, face: [u32; 3]) -> Result<[[f32; 3]; 3], CoreError> {
    let corner = |index: u32| {
        mesh.vertices
            .get(index as usize)
            .copied()
            .ok_or_else(|| CoreError::InvalidMesh(format!("face index {index} is out of range")))
    };
    Ok([corner(face[0])?, corner(face[1])?, corner(face[2])?])
}

/// Closest pair between two triangles: the minimum over each edge of one
/// against the other triangle (this covers vertex-face and edge-edge cases).
fn closest_points_between_triangles(
    first: [[f32; 3]; 3],
    second: [[f32; 3]; 3],
) -> ([f32; 3], [f32; 3]) {
    let mut best = (first[0], second[0], f32::INFINITY);
    for edge in 0..3 {
        let (start, end) = (first[edge], first[(edge + 1) % 3]);
        let (on_segment, on_triangle) =
            closest_points_segment_triangle(start, end, second[0], second[1], second[2]);
        let distance = length_squared(sub(on_segment, on_triangle));
        if distance < best.2 {
            best = (on_segment, on_triangle, distance);
        }
        let (start, end) = (second[edge], second[(edge + 1) % 3]);
        let (on_segment, on_triangle) =
            closest_points_segment_triangle(start, end, first[0], first[1], first[2]);
        let distance = length_squared(sub(on_segment, on_triangle));
        if distance < best.2 {
            best = (on_triangle, on_segment, distance);
        }
    }
    (best.0, best.1)
}

fn closest_points_segment_triangle(
    start: [f32; 3],
    end: [f32; 3],
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
) -> ([f32; 3], [f32; 3]) {
    if let Some(hit) = segment_crosses_triangle(start, end, a, b, c) {
        return (hit, hit);
    }
    let mut best = {
        let on_triangle = closest_point_on_triangle(start, a, b, c);
        (start, on_triangle, length_squared(sub(start, on_triangle)))
    };
    let on_triangle = closest_point_on_triangle(end, a, b, c);
    let distance = length_squared(sub(end, on_triangle));
    if distance < best.2 {
        best = (end, on_triangle, distance);
    }
    for (edge_start, edge_end) in [(a, b), (b, c), (c, a)] {
        let (on_segment, on_edge) = closest_points_segments(start, end, edge_start, edge_end);
        let distance = length_squared(sub(on_segment, on_edge));
        if distance < best.2 {
            best = (on_segment, on_edge, distance);
        }
    }
    (best.0, best.1)
}

fn segment_crosses_triangle(
    start: [f32; 3],
    end: [f32; 3],
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
) -> Option<[f32; 3]> {
    let direction = sub(end, start);
    let edge1 = sub(b, a);
    let edge2 = sub(c, a);
    let normal = cross(edge1, edge2);
    let denominator = dot(normal, direction);
    if denominator.abs() <= f32::EPSILON * length_squared(normal).max(1.0) {
        return None;
    }
    let amount = dot(normal, sub(a, start)) / denominator;
    if !(0.0..=1.0).contains(&amount) {
        return None;
    }
    let point = add(start, scale(direction, amount));
    let inside = |p: [f32; 3], q: [f32; 3]| dot(cross(sub(q, p), sub(point, p)), normal) >= 0.0;
    (inside(a, b) && inside(b, c) && inside(c, a)).then_some(point)
}

/// Closest point on a triangle (Ericson, Real-Time Collision Detection 5.1.5).
fn closest_point_on_triangle(p: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return add(a, scale(ab, v));
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return add(a, scale(ac, w));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return add(b, scale(sub(c, b), w));
    }
    let total = va + vb + vc;
    if total.abs() <= f32::EPSILON {
        return a;
    }
    let v = vb / total;
    let w = vc / total;
    add(a, add(scale(ab, v), scale(ac, w)))
}

/// Closest points between two segments (Ericson 5.1.9).
fn closest_points_segments(
    p1: [f32; 3],
    q1: [f32; 3],
    p2: [f32; 3],
    q2: [f32; 3],
) -> ([f32; 3], [f32; 3]) {
    let d1 = sub(q1, p1);
    let d2 = sub(q2, p2);
    let r = sub(p1, p2);
    let a = dot(d1, d1);
    let e = dot(d2, d2);
    let f = dot(d2, r);
    let epsilon = 1.0e-12;
    let (s, t) = if a <= epsilon && e <= epsilon {
        (0.0, 0.0)
    } else if a <= epsilon {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = dot(d1, r);
        if e <= epsilon {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = dot(d1, d2);
            let denominator = a * e - b * b;
            let mut s = if denominator > epsilon {
                ((b * f - c * e) / denominator).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };
    (add(p1, scale(d1, s)), add(p2, scale(d2, t)))
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f32; 3], amount: f32) -> [f32; 3] {
    [a[0] * amount, a[1] * amount, a[2] * amount]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length_squared(a: [f32; 3]) -> f32 {
    dot(a, a)
}

fn length(a: [f32; 3]) -> f32 {
    length_squared(a).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Transform, make_box, make_sphere};

    fn mesh(result: Result<Mesh, CoreError>) -> Mesh {
        result.unwrap_or_else(|error| panic!("mesh: {error}"))
    }

    fn moved(mesh: &Mesh, offset: [f32; 3]) -> Mesh {
        let transform = Transform::identity()
            .translated(offset)
            .unwrap_or_else(|error| panic!("transform: {error}"));
        mesh.transformed(&transform)
            .unwrap_or_else(|error| panic!("transformed: {error}"))
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0e-3
    }

    #[test]
    fn point_measurement_reports_length_and_signed_axis_parts() {
        let measured = measure_points([1.0, 2.0, 3.0], [4.0, -2.0, 3.0])
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(close(measured.distance_mm, 5.0));
        assert_eq!(measured.delta_mm, [3.0, -4.0, 0.0]);
        assert!(measure_points([f32::NAN, 0.0, 0.0], [0.0; 3]).is_err());
    }

    #[test]
    fn closest_point_on_a_cube_lands_on_the_nearest_face_edge_or_corner() {
        let cube = mesh(make_box([20.0, 20.0, 20.0]));
        let face = closest_point_on_mesh(&cube, [30.0, 0.0, 0.0])
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(close(face[0], 10.0) && close(face[1], 0.0) && close(face[2], 0.0));
        let corner = closest_point_on_mesh(&cube, [30.0, 30.0, 30.0])
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(corner.iter().all(|value| close(*value, 10.0)));
    }

    #[test]
    fn two_cubes_side_by_side_measure_their_gap() {
        let cube = mesh(make_box([20.0, 20.0, 20.0]));
        let other = moved(&cube, [35.0, 5.0, 0.0]);
        let (on_first, on_second) =
            closest_points_between_meshes(&cube, &other).unwrap_or_else(|error| panic!("{error}"));
        let gap = measure_points(on_first, on_second).unwrap_or_else(|error| panic!("{error}"));
        assert!(close(gap.distance_mm, 15.0), "gap {}", gap.distance_mm);
        assert!(close(on_first[0], 10.0) && close(on_second[0], 25.0));
    }

    #[test]
    fn edge_to_edge_and_crossing_shapes_are_measured_exactly() {
        let cube = mesh(make_box([20.0, 20.0, 20.0]));
        // Diagonal neighbour: closest features are two parallel edges.
        let diagonal = moved(&cube, [30.0, 0.0, 30.0]);
        let (a, b) = closest_points_between_meshes(&cube, &diagonal)
            .unwrap_or_else(|error| panic!("{error}"));
        let gap = measure_points(a, b).unwrap_or_else(|error| panic!("{error}"));
        assert!(
            close(gap.distance_mm, 200.0_f32.sqrt()),
            "gap {}",
            gap.distance_mm
        );
        // Overlapping shapes touch: zero.
        let sphere = mesh(make_sphere(12.0, 2));
        let crossing = moved(&sphere, [15.0, 0.0, 0.0]);
        let (a, b) = closest_points_between_meshes(&cube, &crossing)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(close(
            measure_points(a, b).map_or(1.0, |m| m.distance_mm),
            0.0
        ));
    }
}
