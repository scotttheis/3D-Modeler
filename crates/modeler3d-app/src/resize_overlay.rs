//! The Resize-mode overlay, drawn the way Microsoft 3D Builder draws its
//! scale handles.
//!
//! - A screen-aligned white rectangle around the selection's projected box,
//!   with a white dot at each corner. A corner dot scales uniformly.
//! - A double-chevron handle beyond each face: green for the X axis, blue
//!   for the vertical Y axis and for depth (Z). A face whose axis points
//!   away from the viewer hides its handle behind the object, as in
//!   3D Builder, which shows only the front depth handle.
//! - While a face handle is dragged, a pale band crosses the object along
//!   the drag axis; no extra numbers appear.
//!
//! One layout feeds both painting and hit-testing, so what is drawn is
//! exactly what can be grabbed.

use eframe::egui::{self, Color32, Stroke, Vec2};
use modeler3d_viewport::{Mat4, add, dot, scale};

use crate::transform_gesture::{ResizeFrame, ResizeHandle};

/// 3D Builder's lime green for horizontal (X) handles.
const HANDLE_GREEN: Color32 = Color32::from_rgb(118, 214, 46);
/// 3D Builder's sky blue for vertical (Y) and depth (Z) handles.
const HANDLE_BLUE: Color32 = Color32::from_rgb(36, 162, 238);
/// Size of the face-handle chevrons relative to 3D Builder's originals. The
/// owner asked for them twice as big (OF-32); the gap from the box and the
/// pointer reach scale with them.
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
    /// Centre of the double chevron, in viewport points.
    pub(crate) center: egui::Pos2,
    /// Unit screen direction the chevrons point (outward along the axis).
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
    /// Projected centres of each axis's two faces, `[minus, plus]`.
    face_centers: [Option<[egui::Pos2; 2]>; 3],
}

impl ResizeOverlay {
    /// Lays out the overlay for `frame`, optionally moved by an in-progress
    /// drag `delta`. `project` maps a world point to viewport points and
    /// returns `None` only for points behind the camera: off-screen points
    /// still project, so the handles stay usable when the view is zoomed in
    /// past the box. `project_edge` does the same for a box edge, cut where
    /// it crosses the camera. `viewport` keeps a handle whose natural place
    /// is off-screen at the nearest edge. `view_direction` is the camera's
    /// unit forward vector. Returns `None` when no part of the box is in
    /// front of the camera.
    pub(crate) fn layout(
        frame: &ResizeFrame,
        delta: Option<Mat4>,
        project: impl Fn([f32; 3]) -> Option<egui::Pos2>,
        project_edge: impl Fn([f32; 3], [f32; 3]) -> Option<[egui::Pos2; 2]>,
        viewport: egui::Rect,
        view_direction: [f32; 3],
    ) -> Option<Self> {
        let moved = |point: [f32; 3]| {
            delta
                .and_then(|matrix| matrix.transform_point(point).ok())
                .unwrap_or(point)
        };
        let mut projected = Vec::with_capacity(8);
        let mut bounds = egui::Rect::NOTHING;
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    let sides = [x, y, z];
                    if let Some(point) = project(moved(frame.point(sides))) {
                        projected.push((sides, point));
                    }
                    for axis in 0..3 {
                        if sides[axis] > 0.0 {
                            continue;
                        }
                        let mut other = sides;
                        other[axis] = 1.0;
                        if let Some([start, end]) =
                            project_edge(moved(frame.point(sides)), moved(frame.point(other)))
                        {
                            bounds.extend_with(start);
                            bounds.extend_with(end);
                        }
                    }
                }
            }
        }
        if bounds.min.x > bounds.max.x {
            return None;
        }
        // A corner dot stands for one corner; with a corner behind the
        // camera the nearest-corner match would be a guess, so show none.
        let corners = if projected.len() == 8 {
            [
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
            .collect()
        } else {
            Vec::new()
        };

        let largest = frame.extents.iter().copied().fold(0.0_f32, f32::max);
        let probe = (largest * 0.05).max(1.0e-3);
        let mut faces = Vec::with_capacity(6);
        let mut face_centers = [None; 3];
        for (axis, face_slot) in face_centers.iter_mut().enumerate() {
            let mut centers = [egui::Pos2::ZERO; 2];
            let mut complete = true;
            for (slot, side) in [-1.0_f32, 1.0].into_iter().enumerate() {
                let outward = scale(frame.axes[axis], side);
                let world = add(frame.center, scale(outward, frame.extents[axis] * 0.5));
                let Some(center) = project(moved(world)) else {
                    // Behind the camera: this face is past the viewer.
                    complete = false;
                    continue;
                };
                centers[slot] = center;
                // Near the camera the point just outside the face can be the
                // one behind it; step inward instead and flip the direction.
                let screen = match project(moved(add(world, scale(outward, probe)))) {
                    Some(ahead) => ahead - center,
                    None => match project(moved(add(world, scale(outward, -probe)))) {
                        Some(behind) => center - behind,
                        None => continue,
                    },
                };
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
                    center: keep_in_view(center, viewport),
                    direction,
                });
            }
            if complete {
                *face_slot = Some(centers);
            }
        }
        separate_pinned(&mut faces, viewport);
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

    /// Paints the overlay. `emphasized` is the hovered or dragged handle;
    /// `dragging` is the handle of an in-progress drag.
    pub(crate) fn paint(
        &self,
        painter: &egui::Painter,
        emphasized: Option<ResizeHandle>,
        dragging: Option<ResizeHandle>,
    ) {
        if let Some(ResizeHandle::Face { axis, .. } | ResizeHandle::Edge { axis, .. }) = dragging
            && let Some([minus, plus]) = self.face_centers[axis]
        {
            paint_drag_band(painter, minus, plus, self.bounds);
        }
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
        for face in &self.faces {
            let color = if face.axis == 0 {
                HANDLE_GREEN
            } else {
                HANDLE_BLUE
            };
            let size = if emphasized == Some(face.handle) {
                1.15 * FACE_HANDLE_SCALE
            } else {
                FACE_HANDLE_SCALE
            };
            paint_double_chevron(painter, face.center, face.direction, color, size);
        }
    }
}

/// Moves a handle that would sit off-screen to the nearest point of the
/// viewport, inset so the whole chevron shows.
fn keep_in_view(point: egui::Pos2, viewport: egui::Rect) -> egui::Pos2 {
    let area = viewport.shrink2(Vec2::splat(FACE_HANDLE_GAP).min(viewport.size() * 0.5));
    egui::pos2(
        point.x.clamp(area.left(), area.right()),
        point.y.clamp(area.top(), area.bottom()),
    )
}

/// Handles pinned to the viewport edge at extreme zoom can land on top of one
/// another. Pushes overlapping ones apart (along the line between them, or
/// across their pointing direction when they coincide) so each stays visible
/// and grabbable, then keeps them inside the viewport.
fn separate_pinned(faces: &mut [FaceHandle], viewport: egui::Rect) {
    let gap = FACE_HIT_RADIUS * 1.6;
    for _ in 0..4 {
        let mut moved = false;
        for i in 1..faces.len() {
            for j in 0..i {
                let offset = faces[i].center - faces[j].center;
                let distance = offset.length();
                if distance >= gap {
                    continue;
                }
                let push = if distance > 1.0e-3 {
                    offset / distance
                } else {
                    faces[i].direction.rot90()
                };
                let shortfall = (gap - distance) * 0.5;
                faces[i].center = keep_in_view(faces[i].center + push * shortfall, viewport);
                faces[j].center = keep_in_view(faces[j].center - push * shortfall, viewport);
                moved = true;
            }
        }
        if !moved {
            break;
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

/// Two solid stacked triangles pointing along `direction`.
fn paint_double_chevron(
    painter: &egui::Painter,
    center: egui::Pos2,
    direction: Vec2,
    color: Color32,
    size: f32,
) {
    let across = direction.rot90();
    let (length, half_width) = (7.0 * size, 6.5 * size);
    for base in [-length - 0.5 * size, -0.5 * size] {
        let base_center = center + direction * base;
        painter.add(egui::Shape::convex_polygon(
            vec![
                base_center + direction * length,
                base_center + across * half_width,
                base_center - across * half_width,
            ],
            color,
            Stroke::NONE,
        ));
    }
}

/// The pale band 3D Builder draws across the object along the drag axis.
fn paint_drag_band(
    painter: &egui::Painter,
    minus: egui::Pos2,
    plus: egui::Pos2,
    bounds: egui::Rect,
) {
    let span = plus - minus;
    let length = span.length();
    if !length.is_finite() || length < 4.0 {
        return;
    }
    let direction = span / length;
    let across = direction.rot90();
    let perpendicular_extent = across.x.abs() * bounds.width() + across.y.abs() * bounds.height();
    let half_thickness = (perpendicular_extent * 0.09).clamp(4.0, 10.0);
    let inset = length * 0.07;
    let start = minus + direction * inset;
    let end = plus - direction * inset;
    painter.add(egui::Shape::convex_polygon(
        vec![
            start + across * half_thickness,
            end + across * half_thickness,
            end - across * half_thickness,
            start - across * half_thickness,
        ],
        Color32::from_rgba_unmultiplied(255, 255, 255, 125),
        Stroke::NONE,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout_for_test(
        frame: &ResizeFrame,
        delta: Option<Mat4>,
        project: impl Fn([f32; 3]) -> Option<egui::Pos2>,
    ) -> Option<ResizeOverlay> {
        ResizeOverlay::layout(
            frame,
            delta,
            &project,
            |a, b| Some([project(a)?, project(b)?]),
            egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::splat(100_000.0)),
            VIEW,
        )
    }

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
        let overlay = layout_for_test(&frame(), None, project).unwrap_or_else(|| {
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
        let overlay = layout_for_test(&frame(), None, project).expect("overlay");
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            overlay.paint(ui.painter(), None, None);
            overlay.paint(
                ui.painter(),
                None,
                Some(ResizeHandle::Face { axis: 0, side: 1. }),
            );
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
        let overlay = layout_for_test(&frame(), None, project).unwrap_or_else(|| {
            panic!("layout");
        });
        for (handle, point) in overlay.grab_points() {
            assert_eq!(overlay.handle_at(point), Some(handle));
        }
        assert_eq!(
            overlay.handle_at(overlay.bounds.center() + Vec2::new(0.0, -8.0)),
            None
        );
    }

    #[test]
    fn layout_follows_the_drag_and_needs_a_projectable_box() {
        let still = layout_for_test(&frame(), None, project).unwrap_or_else(|| {
            panic!("layout");
        });
        let stretch = Mat4::from_rows([
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
        .ok();
        let dragged = layout_for_test(&frame(), stretch, project).unwrap_or_else(|| {
            panic!("layout");
        });
        assert!(dragged.bounds.width() > still.bounds.width() * 1.9);
        assert!(layout_for_test(&frame(), None, |_| None).is_none());
    }

    #[test]
    fn handles_stay_on_screen_when_the_box_is_larger_than_the_view() {
        // Zoomed in so far that every corner is far outside the viewport:
        // the handles must still be drawn, pinned inside the viewport.
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        let zoomed = |point: [f32; 3]| {
            Some(egui::pos2(
                400.0 + point[0] * 90.0,
                300.0 - (point[1] - 20.0) * 90.0 + point[2] * 30.0,
            ))
        };
        let overlay = ResizeOverlay::layout(
            &frame(),
            None,
            zoomed,
            |a, b| Some([zoomed(a)?, zoomed(b)?]),
            viewport,
            VIEW,
        )
        .expect("overlay survives zooming in");
        assert!(!overlay.faces.is_empty());
        for face in &overlay.faces {
            assert!(viewport.contains(face.center), "{:?}", face.center);
            // Pinned handles can overlap; whichever is nearest is grabbed.
            assert!(overlay.handle_at(face.center).is_some());
        }
    }

    #[test]
    fn pinned_handles_do_not_overlap_at_extreme_zoom() {
        let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 600.0));
        let zoomed = |point: [f32; 3]| {
            Some(egui::pos2(
                400.0 + point[0] * 900.0,
                300.0 - (point[1] - 20.0) * 900.0 + point[2] * 300.0,
            ))
        };
        let overlay = ResizeOverlay::layout(
            &frame(),
            None,
            zoomed,
            |a, b| Some([zoomed(a)?, zoomed(b)?]),
            viewport,
            VIEW,
        )
        .expect("overlay");
        for (i, a) in overlay.faces.iter().enumerate() {
            assert!(viewport.contains(a.center));
            for b in &overlay.faces[i + 1..] {
                assert!(
                    a.center.distance(b.center) >= FACE_HIT_RADIUS,
                    "{:?} overlaps {:?}",
                    a,
                    b
                );
            }
        }
    }
}
