use anyhow::{Context, Result};
use image::GenericImageView;
use std::path::Path;

/// Extracts metadata from an image file.
/// Returns a descriptive string: "[Image: filename | WxH | Format]"
pub fn describe_image(path: &Path) -> Result<String> {
    // We open the image. image::open attempts to guess the format.
    let img = image::open(path).context("Failed to open image")?;
    let (width, height) = img.dimensions();

    // Get format if possible (by extension or asking image crate if we had the reader)
    // For now, simplify to just dimensions and filename.
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| "unknown.img".into());

    // Detect image format from extension for display.
    let ext = path
        .extension()
        .map(|s| s.to_string_lossy().to_string().to_uppercase())
        .unwrap_or_else(|| "IMG".to_string());

    Ok(format!(
        "[Image: {} | {}x{} | {}]",
        filename, width, height, ext
    ))
}
