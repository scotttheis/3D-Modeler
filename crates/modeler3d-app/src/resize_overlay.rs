//! Thin resize guides on the camera-facing selection bounds. Guide strokes and
//! open arrowheads use the viewport depth buffer. The screen rectangle and
//! corner grips remain in the UI. One layout supplies drawing and picking.

use eframe::egui::{self, Color32, Stroke, Vec2};
use modeler3d_viewport::{GizmoLine, Mat4, add, dot, scale};

use crate::transform_gesture::{ResizeFrame, ResizeHandle};

/// 3D Builder's lime green for horizontal (X) handles.
const HANDLE_GREEN: Color32 = Color32::from_rgb(118, 214, 46);
/// 3D Builder's sky blue for vertical (Y) and depth (Z) handles.
const HANDLE_BLUE: Color32 = Color32::from_rgb(36, 162, 238);
/// Preserve the established spacing and generous pointer reach.
const FACE_HANDLE_SCALE: f32 = 2.0;
/// Gap between the box outline and a face handle's centre, in points.
const FACE_HANDLE_GAP: f32 = 16.0 * FACE_HANDLE_SCALE;
/// Pointer reach of a face handle and of a corner dot, in points.
const FACE_HIT_RADIUS: f32 = 14.0 * FACE_HANDLE_SCALE;
const CORNER_HIT_RADIUS: f32 = 9.0;
/// A face axis at least this aligned with the view direction is "seen
/// end-on": its handle sits on the face instead of outside the box, and the
/// far one is hidden.
const END_ON_ALIGNMENT: f32 = 0.8;

/// One face handle as laid out on screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct FaceHandle {
    pub(crate) handle: ResizeHandle,
    pub(crate) axis: usize,
    /// Centre of the open arrowhead, in viewport points.
    pub(crate) center: egui::Pos2,
    /// Unit screen direction the arrow points (outward along the axis).
    pub(crate) direction: Vec2,
}

/// The Resize overlay for one frame, in viewport points.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ResizeOverlay {
    /// Screen-aligned rectangle around the projected box.
    pub(crate) bounds: egui::Rect,
    /// The four corner dots and the uniform-scale corner each one drives.
    pub(crate) corners: Vec<(ResizeHandle, egui::Pos2)>,
    /// Visible face handles.
    pub(crate) faces: Vec<FaceHandle>,
    /// World endpoints of each camera-facing guide, `[minus, plus]`.
    face_centers: [Option<[[f32; 3]; 2]>; 3],
}

impl ResizeOverlay {
    /// Lays out the overlay for `frame`, optionally moved by an in-progress
    /// drag `delta`. `project` maps a world point to viewport points;
    /// `view_direction` is the camera's unit forward vector. Returns `None`
    /// when any box corner cannot be projected (behind the camera).
    pub(crate) fn layout(
        frame: &ResizeFrame,
        delta: Option<Mat4>,
        project: impl Fn([f32; 3]) -> Option<egui::Pos2>,
        view_direction: [f32; 3],
    ) -> Option<Self> {
        let moved = |point: [f32; 3]| {
            delta
                .and_then(|matrix| matrix.transform_point(point).ok())
                .unwrap_or(point)
        };
        let mut projected = Vec::with_capacity(8);
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    let sides = [x, y, z];
                    projected.push((sides, project(moved(frame.point(sides)))?));
                }
            }
        }
        let mut bounds = egui::Rect::NOTHING;
        for (_, point) in &projected {
            bounds.extend_with(*point);
        }
        let corners = [
            bounds.left_top(),
            bounds.right_top(),
            bounds.left_bottom(),
            bounds.right_bottom(),
        ]
        .into_iter()
        .filter_map(|corner| {
            projected
                .iter()
                .min_by(|a, b| a.1.distance(corner).total_cmp(&b.1.distance(corner)))
                .map(|(sides, _)| (ResizeHandle::Corner(*sides), corner))
        })
        .collect();

        let largest = frame.extents.iter().copied().fold(0.0_f32, f32::max);
        let probe = (largest * 0.05).max(1.0e-3);
        let mut faces = Vec::with_capacity(6);
        let mut face_centers = [None; 3];
        for (axis, face_slot) in face_centers.iter_mut().enumerate() {
            let mut centers = [[0.0; 3]; 2];
            // Place each guide on the most camera-facing parallel box face.
            // A small clearance prevents coplanar flicker without lifting it
            // through another object in front of the selection.
            let front_axis = (0..3).filter(|other| *other != axis).max_by(|a, b| {
                dot(frame.axes[*a], view_direction)
                    .abs()
                    .total_cmp(&dot(frame.axes[*b], view_direction).abs())
            })?;
            let front_sign = -dot(frame.axes[front_axis], view_direction).signum();
            let front_offset = scale(
                frame.axes[front_axis],
                front_sign * (frame.extents[front_axis] * 0.5 + largest * 0.002),
            );
            let mut complete = true;
            for (slot, side) in [-1.0_f32, 1.0].into_iter().enumerate() {
                let outward = scale(frame.axes[axis], side);
                let world = add(
                    add(frame.center, front_offset),
                    scale(outward, frame.extents[axis] * 0.5),
                );
                let (Some(center), Some(ahead)) = (
                    project(moved(world)),
                    project(moved(add(world, scale(outward, probe)))),
                ) else {
                    complete = false;
                    continue;
                };
                centers[slot] = moved(world);
                let screen = ahead - center;
                let length = screen.length();
                if !length.is_finite() || length <= 1.0e-4 {
                    continue;
                }
                let direction = screen / length;
                let alignment = dot(outward, view_direction);
                if alignment > END_ON_ALIGNMENT {
                    // Points away from the viewer: hidden behind the object.
                    continue;
                }
                let center = if alignment < -END_ON_ALIGNMENT {
                    // Points at the viewer: sits on the face, as 3D Builder's
                    // depth handle does.
                    center + direction * (FACE_HANDLE_GAP * 0.6)
                } else {
                    center
                        + direction * (exit_distance(bounds, center, direction) + FACE_HANDLE_GAP)
                };
                faces.push(FaceHandle {
                    handle: ResizeHandle::Face { axis, side },
                    axis,
                    center,
                    direction,
                });
            }
            if complete {
                *face_slot = Some(centers);
            }
        }
        Some(Self {
            bounds,
            corners,
            faces,
            face_centers,
        })
    }

    /// The handle under `pointer`, nearest first.
    pub(crate) fn handle_at(&self, pointer: egui::Pos2) -> Option<ResizeHandle> {
        let faces = self
            .faces
            .iter()
            .map(|face| (face.handle, pointer.distance(face.center), FACE_HIT_RADIUS));
        let corners = self
            .corners
            .iter()
            .map(|(handle, point)| (*handle, pointer.distance(*point), CORNER_HIT_RADIUS));
        faces
            .chain(corners)
            .filter(|(_, distance, radius)| distance <= radius)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(handle, _, _)| handle)
    }

    /// Screen points that grab each handle, for tests and diagnostics.
    #[cfg(test)]
    pub(crate) fn grab_points(&self) -> impl Iterator<Item = (ResizeHandle, egui::Pos2)> + '_ {
        self.faces
            .iter()
            .map(|face| (face.handle, face.center))
            .chain(self.corners.iter().copied())
    }

    /// World strokes for both GPU and CPU depth-tested presentation.
    pub(crate) fn depth_lines(
        &self,
        matrix: Mat4,
        rect: egui::Rect,
        emphasized: Option<ResizeHandle>,
    ) -> Vec<GizmoLine> {
        let Ok(inverse) = matrix.inverse() else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        for (axis, centers) in self.face_centers.iter().enumerate() {
            let Some([minus, plus]) = centers else {
                continue;
            };
            let color = if axis == 0 { HANDLE_GREEN } else { HANDLE_BLUE };
            let color = egui::Rgba::from(color).to_array();
            lines.push(GizmoLine {
                start: *minus,
                end: *plus,
                color,
                width_pixels: 1.0,
            });
            for face in self.faces.iter().filter(|face| face.axis == axis) {
                let ResizeHandle::Face { side, .. } = face.handle else {
                    continue;
                };
                let anchor = if side < 0.0 { *minus } else { *plus };
                let Ok(projected) = matrix.transform_point(anchor) else {
                    continue;
                };
                let world = |point: egui::Pos2| {
                    inverse
                        .transform_point([
                            (point.x - rect.left()) / rect.width() * 2.0 - 1.0,
                            1.0 - (point.y - rect.top()) / rect.height() * 2.0,
                            projected[2],
                        ])
                        .ok()
                };
                let size = if emphasized == Some(face.handle) {
                    9.0
                } else {
                    7.0
                };
                let tip = face.center + face.direction * size;
                let base = face.center - face.direction * size;
                let across = face.direction.rot90() * size * 0.65;
                for (start, end) in [
                    (world(base), world(tip)),
                    (world(base + across), world(tip)),
                    (world(base - across), world(tip)),
                ] {
                    if let (Some(start), Some(end)) = (start, end) {
                        lines.push(GizmoLine {
                            start,
                            end,
                            color,
                            width_pixels: 1.0,
                        });
                    }
                }
                if let Some(end) = world(base) {
                    lines.push(GizmoLine {
                        start: anchor,
                        end,
                        color,
                        width_pixels: 1.0,
                    });
                }
            }
        }
        lines
    }

    /// Paints the screen frame. `emphasized` is the hovered or dragged grip.
    pub(crate) fn paint(&self, painter: &egui::Painter, emphasized: Option<ResizeHandle>) {
        painter.rect_stroke(
            self.bounds,
            0.0,
            Stroke::new(1.5, Color32::WHITE),
            egui::StrokeKind::Middle,
        );
        for (handle, point) in &self.corners {
            let radius = if emphasized == Some(*handle) {
                5.0
            } else {
                4.0
            };
            painter.circle(
                *point,
                radius,
                Color32::WHITE,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(0, 0, 0, 60)),
            );
        }
    }
}

/// How far a ray from `point` along unit `direction` travels before it
/// leaves `bounds`; zero when the point is already outside.
fn exit_distance(bounds: egui::Rect, point: egui::Pos2, direction: Vec2) -> f32 {
    if !bounds.contains(point) {
        return 0.0;
    }
    let along = |position: f32, low: f32, high: f32, step: f32| {
        if step > 1.0e-6 {
            (high - position) / step
        } else if step < -1.0e-6 {
            (low - position) / step
        } else {
            f32::INFINITY
        }
    };
    along(point.x, bounds.left(), bounds.right(), direction.x)
        .min(along(point.y, bounds.top(), bounds.bottom(), direction.y))
        .max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> ResizeFrame {
        ResizeFrame {
            center: [0.0, 20.0, 0.0],
            axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            extents: [40.0, 40.0, 40.0],
        }
    }

    /// A camera on +Z looking down -Z, slightly from above: x right,
    /// y up, z toward the viewer and down the screen.
    fn project(point: [f32; 3]) -> Option<egui::Pos2> {
        Some(egui::pos2(
            400.0 + point[0] * 4.0,
            300.0 - point[1] * 4.0 + point[2] * 1.5,
        ))
    }

    const VIEW: [f32; 3] = [0.0, -0.3, -0.954];

    #[test]
    fn overlay_frames_the_box_with_corner_dots_and_visible_face_handles() {
        let overlay = ResizeOverlay::layout(&frame(), None, project, VIEW).unwrap_or_else(|| {
            panic!("layout");
        });
        assert_eq!(overlay.corners.len(), 4);
        for (_, corner) in &overlay.corners {
            assert!(
                (corner.x == overlay.bounds.left() || corner.x == overlay.bounds.right())
                    && (corner.y == overlay.bounds.top() || corner.y == overlay.bounds.bottom())
            );
        }
        // X and Y handles both sides; only the front depth handle.
        let faces = overlay
            .faces
            .iter()
            .map(|face| face.handle)
            .collect::<Vec<_>>();
        assert_eq!(faces.len(), 5);
        assert!(faces.contains(&ResizeHandle::Face { axis: 2, side: 1.0 }));
        assert!(!faces.contains(&ResizeHandle::Face {
            axis: 2,
            side: -1.0
        }));
        // Side handles sit outside the box; the depth handle sits on it.
        for face in &overlay.faces {
            let inside = overlay.bounds.contains(face.center);
            assert_eq!(inside, face.axis == 2, "{face:?}");
        }
    }

    #[test]
    fn resize_overlay_never_paints_a_size_number() {
        let overlay = ResizeOverlay::layout(&frame(), None, project, VIEW).expect("overlay");
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            overlay.paint(ui.painter(), None);
            overlay.paint(ui.painter(), Some(ResizeHandle::Face { axis: 0, side: 1. }));
        });
        output.textures_delta.clear();
        assert!(!output.shapes.is_empty());
        assert!(
            output
                .shapes
                .iter()
                .all(|shape| !matches!(shape.shape, egui::Shape::Text(_)))
        );
    }

    #[test]
    fn every_drawn_handle_is_grabbable_where_it_is_drawn() {
        let overlay = ResizeOverlay::layout(&frame(), None, project, VIEW).unwrap_or_else(|| {
            panic!("layout");
        });
        for (handle, point) in overlay.grab_points() {
            assert_eq!(overlay.handle_at(point), Some(handle));
        }
        assert_eq!(
            overlay.handle_at(overlay.bounds.left_top() - Vec2::splat(100.0)),
            None
        );
    }

    #[test]
    fn layout_follows_the_drag_and_needs_a_projectable_box() {
        let still = ResizeOverlay::layout(&frame(), None, project, VIEW).unwrap_or_else(|| {
            panic!("layout");
        });
        let stretch = Mat4::from_rows([
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
        .ok();
        let dragged =
            ResizeOverlay::layout(&frame(), stretch, project, VIEW).unwrap_or_else(|| {
                panic!("layout");
            });
        assert!(dragged.bounds.width() > still.bounds.width() * 1.9);
        assert!(ResizeOverlay::layout(&frame(), None, |_| None, VIEW).is_none());
    }
}
