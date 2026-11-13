//! Original raster assets and bounded vector fallbacks for the native shell.
//!
//! The copied product artwork is loaded from the Rust package. Vector marks
//! remain only for source-mapped controls without a supplied raster image.

use std::{
    fs,
    path::{Path, PathBuf},
};

use eframe::egui::{self, Align2, Color32, FontId, IconData, Painter, Pos2, Rect, Stroke, Vec2};
use thiserror::Error;

use builder3d_core::PrimitiveKind;

use crate::commands::{CommandId, IconKey};
use crate::theme::{ThemeMode, ThemePalette};

const MAX_ARTWORK_BYTES: usize = 4096;

/// A loaded, compile-time theme asset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtworkAsset {
    /// Stable logical asset name.
    pub name: &'static str,
    /// UTF-8 SVG source.
    pub svg: &'static str,
}

/// Failures from compact artwork validation.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum ArtworkError {
    /// An asset did not satisfy the bounded SVG contract.
    #[error("artwork asset {name} is invalid: {reason}")]
    Invalid {
        /// Logical asset name.
        name: &'static str,
        /// Validation reason.
        reason: &'static str,
    },
    /// An original Builder3D raster asset could not be found or decoded.
    #[error("original artwork {path} is unavailable: {reason}")]
    OriginalAsset {
        /// Path checked beneath the package artwork root.
        path: String,
        /// Read, decode, or validation detail.
        reason: String,
    },
}

/// A source-mapped raster asset used by the current Builder3D design.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum OriginalAsset {
    /// Theme-specific action artwork.
    Action(&'static str),
    /// Theme-specific Insert primitive artwork.
    Insert(&'static str),
    /// Shared product branding image.
    Brandmark,
}

impl OriginalAsset {
    /// Returns the package-relative path for the selected theme.
    #[must_use]
    pub(crate) fn relative_path(self, mode: ThemeMode) -> PathBuf {
        match self {
            Self::Action(name) => PathBuf::from(match mode {
                ThemeMode::Light => "action_icons_light",
                ThemeMode::Dark => "action_icons",
            })
            .join(name),
            Self::Insert(name) => PathBuf::from(match mode {
                ThemeMode::Light => "insert_icons_light",
                ThemeMode::Dark => "insert_icons",
            })
            .join(format!("{name}.png")),
            Self::Brandmark => PathBuf::from("Builder3D.png"),
        }
    }
}

const ACTION_ASSET_FILES: &[&str] = &[
    "Align.png",
    "Arrange.png",
    "Blob.png",
    "Center View.png",
    "Color.png",
    "Copy.png",
    "Crease.png",
    "Cut.png",
    "Delete.png",
    "Duplicate.png",
    "Edit Torus.png",
    "Elastic Grab.png",
    "Emboss.png",
    "ExRay.png",
    "Folder and Open.png",
    "Frame.png",
    "Front.png",
    "Grid.png",
    "Hollow.png",
    "Intersect.png",
    "Material.png",
    "Measure.png",
    "Merge.png",
    "Mirror.png",
    "New Scene.png",
    "Normalize.png",
    "Paint.png",
    "Paste.png",
    "Pin.png",
    "Quick Color.png",
    "Redo.png",
    "Repair.png",
    "Reset View.png",
    "Settle.png",
    "Side.png",
    "Simplify.png",
    "Smooth Brush.png",
    "Smooth.png",
    "Snap.png",
    "Subtract.png",
    "Top.png",
    "Undo.png",
    "UndoRedo.png",
    "Wireframe.png",
];

const INSERT_ASSET_FILES: &[&str] = &[
    "Cone.png",
    "Cube.png",
    "Cylinder.png",
    "Hexagon.png",
    "Pyramid.png",
    "Sphere.png",
    "Torus.png",
    "Wedge.png",
];

/// Locates the copied product assets beside an installed app or in the source tree.
#[must_use]
pub(crate) fn original_asset_root() -> PathBuf {
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        let packaged = parent.join("assets").join("original");
        if packaged.is_dir() {
            return packaged;
        }
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("original")
}

/// Checks every copied source image before opening the native window.
pub(crate) fn validate_original_artwork(root: &Path) -> Result<(), ArtworkError> {
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for &name in ACTION_ASSET_FILES {
            validate_raster(root, OriginalAsset::Action(name), mode)?;
        }
        for &name in INSERT_ASSET_FILES {
            let stem = name.strip_suffix(".png").unwrap_or(name);
            validate_raster(root, OriginalAsset::Insert(stem), mode)?;
        }
    }

    for name in ["Builder3D.png", "pin.jpg", "snap.png"] {
        validate_raster_path(root, Path::new(name))?;
    }
    validate_file(root, Path::new("Builder3D.ico"))
}

fn validate_raster(root: &Path, asset: OriginalAsset, mode: ThemeMode) -> Result<(), ArtworkError> {
    validate_raster_path(root, &asset.relative_path(mode))
}

fn validate_raster_path(root: &Path, relative: &Path) -> Result<(), ArtworkError> {
    let path = root.join(relative);
    validate_file(root, relative)?;
    let (width, height) = image::image_dimensions(&path).map_err(|error| {
        original_asset_error(relative, format!("image header could not be read: {error}"))
    })?;
    if width == 0 || height == 0 {
        return Err(original_asset_error(
            relative,
            String::from("image dimensions must be nonzero"),
        ));
    }
    Ok(())
}

fn validate_file(root: &Path, relative: &Path) -> Result<(), ArtworkError> {
    let path = root.join(relative);
    match fs::metadata(&path) {
        Ok(metadata) if metadata.is_file() && metadata.len() > 0 => Ok(()),
        Ok(_) => Err(original_asset_error(
            relative,
            String::from("expected a nonempty regular file"),
        )),
        Err(error) => Err(original_asset_error(relative, error.to_string())),
    }
}

fn original_asset_error(relative: &Path, reason: String) -> ArtworkError {
    ArtworkError::OriginalAsset {
        path: relative.to_string_lossy().replace('\\', "/"),
        reason,
    }
}

/// Maps desktop commands to the original action and Insert images.
#[must_use]
pub(crate) fn original_asset_for_command(command: CommandId) -> Option<OriginalAsset> {
    use CommandId as Id;

    let action = match command {
        Id::OpenProject => "Folder and Open.png",
        Id::ImportModel => "Folder and Open.png",
        Id::NewScene => "New Scene.png",
        Id::EditTorus => "Edit Torus.png",
        Id::Repair => "Repair.png",
        Id::Duplicate => "Duplicate.png",
        Id::Copy => "Copy.png",
        Id::Paste => "Paste.png",
        Id::Cut => "Cut.png",
        Id::Delete => "Delete.png",
        Id::Settle => "Settle.png",
        Id::Mirror => "Mirror.png",
        Id::Measure => "Measure.png",
        Id::Align => "Align.png",
        Id::Arrange => "Arrange.png",
        Id::Simplify | Id::Subdivide => "Simplify.png",
        Id::Smooth => "Smooth.png",
        Id::Grab => "Elastic Grab.png",
        Id::Crease => "Crease.png",
        Id::SmoothBrush => "Smooth Brush.png",
        Id::Blob => "Blob.png",
        Id::Normalize => "Normalize.png",
        Id::Merge => "Merge.png",
        Id::Emboss => "Emboss.png",
        Id::Hollow => "Hollow.png",
        Id::Intersect => "Intersect.png",
        Id::Subtract => "Subtract.png",
        Id::Material => "Material.png",
        Id::Paint => "Paint.png",
        Id::Color => "Color.png",
        Id::CenterView => "Center View.png",
        Id::FrameSelection => "Frame.png",
        Id::ResetView => "Reset View.png",
        Id::ViewTop => "Top.png",
        Id::ViewSide => "Side.png",
        Id::ToggleGrid => "Grid.png",
        Id::ToggleWireframe => "Wireframe.png",
        Id::ToggleXray => "ExRay.png",
        Id::ToggleSnap => "Snap.png",
        Id::Undo => "Undo.png",
        Id::Redo => "Redo.png",
        _ => {
            return match command {
                Id::AddPrimitive(kind) => insert_asset_for_primitive(kind),
                _ => None,
            };
        }
    };
    Some(OriginalAsset::Action(action))
}

fn insert_asset_for_primitive(kind: PrimitiveKind) -> Option<OriginalAsset> {
    let name = match kind {
        PrimitiveKind::Cube => "Cube",
        PrimitiveKind::Cylinder => "Cylinder",
        PrimitiveKind::Pyramid => "Pyramid",
        PrimitiveKind::Cone => "Cone",
        PrimitiveKind::Sphere => "Sphere",
        PrimitiveKind::Torus => "Torus",
        PrimitiveKind::Hexagon => "Hexagon",
        PrimitiveKind::Wedge => "Wedge",
        PrimitiveKind::Tetrahedron => return None,
    };
    Some(OriginalAsset::Insert(name))
}

/// Decodes one copied raster image without changing its stored source bytes.
pub(crate) fn load_original_image(path: &Path) -> Result<egui::ColorImage, ArtworkError> {
    let bytes = fs::read(path).map_err(|error| original_asset_error(path, error.to_string()))?;
    let decoded = image::load_from_memory(&bytes)
        .map_err(|error| original_asset_error(path, format!("image decode failed: {error}")))?
        .to_rgba8();
    let width = usize::try_from(decoded.width()).map_err(|_| {
        original_asset_error(
            path,
            String::from("image width exceeds this platform's limits"),
        )
    })?;
    let height = usize::try_from(decoded.height()).map_err(|_| {
        original_asset_error(
            path,
            String::from("image height exceeds this platform's limits"),
        )
    })?;
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [width, height],
        decoded.as_raw(),
    ))
}

/// Loads the shared Builder3D splash artwork as a native application icon.
pub(crate) fn load_native_icon(root: &Path) -> Result<IconData, ArtworkError> {
    let path = root.join("Builder3D.png");
    let bytes = fs::read(&path)
        .map_err(|error| original_asset_error(Path::new("Builder3D.png"), error.to_string()))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| original_asset_error(Path::new("Builder3D.png"), error.to_string()))?
        .thumbnail(64, 64)
        .to_rgba8();
    Ok(IconData {
        rgba: image.into_raw(),
        width: 64,
        height: 64,
    })
}

/// Copies the complete original asset tree into a staged native package.
pub(crate) fn stage_original_artwork(destination: &Path) -> Result<(), ArtworkError> {
    let source = original_asset_root();
    let target = destination.join("assets").join("original");
    copy_asset_tree(&source, &source, &target)
}

fn copy_asset_tree(root: &Path, current: &Path, destination: &Path) -> Result<(), ArtworkError> {
    let relative = current.strip_prefix(root).map_err(|error| {
        original_asset_error(Path::new("."), format!("invalid asset root: {error}"))
    })?;
    let target_dir = destination.join(relative);
    fs::create_dir_all(&target_dir)
        .map_err(|error| original_asset_error(relative, error.to_string()))?;
    let entries =
        fs::read_dir(current).map_err(|error| original_asset_error(relative, error.to_string()))?;
    for entry in entries {
        let entry = entry.map_err(|error| original_asset_error(relative, error.to_string()))?;
        let file_type = entry
            .file_type()
            .map_err(|error| original_asset_error(relative, error.to_string()))?;
        let path = entry.path();
        if file_type.is_symlink() {
            return Err(original_asset_error(
                path.strip_prefix(root).unwrap_or(&path),
                String::from("symbolic links are not permitted in the artwork package"),
            ));
        }
        if file_type.is_dir() {
            copy_asset_tree(root, &path, destination)?;
        } else if file_type.is_file() {
            let relative_file = path.strip_prefix(root).map_err(|error| {
                original_asset_error(Path::new("."), format!("invalid asset path: {error}"))
            })?;
            fs::copy(&path, destination.join(relative_file))
                .map_err(|error| original_asset_error(relative_file, error.to_string()))?;
        } else {
            return Err(original_asset_error(
                path.strip_prefix(root).unwrap_or(&path),
                String::from("unsupported artwork file type"),
            ));
        }
    }
    Ok(())
}

/// Theme-aware vector fallback for controls without mapped source artwork.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemeArtwork {
    mode: ThemeMode,
}

impl ThemeArtwork {
    /// Creates a catalog for one shell theme.
    #[must_use]
    pub const fn new(mode: ThemeMode) -> Self {
        Self { mode }
    }

    /// Returns the selected theme.
    #[must_use]
    pub const fn mode(self) -> ThemeMode {
        self.mode
    }

    /// Returns one compile-time asset for a semantic icon key.
    #[must_use]
    pub fn asset(self, key: IconKey) -> ArtworkAsset {
        let name = asset_name(key);
        let svg = match self.mode {
            ThemeMode::Light => light_asset(key),
            ThemeMode::Dark => dark_asset(key),
        };
        ArtworkAsset { name, svg }
    }

    /// Validates every installed theme asset before packaging.
    pub fn validate(self) -> Result<(), ArtworkError> {
        for key in all_icon_keys() {
            let asset = self.asset(key);
            validate_asset(asset)?;
        }
        Ok(())
    }

    /// Draws a compact vector icon using the scoped asset's declared tint.
    pub fn paint_icon(self, painter: &Painter, rect: Rect, key: IconKey, palette: ThemePalette) {
        let asset = self.asset(key);
        let tint = svg_tint(asset.svg).unwrap_or(palette.accent);
        let stroke = Stroke::new(1.7, tint);
        let center = rect.center();
        let size = rect.size().min_elem();
        let half = size * 0.32;
        match key {
            IconKey::Folder => {
                let body = Rect::from_center_size(center, Vec2::new(half * 2.0, half * 1.25));
                painter.rect_filled(body, 3.0, tint.linear_multiply(0.18));
                painter.rect_stroke(body, 3.0, stroke, egui::StrokeKind::Inside);
                painter.line_segment(
                    [
                        Pos2::new(body.left() + 2.0, body.top()),
                        Pos2::new(body.left() + half * 0.35, body.top() - 4.0),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(body.left() + half * 0.35, body.top() - 4.0),
                        Pos2::new(body.left() + half, body.top() - 4.0),
                    ],
                    stroke,
                );
            }
            IconKey::Primitive(kind) => paint_primitive(painter, rect, kind, tint, stroke),
            IconKey::Object => {
                painter.circle_filled(center, half * 0.85, tint.linear_multiply(0.2));
                painter.circle_stroke(center, half * 0.85, stroke);
                painter.line_segment(
                    [
                        Pos2::new(center.x - half, center.y),
                        Pos2::new(center.x + half, center.y),
                    ],
                    stroke,
                );
            }
            IconKey::Edit => {
                painter.line_segment(
                    [
                        Pos2::new(center.x - half, center.y + half * 0.7),
                        Pos2::new(center.x + half * 0.8, center.y - half * 0.8),
                    ],
                    Stroke::new(3.0, tint),
                );
                painter.circle_filled(center, 2.5, tint);
            }
            IconKey::View => {
                painter.circle_stroke(center, half * 0.9, stroke);
                painter.circle_filled(center, half * 0.3, tint);
            }
            IconKey::Save => {
                let body = Rect::from_center_size(center, Vec2::splat(half * 1.8));
                painter.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Inside);
                painter.line_segment(
                    [
                        Pos2::new(body.left() + 4.0, body.bottom() - 5.0),
                        Pos2::new(body.right() - 4.0, body.bottom() - 5.0),
                    ],
                    stroke,
                );
            }
            IconKey::Undo | IconKey::Redo => {
                let glyph = if key == IconKey::Undo { "↶" } else { "↷" };
                painter.text(
                    center,
                    Align2::CENTER_CENTER,
                    glyph,
                    FontId::proportional(size * 0.72),
                    tint,
                );
            }
            IconKey::Settings => {
                painter.circle_stroke(center, half * 0.75, Stroke::new(3.0, tint));
                painter.circle_filled(center, half * 0.25, tint);
            }
            IconKey::Close => {
                painter.line_segment(
                    [
                        Pos2::new(center.x - half, center.y - half),
                        Pos2::new(center.x + half, center.y + half),
                    ],
                    stroke,
                );
                painter.line_segment(
                    [
                        Pos2::new(center.x + half, center.y - half),
                        Pos2::new(center.x - half, center.y + half),
                    ],
                    stroke,
                );
            }
            IconKey::ExRay => {
                painter.circle_stroke(center, half * 0.8, stroke);
                painter.line_segment(
                    [
                        Pos2::new(center.x - half, center.y + half),
                        Pos2::new(center.x + half, center.y - half),
                    ],
                    Stroke::new(2.5, tint),
                );
            }
            IconKey::Subtract => {
                painter.rect_stroke(
                    Rect::from_center_size(center, Vec2::splat(half * 1.5)),
                    2.0,
                    stroke,
                    egui::StrokeKind::Inside,
                );
                painter.line_segment(
                    [
                        Pos2::new(center.x - half * 0.6, center.y),
                        Pos2::new(center.x + half * 0.6, center.y),
                    ],
                    Stroke::new(2.5, tint),
                );
            }
        }
    }
}

fn paint_primitive(
    painter: &Painter,
    rect: Rect,
    kind: PrimitiveKind,
    tint: Color32,
    stroke: Stroke,
) {
    let center = rect.center();
    let half = rect.size().min_elem() * 0.32;
    match kind {
        PrimitiveKind::Cube => {
            let body = Rect::from_center_size(center, Vec2::splat(half * 1.7));
            painter.rect_filled(body, 2.0, tint.linear_multiply(0.18));
            painter.rect_stroke(body, 2.0, stroke, egui::StrokeKind::Inside);
        }
        PrimitiveKind::Cylinder => {
            painter.circle_filled(center, half * 0.8, tint.linear_multiply(0.18));
            painter.circle_stroke(center, half * 0.8, stroke);
            painter.line_segment(
                [
                    Pos2::new(center.x - half * 0.8, center.y - 2.0),
                    Pos2::new(center.x - half * 0.8, center.y + half * 0.8),
                ],
                stroke,
            );
        }
        PrimitiveKind::Pyramid | PrimitiveKind::Cone => {
            let points = vec![
                Pos2::new(center.x, center.y - half),
                Pos2::new(center.x + half, center.y + half),
                Pos2::new(center.x - half, center.y + half),
            ];
            painter.add(egui::Shape::convex_polygon(
                points,
                tint.linear_multiply(0.18),
                stroke,
            ));
        }
        PrimitiveKind::Sphere | PrimitiveKind::Torus => {
            painter.circle_stroke(center, half, stroke);
            painter.add(egui::Shape::ellipse_stroke(
                center,
                Vec2::new(half, half * 0.35),
                stroke,
            ));
        }
        PrimitiveKind::Hexagon | PrimitiveKind::Wedge | PrimitiveKind::Tetrahedron => {
            painter.circle_stroke(center, half, stroke);
            painter.line_segment(
                [
                    Pos2::new(center.x - half, center.y),
                    Pos2::new(center.x + half, center.y),
                ],
                stroke,
            );
        }
    }
}

fn validate_asset(asset: ArtworkAsset) -> Result<(), ArtworkError> {
    if asset.svg.len() > MAX_ARTWORK_BYTES {
        return Err(ArtworkError::Invalid {
            name: asset.name,
            reason: "asset exceeds the bounded SVG size",
        });
    }
    if !asset.svg.starts_with("<svg") || !asset.svg.contains("viewBox") {
        return Err(ArtworkError::Invalid {
            name: asset.name,
            reason: "asset is not a self-contained SVG",
        });
    }
    if asset.svg.contains("http://")
        || asset.svg.contains("https://")
        || asset.svg.contains("<image")
    {
        return Err(ArtworkError::Invalid {
            name: asset.name,
            reason: "asset contains an external or raster dependency",
        });
    }
    Ok(())
}

fn svg_tint(svg: &str) -> Option<Color32> {
    let start = svg.find("fill=\"#")?.saturating_add(7);
    let hex = svg.get(start..start.saturating_add(6))?;
    let red = u8::from_str_radix(hex.get(0..2)?, 16).ok()?;
    let green = u8::from_str_radix(hex.get(2..4)?, 16).ok()?;
    let blue = u8::from_str_radix(hex.get(4..6)?, 16).ok()?;
    Some(Color32::from_rgb(red, green, blue))
}

const fn all_icon_keys() -> [IconKey; 20] {
    [
        IconKey::Folder,
        IconKey::Object,
        IconKey::Primitive(PrimitiveKind::Cube),
        IconKey::Primitive(PrimitiveKind::Cylinder),
        IconKey::Primitive(PrimitiveKind::Pyramid),
        IconKey::Primitive(PrimitiveKind::Cone),
        IconKey::Primitive(PrimitiveKind::Sphere),
        IconKey::Primitive(PrimitiveKind::Torus),
        IconKey::Primitive(PrimitiveKind::Hexagon),
        IconKey::Primitive(PrimitiveKind::Wedge),
        IconKey::Edit,
        IconKey::View,
        IconKey::Save,
        IconKey::Undo,
        IconKey::Redo,
        IconKey::Settings,
        IconKey::Close,
        IconKey::ExRay,
        IconKey::Subtract,
        IconKey::Primitive(PrimitiveKind::Tetrahedron),
    ]
}

const fn asset_name(key: IconKey) -> &'static str {
    match key {
        IconKey::Folder => "folder.svg",
        IconKey::Object => "object.svg",
        IconKey::Primitive(PrimitiveKind::Cube) => "cube.svg",
        IconKey::Primitive(PrimitiveKind::Cylinder) => "cylinder.svg",
        IconKey::Primitive(PrimitiveKind::Pyramid) => "pyramid.svg",
        IconKey::Primitive(PrimitiveKind::Cone) => "cone.svg",
        IconKey::Primitive(PrimitiveKind::Sphere) => "sphere.svg",
        IconKey::Primitive(PrimitiveKind::Torus) => "torus.svg",
        IconKey::Primitive(PrimitiveKind::Hexagon) => "hexagon.svg",
        IconKey::Primitive(PrimitiveKind::Wedge) => "wedge.svg",
        IconKey::Primitive(PrimitiveKind::Tetrahedron) => "tetrahedron.svg",
        IconKey::Edit => "edit.svg",
        IconKey::View => "view.svg",
        IconKey::Save => "save.svg",
        IconKey::Undo => "undo.svg",
        IconKey::Redo => "redo.svg",
        IconKey::Settings => "settings.svg",
        IconKey::Close => "close.svg",
        IconKey::ExRay => "ExRay.svg",
        IconKey::Subtract => "Subtract.svg",
    }
}

fn light_asset(key: IconKey) -> &'static str {
    match key {
        IconKey::Folder => include_str!("../assets/icons/light/folder.svg"),
        IconKey::Object => include_str!("../assets/icons/light/object.svg"),
        IconKey::Primitive(PrimitiveKind::Cube) => include_str!("../assets/icons/light/cube.svg"),
        IconKey::Primitive(PrimitiveKind::Cylinder) => {
            include_str!("../assets/icons/light/cylinder.svg")
        }
        IconKey::Primitive(PrimitiveKind::Pyramid) => {
            include_str!("../assets/icons/light/pyramid.svg")
        }
        IconKey::Primitive(PrimitiveKind::Cone) => include_str!("../assets/icons/light/cone.svg"),
        IconKey::Primitive(PrimitiveKind::Sphere) => {
            include_str!("../assets/icons/light/sphere.svg")
        }
        IconKey::Primitive(PrimitiveKind::Torus) => include_str!("../assets/icons/light/torus.svg"),
        IconKey::Primitive(PrimitiveKind::Hexagon) => {
            include_str!("../assets/icons/light/hexagon.svg")
        }
        IconKey::Primitive(PrimitiveKind::Wedge) => include_str!("../assets/icons/light/wedge.svg"),
        IconKey::Primitive(PrimitiveKind::Tetrahedron) => {
            include_str!("../assets/icons/light/tetrahedron.svg")
        }
        IconKey::Edit => include_str!("../assets/icons/light/edit.svg"),
        IconKey::View => include_str!("../assets/icons/light/view.svg"),
        IconKey::Save => include_str!("../assets/icons/light/save.svg"),
        IconKey::Undo => include_str!("../assets/icons/light/undo.svg"),
        IconKey::Redo => include_str!("../assets/icons/light/redo.svg"),
        IconKey::Settings => include_str!("../assets/icons/light/settings.svg"),
        IconKey::Close => include_str!("../assets/icons/light/close.svg"),
        IconKey::ExRay => include_str!("../assets/icons/light/ExRay.svg"),
        IconKey::Subtract => include_str!("../assets/icons/light/Subtract.svg"),
    }
}

fn dark_asset(key: IconKey) -> &'static str {
    match key {
        IconKey::Folder => include_str!("../assets/icons/dark/folder.svg"),
        IconKey::Object => include_str!("../assets/icons/dark/object.svg"),
        IconKey::Primitive(PrimitiveKind::Cube) => include_str!("../assets/icons/dark/cube.svg"),
        IconKey::Primitive(PrimitiveKind::Cylinder) => {
            include_str!("../assets/icons/dark/cylinder.svg")
        }
        IconKey::Primitive(PrimitiveKind::Pyramid) => {
            include_str!("../assets/icons/dark/pyramid.svg")
        }
        IconKey::Primitive(PrimitiveKind::Cone) => include_str!("../assets/icons/dark/cone.svg"),
        IconKey::Primitive(PrimitiveKind::Sphere) => {
            include_str!("../assets/icons/dark/sphere.svg")
        }
        IconKey::Primitive(PrimitiveKind::Torus) => include_str!("../assets/icons/dark/torus.svg"),
        IconKey::Primitive(PrimitiveKind::Hexagon) => {
            include_str!("../assets/icons/dark/hexagon.svg")
        }
        IconKey::Primitive(PrimitiveKind::Wedge) => include_str!("../assets/icons/dark/wedge.svg"),
        IconKey::Primitive(PrimitiveKind::Tetrahedron) => {
            include_str!("../assets/icons/dark/tetrahedron.svg")
        }
        IconKey::Edit => include_str!("../assets/icons/dark/edit.svg"),
        IconKey::View => include_str!("../assets/icons/dark/view.svg"),
        IconKey::Save => include_str!("../assets/icons/dark/save.svg"),
        IconKey::Undo => include_str!("../assets/icons/dark/undo.svg"),
        IconKey::Redo => include_str!("../assets/icons/dark/redo.svg"),
        IconKey::Settings => include_str!("../assets/icons/dark/settings.svg"),
        IconKey::Close => include_str!("../assets/icons/dark/close.svg"),
        IconKey::ExRay => include_str!("../assets/icons/dark/ExRay.svg"),
        IconKey::Subtract => include_str!("../assets/icons/dark/Subtract.svg"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_theme_catalogs_validate_every_compact_asset() {
        assert!(ThemeArtwork::new(ThemeMode::Light).validate().is_ok());
        assert!(ThemeArtwork::new(ThemeMode::Dark).validate().is_ok());
    }

    #[test]
    fn theme_artwork_is_scoped_and_not_shared_raster_data() {
        let light = ThemeArtwork::new(ThemeMode::Light).asset(IconKey::ExRay);
        let dark = ThemeArtwork::new(ThemeMode::Dark).asset(IconKey::ExRay);
        assert_ne!(light.svg, dark.svg);
        assert!(light.svg.len() < MAX_ARTWORK_BYTES);
        assert!(dark.svg.len() < MAX_ARTWORK_BYTES);
    }

    #[test]
    fn staged_package_artwork_validates_beside_the_executable() {
        let destination = std::env::temp_dir().join(format!(
            "builder3d-stage-artwork-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let result = stage_original_artwork(&destination);
        let staged_root = destination.join("assets").join("original");
        let validation = validate_original_artwork(&staged_root);
        let _ = fs::remove_dir_all(&destination);
        assert!(result.is_ok(), "staging failed: {result:?}");
        assert!(validation.is_ok(), "staged artwork invalid: {validation:?}");
    }
}
