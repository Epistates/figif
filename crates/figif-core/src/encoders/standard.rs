//! Standard lossless GIF encoder using the gif crate.
//!
//! Features delta encoding for smaller file sizes by only encoding
//! the changed pixels between consecutive frames.

use crate::error::{FigifError, Result};
use crate::traits::GifEncoder;
use crate::types::{EncodableFrame, EncodeConfig};
use gif::{DisposalMethod, Encoder, Frame, Repeat};
use image::RgbaImage;
use image::imageops::FilterType;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Standard lossless GIF encoder.
///
/// This encoder produces standard GIF files using the `gif` crate.
/// It supports resizing and basic optimization but does not perform
/// lossy compression.
///
/// # Example
///
/// ```ignore
/// use figif_core::encoders::StandardEncoder;
/// use figif_core::traits::GifEncoder;
///
/// let encoder = StandardEncoder::new();
/// let bytes = encoder.encode(&frames, &EncodeConfig::default())?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct StandardEncoder {
    /// Resize filter to use when resizing frames.
    resize_filter: ResizeFilter,
}

/// Filter type for resizing operations.
#[derive(Debug, Clone, Copy, Default)]
pub enum ResizeFilter {
    /// Nearest neighbor - fast, pixelated
    Nearest,
    /// Triangle (bilinear) - good balance
    #[default]
    Triangle,
    /// Catmull-Rom - smooth, good for downscaling
    CatmullRom,
    /// Lanczos3 - highest quality, slowest
    Lanczos3,
}

impl From<ResizeFilter> for FilterType {
    fn from(filter: ResizeFilter) -> Self {
        match filter {
            ResizeFilter::Nearest => FilterType::Nearest,
            ResizeFilter::Triangle => FilterType::Triangle,
            ResizeFilter::CatmullRom => FilterType::CatmullRom,
            ResizeFilter::Lanczos3 => FilterType::Lanczos3,
        }
    }
}

impl StandardEncoder {
    /// Create a new standard encoder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the resize filter.
    pub fn with_resize_filter(mut self, filter: ResizeFilter) -> Self {
        self.resize_filter = filter;
        self
    }

    /// Encode frames to a writer.
    fn encode_to<W: Write>(
        &self,
        frames: &[EncodableFrame],
        mut writer: W,
        config: &EncodeConfig,
    ) -> Result<()> {
        if frames.is_empty() {
            return Err(FigifError::NoFrames);
        }

        // Determine output dimensions
        let first_frame = &frames[0];
        let (src_width, src_height) = first_frame.image.dimensions();

        let (out_width, out_height) = match (config.width, config.height) {
            (Some(w), Some(h)) => (w as u32, h as u32),
            (Some(w), None) => {
                let ratio = w as f64 / src_width as f64;
                (w as u32, (src_height as f64 * ratio).round() as u32)
            }
            (None, Some(h)) => {
                let ratio = h as f64 / src_height as f64;
                ((src_width as f64 * ratio).round() as u32, h as u32)
            }
            (None, None) => (src_width, src_height),
        };

        // Create encoder
        let mut encoder = Encoder::new(&mut writer, out_width as u16, out_height as u16, &[])
            .map_err(|e| FigifError::EncodeError {
                reason: e.to_string(),
            })?;

        // Set repeat/loop behavior
        let repeat: Repeat = config.loop_count.into();
        encoder
            .set_repeat(repeat)
            .map_err(|e| FigifError::EncodeError {
                reason: e.to_string(),
            })?;

        // Encode each frame with delta optimization
        let needs_resize = out_width != src_width || out_height != src_height;
        let mut prev_image: Option<RgbaImage> = None;

        for (idx, encodable) in frames.iter().enumerate() {
            let image = if needs_resize {
                image::imageops::resize(
                    &encodable.image,
                    out_width,
                    out_height,
                    self.resize_filter.into(),
                )
            } else {
                encodable.image.clone()
            };

            // First frame or frames with no previous: encode full frame
            let frame = if let Some(prev) = prev_image.as_ref().filter(|_| idx != 0) {
                // Compute delta from previous frame
                match compute_delta_frame(&image, prev, encodable.delay_centiseconds) {
                    Some(delta_frame) => delta_frame,
                    None => {
                        // Frames are identical - still need to emit a frame for timing
                        // Use a 1x1 transparent frame at 0,0
                        Frame {
                            width: 1,
                            height: 1,
                            left: 0,
                            top: 0,
                            delay: encodable.delay_centiseconds,
                            dispose: DisposalMethod::Keep,
                            transparent: Some(0),
                            palette: Some(vec![0, 0, 0]), // Single transparent color
                            buffer: std::borrow::Cow::Owned(vec![0]),
                            ..Default::default()
                        }
                    }
                }
            } else {
                let mut f = rgba_to_gif_frame(&image, encodable.delay_centiseconds)?;
                f.dispose = DisposalMethod::Keep;
                f
            };

            encoder
                .write_frame(&frame)
                .map_err(|e| FigifError::EncodeError {
                    reason: e.to_string(),
                })?;

            prev_image = Some(image);
        }

        Ok(())
    }
}

impl GifEncoder for StandardEncoder {
    fn encode(&self, frames: &[EncodableFrame], config: &EncodeConfig) -> Result<Vec<u8>> {
        let mut buffer = Vec::new();
        self.encode_to(frames, &mut buffer, config)?;
        Ok(buffer)
    }

    fn encode_to_file(
        &self,
        frames: &[EncodableFrame],
        path: impl AsRef<Path>,
        config: &EncodeConfig,
    ) -> Result<()> {
        let path = path.as_ref();
        let file = File::create(path).map_err(|e| FigifError::FileWrite {
            path: path.to_path_buf(),
            source: e,
        })?;
        let writer = BufWriter::new(file);
        self.encode_to(frames, writer, config)
    }

    fn encode_to_writer<W: Write>(
        &self,
        frames: &[EncodableFrame],
        writer: W,
        config: &EncodeConfig,
    ) -> Result<()> {
        self.encode_to(frames, writer, config)
    }

    fn supports_lossy(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "standard"
    }
}

/// Compute a delta frame that only contains changed pixels from the previous frame.
/// Returns None if frames are identical.
fn compute_delta_frame(
    current: &RgbaImage,
    prev: &RgbaImage,
    delay: u16,
) -> Option<Frame<'static>> {
    let (width, height) = current.dimensions();

    // Find bounding box of changed pixels
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0u32;
    let mut max_y = 0u32;

    for y in 0..height {
        for x in 0..width {
            let curr_pixel = current.get_pixel(x, y);
            let prev_pixel = prev.get_pixel(x, y);
            if curr_pixel != prev_pixel {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    // No changes - frames are identical
    if max_x < min_x || max_y < min_y {
        return None;
    }

    // Extract the changed region
    let delta_width = max_x - min_x + 1;
    let delta_height = max_y - min_y + 1;

    // Build palette and indices for just the changed region
    // Use transparent for unchanged pixels within the bounding box
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut indices: Vec<u8> = Vec::with_capacity((delta_width * delta_height) as usize);
    let mut color_map: std::collections::HashMap<[u8; 3], u8> = std::collections::HashMap::new();

    // Reserve index 0 for transparent (unchanged pixels)
    let transparent_index: u8 = 0;
    palette.push([0, 0, 0]); // Transparent placeholder

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let curr_pixel = current.get_pixel(x, y);
            let prev_pixel = prev.get_pixel(x, y);

            if curr_pixel == prev_pixel {
                // Unchanged pixel - use transparent
                indices.push(transparent_index);
            } else {
                let [r, g, b, a] = curr_pixel.0;

                if a < 128 {
                    // Transparent in current frame
                    indices.push(transparent_index);
                } else {
                    let color = [r, g, b];
                    let index = if let Some(&idx) = color_map.get(&color) {
                        idx
                    } else if palette.len() < 256 {
                        let idx = palette.len() as u8;
                        color_map.insert(color, idx);
                        palette.push(color);
                        idx
                    } else {
                        // Palette full, find closest
                        find_closest_color(&palette, color)
                    };
                    indices.push(index);
                }
            }
        }
    }

    // Ensure palette has at least 2 colors
    while palette.len() < 2 {
        palette.push([0, 0, 0]);
    }

    // Pad palette to power of 2
    let palette_size = palette.len().next_power_of_two().max(2);
    while palette.len() < palette_size {
        palette.push([0, 0, 0]);
    }

    // Flatten palette
    let flat_palette: Vec<u8> = palette.iter().flat_map(|c| c.iter().copied()).collect();

    // Create the delta frame
    let mut frame = Frame::from_palette_pixels(
        delta_width as u16,
        delta_height as u16,
        indices,
        flat_palette,
        Some(transparent_index),
    );

    frame.left = min_x as u16;
    frame.top = min_y as u16;
    frame.delay = delay;
    frame.dispose = DisposalMethod::Keep;

    Some(frame)
}

/// Convert an RGBA image to a GIF frame with color quantization.
fn rgba_to_gif_frame(image: &RgbaImage, delay: u16) -> Result<Frame<'static>> {
    let (width, height) = image.dimensions();

    // Simple color quantization using NeuQuant
    // The gif crate will handle this internally, but we need to prepare the data

    // For now, use a simpler approach: convert to indexed color
    // by building a palette from the unique colors

    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut indices: Vec<u8> = Vec::with_capacity((width * height) as usize);
    let mut color_map: std::collections::HashMap<[u8; 3], u8> = std::collections::HashMap::new();
    let mut transparent_index: Option<u8> = None;

    for pixel in image.pixels() {
        let [r, g, b, a] = pixel.0;

        if a < 128 {
            // Transparent pixel
            if transparent_index.is_none() && palette.len() < 256 {
                transparent_index = Some(palette.len() as u8);
                palette.push([0, 0, 0]); // Placeholder for transparent
            }
            indices.push(transparent_index.unwrap_or(0));
        } else {
            let color = [r, g, b];
            let index = if let Some(&idx) = color_map.get(&color) {
                idx
            } else if palette.len() < 256 {
                let idx = palette.len() as u8;
                color_map.insert(color, idx);
                palette.push(color);
                idx
            } else {
                // Palette is full, find closest color
                find_closest_color(&palette, color)
            };
            indices.push(index);
        }
    }

    // Ensure palette has at least 2 colors (GIF requirement)
    while palette.len() < 2 {
        palette.push([0, 0, 0]);
    }

    // Pad palette to power of 2
    let palette_size = palette.len().next_power_of_two().max(2);
    while palette.len() < palette_size {
        palette.push([0, 0, 0]);
    }

    // Flatten palette
    let flat_palette: Vec<u8> = palette.iter().flat_map(|c| c.iter().copied()).collect();

    // Create frame
    let mut frame = Frame::from_palette_pixels(
        width as u16,
        height as u16,
        indices,
        flat_palette,
        transparent_index,
    );

    frame.delay = delay;

    Ok(frame)
}

/// Find the closest color in the palette using simple Euclidean distance.
fn find_closest_color(palette: &[[u8; 3]], target: [u8; 3]) -> u8 {
    let mut best_idx = 0u8;
    let mut best_dist = u32::MAX;

    for (idx, color) in palette.iter().enumerate() {
        let dr = (color[0] as i32 - target[0] as i32).pow(2) as u32;
        let dg = (color[1] as i32 - target[1] as i32).pow(2) as u32;
        let db = (color[2] as i32 - target[2] as i32).pow(2) as u32;
        let dist = dr + dg + db;

        if dist < best_dist {
            best_dist = dist;
            best_idx = idx as u8;
        }
    }

    best_idx
}
