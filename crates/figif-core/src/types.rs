//! Core data types for figif-core.

use image::RgbaImage;
use std::collections::HashMap;
use std::ops::Range;

/// Loop behavior for animated GIFs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopCount {
    /// Loop forever (most common for GIFs).
    #[default]
    Infinite,
    /// Play exactly once (no loop).
    Once,
    /// Loop a specific number of times.
    Finite(u16),
}

impl From<gif::Repeat> for LoopCount {
    fn from(repeat: gif::Repeat) -> Self {
        match repeat {
            gif::Repeat::Infinite => LoopCount::Infinite,
            gif::Repeat::Finite(0) => LoopCount::Once,
            gif::Repeat::Finite(n) => LoopCount::Finite(n),
        }
    }
}

impl From<LoopCount> for gif::Repeat {
    fn from(lc: LoopCount) -> Self {
        match lc {
            LoopCount::Infinite => gif::Repeat::Infinite,
            LoopCount::Once => gif::Repeat::Finite(0),
            LoopCount::Finite(n) => gif::Repeat::Finite(n),
        }
    }
}

/// How to dispose of a frame before drawing the next one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisposalMethod {
    /// No disposal specified (keep the frame).
    #[default]
    Keep,
    /// Do not dispose, leave the canvas as-is.
    None,
    /// Restore to background color.
    Background,
    /// Restore to previous frame.
    Previous,
}

impl From<gif::DisposalMethod> for DisposalMethod {
    fn from(dm: gif::DisposalMethod) -> Self {
        match dm {
            gif::DisposalMethod::Keep => DisposalMethod::Keep,
            gif::DisposalMethod::Any => DisposalMethod::None,
            gif::DisposalMethod::Background => DisposalMethod::Background,
            gif::DisposalMethod::Previous => DisposalMethod::Previous,
        }
    }
}

impl From<DisposalMethod> for gif::DisposalMethod {
    fn from(dm: DisposalMethod) -> Self {
        match dm {
            DisposalMethod::Keep => gif::DisposalMethod::Keep,
            DisposalMethod::None => gif::DisposalMethod::Any,
            DisposalMethod::Background => gif::DisposalMethod::Background,
            DisposalMethod::Previous => gif::DisposalMethod::Previous,
        }
    }
}

/// Metadata about a GIF without frame pixel data.
#[derive(Debug, Clone)]
pub struct GifMetadata {
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
    /// Total number of frames.
    pub frame_count: usize,
    /// Total duration in milliseconds.
    pub total_duration_ms: u64,
    /// Whether the GIF uses transparency.
    pub has_transparency: bool,
    /// Loop behavior.
    pub loop_count: LoopCount,
    /// Global color table if present.
    pub global_palette: Option<Vec<u8>>,
}

/// A decoded frame ready for analysis.
#[derive(Debug, Clone)]
pub struct DecodedFrame {
    /// Zero-based index in the original GIF.
    pub index: usize,
    /// The fully composited RGBA image for this frame.
    pub image: RgbaImage,
    /// Frame delay in centiseconds (1/100th of a second).
    pub delay_centiseconds: u16,
    /// How to dispose of this frame.
    pub disposal: DisposalMethod,
    /// Frame position offset (x).
    pub left: u16,
    /// Frame position offset (y).
    pub top: u16,
}

impl DecodedFrame {
    /// Get the delay in milliseconds.
    #[inline]
    pub fn delay_ms(&self) -> u32 {
        self.delay_centiseconds as u32 * 10
    }
}

/// A frame with its computed perceptual hash.
#[derive(Debug, Clone)]
pub struct AnalyzedFrame<H> {
    /// The decoded frame data.
    pub frame: DecodedFrame,
    /// The perceptual hash of this frame.
    pub hash: H,
    /// The segment this frame belongs to, if assigned.
    pub segment_id: Option<usize>,
    /// Hash distance to the previous frame (0 = identical, higher = more different).
    /// None for the first frame.
    pub distance_to_prev: Option<u32>,
}

impl<H> AnalyzedFrame<H> {
    /// Create a new analyzed frame.
    pub fn new(frame: DecodedFrame, hash: H) -> Self {
        Self {
            frame,
            hash,
            segment_id: None,
            distance_to_prev: None,
        }
    }

    /// Create a new analyzed frame with distance to previous frame.
    pub fn with_distance(frame: DecodedFrame, hash: H, distance_to_prev: Option<u32>) -> Self {
        Self {
            frame,
            hash,
            segment_id: None,
            distance_to_prev,
        }
    }

    /// Get the frame index.
    #[inline]
    pub fn index(&self) -> usize {
        self.frame.index
    }

    /// Get the delay in centiseconds.
    #[inline]
    pub fn delay_cs(&self) -> u16 {
        self.frame.delay_centiseconds
    }
}

/// A group of similar or consecutive frames.
#[derive(Debug, Clone)]
pub struct Segment {
    /// Unique segment identifier.
    pub id: usize,
    /// Range of frame indices in this segment (start..end).
    pub frame_range: Range<usize>,
    /// Total duration of this segment in centiseconds.
    pub total_duration_cs: u16,
    /// Average similarity score within the segment (0.0 = identical).
    pub avg_distance: f64,
    /// Whether all frames in this segment are nearly identical (static).
    pub is_static: bool,
}

impl Segment {
    /// Number of frames in this segment.
    #[inline]
    pub fn frame_count(&self) -> usize {
        self.frame_range.len()
    }

    /// Total duration in milliseconds.
    #[inline]
    pub fn duration_ms(&self) -> u32 {
        self.total_duration_cs as u32 * 10
    }
}

/// Operations that can be applied to segments.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SegmentOp {
    /// Keep the segment as-is.
    #[default]
    Keep,
    /// Remove the segment entirely.
    Remove,
    /// Collapse to a single frame with the specified delay.
    Collapse {
        /// Delay for the collapsed frame in centiseconds.
        delay_cs: u16,
    },
    /// Set the total duration for the segment, distributing evenly.
    SetDuration {
        /// Total duration in centiseconds.
        total_cs: u16,
    },
    /// Scale the timing by a factor (0.5 = 2x faster, 2.0 = 2x slower).
    Scale {
        /// Scaling factor for delays.
        factor: f64,
    },
    /// Set a fixed delay for each frame in the segment.
    SetFrameDelay {
        /// Per-frame delay in centiseconds.
        delay_cs: u16,
    },
}

/// A frame prepared for encoding.
#[derive(Debug, Clone)]
pub struct EncodableFrame {
    /// The RGBA image data.
    pub image: RgbaImage,
    /// Frame delay in centiseconds.
    pub delay_centiseconds: u16,
}

impl EncodableFrame {
    /// Create a new encodable frame.
    pub fn new(image: RgbaImage, delay_centiseconds: u16) -> Self {
        Self {
            image,
            delay_centiseconds,
        }
    }

    /// Create from a decoded frame.
    pub fn from_decoded(frame: &DecodedFrame) -> Self {
        Self {
            image: frame.image.clone(),
            delay_centiseconds: frame.delay_centiseconds,
        }
    }
}

/// Configuration for encoding output GIFs.
#[derive(Debug, Clone)]
pub struct EncodeConfig {
    /// Target width (None = preserve original).
    pub width: Option<u16>,
    /// Target height (None = preserve original).
    pub height: Option<u16>,
    /// Loop behavior.
    pub loop_count: LoopCount,
    /// Lossy quality 1-100 (None = lossless).
    pub lossy_quality: Option<u8>,
}

impl Default for EncodeConfig {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            loop_count: LoopCount::Infinite,
            lossy_quality: None,
        }
    }
}

impl EncodeConfig {
    /// Create a new encode config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the target width.
    pub fn with_width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    /// Set the target height.
    pub fn with_height(mut self, height: u16) -> Self {
        self.height = Some(height);
        self
    }

    /// Set both width and height.
    pub fn with_size(mut self, width: u16, height: u16) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Set the loop count.
    pub fn with_loop_count(mut self, loop_count: LoopCount) -> Self {
        self.loop_count = loop_count;
        self
    }

    /// Set lossy quality (1-100, higher = better quality).
    pub fn with_lossy_quality(mut self, quality: u8) -> Self {
        self.lossy_quality = Some(quality.clamp(1, 100));
        self
    }

    /// Use lossless encoding.
    pub fn lossless(mut self) -> Self {
        self.lossy_quality = None;
        self
    }
}

/// A map of segment operations keyed by segment ID.
pub type SegmentOps = HashMap<usize, SegmentOp>;

/// Operations that can be applied to individual frames within a segment.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FrameOp {
    /// Keep the frame as-is.
    #[default]
    Keep,
    /// Remove this frame from the output.
    Remove,
    /// Create a segment boundary after this frame.
    /// This splits the segment at this point.
    SplitAfter,
}

/// A map of frame operations keyed by (segment_id, frame_index_within_segment).
pub type FrameOps = HashMap<(usize, usize), FrameOp>;
