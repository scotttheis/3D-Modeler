//! Shrinks the artwork inside a staged release package (never the source tree).
//!
//! The packaging scripts run `Modeler3D --shrink-assets <package assets/original>`
//! on their staged copy. Any PNG in the four icon folders that is larger than
//! the limit on a side is scaled down with a smooth filter, keeping its shape;
//! smaller pictures are left alone. The full-size artwork stays in
//! `crates/modeler3d-app/assets`. This is platform independent.
use image::{ImageReader, imageops::FilterType};
use std::{fs, io, path::Path};

/// Longest side, in pixels, of an icon in a release package (owner, 2026-11-19).
pub(crate) const RELEASE_ICON_PIXELS: u32 = 128;

const ICON_FOLDERS: [&str; 4] = [
    "action_icons",
    "action_icons_light",
    "insert_icons",
    "insert_icons_light",
];

/// What a shrink run changed.
#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct ShrinkReport {
    /// Pictures that were scaled down.
    pub(crate) shrunk: u32,
    /// Their total size before, in bytes.
    pub(crate) bytes_before: u64,
    /// Their total size after, in bytes.
    pub(crate) bytes_after: u64,
}

/// Scales every oversized icon under `assets_root` down to `max_pixels`.
pub(crate) fn shrink_assets(assets_root: &Path, max_pixels: u32) -> io::Result<ShrinkReport> {
    let mut report = ShrinkReport::default();
    for folder in ICON_FOLDERS {
        for entry in fs::read_dir(assets_root.join(folder))? {
            let path = entry?.path();
            if path
                .extension()
                .is_none_or(|extension| !extension.eq_ignore_ascii_case("png"))
            {
                continue;
            }
            let before = fs::metadata(&path)?.len();
            let picture = ImageReader::open(&path)?
                .decode()
                .map_err(io::Error::other)?;
            if picture.width() <= max_pixels && picture.height() <= max_pixels {
                continue;
            }
            picture
                .resize(max_pixels, max_pixels, FilterType::Lanczos3)
                .save(&path)
                .map_err(io::Error::other)?;
            report.shrunk += 1;
            report.bytes_before += before;
            report.bytes_after += fs::metadata(&path)?.len();
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn big_icons_shrink_small_ones_stay_and_the_shape_is_kept() {
        let root = std::env::temp_dir().join(format!("modeler3d-shrink-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for folder in ICON_FOLDERS {
            fs::create_dir_all(root.join(folder)).expect("folder");
        }
        let big = root.join("action_icons").join("big.png");
        image::RgbaImage::new(400, 300).save(&big).expect("big");
        let small = root.join("insert_icons_light").join("small.png");
        image::RgbaImage::new(100, 100).save(&small).expect("small");
        let small_bytes = fs::read(&small).expect("small bytes");

        let report = shrink_assets(&root, 128).expect("shrink");

        assert_eq!(report.shrunk, 1);
        assert_eq!(image::image_dimensions(&big).expect("big size"), (128, 96));
        assert_eq!(fs::read(&small).expect("small again"), small_bytes);
        let _ = fs::remove_dir_all(&root);
    }
}
