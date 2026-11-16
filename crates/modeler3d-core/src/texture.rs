//! Material textures (OF-46): one image per object, box-mapped in object
//! space and stored inside the `.3dp`.
//!
//! The image itself lives in `scene.json` under a top-level `textures` table
//! (content-addressed by SHA-256, base64), because the Python reference
//! rejects any ZIP member it does not expect but ignores unknown top-level
//! keys. An object refers to its image through the `texture` metadata key,
//! so the Python build opens a textured project and simply shows the solid
//! colour; a project resaved there keeps the reference but loses the image,
//! and the Rust build then also falls back to the colour.
//!
//! Mapping is a box projection in object-local millimetres: each surface
//! takes the image from the axis its normal faces most, repeated every
//! `tile_mm`. The GPU, the CPU fallback and 3MF export all use [`box_uv`] /
//! the same axis rule, so the texture stays put while the object moves and
//! survives sculpting and other topology edits.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex, OnceLock};

use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader, Limits};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::CoreError;

/// Object metadata key holding `{"id": <sha256 hex>, "tile_mm": <f64>}`.
pub const TEXTURE_METADATA_KEY: &str = "texture";
/// Longest stored side in pixels; larger images are scaled down on import.
pub const MAX_TEXTURE_SIDE_PX: u32 = 1024;
/// Largest stored (encoded) image in bytes.
pub const MAX_TEXTURE_STORED_BYTES: usize = 1_536 * 1024;
/// Largest total of distinct stored images in one project, in bytes. With
/// base64 this keeps `scene.json` below its 8 MiB limit in both builds.
pub const MAX_PROJECT_TEXTURE_BYTES: usize = 5 * 1024 * 1024;
/// Largest image file accepted for import.
pub const MAX_TEXTURE_SOURCE_BYTES: usize = 64 * 1024 * 1024;
/// Largest source side accepted for import, in pixels.
pub const MAX_TEXTURE_SOURCE_SIDE_PX: u32 = 16_384;
/// Smallest and largest repeat size in millimetres.
pub const MIN_TEXTURE_TILE_MM: f32 = 1.0;
/// Largest repeat size in millimetres.
pub const MAX_TEXTURE_TILE_MM: f32 = 1_000.0;
/// Repeat size used when none is given.
pub const DEFAULT_TEXTURE_TILE_MM: f32 = 50.0;
/// JPEG quality for opaque images.
const JPEG_QUALITY: u8 = 88;

/// How a stored texture image is encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureEncoding {
    /// PNG, used when the image has transparency.
    Png,
    /// JPEG, used for opaque images.
    Jpeg,
}

impl TextureEncoding {
    /// MIME type written to `scene.json` and to 3MF.
    #[must_use]
    pub const fn mime(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
        }
    }

    /// File extension used in exports.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
        }
    }

    fn from_mime(mime: &str) -> Option<Self> {
        match mime {
            "image/png" => Some(Self::Png),
            "image/jpeg" => Some(Self::Jpeg),
            _ => None,
        }
    }

    const fn format(self) -> ImageFormat {
        match self {
            Self::Png => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
        }
    }
}

/// One validated texture image: its stored bytes and its decoded pixels.
#[derive(Debug)]
pub struct TextureImage {
    id: String,
    encoding: TextureEncoding,
    encoded: Arc<[u8]>,
    width: u32,
    height: u32,
    rgba: Arc<[u8]>,
    average: [f32; 4],
}

impl TextureImage {
    /// Imports any supported image file: decodes it within bounded limits,
    /// scales it so its longest side is at most [`MAX_TEXTURE_SIDE_PX`] and
    /// re-encodes it (JPEG when opaque, PNG otherwise) for storage.
    pub fn from_source_bytes(bytes: &[u8]) -> Result<Self, CoreError> {
        if bytes.len() > MAX_TEXTURE_SOURCE_BYTES {
            return Err(CoreError::InvalidTexture(format!(
                "the image file is larger than {} MB",
                MAX_TEXTURE_SOURCE_BYTES / (1024 * 1024)
            )));
        }
        let decoded = decode_bounded(bytes, None, MAX_TEXTURE_SOURCE_SIDE_PX)?;
        let (width, height) = decoded.dimensions();
        let scaled = if width.max(height) > MAX_TEXTURE_SIDE_PX {
            decoded.resize(
                MAX_TEXTURE_SIDE_PX,
                MAX_TEXTURE_SIDE_PX,
                image::imageops::FilterType::CatmullRom,
            )
        } else {
            decoded
        };
        let rgba = scaled.to_rgba8();
        let opaque = rgba.pixels().all(|pixel| pixel[3] == u8::MAX);
        let mut encoded = Vec::new();
        let encoding = if opaque {
            let rgb = DynamicImage::ImageRgba8(rgba.clone()).to_rgb8();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, JPEG_QUALITY)
                .encode_image(&rgb)
                .map_err(|error| {
                    CoreError::InvalidTexture(format!("the image could not be stored: {error}"))
                })?;
            TextureEncoding::Jpeg
        } else {
            DynamicImage::ImageRgba8(rgba)
                .write_to(&mut Cursor::new(&mut encoded), ImageFormat::Png)
                .map_err(|error| {
                    CoreError::InvalidTexture(format!("the image could not be stored: {error}"))
                })?;
            TextureEncoding::Png
        };
        Self::from_stored(encoding, &encoded)
    }

    /// Rebuilds a stored texture from `scene.json` (or after import),
    /// checking its size, encoding and dimensions.
    pub fn from_stored(encoding: TextureEncoding, bytes: &[u8]) -> Result<Self, CoreError> {
        if bytes.len() > MAX_TEXTURE_STORED_BYTES {
            return Err(CoreError::InvalidTexture(format!(
                "a stored texture is larger than {} KB; use a smaller or simpler image",
                MAX_TEXTURE_STORED_BYTES / 1024
            )));
        }
        let decoded = decode_bounded(bytes, Some(encoding.format()), MAX_TEXTURE_SIDE_PX)?;
        let rgba = decoded.to_rgba8();
        let (width, height) = rgba.dimensions();
        let pixels = rgba.into_raw();
        let average = average_rgba(&pixels);
        Ok(Self {
            id: texture_id(bytes),
            encoding,
            encoded: Arc::from(bytes),
            width,
            height,
            rgba: Arc::from(pixels),
            average,
        })
    }

    /// Content address: lowercase SHA-256 hex of the stored bytes.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Stored encoding.
    #[must_use]
    pub const fn encoding(&self) -> TextureEncoding {
        self.encoding
    }

    /// Stored (encoded) bytes.
    #[must_use]
    pub fn encoded(&self) -> &[u8] {
        &self.encoded
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Decoded sRGB RGBA8 pixels, row-major, top row first.
    #[must_use]
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    /// Average sRGB colour in 0..=1, used where a single colour stands in
    /// for the image (reflections, flat previews).
    #[must_use]
    pub const fn average_color(&self) -> [f32; 4] {
        self.average
    }

    /// Bilinear, repeating sample at texture coordinates (1.0 = one tile),
    /// as sRGB RGBA in 0..=1. `v` grows upward in the model, so image rows
    /// are read top-down from `1 - v`.
    #[must_use]
    pub fn sample(&self, uv: [f32; 2]) -> [f32; 4] {
        if self.width == 0 || self.height == 0 || !uv[0].is_finite() || !uv[1].is_finite() {
            return self.average;
        }
        let x = uv[0].rem_euclid(1.0) * self.width as f32 - 0.5;
        let y = (1.0 - uv[1].rem_euclid(1.0)) * self.height as f32 - 0.5;
        let x0 = x.floor();
        let y0 = y.floor();
        let fx = x - x0;
        let fy = y - y0;
        let wrap = |value: f32, size: u32| (value as i64).rem_euclid(i64::from(size)) as usize;
        let (x0i, x1i) = (wrap(x0, self.width), wrap(x0 + 1.0, self.width));
        let (y0i, y1i) = (wrap(y0, self.height), wrap(y0 + 1.0, self.height));
        let texel = |x: usize, y: usize| -> [f32; 4] {
            let offset = (y * self.width as usize + x) * 4;
            self.rgba
                .get(offset..offset + 4)
                .map_or(self.average, |pixel| {
                    [
                        f32::from(pixel[0]) / 255.0,
                        f32::from(pixel[1]) / 255.0,
                        f32::from(pixel[2]) / 255.0,
                        f32::from(pixel[3]) / 255.0,
                    ]
                })
        };
        let (a, b, c, d) = (
            texel(x0i, y0i),
            texel(x1i, y0i),
            texel(x0i, y1i),
            texel(x1i, y1i),
        );
        let mut result = [0.0; 4];
        for channel in 0..4 {
            let top = a[channel] + (b[channel] - a[channel]) * fx;
            let bottom = c[channel] + (d[channel] - c[channel]) * fx;
            result[channel] = top + (bottom - top) * fy;
        }
        result
    }
}

impl TextureImage {
    /// Box-projected sample at a local point with its (interpolated) normal,
    /// blending the three axis projections by [`box_weights`] exactly as the
    /// GPU shader does. sRGB RGBA in 0..=1.
    #[must_use]
    pub fn sample_box(&self, position: [f32; 3], normal: [f32; 3], tile_mm: f32) -> [f32; 4] {
        let weights = box_weights(normal);
        let mut result = [0.0; 4];
        for (axis, weight) in weights.into_iter().enumerate() {
            if weight <= 0.001 {
                continue;
            }
            let texel = self.sample(box_uv(position, normal, axis, tile_mm));
            for (total, value) in result.iter_mut().zip(texel) {
                *total += value * weight;
            }
        }
        result
    }
}

/// How sharply the box projection blends between axes near an edge. Keep
/// in step with `TEXTURE_BLEND_POWER` in the renderer's WGSL.
pub const BOX_BLEND_POWER: i32 = 8;

/// Blend weights of the X, Y and Z projections for a normal; they sum to 1.
#[must_use]
pub fn box_weights(normal: [f32; 3]) -> [f32; 3] {
    let weights = normal.map(|value| value.abs().powi(BOX_BLEND_POWER));
    let sum = weights[0] + weights[1] + weights[2];
    if !sum.is_finite() || sum <= f32::EPSILON {
        let mut fallback = [0.0; 3];
        fallback[box_axis(normal)] = 1.0;
        return fallback;
    }
    weights.map(|value| value / sum)
}

fn decode_bounded(
    bytes: &[u8],
    format: Option<ImageFormat>,
    max_side: u32,
) -> Result<DynamicImage, CoreError> {
    let mut reader = ImageReader::new(Cursor::new(bytes));
    if let Some(format) = format {
        reader.set_format(format);
    } else {
        reader = reader.with_guessed_format().map_err(|_| {
            CoreError::InvalidTexture("the file is not a readable image".to_owned())
        })?;
        if reader.format().is_none() {
            return Err(CoreError::InvalidTexture(
                "the file is not a readable image".to_owned(),
            ));
        }
    }
    let mut limits = Limits::default();
    limits.max_image_width = Some(max_side);
    limits.max_image_height = Some(max_side);
    limits.max_alloc = Some(1024 * 1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().map_err(|error| match error {
        image::ImageError::Limits(_) => CoreError::InvalidTexture(format!(
            "the image is larger than {max_side} pixels on a side"
        )),
        image::ImageError::Unsupported(_) => CoreError::InvalidTexture(
            "that image type is not supported; use PNG, JPEG, BMP, GIF, TGA, TIFF or WebP"
                .to_owned(),
        ),
        _ => CoreError::InvalidTexture("the file is not a readable image".to_owned()),
    })?;
    if image.width() == 0 || image.height() == 0 {
        return Err(CoreError::InvalidTexture("the image is empty".to_owned()));
    }
    Ok(image)
}

fn average_rgba(pixels: &[u8]) -> [f32; 4] {
    let mut sum = [0u64; 4];
    let mut count = 0u64;
    for pixel in pixels.as_chunks::<4>().0 {
        for (total, value) in sum.iter_mut().zip(pixel) {
            *total += u64::from(*value);
        }
        count += 1;
    }
    if count == 0 {
        return [1.0; 4];
    }
    sum.map(|total| (total as f64 / count as f64 / 255.0) as f32)
}

fn texture_id(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Process-wide, content-addressed store of texture images. Entries are
/// immutable and keyed by their own hash, so sharing them between scenes,
/// undo history and adapters cannot change any scene's meaning.
fn registry() -> &'static Mutex<HashMap<String, Arc<TextureImage>>> {
    static REGISTRY: OnceLock<Mutex<HashMap<String, Arc<TextureImage>>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Adds an image to the store (or returns the identical one already there).
pub fn register_texture(image: TextureImage) -> Arc<TextureImage> {
    let mut store = registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    store
        .entry(image.id.clone())
        .or_insert_with(|| Arc::new(image))
        .clone()
}

/// Reads, imports and registers an image file in one step; the desktop
/// Material dialog and MCP `apply_texture` both use it.
pub fn import_texture_file(path: &std::path::Path) -> Result<Arc<TextureImage>, CoreError> {
    let unreadable = || {
        CoreError::InvalidTexture(format!(
            "the image file could not be read: {}",
            path.display()
        ))
    };
    let size = std::fs::metadata(path).map_err(|_| unreadable())?.len();
    if size > MAX_TEXTURE_SOURCE_BYTES as u64 {
        return Err(CoreError::InvalidTexture(format!(
            "the image file is larger than {} MB",
            MAX_TEXTURE_SOURCE_BYTES / (1024 * 1024)
        )));
    }
    let bytes = std::fs::read(path).map_err(|_| unreadable())?;
    Ok(register_texture(TextureImage::from_source_bytes(&bytes)?))
}

/// Looks up a stored image by its id.
#[must_use]
pub fn texture_by_id(id: &str) -> Option<Arc<TextureImage>> {
    registry()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(id)
        .cloned()
}

/// An object's texture reference, read from its metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectTexture {
    /// Content id of the image.
    pub id: String,
    /// Repeat size in millimetres.
    pub tile_mm: f32,
}

impl ObjectTexture {
    /// Builds a reference, clamping the tile to its allowed range.
    #[must_use]
    pub fn new(id: impl Into<String>, tile_mm: f32) -> Self {
        Self {
            id: id.into(),
            tile_mm: clamp_tile_mm(tile_mm),
        }
    }
}

/// Clamps a repeat size to the allowed range; non-finite values take the
/// default.
#[must_use]
pub fn clamp_tile_mm(tile_mm: f32) -> f32 {
    if tile_mm.is_finite() {
        tile_mm.clamp(MIN_TEXTURE_TILE_MM, MAX_TEXTURE_TILE_MM)
    } else {
        DEFAULT_TEXTURE_TILE_MM
    }
}

/// Reads a well-formed texture reference from object metadata.
#[must_use]
pub fn object_texture(metadata: &Value) -> Option<ObjectTexture> {
    let entry = metadata.get(TEXTURE_METADATA_KEY)?.as_object()?;
    let id = entry.get("id")?.as_str()?;
    if id.len() != 64 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let tile_mm = entry
        .get("tile_mm")
        .and_then(Value::as_f64)
        .map_or(DEFAULT_TEXTURE_TILE_MM, |value| value as f32);
    Some(ObjectTexture::new(id, tile_mm))
}

/// Returns the object's texture image when its reference resolves.
#[must_use]
pub fn resolved_texture(metadata: &Value) -> Option<(Arc<TextureImage>, f32)> {
    let reference = object_texture(metadata)?;
    texture_by_id(&reference.id).map(|image| (image, reference.tile_mm))
}

/// Writes a texture reference, keeping all other metadata.
#[must_use]
pub fn with_texture(metadata: &Value, texture: &ObjectTexture) -> Value {
    let mut updated = metadata.clone();
    if let Some(fields) = updated.as_object_mut() {
        let mut entry = Map::new();
        entry.insert("id".to_owned(), Value::from(texture.id.clone()));
        entry.insert(
            "tile_mm".to_owned(),
            Value::from(f64::from(clamp_tile_mm(texture.tile_mm))),
        );
        fields.insert(TEXTURE_METADATA_KEY.to_owned(), Value::Object(entry));
    }
    updated
}

/// Removes the texture reference, keeping all other metadata.
#[must_use]
pub fn without_texture(metadata: &Value) -> Value {
    let mut updated = metadata.clone();
    if let Some(fields) = updated.as_object_mut() {
        fields.remove(TEXTURE_METADATA_KEY);
    }
    updated
}

/// The box-projection axis for a surface normal: 0 = X, 1 = Y, 2 = Z.
#[must_use]
pub fn box_axis(normal: [f32; 3]) -> usize {
    let [x, y, z] = normal.map(f32::abs);
    if x >= y && x >= z {
        0
    } else if y >= z {
        1
    } else {
        2
    }
}

/// Texture coordinates (in tiles) of a local point on a surface projected
/// along `axis`, facing the sign of `normal` on that axis. The image reads
/// upright and unmirrored on every side of a box standing on the floor.
#[must_use]
pub fn box_uv(position: [f32; 3], normal: [f32; 3], axis: usize, tile_mm: f32) -> [f32; 2] {
    let tile = clamp_tile_mm(tile_mm);
    let [x, y, z] = position;
    let uv = match axis {
        0 => {
            let sign = if normal[0] >= 0.0 { -1.0 } else { 1.0 };
            [sign * z, y]
        }
        1 => {
            let sign = if normal[1] >= 0.0 { 1.0 } else { -1.0 };
            [x, sign * -z]
        }
        _ => {
            let sign = if normal[2] >= 0.0 { 1.0 } else { -1.0 };
            [sign * x, y]
        }
    };
    [uv[0] / tile, uv[1] / tile]
}

/// Standard base64 (RFC 4648, padded).
#[must_use]
pub(crate) fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for (index, shift) in [18u32, 12, 6, 0].into_iter().enumerate() {
            if index <= chunk.len() {
                output.push(char::from(ALPHABET[((n >> shift) & 63) as usize]));
            } else {
                output.push('=');
            }
        }
    }
    output
}

/// Decodes standard padded base64; `None` on any malformed input.
#[must_use]
pub(crate) fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    let value = |byte: u8| -> Option<u32> {
        Some(u32::from(match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        }))
    };
    let mut output = Vec::with_capacity(bytes.len() / 4 * 3);
    let chunks = bytes.as_chunks::<4>().0;
    let last = chunks.len().saturating_sub(1);
    for (index, chunk) in chunks.iter().enumerate() {
        let padding = chunk.iter().rev().take_while(|byte| **byte == b'=').count();
        if padding > 2 || (padding > 0 && index != last) {
            return None;
        }
        let mut n = 0u32;
        for (position, byte) in chunk.iter().enumerate() {
            let digit = if position >= 4 - padding {
                0
            } else {
                value(*byte)?
            };
            n = (n << 6) | digit;
        }
        let decoded = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        output.extend_from_slice(&decoded[..3 - padding]);
    }
    Some(output)
}

/// Reads one `textures` table entry from `scene.json`.
pub(crate) fn texture_from_json(id: &str, entry: &Value) -> Result<TextureImage, CoreError> {
    let invalid = |reason: &str| CoreError::ProjectFormat(format!("texture {id}: {reason}"));
    let entry = entry
        .as_object()
        .ok_or_else(|| invalid("entry must be an object"))?;
    let encoding = entry
        .get("mime")
        .and_then(Value::as_str)
        .and_then(TextureEncoding::from_mime)
        .ok_or_else(|| invalid("mime must be image/png or image/jpeg"))?;
    let data = entry
        .get("data")
        .and_then(Value::as_str)
        .and_then(base64_decode)
        .ok_or_else(|| invalid("data must be base64"))?;
    let image =
        TextureImage::from_stored(encoding, &data).map_err(|error| invalid(&error.to_string()))?;
    if image.id() != id {
        return Err(invalid("data does not match its id"));
    }
    Ok(image)
}

/// Writes one `textures` table entry for `scene.json`.
pub(crate) fn texture_to_json(image: &TextureImage) -> Value {
    let mut entry = Map::new();
    entry.insert("mime".to_owned(), Value::from(image.encoding().mime()));
    entry.insert(
        "data".to_owned(),
        Value::from(base64_encode(image.encoded())),
    );
    Value::Object(entry)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checker_png(size: u32, alpha: u8) -> Vec<u8> {
        let image = image::RgbaImage::from_fn(size, size, |x, y| {
            if (x / 8 + y / 8) % 2 == 0 {
                image::Rgba([200, 30, 30, alpha])
            } else {
                image::Rgba([30, 30, 200, alpha])
            }
        });
        let mut bytes = Vec::new();
        DynamicImage::ImageRgba8(image)
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .expect("png");
        bytes
    }

    #[test]
    fn base64_round_trips_every_length() {
        for length in 0..40 {
            let bytes = (0..length)
                .map(|value| (value * 37 + 11) as u8)
                .collect::<Vec<_>>();
            let text = base64_encode(&bytes);
            assert_eq!(base64_decode(&text), Some(bytes));
        }
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert!(base64_decode("TW=u").is_none());
        assert!(base64_decode("TWE").is_none());
    }

    #[test]
    fn large_opaque_images_shrink_to_a_bounded_jpeg() {
        let source = checker_png(2048, 255);
        let image = TextureImage::from_source_bytes(&source).expect("import");
        assert_eq!(image.encoding(), TextureEncoding::Jpeg);
        assert_eq!((image.width(), image.height()), (1024, 1024));
        assert!(image.encoded().len() <= MAX_TEXTURE_STORED_BYTES);
        assert_eq!(image.id().len(), 64);
    }

    #[test]
    fn transparent_images_stay_png_and_round_trip_through_json() {
        let image = TextureImage::from_source_bytes(&checker_png(64, 128)).expect("import");
        assert_eq!(image.encoding(), TextureEncoding::Png);
        let json = texture_to_json(&image);
        let restored = texture_from_json(image.id(), &json).expect("restore");
        assert_eq!(restored.id(), image.id());
        assert_eq!(restored.rgba(), image.rgba());
        assert!(texture_from_json(&"0".repeat(64), &json).is_err());
    }

    #[test]
    fn non_images_are_rejected_in_plain_words() {
        let error = TextureImage::from_source_bytes(b"not an image").expect_err("reject");
        assert!(error.to_string().contains("not a readable image"));
    }

    #[test]
    fn metadata_reference_round_trips_and_clamps_the_tile() {
        let metadata = serde_json::json!({ "kept": 1 });
        let id = "a".repeat(64);
        let textured = with_texture(&metadata, &ObjectTexture::new(id.clone(), 5_000.0));
        let reference = object_texture(&textured).expect("reference");
        assert_eq!(reference.id, id);
        assert!((reference.tile_mm - MAX_TEXTURE_TILE_MM).abs() < f32::EPSILON);
        assert_eq!(textured["kept"], 1);
        assert!(object_texture(&without_texture(&textured)).is_none());
        assert!(object_texture(&serde_json::json!({ "texture": { "id": "bad" } })).is_none());
    }

    #[test]
    fn box_mapping_reads_upright_on_the_front_and_repeats_per_tile() {
        // Front face (+Z): u follows +X, v follows +Y.
        assert_eq!(
            box_uv([25.0, 50.0, 10.0], [0.0, 0.0, 1.0], 2, 50.0),
            [0.5, 1.0]
        );
        // Back face (-Z) is mirrored so it also reads left to right.
        assert_eq!(
            box_uv([25.0, 50.0, -10.0], [0.0, 0.0, -1.0], 2, 50.0),
            [-0.5, 1.0]
        );
        assert_eq!(box_axis([0.2, -0.9, 0.1]), 1);
        assert_eq!(box_axis([0.6, 0.6, 0.1]), 0);
    }

    #[test]
    fn sampling_repeats_and_reads_rows_top_down() {
        let image = TextureImage::from_source_bytes(&checker_png(16, 128)).expect("import");
        // Top-left texel block is red; v = 1 is the top row.
        let red = image.sample([0.1, 0.95]);
        assert!(red[0] > red[2]);
        let repeated = image.sample([1.1, 1.95]);
        assert!((red[0] - repeated[0]).abs() < 1e-4);
    }

    fn textured_scene() -> (crate::Scene, Arc<TextureImage>) {
        let image = register_texture(
            TextureImage::from_source_bytes(&checker_png(32, 255)).expect("import"),
        );
        let mut scene = crate::Scene::new();
        scene
            .add_object(crate::SceneObject::new(
                crate::make_box([20.0, 20.0, 20.0]).expect("box"),
            ))
            .expect("add");
        scene.apply_texture(&[0], &image, 30.0).expect("texture");
        (scene, image)
    }

    #[test]
    fn a_textured_project_round_trips_with_only_reference_members() {
        let (scene, image) = textured_scene();
        let bytes = crate::save_project_bytes(&scene).expect("save");
        // The Python build rejects unknown ZIP members, so the image must
        // live inside scene.json and the archive keeps its usual entries.
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes.clone())).expect("zip");
        let names = (0..archive.len())
            .map(|index| {
                archive
                    .by_index(index)
                    .expect("entry")
                    .name()
                    .expect("name")
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["meshes/mesh_0.stl", "scene.json"]);
        let mut json = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("scene.json").expect("scene"),
            &mut json,
        )
        .expect("read");
        let root: Value = serde_json::from_str(&json).expect("json");
        assert!(root["textures"][image.id()]["data"].is_string());
        let loaded = crate::load_project_bytes(&bytes).expect("load");
        let reference = object_texture(loaded.objects()[0].metadata()).expect("reference");
        assert_eq!(reference.id, image.id());
        assert!((reference.tile_mm - 30.0).abs() < 1e-4);
    }

    #[test]
    fn a_project_without_the_image_still_opens_and_shows_its_colour() {
        let (scene, _) = textured_scene();
        let bytes = crate::save_project_bytes(&scene).expect("save");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).expect("zip");
        let mut json = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("scene.json").expect("scene"),
            &mut json,
        )
        .expect("read");
        let mut root: Value = serde_json::from_str(&json).expect("json");
        // What the Python build writes after resaving: reference kept,
        // table dropped, and an id this process has never seen.
        root.as_object_mut().expect("root").remove("textures");
        root["objects"][0]["metadata"]["texture"]["id"] = Value::from("b".repeat(64));
        let mut stl = Vec::new();
        std::io::Read::read_to_end(
            &mut archive.by_name("meshes/mesh_0.stl").expect("mesh"),
            &mut stl,
        )
        .expect("mesh");
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        writer
            .start_file("meshes/mesh_0.stl", options)
            .expect("start");
        std::io::Write::write_all(&mut writer, &stl).expect("write");
        writer.start_file("scene.json", options).expect("start");
        std::io::Write::write_all(&mut writer, &serde_json::to_vec(&root).expect("json"))
            .expect("write");
        let rewritten = writer.finish().expect("finish").into_inner();
        let loaded = crate::load_project_bytes(&rewritten).expect("load");
        assert!(resolved_texture(loaded.objects()[0].metadata()).is_none());
        assert!(crate::save_project_bytes(&loaded).is_ok());
    }

    #[test]
    fn a_solid_colour_replaces_the_texture_and_undo_brings_it_back() {
        let (mut scene, _) = textured_scene();
        let color = scene.objects()[0].color();
        let result = scene
            .execute(crate::commands::SetColorCommand::new(0, color))
            .expect("color");
        assert!(result.changed, "same colour still removes the texture");
        assert!(object_texture(scene.objects()[0].metadata()).is_none());
        scene.undo().expect("undo");
        assert!(object_texture(scene.objects()[0].metadata()).is_some());
        let removed = scene.remove_texture(&[0]).expect("remove");
        assert!(removed.changed);
        let again = scene.remove_texture(&[0]).expect("remove");
        assert!(!again.changed, "removing nothing is a no-op");
    }

    #[test]
    fn a_textured_object_exports_a_3mf_texture_and_still_imports() {
        let (scene, image) = textured_scene();
        let object = &scene.objects()[0];
        let mut item = crate::ThreeMfItem::new(
            object.mesh().clone(),
            *object.transform(),
            object.name().to_owned(),
            object.color(),
        )
        .expect("item");
        item.texture = object_texture(object.metadata());
        let bytes = crate::export_3mf(&[item], None, None).expect("export");
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes.clone())).expect("zip");
        let texture_path = format!("3D/Textures/texture_{}.jpg", &image.id()[..16]);
        let mut stored = Vec::new();
        std::io::Read::read_to_end(
            &mut archive.by_name(&texture_path).expect("texture part"),
            &mut stored,
        )
        .expect("read");
        assert_eq!(stored, image.encoded());
        let mut model = String::new();
        std::io::Read::read_to_string(
            &mut archive.by_name("3D/3dmodel.model").expect("model"),
            &mut model,
        )
        .expect("read");
        assert!(model.contains("<m:texture2d "));
        assert_eq!(model.matches("<m:tex2coord ").count(), 12 * 3);
        let mut rels = String::new();
        std::io::Read::read_to_string(
            &mut archive
                .by_name("3D/_rels/3dmodel.model.rels")
                .expect("rels"),
            &mut rels,
        )
        .expect("read");
        assert!(rels.contains("3dtexture"));
        let imported = crate::load_3mf_bytes(&bytes).expect("import");
        assert_eq!(imported.items[0].mesh.face_count(), 12);
    }
}
