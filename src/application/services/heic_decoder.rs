#[cfg(feature = "heic")]
use std::io::Cursor;

#[cfg(feature = "heic")]
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb};
#[cfg(feature = "heic")]
use libheif_rs::{ColorSpace, HeifContext, LibHeif, RgbChroma, SecurityLimits};

use crate::error::ApiError;

#[cfg(feature = "heic")]
const MAX_IMAGE_WIDTH: u32 = 8_000;
#[cfg(feature = "heic")]
const MAX_IMAGE_HEIGHT: u32 = 8_000;
#[cfg(feature = "heic")]
const MAX_IMAGE_PIXELS: u64 = 20_000_000;
#[cfg(feature = "heic")]
const MAX_HEIF_ITEMS: u32 = 8;
#[cfg(feature = "heic")]
const MAX_HEIF_TILES: u64 = 64;
#[cfg(feature = "heic")]
const MAX_COLOR_PROFILE_BYTES: u32 = 512 * 1024;
#[cfg(feature = "heic")]
const MAX_HEIF_MEMORY_BLOCK_BYTES: u64 = 96 * 1024 * 1024;

/// Decodes a HEIC/HEIF image under explicit parser limits and serializes only
/// RGB pixels into a fresh JPEG. No source metadata is copied to the result.
pub fn decode_heic_to_jpeg(bytes: &[u8], max_image_bytes: usize) -> Result<Vec<u8>, ApiError> {
    #[cfg(not(feature = "heic"))]
    {
        let _ = (bytes, max_image_bytes);
        return Err(ApiError::Validation(
            "HEIC processing is not enabled on this server".to_owned(),
        ));
    }
    #[cfg(feature = "heic")]
    {
        let mut context = HeifContext::new().map_err(decode_error)?;
        let mut limits = SecurityLimits::new();
        limits.set_max_image_size_pixels(MAX_IMAGE_PIXELS);
        limits.set_max_number_of_tiles(MAX_HEIF_TILES);
        limits.set_max_items(MAX_HEIF_ITEMS);
        limits.set_max_color_profile_size(MAX_COLOR_PROFILE_BYTES);
        limits.set_max_memory_block_size(MAX_HEIF_MEMORY_BLOCK_BYTES);
        limits.set_max_components(8);
        limits.set_max_iloc_extents_per_item(1_024);
        limits.set_max_size_entity_group(128);
        limits.set_max_children_per_box(256);
        context.set_security_limits(&limits).map_err(decode_error)?;
        context.set_max_decoding_threads(1);
        context.read_bytes(bytes).map_err(decode_error)?;

        let handle = context.primary_image_handle().map_err(decode_error)?;
        let width = handle.width();
        let height = handle.height();
        validate_dimensions(width, height)?;

        let decoded = LibHeif::new()
            .decode(&handle, ColorSpace::Rgb(RgbChroma::Rgb), None)
            .map_err(decode_error)?;
        let plane = decoded
            .planes()
            .interleaved
            .ok_or_else(|| ApiError::Validation("image data could not be decoded".to_owned()))?;
        if plane.width != width
            || plane.height != height
            || plane.bits_per_pixel != 24
            || plane.storage_bits_per_pixel != 24
        {
            return Err(ApiError::Validation(
                "image data could not be decoded".to_owned(),
            ));
        }

        let row_size = usize::try_from(width)
            .ok()
            .and_then(|value| value.checked_mul(3))
            .ok_or_else(|| {
                ApiError::Validation("image dimensions exceed the allowed limit".to_owned())
            })?;
        let output_len = row_size
            .checked_mul(usize::try_from(height).unwrap_or(usize::MAX))
            .ok_or_else(|| {
                ApiError::Validation("image dimensions exceed the allowed limit".to_owned())
            })?;
        if plane.stride < row_size
            || plane
                .stride
                .checked_mul(usize::try_from(height).unwrap_or(usize::MAX))
                .is_none_or(|required| required > plane.data.len())
        {
            return Err(ApiError::Validation(
                "image data could not be decoded".to_owned(),
            ));
        }

        let mut rgb = vec![0; output_len];
        for (destination, source) in rgb.chunks_exact_mut(row_size).zip(
            plane
                .data
                .chunks_exact(plane.stride)
                .take(usize::try_from(height).unwrap_or_default())
                .map(|row| &row[..row_size]),
        ) {
            destination.copy_from_slice(source);
        }
        let image = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(width, height, rgb)
            .ok_or_else(|| ApiError::Validation("image data could not be decoded".to_owned()))?;
        let mut output = Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(image)
            .write_to(&mut output, ImageFormat::Jpeg)
            .map_err(|_| ApiError::Validation("image could not be normalized".to_owned()))?;
        let output = output.into_inner();
        if output.len() > max_image_bytes {
            return Err(ApiError::Validation(format!(
                "normalized image exceeds the {max_image_bytes}-byte limit"
            )));
        }
        Ok(output)
    }
}

#[cfg(feature = "heic")]
fn validate_dimensions(width: u32, height: u32) -> Result<(), ApiError> {
    if width == 0
        || height == 0
        || width > MAX_IMAGE_WIDTH
        || height > MAX_IMAGE_HEIGHT
        || u64::from(width) * u64::from(height) > MAX_IMAGE_PIXELS
    {
        return Err(ApiError::Validation(
            "image dimensions exceed the allowed limit".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(feature = "heic")]
fn decode_error(_: impl std::fmt::Display) -> ApiError {
    ApiError::Validation("image data could not be decoded".to_owned())
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "heic")]
    use super::validate_dimensions;

    #[cfg(feature = "heic")]
    #[test]
    fn dimensions_reject_zero_and_resource_exhaustion_inputs() {
        assert!(validate_dimensions(0, 100).is_err());
        assert!(validate_dimensions(8_001, 10).is_err());
        assert!(validate_dimensions(8_000, 8_000).is_err());
        assert!(validate_dimensions(4_000, 5_000).is_ok());
    }
}
