//! Buffered GIF decoder that loads all frames into memory.

use crate::error::{FigifError, Result};
use crate::traits::{BufferedGifDecoder, GifDecoder};
use crate::types::{DecodedFrame, DisposalMethod, GifMetadata, LoopCount};
use gif::DecodeOptions;
use image::{Rgba, RgbaImage};
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

/// A buffered GIF decoder that loads and composites all frames.
///
/// This decoder properly handles GIF disposal methods and transparency,
/// producing fully composited RGBA frames suitable for analysis.
///
/// # Example
///
/// ```ignore
/// use figif_core::decoders::BufferedDecoder;
/// use figif_core::traits::GifDecoder;
///
/// let decoder = BufferedDecoder::new();
/// let frames: Vec<_> = decoder.decode_file("animation.gif")?.collect::<Result<Vec<_>, _>>()?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct BufferedDecoder {
    /// Memory limit for decoding (0 = no limit).
    memory_limit: usize,
}

impl BufferedDecoder {
    /// Create a new buffered decoder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a memory limit for decoding.
    ///
    /// If the GIF would require more memory than this limit,
    /// decoding will fail with an error.
    pub fn with_memory_limit(mut self, limit: usize) -> Self {
        self.memory_limit = limit;
        self
    }

    /// Decode all frames from a reader into a vector.
    fn decode_all_frames<R: Read>(&self, reader: R) -> Result<Vec<DecodedFrame>> {
        let mut options = DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);

        let mut decoder = options.read_info(reader)?;

        let width = decoder.width() as u32;
        let height = decoder.height() as u32;

        // Check memory limit
        if self.memory_limit > 0 {
            let frame_size = (width * height * 4) as usize;
            // Rough estimate: we need at least 2 frames worth of memory
            if frame_size * 2 > self.memory_limit {
                return Err(FigifError::InvalidConfig {
                    message: format!("GIF dimensions {}x{} exceed memory limit", width, height),
                });
            }
        }

        // Canvas for compositing frames
        let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
        let mut frames = Vec::new();
        let mut index = 0;

        // Previous frame for "Previous" disposal method
        let mut previous_canvas: Option<RgbaImage> = None;

        while let Some(frame) = decoder.read_next_frame()? {
            let delay = frame.delay;
            let disposal = DisposalMethod::from(frame.dispose);
            let left = frame.left;
            let top = frame.top;
            let frame_width = frame.width as u32;
            let frame_height = frame.height as u32;

            // Save canvas before applying this frame (for Previous disposal)
            if matches!(disposal, DisposalMethod::Previous) {
                previous_canvas = Some(canvas.clone());
            }

            // Composite frame onto canvas
            let frame_buffer = &frame.buffer;
            for y in 0..frame_height {
                for x in 0..frame_width {
                    let src_idx = ((y * frame_width + x) * 4) as usize;
                    if src_idx + 3 < frame_buffer.len() {
                        let pixel = Rgba([
                            frame_buffer[src_idx],
                            frame_buffer[src_idx + 1],
                            frame_buffer[src_idx + 2],
                            frame_buffer[src_idx + 3],
                        ]);

                        let canvas_x = left as u32 + x;
                        let canvas_y = top as u32 + y;

                        if canvas_x < width && canvas_y < height {
                            // Only draw non-transparent pixels
                            if pixel[3] > 0 {
                                canvas.put_pixel(canvas_x, canvas_y, pixel);
                            }
                        }
                    }
                }
            }

            // Store the composited frame
            frames.push(DecodedFrame {
                index,
                image: canvas.clone(),
                delay_centiseconds: delay,
                disposal,
                left,
                top,
            });

            // Apply disposal method for next frame
            match disposal {
                DisposalMethod::Background => {
                    // Clear the frame area to background/transparent
                    for y in 0..frame_height {
                        for x in 0..frame_width {
                            let canvas_x = left as u32 + x;
                            let canvas_y = top as u32 + y;
                            if canvas_x < width && canvas_y < height {
                                canvas.put_pixel(canvas_x, canvas_y, Rgba([0, 0, 0, 0]));
                            }
                        }
                    }
                }
                DisposalMethod::Previous => {
                    // Restore previous canvas
                    if let Some(ref prev) = previous_canvas {
                        canvas = prev.clone();
                    }
                }
                DisposalMethod::Keep | DisposalMethod::None => {
                    // Keep canvas as-is
                }
            }

            index += 1;
        }

        if frames.is_empty() {
            return Err(FigifError::NoFrames);
        }

        Ok(frames)
    }

    /// Extract metadata from a decoder.
    fn extract_metadata<R: Read>(&self, reader: R) -> Result<GifMetadata> {
        let mut options = DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);

        let mut decoder = options.read_info(reader)?;

        let width = decoder.width();
        let height = decoder.height();
        let global_palette = decoder.global_palette().map(|p| p.to_vec());
        let has_transparency = global_palette
            .as_ref()
            .is_some_and(|_| decoder.bg_color().is_some());

        // Count frames and total duration
        let mut frame_count = 0;
        let mut total_duration_cs: u64 = 0;

        while let Some(frame) = decoder.read_next_frame()? {
            frame_count += 1;
            total_duration_cs += frame.delay as u64;
        }

        // Get repeat info (need to check extensions)
        let loop_count = LoopCount::Infinite; // Default, would need extension parsing

        Ok(GifMetadata {
            width,
            height,
            frame_count,
            total_duration_ms: total_duration_cs * 10,
            has_transparency,
            loop_count,
            global_palette,
        })
    }
}

/// Iterator adapter for buffered frames.
pub struct BufferedFrameIter {
    frames: std::vec::IntoIter<DecodedFrame>,
}

impl Iterator for BufferedFrameIter {
    type Item = Result<DecodedFrame>;

    fn next(&mut self) -> Option<Self::Item> {
        self.frames.next().map(Ok)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.frames.size_hint()
    }
}

impl ExactSizeIterator for BufferedFrameIter {}

impl GifDecoder for BufferedDecoder {
    type FrameIter = BufferedFrameIter;

    fn decode_file(&self, path: impl AsRef<Path>) -> Result<Self::FrameIter> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| FigifError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;
        let reader = BufReader::new(file);
        let frames = self.decode_all_frames(reader)?;
        Ok(BufferedFrameIter {
            frames: frames.into_iter(),
        })
    }

    fn decode_bytes(&self, data: &[u8]) -> Result<Self::FrameIter> {
        if data.is_empty() {
            return Err(FigifError::EmptyData);
        }
        let reader = Cursor::new(data);
        let frames = self.decode_all_frames(reader)?;
        Ok(BufferedFrameIter {
            frames: frames.into_iter(),
        })
    }

    fn decode_reader<R: Read + Send>(&self, reader: R) -> Result<Self::FrameIter> {
        let frames = self.decode_all_frames(reader)?;
        Ok(BufferedFrameIter {
            frames: frames.into_iter(),
        })
    }

    fn metadata_from_bytes(&self, data: &[u8]) -> Result<GifMetadata> {
        if data.is_empty() {
            return Err(FigifError::EmptyData);
        }
        let reader = Cursor::new(data);
        self.extract_metadata(reader)
    }

    fn metadata_from_file(&self, path: impl AsRef<Path>) -> Result<GifMetadata> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| FigifError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;
        let reader = BufReader::new(file);
        self.extract_metadata(reader)
    }

    fn name(&self) -> &'static str {
        "buffered"
    }
}

impl BufferedGifDecoder for BufferedDecoder {}
