//! High-level processing pipeline.

use crate::analysis::{AnalysisConfig, analyze_frames};
use crate::decoders::BufferedDecoder;
use crate::error::Result;
use crate::hashers::DHasher;
use crate::segment::{apply_operations, apply_segment_operations};
use crate::traits::{FrameHasher, GifDecoder, GifEncoder};
use crate::types::{
    AnalyzedFrame, EncodableFrame, EncodeConfig, FrameOps, GifMetadata, Segment, SegmentOp,
    SegmentOps,
};
use std::path::Path;
use std::sync::Arc;

#[cfg(feature = "parallel")]
use crate::analysis::analyze_frames_parallel;

/// Callback for reporting progress during analysis.
/// First argument is current frame count, second is total frame count.
pub type ProgressCallback = Arc<dyn Fn(usize, usize) + Send + Sync>;

/// Main entry point for GIF analysis and manipulation.
///
/// `Figif` provides a builder-style API for configuring the analysis pipeline
/// and processing GIF files.
///
/// # Type Parameter
///
/// `H` is the hasher type used for frame comparison. Defaults to [`DHasher`].
///
/// # Example
///
/// ```ignore
/// use figif_core::{Figif, SegmentOp};
/// use std::collections::HashMap;
///
/// // Create analyzer with default settings
/// let figif = Figif::new();
///
/// // Analyze a GIF
/// let analysis = figif.analyze_file("demo.gif")?;
///
/// // Print segment info
/// for segment in &analysis.segments {
///     println!("Segment {}: {} frames, {}ms",
///         segment.id,
///         segment.frame_count(),
///         segment.duration_ms());
/// }
///
/// // Define operations
/// let mut ops = HashMap::new();
/// ops.insert(1, SegmentOp::Collapse { delay_cs: 50 });
///
/// // Export with operations applied
/// use figif_core::encoders::StandardEncoder;
/// let encoder = StandardEncoder::new();
/// let bytes = analysis.export(&encoder, &ops, &EncodeConfig::default())?;
/// ```
#[derive(Clone)]
pub struct Figif<H: FrameHasher = DHasher> {
    hasher: H,
    config: AnalysisConfig,
    decoder: BufferedDecoder,
    progress_callback: Option<ProgressCallback>,
}

impl<H: FrameHasher + std::fmt::Debug> std::fmt::Debug for Figif<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Figif")
            .field("hasher", &self.hasher)
            .field("config", &self.config)
            .field("decoder", &self.decoder)
            .field(
                "progress_callback",
                &self.progress_callback.as_ref().map(|_| "Some(callback)"),
            )
            .finish()
    }
}

impl Default for Figif<DHasher> {
    fn default() -> Self {
        Self::new()
    }
}

impl Figif<DHasher> {
    /// Create a new Figif instance with default settings.
    ///
    /// Uses DHasher for frame comparison with sensible defaults.
    pub fn new() -> Self {
        Self {
            hasher: DHasher::new(),
            config: AnalysisConfig::default(),
            decoder: BufferedDecoder::new(),
            progress_callback: None,
        }
    }
}

impl<H: FrameHasher> Figif<H> {
    /// Replace the hasher with a different implementation.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use figif_core::{Figif, hashers::PHasher};
    ///
    /// let figif = Figif::new().with_hasher(PHasher::new());
    /// ```
    pub fn with_hasher<H2: FrameHasher>(self, hasher: H2) -> Figif<H2> {
        Figif {
            hasher,
            config: self.config,
            decoder: self.decoder,
            progress_callback: self.progress_callback,
        }
    }

    /// Set a progress callback for analysis.
    pub fn with_progress_callback(mut self, callback: ProgressCallback) -> Self {
        self.progress_callback = Some(callback);
        self
    }

    /// Set the similarity threshold for segment detection.
    ///
    /// Lower values are more strict (fewer frames considered similar).
    /// Default is 5 (suitable for most use cases).
    pub fn similarity_threshold(mut self, threshold: u32) -> Self {
        self.config.similarity_threshold = threshold;
        self
    }

    /// Set the minimum number of frames to form a segment.
    ///
    /// Default is 2.
    pub fn min_segment_frames(mut self, min: usize) -> Self {
        self.config.min_segment_frames = min.max(1);
        self
    }

    /// Enable or disable static frame detection.
    ///
    /// When enabled, segments with all identical frames are marked as static.
    /// Default is true.
    pub fn detect_static(mut self, enabled: bool) -> Self {
        self.config.detect_static = enabled;
        self
    }

    /// Set the threshold for identical frame detection.
    ///
    /// Frames with distance <= this threshold are considered identical.
    /// Default is 0 (exact matches only).
    pub fn identical_threshold(mut self, threshold: u32) -> Self {
        self.config.identical_threshold = threshold;
        self
    }

    /// Set a memory limit for the decoder.
    pub fn memory_limit(mut self, limit: usize) -> Self {
        self.decoder = self.decoder.with_memory_limit(limit);
        self
    }

    /// Get the current analysis configuration.
    pub fn config(&self) -> &AnalysisConfig {
        &self.config
    }

    /// Get a reference to the hasher.
    pub fn hasher(&self) -> &H {
        &self.hasher
    }

    /// Analyze a GIF file.
    ///
    /// Loads the GIF, computes frame hashes, and detects segments.
    pub fn analyze_file(&self, path: impl AsRef<Path>) -> Result<Analysis<H::Hash>>
    where
        H::Hash: Send,
    {
        #[cfg(feature = "parallel")]
        {
            self.analyze_file_parallel(path)
        }
        #[cfg(not(feature = "parallel"))]
        {
            let frames: Vec<_> = self
                .decoder
                .decode_file(path)?
                .collect::<Result<Vec<_>>>()?;
            self.analyze_frames(frames)
        }
    }

    /// Analyze a GIF from bytes.
    pub fn analyze_bytes(&self, data: &[u8]) -> Result<Analysis<H::Hash>>
    where
        H::Hash: Send,
    {
        #[cfg(feature = "parallel")]
        {
            self.analyze_bytes_parallel(data)
        }
        #[cfg(not(feature = "parallel"))]
        {
            let frames: Vec<_> = self
                .decoder
                .decode_bytes(data)?
                .collect::<Result<Vec<_>>>()?;
            self.analyze_frames(frames)
        }
    }

    /// Analyze a GIF from pre-decoded frames.
    pub fn analyze_frames(
        &self,
        frames: Vec<crate::types::DecodedFrame>,
    ) -> Result<Analysis<H::Hash>> {
        // Get metadata from frames
        let metadata = if frames.is_empty() {
            GifMetadata {
                width: 0,
                height: 0,
                frame_count: 0,
                total_duration_ms: 0,
                has_transparency: false,
                loop_count: crate::types::LoopCount::Infinite,
                global_palette: None,
            }
        } else {
            let (width, height) = frames[0].image.dimensions();
            let total_duration_ms: u64 = frames.iter().map(|f| f.delay_ms() as u64).sum();
            GifMetadata {
                width: width as u16,
                height: height as u16,
                frame_count: frames.len(),
                total_duration_ms,
                has_transparency: true, // Conservative assumption
                loop_count: crate::types::LoopCount::Infinite,
                global_palette: None,
            }
        };

        // Analyze frames
        let progress = self
            .progress_callback
            .as_ref()
            .map(|c| c.as_ref() as &(dyn Fn(usize, usize) + Send + Sync));
        let (analyzed_frames, segments) =
            analyze_frames(frames, &self.hasher, &self.config, progress);

        Ok(Analysis {
            metadata,
            frames: analyzed_frames,
            segments,
        })
    }

    /// Analyze a GIF file using parallel processing.
    ///
    /// Requires the `parallel` feature.
    #[cfg(feature = "parallel")]
    pub fn analyze_file_parallel(&self, path: impl AsRef<Path>) -> Result<Analysis<H::Hash>>
    where
        H::Hash: Send,
    {
        let frames: Vec<_> = self
            .decoder
            .decode_file(path)?
            .collect::<Result<Vec<_>>>()?;
        self.analyze_frames_parallel(frames)
    }

    /// Analyze a GIF from bytes using parallel processing.
    #[cfg(feature = "parallel")]
    pub fn analyze_bytes_parallel(&self, data: &[u8]) -> Result<Analysis<H::Hash>>
    where
        H::Hash: Send,
    {
        let frames: Vec<_> = self
            .decoder
            .decode_bytes(data)?
            .collect::<Result<Vec<_>>>()?;
        self.analyze_frames_parallel(frames)
    }

    /// Analyze pre-decoded frames using parallel processing.
    #[cfg(feature = "parallel")]
    pub fn analyze_frames_parallel(
        &self,
        frames: Vec<crate::types::DecodedFrame>,
    ) -> Result<Analysis<H::Hash>>
    where
        H::Hash: Send,
    {
        let metadata = if frames.is_empty() {
            GifMetadata {
                width: 0,
                height: 0,
                frame_count: 0,
                total_duration_ms: 0,
                has_transparency: false,
                loop_count: crate::types::LoopCount::Infinite,
                global_palette: None,
            }
        } else {
            let (width, height) = frames[0].image.dimensions();
            let total_duration_ms: u64 = frames.iter().map(|f| f.delay_ms() as u64).sum();
            GifMetadata {
                width: width as u16,
                height: height as u16,
                frame_count: frames.len(),
                total_duration_ms,
                has_transparency: true,
                loop_count: crate::types::LoopCount::Infinite,
                global_palette: None,
            }
        };

        let progress = self
            .progress_callback
            .as_ref()
            .map(|c| c.as_ref() as &(dyn Fn(usize, usize) + Send + Sync));
        let (analyzed_frames, segments) =
            analyze_frames_parallel(frames, &self.hasher, &self.config, progress);

        Ok(Analysis {
            metadata,
            frames: analyzed_frames,
            segments,
        })
    }
}

/// Result of GIF analysis containing frames and segments.
#[derive(Debug, Clone)]
pub struct Analysis<H> {
    /// Metadata about the original GIF.
    pub metadata: GifMetadata,
    /// Analyzed frames with hashes and segment assignments.
    pub frames: Vec<AnalyzedFrame<H>>,
    /// Detected segments.
    pub segments: Vec<Segment>,
}

impl<H: Clone + Sync + Send> Analysis<H> {
    /// Get the number of frames.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get the number of segments.
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    /// Get the total duration in milliseconds.
    pub fn total_duration_ms(&self) -> u64 {
        self.metadata.total_duration_ms
    }

    /// Get segments that are marked as static.
    pub fn static_segments(&self) -> Vec<&Segment> {
        self.segments.iter().filter(|s| s.is_static).collect()
    }

    /// Apply segment operations and get encodable frames.
    ///
    /// Operations not in the map default to `Keep`.
    pub fn apply_operations(&self, ops: &SegmentOps) -> Vec<EncodableFrame> {
        apply_segment_operations(&self.frames, &self.segments, ops)
    }

    /// Apply operations and export using the specified encoder.
    pub fn export<E: GifEncoder>(
        &self,
        encoder: &E,
        ops: &SegmentOps,
        config: &EncodeConfig,
    ) -> Result<Vec<u8>> {
        let frames = self.apply_operations(ops);
        encoder.encode(&frames, config)
    }

    /// Apply operations and export to a file.
    pub fn export_to_file<E: GifEncoder>(
        &self,
        encoder: &E,
        ops: &SegmentOps,
        path: impl AsRef<Path>,
        config: &EncodeConfig,
    ) -> Result<()> {
        let frames = self.apply_operations(ops);
        encoder.encode_to_file(&frames, path, config)
    }

    /// Apply both segment and frame operations and get encodable frames.
    ///
    /// This is the enhanced version that handles frame-level operations like
    /// individual frame removal and segment splitting.
    ///
    /// Operations not in the maps default to `Keep`.
    pub fn apply_all_operations(
        &self,
        segment_ops: &SegmentOps,
        frame_ops: &FrameOps,
    ) -> Vec<EncodableFrame> {
        apply_operations(&self.frames, &self.segments, segment_ops, frame_ops)
    }

    /// Calculate the resulting frame count and duration without cloning images.
    ///
    /// Returns (total_frames, total_duration_ms).
    pub fn calculate_impact(&self, segment_ops: &SegmentOps, frame_ops: &FrameOps) -> (usize, u64) {
        let (frames, cs) = crate::segment::dry_run_all_operations(
            &self.frames,
            &self.segments,
            segment_ops,
            frame_ops,
        );
        (frames, cs * 10)
    }

    /// Apply both segment and frame operations and export using the specified encoder.
    pub fn export_with_frame_ops<E: GifEncoder>(
        &self,
        encoder: &E,
        segment_ops: &SegmentOps,
        frame_ops: &FrameOps,
        config: &EncodeConfig,
    ) -> Result<Vec<u8>> {
        let frames = self.apply_all_operations(segment_ops, frame_ops);
        encoder.encode(&frames, config)
    }

    /// Apply both segment and frame operations and export to a file.
    pub fn export_to_file_with_frame_ops<E: GifEncoder>(
        &self,
        encoder: &E,
        segment_ops: &SegmentOps,
        frame_ops: &FrameOps,
        path: impl AsRef<Path>,
        config: &EncodeConfig,
    ) -> Result<()> {
        let frames = self.apply_all_operations(segment_ops, frame_ops);
        encoder.encode_to_file(&frames, path, config)
    }

    /// Logically split segments at frames marked with `FrameOp::SplitAfter`.
    ///
    /// This returns a new `Analysis` where segments have been partitioned
    /// based on the split points. This allows applying different segment-level
    /// operations to the newly created parts.
    pub fn split_segments(&self, frame_ops: &FrameOps) -> Analysis<H> {
        let (new_frames, new_segments) =
            crate::segment::split_segments_at_points(&self.frames, &self.segments, frame_ops);

        Analysis {
            metadata: self.metadata.clone(),
            frames: new_frames,
            segments: new_segments,
        }
    }

    /// Get frames as encodable without any operations applied.
    pub fn as_encodable(&self) -> Vec<EncodableFrame> {
        self.frames
            .iter()
            .map(|f| EncodableFrame::from_decoded(&f.frame))
            .collect()
    }

    // =========================================================================
    // Fluent Segment Selectors
    // =========================================================================

    /// Select all static segments (pauses/duplicate frames).
    ///
    /// Returns a [`SegmentSelector`] that can be filtered and operated on.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Cap all pauses to 300ms
    /// let ops = analysis.pauses().cap(300);
    ///
    /// // Collapse only long pauses
    /// let ops = analysis.pauses()
    ///     .longer_than(500)
    ///     .collapse(200);
    /// ```
    pub fn pauses(&self) -> crate::selector::SegmentSelector<'_> {
        let segments = self.segments.iter().filter(|s| s.is_static).collect();
        crate::selector::SegmentSelector::new(segments)
    }

    /// Select all motion segments (non-static, actually changing content).
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Speed up all motion by 1.5x
    /// let ops = analysis.motion().speed_up(1.5);
    /// ```
    pub fn motion(&self) -> crate::selector::SegmentSelector<'_> {
        let segments = self.segments.iter().filter(|s| !s.is_static).collect();
        crate::selector::SegmentSelector::new(segments)
    }

    /// Select all segments.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Speed up everything by 2x
    /// let ops = analysis.all().speed_up(2.0);
    /// ```
    pub fn all(&self) -> crate::selector::SegmentSelector<'_> {
        let segments = self.segments.iter().collect();
        crate::selector::SegmentSelector::new(segments)
    }

    /// Select a single segment by ID.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Remove segment 5
    /// let ops = analysis.segment(5).remove();
    /// ```
    pub fn segment(&self, id: usize) -> crate::selector::SegmentSelector<'_> {
        let segments = self.segments.iter().filter(|s| s.id == id).collect();
        crate::selector::SegmentSelector::new(segments)
    }

    /// Select multiple segments by ID.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Collapse specific segments
    /// let ops = analysis.segments_by_id(&[1, 3, 5]).collapse(100);
    /// ```
    pub fn segments_by_id(&self, ids: &[usize]) -> crate::selector::SegmentSelector<'_> {
        let segments = self
            .segments
            .iter()
            .filter(|s| ids.contains(&s.id))
            .collect();
        crate::selector::SegmentSelector::new(segments)
    }

    /// Select segments by frame index range.
    ///
    /// Selects any segment that overlaps with the given frame range.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Speed up the first 100 frames
    /// let ops = analysis.frames_range(0..100).speed_up(2.0);
    /// ```
    pub fn frames_range(
        &self,
        range: std::ops::Range<usize>,
    ) -> crate::selector::SegmentSelector<'_> {
        let segments = self
            .segments
            .iter()
            .filter(|s| {
                // Check if segment overlaps with range
                s.frame_range.start < range.end && s.frame_range.end > range.start
            })
            .collect();
        crate::selector::SegmentSelector::new(segments)
    }

    // ========================================================================
    // Convenience methods for common operations (legacy, use selectors instead)
    // ========================================================================

    /// Cap all static segments (pause points) to a maximum duration.
    ///
    /// Static segments longer than `max_ms` will be collapsed to a single
    /// frame with that duration. Shorter segments are unchanged.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Make all pauses last at most 300ms
    /// let ops = analysis.cap_pauses(300);
    /// let frames = analysis.apply_operations(&ops);
    /// ```
    pub fn cap_pauses(&self, max_ms: u32) -> SegmentOps {
        let max_cs = (max_ms / 10) as u16;
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            if segment.is_static && segment.total_duration_cs > max_cs {
                ops.insert(segment.id, SegmentOp::Collapse { delay_cs: max_cs });
            }
        }

        ops
    }

    /// Collapse all static segments to a fixed duration.
    ///
    /// Every static segment becomes a single frame with the specified delay,
    /// regardless of its original length.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Make all pauses exactly 200ms
    /// let ops = analysis.collapse_all_pauses(200);
    /// ```
    pub fn collapse_all_pauses(&self, duration_ms: u32) -> SegmentOps {
        let delay_cs = (duration_ms / 10) as u16;
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            if segment.is_static {
                ops.insert(segment.id, SegmentOp::Collapse { delay_cs });
            }
        }

        ops
    }

    /// Remove all static segments longer than the specified duration.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Remove any pause longer than 2 seconds
    /// let ops = analysis.remove_long_pauses(2000);
    /// ```
    pub fn remove_long_pauses(&self, min_ms: u32) -> SegmentOps {
        let min_cs = (min_ms / 10) as u16;
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            if segment.is_static && segment.total_duration_cs >= min_cs {
                ops.insert(segment.id, SegmentOp::Remove);
            }
        }

        ops
    }

    /// Speed up all static segments by a factor.
    ///
    /// A factor of 2.0 makes pauses twice as fast (half duration).
    /// A factor of 0.5 makes them half as fast (double duration).
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Make all pauses 3x faster
    /// let ops = analysis.speed_up_pauses(3.0);
    /// ```
    pub fn speed_up_pauses(&self, factor: f64) -> SegmentOps {
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            if segment.is_static {
                ops.insert(
                    segment.id,
                    SegmentOp::Scale {
                        factor: 1.0 / factor,
                    },
                );
            }
        }

        ops
    }

    /// Speed up the entire GIF by a factor.
    ///
    /// Affects all segments, not just static ones.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Make everything 1.5x faster
    /// let ops = analysis.speed_up_all(1.5);
    /// ```
    pub fn speed_up_all(&self, factor: f64) -> SegmentOps {
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            ops.insert(
                segment.id,
                SegmentOp::Scale {
                    factor: 1.0 / factor,
                },
            );
        }

        ops
    }

    /// Create operations that optimize the GIF for a target duration.
    ///
    /// This will collapse/remove static segments as needed to try to
    /// reach the target duration. Non-static segments are preserved.
    ///
    /// Returns `None` if the target is not achievable (non-static content
    /// already exceeds the target).
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Try to get the GIF under 30 seconds
    /// if let Some(ops) = analysis.target_duration(30_000) {
    ///     let frames = analysis.apply_operations(&ops);
    /// }
    /// ```
    pub fn target_duration(&self, target_ms: u64) -> Option<SegmentOps> {
        let current_ms = self.total_duration_ms();
        if current_ms <= target_ms {
            return Some(SegmentOps::new()); // Already under target
        }

        // Calculate how much we need to remove
        let to_remove_ms = current_ms - target_ms;

        // Calculate total static duration
        let static_duration_ms: u64 = self
            .segments
            .iter()
            .filter(|s| s.is_static)
            .map(|s| s.duration_ms() as u64)
            .sum();

        // If we can't remove enough from static segments, return None
        if static_duration_ms < to_remove_ms {
            return None;
        }

        // Sort static segments by duration (longest first)
        let mut static_segs: Vec<_> = self.segments.iter().filter(|s| s.is_static).collect();
        static_segs.sort_by(|a, b| b.total_duration_cs.cmp(&a.total_duration_cs));

        let mut ops = SegmentOps::new();
        let mut removed_ms: u64 = 0;
        let min_pause_ms = 100; // Keep at least 100ms per pause

        for segment in static_segs {
            if removed_ms >= to_remove_ms {
                break;
            }

            let segment_ms = segment.duration_ms() as u64;
            let can_remove = segment_ms.saturating_sub(min_pause_ms as u64);

            if can_remove > 0 {
                let to_remove_from_this = can_remove.min(to_remove_ms - removed_ms);
                let new_duration_ms = segment_ms - to_remove_from_this;

                if new_duration_ms <= min_pause_ms as u64 {
                    // Collapse to minimum
                    ops.insert(
                        segment.id,
                        SegmentOp::Collapse {
                            delay_cs: (min_pause_ms / 10) as u16,
                        },
                    );
                } else {
                    // Set specific duration
                    ops.insert(
                        segment.id,
                        SegmentOp::Collapse {
                            delay_cs: (new_duration_ms / 10) as u16,
                        },
                    );
                }

                removed_ms += to_remove_from_this;
            }
        }

        Some(ops)
    }

    /// Merge multiple operation sets, with later ops overriding earlier ones.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let base = analysis.cap_pauses(500);
    /// let extra = analysis.speed_up_all(1.2);
    /// let combined = Analysis::<H>::merge_ops(&[base, extra]);
    /// ```
    pub fn merge_ops(op_sets: &[SegmentOps]) -> SegmentOps {
        let mut merged = SegmentOps::new();
        for ops in op_sets {
            merged.extend(ops.iter().map(|(k, v)| (*k, v.clone())));
        }
        merged
    }
}
