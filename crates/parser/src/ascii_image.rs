//! Image-to-ASCII conversion for terminal display.

use anyhow::{Context, Result};
use image::GenericImageView;
use std::io::Cursor;

/// ASCII characters for grayscale levels (light to dark)
const GRAYSCALE_CHARS: &[char] = &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

/// Convert an image to ASCII art string.
///
/// The output width is specified in characters; height is calculated
/// based on the aspect ratio (2 chars per pixel height for terminal).
pub fn image_to_ascii(image_bytes: &[u8], width: usize) -> Result<String> {
    let img = image::load_from_memory(image_bytes)
        .context("Failed to decode image")?;

    let (orig_width, orig_height) = img.dimensions();
    
    // Calculate target dimensions maintaining aspect ratio
    // Terminal characters are roughly 2x taller than wide, so we use half the height
    let target_height = (width as f64 * orig_height as f64 / orig_width as f64 * 0.5) as usize;
    if target_height == 0 {
        return Ok(String::new());
    }

    // Resize image to target dimensions
    let resized = img.resize(width as u32, target_height as u32, image::imageops::FilterType::Triangle);

    let mut output = String::new();

    for y in 0..target_height {
        for x in 0..width {
            let pixel = resized.get_pixel(x as u32, y as u32);
            let gray = luminance(pixel);
            
            // Map luminance to ASCII character (0-255 -> index 0-9)
            let idx = ((255 - gray) as usize * GRAYSCALE_CHARS.len()) / 256;
            let idx = idx.min(GRAYSCALE_CHARS.len() - 1);
            
            output.push(GRAYSCALE_CHARS[idx]);
        }
        output.push('\n');
    }

    Ok(output)
}

/// Calculate relative luminance from an RGBA pixel.
fn luminance(pixel: image::Rgba<u8>) -> u8 {
    let r = pixel[0] as f64;
    let g = pixel[1] as f64;
    let b = pixel[2] as f64;
    
    // Standard luminance formula
    let lum = 0.299 * r + 0.587 * g + 0.114 * b;
    lum as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luminance_white() {
        let pixel = image::Rgba([255, 255, 255, 255]);
        assert_eq!(luminance(pixel), 255);
    }

    #[test]
    fn test_luminance_black() {
        let pixel = image::Rgba([0, 0, 0, 255]);
        assert_eq!(luminance(pixel), 0);
    }
}