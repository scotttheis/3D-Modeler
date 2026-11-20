//! On-surface area-of-effect outline for the radial sculpt brushes.
//!
//! The brush acts on every vertex within `radius_mm` of the stroke in the
//! object's own millimetre space, so the outline is a disc of that radius laid
//! on the surface tangent plane under the pointer, projected to the screen.
//! Pure presentation: it never touches the model.

use eframe::egui::{self, Color32, Pos2, Shape, Stroke};

/// Segments in the outline circle; enough to look round at any zoom.
const SEGMENTS: usize = 64;

/// Points around a disc of `radius` at `center`, perpendicular to `normal`,
/// in the same space as the inputs. Returns `None` for a degenerate normal or
/// a non-positive radius.
#[must_use]
pub(crate) fn disc_points(
    center: [f32; 3],
    normal: [f32; 3],
    radius: f32,
) -> Option<Vec<[f32; 3]>> {
    if !(radius.is_finite() && radius > 0.0) {
        return None;
    }
    let n = normalized(normal)?;
    // Any vector not parallel to the normal seeds the tangent basis.
    let seed = if n[0].abs() < 0.9 {
        [1.0, 0.0, 0.0]
    } else {
        [0.0, 1.0, 0.0]
    };
    let u = normalized(cross(n, seed))?;
    let v = cross(n, u);
    Some(
        (0..SEGMENTS)
            .map(|step| {
                let angle = step as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                let (sin, cos) = angle.sin_cos();
                [
                    center[0] + radius * (cos * u[0] + sin * v[0]),
                    center[1] + radius * (cos * u[1] + sin * v[1]),
                    center[2] + radius * (cos * u[2] + sin * v[2]),
                ]
            })
            .collect(),
    )
}

/// Unit normal of a triangle, or `None` when it has no area.
#[must_use]
pub(crate) fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Option<[f32; 3]> {
    normalized(cross(sub(b, a), sub(c, a)))
}

/// Paints the projected outline: a soft fill, a dashed rim with a contrasting
/// halo so it reads on any material colour, and a centre dot. `armed` is true
/// while Shift is held or a stroke is in progress; otherwise it is drawn
/// fainter as an aiming guide.
pub(crate) fn paint(
    painter: &egui::Painter,
    outline: &[Pos2],
    center: Pos2,
    accent: Color32,
    halo: Color32,
    armed: bool,
) {
    if outline.len() < 3 {
        return;
    }
    let strength = if armed { 1.0 } else { 0.55 };
    painter.add(Shape::convex_polygon(
        outline.to_vec(),
        accent.gamma_multiply(0.14 * strength),
        Stroke::NONE,
    ));
    let mut closed = outline.to_vec();
    closed.push(outline[0]);
    painter.add(Shape::closed_line(
        outline.to_vec(),
        Stroke::new(3.0, halo.gamma_multiply(0.45 * strength)),
    ));
    painter.extend(Shape::dashed_line(
        &closed,
        Stroke::new(1.6, accent.gamma_multiply(strength)),
        6.0,
        4.0,
    ));
    painter.circle_filled(center, 2.5, halo.gamma_multiply(0.6 * strength));
    painter.circle_filled(center, 1.6, accent.gamma_multiply(strength));
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalized(value: [f32; 3]) -> Option<[f32; 3]> {
    let length = (value[0] * value[0] + value[1] * value[1] + value[2] * value[2]).sqrt();
    (length.is_finite() && length > 1.0e-12).then(|| value.map(|part| part / length))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disc_points_lie_on_the_tangent_circle() {
        let center = [1.0, 2.0, 3.0];
        let normal = [0.0, 0.0, 2.0];
        let points = disc_points(center, normal, 5.0).unwrap_or_default();
        assert_eq!(points.len(), SEGMENTS);
        for point in points {
            let delta = sub(point, center);
            let distance = (delta[0].powi(2) + delta[1].powi(2) + delta[2].powi(2)).sqrt();
            assert!((distance - 5.0).abs() < 1.0e-4);
            assert!(
                delta[2].abs() < 1.0e-5,
                "point must stay in the tangent plane"
            );
        }
    }

    #[test]
    fn degenerate_inputs_draw_nothing() {
        assert!(disc_points([0.0; 3], [0.0; 3], 5.0).is_none());
        assert!(disc_points([0.0; 3], [1.0, 0.0, 0.0], 0.0).is_none());
        assert!(face_normal([0.0; 3], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn face_normal_follows_winding() {
        let normal = face_normal([0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]);
        assert_eq!(normal, Some([0.0, 0.0, 1.0]));
    }
}
