//! Streaming GIF decoder that processes frames lazily.

use crate::error::{FigifError, Result};
use crate::traits::GifDecoder;
use crate::types::{DecodedFrame, DisposalMethod, GifMetadata, LoopCount};
use gif::{DecodeOptions, Decoder};
use image::{Rgba, RgbaImage};
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::Path;

/// A streaming GIF decoder that processes frames on-demand.
///
/// This decoder is more memory-efficient for large GIFs as it doesn't
/// need to hold all frames in memory simultaneously. However, it requires
/// maintaining decoder state and is less suitable for random access.
///
/// # Example
///
/// ```ignore
/// use figif_core::decoders::StreamingDecoder;
/// use figif_core::traits::GifDecoder;
///
/// let decoder = StreamingDecoder::new();
/// for frame in decoder.decode_file("large_animation.gif")? {
///     let frame = frame?;
///     // Process frame immediately
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct StreamingDecoder;

impl StreamingDecoder {
    /// Create a new streaming decoder.
    pub fn new() -> Self {
        Self
    }
}

/// Iterator that yields frames as they are decoded.
pub struct StreamingFrameIter<R: Read> {
    decoder: Decoder<R>,
    canvas: RgbaImage,
    previous_canvas: Option<RgbaImage>,
    index: usize,
    width: u32,
    height: u32,
}

impl<R: Read> StreamingFrameIter<R> {
    fn new(decoder: Decoder<R>) -> Self {
        let width = decoder.width() as u32;
        let height = decoder.height() as u32;
        let canvas = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));

        Self {
            decoder,
            canvas,
            previous_canvas: None,
            index: 0,
            width,
            height,
        }
    }
}

impl<R: Read> Iterator for StreamingFrameIter<R> {
    type Item = Result<DecodedFrame>;

    fn next(&mut self) -> Option<Self::Item> {
        let frame = match self.decoder.read_next_frame() {
            Ok(Some(frame)) => frame,
            Ok(None) => return None,
            Err(e) => return Some(Err(e.into())),
        };

        let delay = frame.delay;
        let disposal = DisposalMethod::from(frame.dispose);
        let left = frame.left;
        let top = frame.top;
        let frame_width = frame.width as u32;
        let frame_height = frame.height as u32;

        // Save canvas before applying this frame (for Previous disposal)
        if matches!(disposal, DisposalMethod::Previous) {
            self.previous_canvas = Some(self.canvas.clone());
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

                    if canvas_x < self.width && canvas_y < self.height && pixel[3] > 0 {
                        self.canvas.put_pixel(canvas_x, canvas_y, pixel);
                    }
                }
            }
        }

        // Create the decoded frame
        let decoded = DecodedFrame {
            index: self.index,
            image: self.canvas.clone(),
            delay_centiseconds: delay,
            disposal,
            left,
            top,
        };

        // Apply disposal method for next frame
        match disposal {
            DisposalMethod::Background => {
                for y in 0..frame_height {
                    for x in 0..frame_width {
                        let canvas_x = left as u32 + x;
                        let canvas_y = top as u32 + y;
                        if canvas_x < self.width && canvas_y < self.height {
                            self.canvas
                                .put_pixel(canvas_x, canvas_y, Rgba([0, 0, 0, 0]));
                        }
                    }
                }
            }
            DisposalMethod::Previous => {
                if let Some(ref prev) = self.previous_canvas {
                    self.canvas = prev.clone();
                }
            }
            DisposalMethod::Keep | DisposalMethod::None => {}
        }

        self.index += 1;
        Some(Ok(decoded))
    }
}

/// Wrapper to handle different reader types with a common iterator type.
pub enum StreamingIterWrapper {
    /// Iterator reading from a file.
    File(StreamingFrameIter<BufReader<File>>),
    /// Iterator reading from in-memory bytes.
    Bytes(StreamingFrameIter<Cursor<Vec<u8>>>),
}

impl Iterator for StreamingIterWrapper {
    type Item = Result<DecodedFrame>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            StreamingIterWrapper::File(iter) => iter.next(),
            StreamingIterWrapper::Bytes(iter) => iter.next(),
        }
    }
}

impl GifDecoder for StreamingDecoder {
    type FrameIter = StreamingIterWrapper;

    fn decode_file(&self, path: impl AsRef<Path>) -> Result<Self::FrameIter> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| FigifError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;
        let reader = BufReader::new(file);

        let mut options = DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let decoder = options.read_info(reader)?;

        Ok(StreamingIterWrapper::File(StreamingFrameIter::new(decoder)))
    }

    fn decode_bytes(&self, data: &[u8]) -> Result<Self::FrameIter> {
        if data.is_empty() {
            return Err(FigifError::EmptyData);
        }

        let reader = Cursor::new(data.to_vec());

        let mut options = DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let decoder = options.read_info(reader)?;

        Ok(StreamingIterWrapper::Bytes(StreamingFrameIter::new(
            decoder,
        )))
    }

    fn decode_reader<R: Read + Send>(&self, reader: R) -> Result<Self::FrameIter> {
        // For generic readers, we need to buffer into memory
        let mut buffer = Vec::new();
        let mut reader = reader;
        reader
            .read_to_end(&mut buffer)
            .map_err(|e| FigifError::DecodeError {
                reason: format!("failed to read GIF data: {}", e),
            })?;

        self.decode_bytes(&buffer)
    }

    fn metadata_from_bytes(&self, data: &[u8]) -> Result<GifMetadata> {
        if data.is_empty() {
            return Err(FigifError::EmptyData);
        }

        let reader = Cursor::new(data);
        let mut options = DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options.read_info(reader)?;

        let width = decoder.width();
        let height = decoder.height();
        let global_palette = decoder.global_palette().map(|p| p.to_vec());

        let mut frame_count = 0;
        let mut total_duration_cs: u64 = 0;

        while let Some(frame) = decoder.read_next_frame()? {
            frame_count += 1;
            total_duration_cs += frame.delay as u64;
        }

        Ok(GifMetadata {
            width,
            height,
            frame_count,
            total_duration_ms: total_duration_cs * 10,
            has_transparency: global_palette.is_some(),
            loop_count: LoopCount::Infinite,
            global_palette,
        })
    }

    fn metadata_from_file(&self, path: impl AsRef<Path>) -> Result<GifMetadata> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|e| FigifError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;
        self.metadata_from_bytes(&data)
    }

    fn name(&self) -> &'static str {
        "streaming"
    }
}
