//! Bounded average image-color sampling for the application layer.

use std::{
    fs::File,
    io::{Cursor, Read},
    path::Path,
};

use image::ImageReader;
use thiserror::Error;

/// Maximum encoded image size accepted by the desktop sampler.
pub const MAX_IMAGE_BYTES: u64 = 64 * 1024 * 1024;
/// Maximum decoded pixels accepted by one color sample.
pub const MAX_IMAGE_PIXELS: u64 = 16 * 1024 * 1024;

/// Typed errors from image-color sampling.
#[derive(Debug, Error)]
pub enum ImageColorError {
    /// A zero sampling step cannot make progress.
    #[error("sample step must be at least 1")]
    InvalidSampleStep,
    /// The encoded image exceeded the bounded input contract.
    #[error("image exceeds the bounded sample size ({bytes} bytes > {MAX_IMAGE_BYTES} bytes)")]
    ImageTooLarge { bytes: u64 },
    /// The bounded input buffer could not be reserved.
    #[error("image could not be buffered within the configured sample limit")]
    BufferAllocation,
    /// The decoded image exceeded the bounded pixel contract.
    #[error("image exceeds the bounded pixel count ({pixels} > {MAX_IMAGE_PIXELS})")]
    TooManyPixels { pixels: u64 },
    /// The image path could not be read.
    #[error("image read failed: {0}")]
    Io(#[from] std::io::Error),
    /// The image bytes were not a supported/valid configured format.
    #[error("image decode failed: {0}")]
    Decode(#[from] image::ImageError),
    /// A valid image contained no sampleable pixels.
    #[error("image contains no pixels")]
    EmptyImage,
}

/// Returns an opaque RGBA average using a bounded regular pixel sample.
pub fn average_image_color(path: &Path, sample_step: usize) -> Result<[f32; 4], ImageColorError> {
    if sample_step == 0 {
        return Err(ImageColorError::InvalidSampleStep);
    }
    let mut file = File::open(path)?;
    let bytes = file.metadata()?.len();
    if bytes > MAX_IMAGE_BYTES {
        return Err(ImageColorError::ImageTooLarge { bytes });
    }
    let capacity = usize::try_from(bytes).map_err(|_| ImageColorError::ImageTooLarge { bytes })?;
    let mut encoded = Vec::new();
    encoded
        .try_reserve_exact(capacity)
        .map_err(|_| ImageColorError::BufferAllocation)?;
    file.by_ref()
        .take(MAX_IMAGE_BYTES.saturating_add(1))
        .read_to_end(&mut encoded)?;
    let encoded_bytes = u64::try_from(encoded.len()).unwrap_or(u64::MAX);
    if encoded_bytes > MAX_IMAGE_BYTES {
        return Err(ImageColorError::ImageTooLarge {
            bytes: encoded_bytes,
        });
    }

    let dimensions = ImageReader::new(Cursor::new(encoded.as_slice()))
        .with_guessed_format()?
        .into_dimensions()?;
    let pixels = u64::from(dimensions.0) * u64::from(dimensions.1);
    if pixels > MAX_IMAGE_PIXELS {
        return Err(ImageColorError::TooManyPixels { pixels });
    }
    if pixels == 0 {
        return Err(ImageColorError::EmptyImage);
    }

    let image = ImageReader::new(Cursor::new(encoded.as_slice()))
        .with_guessed_format()?
        .decode()?;
    let rgb = image.to_rgb8();

    let mut sums = [0_u64; 3];
    let mut samples = 0_u64;
    for y in (0..rgb.height()).step_by(sample_step) {
        for x in (0..rgb.width()).step_by(sample_step) {
            let channels = rgb.get_pixel(x, y).0;
            sums[0] += u64::from(channels[0]);
            sums[1] += u64::from(channels[1]);
            sums[2] += u64::from(channels[2]);
            samples += 1;
        }
    }
    if samples == 0 {
        return Err(ImageColorError::EmptyImage);
    }
    Ok([
        sums[0] as f32 / (255.0 * samples as f32),
        sums[1] as f32 / (255.0 * samples as f32),
        sums[2] as f32 / (255.0 * samples as f32),
        1.0,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

    fn temp_ppm() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "modeler3d-image-color-{}-{}.ppm",
            std::process::id(),
            NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[test]
    fn average_color_matches_reference_ppm_sampling() {
        let path = temp_ppm();
        let write_result = fs::write(&path, b"P3\n2 1\n255\n255 0 0 0 0 255\n");
        assert!(write_result.is_ok());
        let color = average_image_color(&path, 1);
        assert!(color.is_ok());
        assert_eq!(color.ok(), Some([0.5, 0.0, 0.5, 1.0]));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn sample_step_and_encoded_size_are_bounded() {
        let path = temp_ppm();
        assert!(fs::write(&path, b"P3\n1 1\n255\n0 255 0\n").is_ok());
        assert!(matches!(
            average_image_color(&path, 0),
            Err(ImageColorError::InvalidSampleStep)
        ));
        assert_eq!(
            average_image_color(&path, 2).ok(),
            Some([0.0, 1.0, 0.0, 1.0])
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn corrupt_and_unsupported_inputs_return_decode_errors() {
        let path = temp_ppm();
        assert!(fs::write(&path, b"not an image").is_ok());
        assert!(matches!(
            average_image_color(&path, 1),
            Err(ImageColorError::Decode(_))
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn encoded_size_limit_is_checked_before_reading() {
        let path = temp_ppm();
        let file = fs::File::create(&path);
        assert!(file.is_ok());
        if let Ok(file) = file {
            assert!(file.set_len(MAX_IMAGE_BYTES + 1).is_ok());
        }
        assert!(matches!(
            average_image_color(&path, 1),
            Err(ImageColorError::ImageTooLarge { bytes }) if bytes == MAX_IMAGE_BYTES + 1
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn decoded_pixel_limit_is_checked_from_header_before_decode() {
        let path = temp_ppm();
        let image = image::RgbImage::from_pixel(4_097, 4_097, image::Rgb([10, 20, 30]));
        let write = image.save_with_format(&path, image::ImageFormat::Png);
        assert!(write.is_ok());
        assert!(matches!(
            average_image_color(&path, 1),
            Err(ImageColorError::TooManyPixels { pixels }) if pixels > MAX_IMAGE_PIXELS
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn unsupported_or_missing_paths_return_typed_errors() {
        let path = std::env::temp_dir().join(format!(
            "modeler3d-image-color-missing-{}.png",
            std::process::id()
        ));
        assert!(matches!(
            average_image_color(&path, 1),
            Err(ImageColorError::Io(_))
        ));
    }

    #[test]
    fn directory_input_is_rejected_as_an_unreadable_image_file() {
        let path = std::env::temp_dir().join(format!(
            "modeler3d-image-color-directory-{}-{}",
            std::process::id(),
            NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        assert!(fs::create_dir_all(&path).is_ok());
        assert!(matches!(
            average_image_color(&path, 1),
            Err(ImageColorError::Io(_))
        ));
        assert!(fs::remove_dir(path).is_ok());
    }
}
