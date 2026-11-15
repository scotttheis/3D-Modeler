//! Measure ruler (OF-55). The Measure button opens the Measure bar in place
//! of the ribbon. Click a point on a model, then a second point (on the same
//! model or another): a ruler is drawn between them with its length and its
//! X, Y and Z parts, live while the pointer moves before the second click.
//! The pointer snaps to corners and sharp edges near it (a cube's corners
//! and edges, a cylinder's rim); hold Shift to place a point anywhere on the
//! surface. Dragging still turns the view. Measuring never changes the model
//! and adds nothing to Undo. MCP `measure` uses the same core distance.

use eframe::egui::{self, Color32, FontId, Stroke, Ui, Vec2};

use super::paint_bar::{paint_bar_button, paint_bar_divider};
use super::{Builder3dApp, RIBBON_BUTTON_HEIGHT, ViewportRenderer, viewport_pixel_position};
use crate::preferences::DisplayUnit;
use crate::theme::ThemePalette;

/// Screen distance (points) within which the pointer snaps to a corner.
const CORNER_SNAP_POINTS: f32 = 12.0;
/// Screen distance (points) within which the pointer snaps to a sharp edge.
const EDGE_SNAP_POINTS: f32 = 8.0;
/// Faces meeting at more than this angle form a sharp edge worth snapping to.
const SHARP_EDGE_DEGREES: f32 = 20.0;

/// What a ruler end is attached to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RulerSnap {
    Corner,
    Edge,
    Surface,
}

/// One end of the ruler, in world millimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RulerPoint {
    pub(super) world: [f32; 3],
    pub(super) snap: RulerSnap,
}

/// Live state of the Measure bar.
#[derive(Clone, Debug, Default)]
pub(super) struct MeasureMode {
    pub(super) start: Option<RulerPoint>,
    pub(super) end: Option<RulerPoint>,
    hover: Option<RulerPoint>,
    /// Sharp-edge flags of the last hovered face: (object id, face, scene
    /// revision, flags), so the neighbour search runs once per face.
    sharp_cache: Option<(String, usize, u64, [bool; 3])>,
    copied_at: Option<f64>,
}

impl MeasureMode {
    /// The ruler being shown: both ends fixed, or the first end and the
    /// point under the pointer.
    fn ruler(&self) -> Option<(RulerPoint, RulerPoint, bool)> {
        let start = self.start?;
        match self.end {
            Some(end) => Some((start, end, false)),
            None => self.hover.map(|hover| (start, hover, true)),
        }
    }

    /// Sets both ends directly (used by the review harness and tests).
    pub(super) fn with_points(start: RulerPoint, end: Option<RulerPoint>) -> Self {
        Self {
            start: Some(start),
            end,
            ..Self::default()
        }
    }
}

impl Builder3dApp {
    /// Measure button: opens the Measure bar, or closes it when open.
    pub(super) fn toggle_measure_mode(&mut self) {
        if self.measure_mode.is_some() {
            self.finish_measure_mode();
            return;
        }
        if self.commands.scene().objects().is_empty() {
            self.commands.set_status("Add an object to measure");
            return;
        }
        if self.paint_mode.is_some() {
            self.finish_paint_mode(true);
        }
        self.active_brush = None;
        self.brush_settings_open = false;
        self.boolean_pick = None;
        self.viewport_drag = None;
        self.measure_mode = Some(MeasureMode::default());
        self.commands.set_status(
            "Measure: click a point on a model, then a second point. Corners and edges snap; \
             hold Shift to place anywhere.",
        );
    }

    /// Review harness (`measure`, `measure-start`): opens Measure with the
    /// first end on the cube's front bottom-left corner and, when
    /// `finished`, the second on the cylinder's top rim.
    pub(super) fn stage_review_ruler(&mut self, finished: bool) {
        self.commands.set_selection_indices(std::iter::empty());
        self.toggle_measure_mode();
        let world = |index: usize| {
            self.commands
                .scene()
                .object(index)
                .ok()
                .and_then(|object| object.transformed_mesh().ok())
        };
        let (Some(cube), Some(cylinder)) = (world(0), world(1)) else {
            return;
        };
        let corner = cube.vertices.iter().copied().max_by(|left, right| {
            (left[2] - left[0] - left[1]).total_cmp(&(right[2] - right[0] - right[1]))
        });
        let top = cylinder
            .vertices
            .iter()
            .map(|vertex| vertex[1])
            .fold(f32::MIN, f32::max);
        let mut rim = cylinder
            .vertices
            .iter()
            .copied()
            .filter(|vertex| (vertex[1] - top).abs() < 1.0e-3)
            .collect::<Vec<_>>();
        rim.sort_by(|left, right| (right[0] - right[2]).total_cmp(&(left[0] - left[2])));
        let Some(start) = corner else {
            return;
        };
        let end = match (finished, rim.first(), rim.get(1)) {
            (true, Some(first), Some(second)) => Some(RulerPoint {
                world: [
                    (first[0] + second[0]) * 0.5,
                    (first[1] + second[1]) * 0.5,
                    (first[2] + second[2]) * 0.5,
                ],
                snap: RulerSnap::Edge,
            }),
            _ => None,
        };
        self.measure_mode = Some(MeasureMode::with_points(
            RulerPoint {
                world: start,
                snap: RulerSnap::Corner,
            },
            end,
        ));
        if let Some(end) = end {
            let unit = self.commands.preferences().display_unit();
            self.commands
                .set_status(format!("Measured {}", ruler_text(start, end.world, unit)));
        }
    }

    /// Leaves Measure. Nothing to keep or undo: measuring changes nothing.
    pub(super) fn finish_measure_mode(&mut self) {
        if self.measure_mode.take().is_some() {
            self.commands.set_status("Measure closed");
        }
    }

    /// Escape: drops a half-placed ruler first, then leaves Measure.
    pub(super) fn measure_escape(&mut self) {
        match self.measure_mode.as_mut() {
            Some(mode) if mode.start.is_some() && mode.end.is_none() => {
                mode.start = None;
                self.commands.set_status("Measure: click the first point");
            }
            Some(_) => self.finish_measure_mode(),
            None => {}
        }
    }

    /// One click in the view while measuring: first end, second end, then a
    /// fresh ruler.
    pub(super) fn measure_click(&mut self, rect: egui::Rect, pointer: egui::Pos2, free: bool) {
        if self.measure_mode.is_none() {
            return;
        }
        let Some(point) = self.ruler_point_at(rect, pointer, free) else {
            self.commands.set_status("Measure: click on a model");
            return;
        };
        let unit = self.commands.preferences().display_unit();
        let Some(mode) = self.measure_mode.as_mut() else {
            return;
        };
        if mode.start.is_none() || mode.end.is_some() {
            mode.start = Some(point);
            mode.end = None;
            self.commands
                .set_status("Measure: click the second point (Esc to start again)");
        } else if let Some(start) = mode.start {
            mode.end = Some(point);
            let text = ruler_text(start.world, point.world, unit);
            self.commands.set_status(format!("Measured {text}"));
        }
    }

    /// Updates the point under the pointer (`None` hides the hover marker).
    pub(super) fn update_measure_hover(
        &mut self,
        rect: egui::Rect,
        pointer: Option<egui::Pos2>,
        free: bool,
    ) {
        let hover = pointer.and_then(|pointer| self.ruler_point_at(rect, pointer, free));
        if let Some(mode) = self.measure_mode.as_mut() {
            mode.hover = hover;
        }
    }

    /// The surface point under `pointer`, snapped to a nearby corner or sharp
    /// edge of the face it lands on unless `free`.
    fn ruler_point_at(
        &mut self,
        rect: egui::Rect,
        pointer: egui::Pos2,
        free: bool,
    ) -> Option<RulerPoint> {
        let scene = self.preview.as_ref()?;
        let size = self
            .viewport_renderer
            .as_ref()
            .map_or(self.camera.viewport_size(), ViewportRenderer::size);
        let screen = viewport_pixel_position(pointer, rect, size);
        let hit = builder3d_viewport::pick_scene(scene, &self.camera, screen[0], screen[1])
            .ok()
            .flatten()?;
        let surface = RulerPoint {
            world: hit.position,
            snap: RulerSnap::Surface,
        };
        if free {
            return Some(surface);
        }
        let index = self.commands.scene().object_index(&hit.object_id).ok()?;
        let object = self.commands.scene().object(index).ok()?;
        let face_index = usize::try_from(hit.face_index).ok()?;
        let mesh = object.mesh();
        let face = *mesh.faces.get(face_index)?;
        let local = face.map(|vertex| mesh.vertices.get(vertex as usize).copied());
        let [Some(a), Some(b), Some(c)] = local else {
            return Some(surface);
        };
        let revision = self.commands.scene().revision();
        let cached = self.measure_mode.as_ref().and_then(|mode| {
            mode.sharp_cache
                .as_ref()
                .filter(|(id, cached_face, cached_revision, _)| {
                    id == &hit.object_id
                        && *cached_face == face_index
                        && *cached_revision == revision
                })
                .map(|(_, _, _, flags)| *flags)
        });
        let sharp = cached.unwrap_or_else(|| sharp_edges(mesh, face_index, [a, b, c]));
        let transform = *object.transform();
        if let Some(mode) = self.measure_mode.as_mut() {
            mode.sharp_cache = Some((hit.object_id.clone(), face_index, revision, sharp));
        }
        let corners = [a, b, c].map(|point| transform.transform_point(point));
        let matrix = self
            .camera
            .view_projection(self.current_scene_radius().unwrap_or(1.0))
            .ok()?;
        let projected = corners.map(|corner| super::project_to_viewport(matrix, rect, corner));
        let radius = self.current_scene_radius().unwrap_or(1.0);
        let ray = self.camera.screen_ray(screen[0], screen[1], radius).ok()?;
        Some(snap_to_features(
            pointer, corners, projected, sharp, surface, ray,
        ))
    }

    /// Draws the ruler, its markers and its label over the view.
    pub(super) fn paint_measure_overlay(
        &self,
        painter: &egui::Painter,
        rect: egui::Rect,
        palette: ThemePalette,
    ) {
        let Some(mode) = self.measure_mode.as_ref() else {
            return;
        };
        let Ok(matrix) = self
            .camera
            .view_projection(self.current_scene_radius().unwrap_or(1.0))
        else {
            return;
        };
        let project = |point: [f32; 3]| {
            super::project_to_viewport(matrix, rect, point).map(|(position, _)| position)
        };
        let unit = self.commands.preferences().display_unit();
        let ruler_color = palette.accent_control;
        if let Some((start, end, live)) = mode.ruler()
            && let (Some(from), Some(to)) = (project(start.world), project(end.world))
        {
            // The X, Y and Z legs, dashed in the arrow colours.
            let mut corner = start.world;
            let mut previous = from;
            for (axis, color) in crate::chrome::AXIS_COLORS.into_iter().enumerate() {
                corner[axis] = end.world[axis];
                if let Some(next) = project(corner)
                    && previous.distance(next) > 2.0
                {
                    painter.add(egui::Shape::dashed_line(
                        &[previous, next],
                        Stroke::new(3.5, palette.panel.gamma_multiply(0.6)),
                        7.0,
                        4.0,
                    ));
                    painter.add(egui::Shape::dashed_line(
                        &[previous, next],
                        Stroke::new(2.0, color),
                        7.0,
                        4.0,
                    ));
                    previous = next;
                }
            }
            painter.line_segment([from, to], Stroke::new(5.0, palette.panel));
            painter.line_segment([from, to], Stroke::new(2.5, ruler_color));
            paint_ruler_ticks(painter, from, to, ruler_color);
            paint_marker(painter, from, start.snap, ruler_color, palette);
            paint_marker(painter, to, end.snap, ruler_color, palette);
            paint_ruler_label(
                painter,
                rect,
                from,
                to,
                start.world,
                end.world,
                unit,
                live,
                palette,
            );
        } else if let Some(start) = mode.start
            && let Some(from) = project(start.world)
        {
            paint_marker(painter, from, start.snap, ruler_color, palette);
        }
        if mode.end.is_none()
            && let Some(hover) = mode.hover
            && let Some(at) = project(hover.world)
        {
            paint_marker(painter, at, hover.snap, palette.accent, palette);
        }
    }

    /// The Measure bar, drawn where the ribbon commands normally are.
    pub(super) fn render_measure_bar(&mut self, ui: &mut Ui, palette: ThemePalette) {
        let Some(mut mode) = self.measure_mode.take() else {
            return;
        };
        let unit = self.commands.preferences().display_unit();
        let now = ui.ctx().input(|input| input.time);
        let mut done = false;
        ui.horizontal(|ui| {
            ui.set_height(RIBBON_BUTTON_HEIGHT);
            ui.spacing_mut().item_spacing.x = 6.0;
            if paint_bar_button(
                ui,
                "Done",
                "Leave Measure",
                palette,
                |painter, rect, color| {
                    let mid = rect.center();
                    let stroke = Stroke::new(2.0, color);
                    painter.line_segment(
                        [mid + Vec2::new(-8.0, 0.0), mid + Vec2::new(-2.0, 6.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [mid + Vec2::new(-2.0, 6.0), mid + Vec2::new(9.0, -7.0)],
                        stroke,
                    );
                },
            ) {
                done = true;
            }
            if paint_bar_button(
                ui,
                "Clear",
                "Remove the ruler and start again",
                palette,
                |painter, rect, color| {
                    let mid = rect.center();
                    let stroke = Stroke::new(2.0, color);
                    painter.line_segment(
                        [mid + Vec2::new(-7.0, -7.0), mid + Vec2::new(7.0, 7.0)],
                        stroke,
                    );
                    painter.line_segment(
                        [mid + Vec2::new(-7.0, 7.0), mid + Vec2::new(7.0, -7.0)],
                        stroke,
                    );
                },
            ) {
                mode.start = None;
                mode.end = None;
            }
            paint_bar_divider(ui, palette);
            match mode.ruler() {
                Some((start, end, _)) => {
                    let (delta, distance) = ruler_measurement(start.world, end.world);
                    ui.label(egui::RichText::new("Distance").color(palette.muted));
                    ui.label(
                        egui::RichText::new(format_length(distance, unit))
                            .size(20.0)
                            .strong()
                            .color(palette.text),
                    );
                    paint_bar_divider(ui, palette);
                    for (axis, (name, color)) in ["X", "Y", "Z"]
                        .into_iter()
                        .zip(crate::chrome::AXIS_COLORS)
                        .enumerate()
                    {
                        ui.label(egui::RichText::new(name).strong().color(color));
                        ui.label(
                            egui::RichText::new(format_length(delta[axis].abs(), unit))
                                .color(palette.text),
                        );
                        ui.add_space(4.0);
                    }
                    paint_bar_divider(ui, palette);
                    let copied = mode.copied_at.is_some_and(|at| now - at < 1.5);
                    let copy = ui
                        .button(if copied { "Copied" } else { "Copy" })
                        .on_hover_text("Copy the measurement");
                    if copy.clicked() {
                        ui.ctx().copy_text(ruler_text(start.world, end.world, unit));
                        mode.copied_at = Some(now);
                    }
                }
                None => {
                    ui.label(
                        egui::RichText::new(if mode.start.is_some() {
                            "Click the second point."
                        } else {
                            "Click a point on a model, then a second point."
                        })
                        .color(palette.text),
                    );
                }
            }
            paint_bar_divider(ui, palette);
            ui.label(
                egui::RichText::new(
                    "Corners and edges snap. Hold Shift to place anywhere. Drag to turn the view.",
                )
                .color(palette.muted),
            );
        });
        if done {
            self.measure_mode = Some(mode);
            self.finish_measure_mode();
        } else {
            self.measure_mode = Some(mode);
        }
    }
}

/// Picks a corner of the hit face within reach of the pointer, else the
/// nearest point on a sharp edge within reach, else the surface point.
fn snap_to_features(
    pointer: egui::Pos2,
    corners: [[f32; 3]; 3],
    projected: [Option<(egui::Pos2, f32)>; 3],
    sharp: [bool; 3],
    surface: RulerPoint,
    ray: builder3d_viewport::Ray,
) -> RulerPoint {
    // A corner counts when one of the face's sharp edges ends there.
    let mut best_corner: Option<(f32, usize)> = None;
    for vertex in 0..3 {
        let on_sharp = sharp[vertex] || sharp[(vertex + 2) % 3];
        let Some((position, _)) = projected[vertex] else {
            continue;
        };
        let distance = position.distance(pointer);
        if on_sharp
            && distance <= CORNER_SNAP_POINTS
            && best_corner.is_none_or(|(best, _)| distance < best)
        {
            best_corner = Some((distance, vertex));
        }
    }
    if let Some((_, vertex)) = best_corner {
        return RulerPoint {
            world: corners[vertex],
            snap: RulerSnap::Corner,
        };
    }
    let mut best_edge: Option<(f32, usize)> = None;
    for edge in 0..3 {
        let (Some((start, _)), Some((end, _))) = (projected[edge], projected[(edge + 1) % 3])
        else {
            continue;
        };
        if !sharp[edge] {
            continue;
        }
        let distance = super::distance_to_screen_segment(pointer, start, end);
        if distance <= EDGE_SNAP_POINTS && best_edge.is_none_or(|(best, _)| distance < best) {
            best_edge = Some((distance, edge));
        }
    }
    if let Some((_, edge)) = best_edge {
        return RulerPoint {
            world: closest_on_segment_to_ray(corners[edge], corners[(edge + 1) % 3], ray),
            snap: RulerSnap::Edge,
        };
    }
    surface
}

/// For each edge `i` (corner `i` to corner `i + 1`) of `face`: whether the
/// faces meeting there bend by more than [`SHARP_EDGE_DEGREES`] or the edge
/// is open. Neighbours are found by shared positions, so meshes with split
/// vertices still work.
fn sharp_edges(
    mesh: &builder3d_core::Mesh,
    face_index: usize,
    corners: [[f32; 3]; 3],
) -> [bool; 3] {
    let normal = face_normal(corners);
    let threshold = SHARP_EDGE_DEGREES.to_radians().cos();
    let mut sharp = [true; 3];
    let mut found = [false; 3];
    for (other_index, other) in mesh.faces.iter().enumerate() {
        if other_index == face_index {
            continue;
        }
        let other_corners = other.map(|vertex| mesh.vertices.get(vertex as usize).copied());
        let [Some(p), Some(q), Some(r)] = other_corners else {
            continue;
        };
        let other_points = [p, q, r];
        for edge in 0..3 {
            if found[edge] {
                continue;
            }
            let (start, end) = (corners[edge], corners[(edge + 1) % 3]);
            if other_points.contains(&start) && other_points.contains(&end) {
                found[edge] = true;
                sharp[edge] = dot(normal, face_normal(other_points)) < threshold;
            }
        }
        if found.iter().all(|edge| *edge) {
            break;
        }
    }
    sharp
}

fn face_normal(corners: [[f32; 3]; 3]) -> [f32; 3] {
    let normal = cross(sub(corners[1], corners[0]), sub(corners[2], corners[0]));
    let size = length(normal);
    if size <= f32::EPSILON {
        [0.0; 3]
    } else {
        normal.map(|value| value / size)
    }
}

/// The point on segment `start..end` nearest to the pointer ray.
fn closest_on_segment_to_ray(
    start: [f32; 3],
    end: [f32; 3],
    ray: builder3d_viewport::Ray,
) -> [f32; 3] {
    let edge = sub(end, start);
    let offset = sub(start, ray.origin);
    let a = dot(edge, edge);
    let b = dot(edge, ray.direction);
    let c = dot(ray.direction, ray.direction);
    let d = dot(edge, offset);
    let e = dot(ray.direction, offset);
    let denominator = a * c - b * b;
    let amount = if denominator.abs() <= f32::EPSILON || a <= f32::EPSILON {
        0.0
    } else {
        ((b * e - c * d) / denominator).clamp(0.0, 1.0)
    };
    [
        start[0] + edge[0] * amount,
        start[1] + edge[1] * amount,
        start[2] + edge[2] * amount,
    ]
}

fn paint_marker(
    painter: &egui::Painter,
    at: egui::Pos2,
    snap: RulerSnap,
    color: Color32,
    palette: ThemePalette,
) {
    let outline = Stroke::new(1.5, palette.panel);
    match snap {
        RulerSnap::Corner => {
            let square = egui::Rect::from_center_size(at, Vec2::splat(10.0));
            painter.rect_filled(square, 1.0, color);
            painter.rect_stroke(square, 1.0, outline, egui::StrokeKind::Outside);
        }
        RulerSnap::Edge => {
            let points = vec![
                at + Vec2::new(0.0, -7.0),
                at + Vec2::new(7.0, 0.0),
                at + Vec2::new(0.0, 7.0),
                at + Vec2::new(-7.0, 0.0),
            ];
            painter.add(egui::Shape::convex_polygon(points, color, outline));
        }
        RulerSnap::Surface => {
            painter.circle_filled(at, 5.5, palette.panel);
            painter.circle_filled(at, 4.0, color);
        }
    }
}

/// Small cross ticks at both ends so the ruler reads as a ruler.
fn paint_ruler_ticks(painter: &egui::Painter, from: egui::Pos2, to: egui::Pos2, color: Color32) {
    let direction = to - from;
    let size = direction.length();
    if size < 1.0 {
        return;
    }
    let across = Vec2::new(-direction.y, direction.x) / size * 7.0;
    for end in [from, to] {
        painter.line_segment([end - across, end + across], Stroke::new(2.0, color));
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_ruler_label(
    painter: &egui::Painter,
    rect: egui::Rect,
    from: egui::Pos2,
    to: egui::Pos2,
    start: [f32; 3],
    end: [f32; 3],
    unit: DisplayUnit,
    live: bool,
    palette: ThemePalette,
) {
    let (delta, distance) = ruler_measurement(start, end);
    let title = painter.layout_no_wrap(
        format_length(distance, unit),
        FontId::proportional(16.0),
        palette.text,
    );
    let parts = ["X", "Y", "Z"]
        .into_iter()
        .zip(crate::chrome::AXIS_COLORS)
        .enumerate()
        .map(|(axis, (name, color))| {
            (
                painter.layout_no_wrap(name.to_owned(), FontId::proportional(11.0), color),
                painter.layout_no_wrap(
                    format_length(delta[axis].abs(), unit),
                    FontId::proportional(11.0),
                    palette.muted,
                ),
            )
        })
        .collect::<Vec<_>>();
    let parts_width = parts
        .iter()
        .map(|(name, value)| name.size().x + 3.0 + value.size().x)
        .sum::<f32>()
        + 10.0 * (parts.len().saturating_sub(1)) as f32;
    let width = title.size().x.max(parts_width) + 16.0;
    let height = title.size().y + parts.first().map_or(0.0, |part| part.1.size().y) + 12.0;
    // Beside the middle of the ruler (on its upper side, clear of the
    // line), kept inside the view.
    let middle = from + (to - from) * 0.5;
    let along = to - from;
    let mut normal = if along.length() > 1.0 {
        Vec2::new(-along.y, along.x) / along.length()
    } else {
        Vec2::new(0.0, -1.0)
    };
    if normal.y > 0.0 {
        normal = -normal;
    }
    let clearance = 0.5 * (normal.x.abs() * width + normal.y.abs() * height) + 10.0;
    let mut center = middle + normal * clearance;
    center.x = center.x.clamp(
        rect.left() + width * 0.5 + 4.0,
        rect.right() - width * 0.5 - 4.0,
    );
    center.y = center.y.clamp(
        rect.top() + height * 0.5 + 4.0,
        rect.bottom() - height * 0.5 - 4.0,
    );
    let frame = egui::Rect::from_center_size(center, Vec2::new(width, height));
    painter.rect_filled(
        frame,
        5.0,
        palette.panel.gamma_multiply(if live { 0.92 } else { 1.0 }),
    );
    painter.rect_stroke(
        frame,
        5.0,
        Stroke::new(1.0, palette.accent_control),
        egui::StrokeKind::Inside,
    );
    let title_size = title.size();
    painter.galley(
        egui::pos2(frame.center().x - title_size.x * 0.5, frame.top() + 5.0),
        title,
        palette.text,
    );
    let mut x = frame.center().x - parts_width * 0.5;
    let y = frame.top() + 5.0 + title_size.y + 1.0;
    for (name, value) in parts {
        let name_width = name.size().x;
        let value_width = value.size().x;
        painter.galley(egui::pos2(x, y), name, palette.text);
        x += name_width + 3.0;
        painter.galley(egui::pos2(x, y), value, palette.muted);
        x += value_width + 10.0;
    }
}

/// The ruler's length and axis parts, from the same core function MCP
/// `measure` uses. Non-finite ends (never produced by picking) read as zero.
pub(super) fn ruler_measurement(start: [f32; 3], end: [f32; 3]) -> ([f32; 3], f32) {
    builder3d_core::measure_points(start, end).map_or(([0.0; 3], 0.0), |measured| {
        (measured.delta_mm, measured.distance_mm)
    })
}

/// A length in the chosen unit: two decimals in millimetres, three in inches.
fn format_length(value_mm: f32, unit: DisplayUnit) -> String {
    let value = unit.from_mm(value_mm);
    let value = if value.abs() < 0.000_5 { 0.0 } else { value };
    match unit {
        DisplayUnit::Inch => format!("{value:.3} {}", unit.label()),
        DisplayUnit::Millimetre => format!("{value:.2} {}", unit.label()),
    }
}

/// "42.30 mm (X 30.00 mm, Y 0.00 mm, Z 29.83 mm)" — for the status bar and Copy.
fn ruler_text(start: [f32; 3], end: [f32; 3], unit: DisplayUnit) -> String {
    let (delta, distance) = ruler_measurement(start, end);
    format!(
        "{} (X {}, Y {}, Z {})",
        format_length(distance, unit),
        format_length(delta[0].abs(), unit),
        format_length(delta[1].abs(), unit),
        format_length(delta[2].abs(), unit)
    )
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
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

fn length(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_faces_have_sharp_rims_and_a_soft_diagonal() {
        let cube = builder3d_core::make_box([20.0; 3]).unwrap_or_else(|error| panic!("{error}"));
        for (index, face) in cube.faces.iter().enumerate() {
            let corners = face.map(|vertex| cube.vertices[vertex as usize]);
            let sharp = sharp_edges(&cube, index, corners);
            // Each cube triangle has two rim edges and one flat diagonal.
            assert_eq!(
                sharp.iter().filter(|edge| **edge).count(),
                2,
                "face {index}"
            );
        }
        let sphere = builder3d_core::make_sphere(20.0, 3).unwrap_or_else(|error| panic!("{error}"));
        let corners = sphere.faces[0].map(|vertex| sphere.vertices[vertex as usize]);
        assert_eq!(sharp_edges(&sphere, 0, corners), [false; 3]);
    }

    #[test]
    fn pointer_snaps_to_a_near_corner_then_a_near_edge_else_stays_on_the_surface() {
        let corners = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]];
        let projected = [
            Some((egui::pos2(100.0, 100.0), 0.5)),
            Some((egui::pos2(200.0, 100.0), 0.5)),
            Some((egui::pos2(100.0, 0.0), 0.5)),
        ];
        let surface = RulerPoint {
            world: [3.0, 3.0, 0.0],
            snap: RulerSnap::Surface,
        };
        let ray = builder3d_viewport::Ray::new([5.0, 1.0, 10.0], [0.0, 0.0, -1.0])
            .unwrap_or_else(|error| panic!("{error}"));
        let sharp = [true, false, false];
        let corner = snap_to_features(
            egui::pos2(195.0, 104.0),
            corners,
            projected,
            sharp,
            surface,
            ray,
        );
        assert_eq!(corner.snap, RulerSnap::Corner);
        assert_eq!(corner.world, corners[1]);
        let edge = snap_to_features(
            egui::pos2(150.0, 105.0),
            corners,
            projected,
            sharp,
            surface,
            ray,
        );
        assert_eq!(edge.snap, RulerSnap::Edge);
        assert!((edge.world[0] - 5.0).abs() < 1e-4 && edge.world[1].abs() < 1e-4);
        // The edge from corner 1 to corner 2 is not sharp: no snap there.
        let none = snap_to_features(
            egui::pos2(150.0, 50.0),
            corners,
            projected,
            sharp,
            surface,
            ray,
        );
        assert_eq!(none, surface);
        // Corner 2 touches no sharp edge, so it does not catch the pointer.
        let soft = snap_to_features(
            egui::pos2(102.0, 3.0),
            corners,
            projected,
            sharp,
            surface,
            ray,
        );
        assert_eq!(soft, surface);
    }

    #[test]
    fn ruler_text_reads_length_and_axis_parts_in_the_display_unit() {
        let text = ruler_text([0.0; 3], [30.0, -40.0, 0.0], DisplayUnit::Millimetre);
        assert_eq!(text, "50.00 mm (X 30.00 mm, Y 40.00 mm, Z 0.00 mm)");
        let inches = ruler_text([0.0; 3], [25.4, 0.0, 0.0], DisplayUnit::Inch);
        assert!(inches.starts_with("1.000 in"), "{inches}");
    }
}
