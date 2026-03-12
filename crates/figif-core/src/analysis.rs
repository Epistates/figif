//! Frame analysis and segment detection.

use crate::traits::FrameHasher;
use crate::types::{AnalyzedFrame, DecodedFrame, Segment};

/// Configuration for segment analysis.
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    /// Maximum hash distance for frames to be considered similar.
    pub similarity_threshold: u32,
    /// Minimum number of consecutive similar frames to form a segment.
    pub min_segment_frames: usize,
    /// Whether to mark segments with all identical frames as static.
    pub detect_static: bool,
    /// Distance threshold for considering frames identical (for static detection).
    pub identical_threshold: u32,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 5,
            min_segment_frames: 2,
            detect_static: true,
            identical_threshold: 0,
        }
    }
}

impl AnalysisConfig {
    /// Create a new analysis config with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the similarity threshold.
    pub fn with_similarity_threshold(mut self, threshold: u32) -> Self {
        self.similarity_threshold = threshold;
        self
    }

    /// Set the minimum segment frames.
    pub fn with_min_segment_frames(mut self, min: usize) -> Self {
        self.min_segment_frames = min.max(1);
        self
    }

    /// Enable or disable static frame detection.
    pub fn with_static_detection(mut self, enabled: bool) -> Self {
        self.detect_static = enabled;
        self
    }

    /// Set the threshold for identical frame detection.
    pub fn with_identical_threshold(mut self, threshold: u32) -> Self {
        self.identical_threshold = threshold;
        self
    }
}

/// Analyze frames and detect segments of similar consecutive frames.
///
/// This function:
/// 1. Computes hashes for all frames using the provided hasher
/// 2. Compares adjacent frames to find similarity
/// 3. Groups consecutive similar frames into segments
///
/// # Arguments
///
/// * `frames` - Decoded frames to analyze
/// * `hasher` - The hash algorithm to use
/// * `config` - Analysis configuration
///
/// Returns
///
/// A tuple of (analyzed_frames, segments)
pub fn analyze_frames<H: FrameHasher>(
    frames: Vec<DecodedFrame>,
    hasher: &H,
    config: &AnalysisConfig,
    progress: Option<&(dyn Fn(usize, usize) + Send + Sync)>,
) -> (Vec<AnalyzedFrame<H::Hash>>, Vec<Segment>) {
    if frames.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let total_frames = frames.len();

    // Compute hashes for all frames
    let mut analyzed: Vec<AnalyzedFrame<H::Hash>> = Vec::with_capacity(total_frames);
    for (i, frame) in frames.into_iter().enumerate() {
        let hash = hasher.hash_frame(&frame.image);
        analyzed.push(AnalyzedFrame::new(frame, hash));

        if let Some(callback) = progress {
            callback(i + 1, total_frames);
        }
    }

    // Compute distances between adjacent frames and store in each frame
    let mut distances: Vec<u32> = Vec::with_capacity(analyzed.len().saturating_sub(1));
    for i in 0..analyzed.len().saturating_sub(1) {
        let dist = hasher.distance(&analyzed[i].hash, &analyzed[i + 1].hash);
        distances.push(dist);
        // Store distance in the NEXT frame (distance from previous)
        analyzed[i + 1].distance_to_prev = Some(dist);
    }

    // Detect segments based on similarity threshold
    let segments = detect_segments(&mut analyzed, &distances, config);

    (analyzed, segments)
}

/// Analyze frames in parallel (requires `parallel` feature).
#[cfg(feature = "parallel")]
pub fn analyze_frames_parallel<H: FrameHasher>(
    frames: Vec<DecodedFrame>,
    hasher: &H,
    config: &AnalysisConfig,
    progress: Option<&(dyn Fn(usize, usize) + Send + Sync)>,
) -> (Vec<AnalyzedFrame<H::Hash>>, Vec<Segment>)
where
    H::Hash: Send,
{
    use rayon::prelude::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    if frames.is_empty() {
        return (Vec::new(), Vec::new());
    }

    let total_frames = frames.len();
    let current_frame = AtomicUsize::new(0);

    // Compute hashes for all frames in parallel
    let mut analyzed: Vec<AnalyzedFrame<H::Hash>> = frames
        .into_par_iter()
        .map(|frame| {
            let hash = hasher.hash_frame(&frame.image);

            if let Some(callback) = progress {
                let current = current_frame.fetch_add(1, Ordering::Relaxed) + 1;
                callback(current, total_frames);
            }

            AnalyzedFrame::new(frame, hash)
        })
        .collect();

    // Parallel sort back to original order if needed (map preserves order)
    // and compute distances between adjacent frames
    let mut distances: Vec<u32> = Vec::with_capacity(analyzed.len().saturating_sub(1));
    for i in 0..analyzed.len().saturating_sub(1) {
        let dist = hasher.distance(&analyzed[i].hash, &analyzed[i + 1].hash);
        distances.push(dist);
        // Store distance in the NEXT frame (distance from previous)
        analyzed[i + 1].distance_to_prev = Some(dist);
    }

    // Detect segments
    let segments = detect_segments(&mut analyzed, &distances, config);

    (analyzed, segments)
}

/// Detect segments from analyzed frames and their distances.
fn detect_segments<H>(
    analyzed: &mut [AnalyzedFrame<H>],
    distances: &[u32],
    config: &AnalysisConfig,
) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut segment_id = 0;

    let mut i = 0;
    while i < analyzed.len() {
        // Find the end of this segment (consecutive similar frames)
        let segment_start = i;
        let mut segment_end = i + 1;
        let mut total_distance: u64 = 0;
        let mut distance_count: usize = 0;
        let mut all_identical = true;

        while segment_end < analyzed.len() {
            let dist_idx = segment_end - 1;
            if dist_idx < distances.len() {
                let dist = distances[dist_idx];
                if dist <= config.similarity_threshold {
                    total_distance += dist as u64;
                    distance_count += 1;

                    // Check if frames are truly identical if threshold is 0
                    if dist > config.identical_threshold {
                        all_identical = false;
                    } else if config.identical_threshold == 0 {
                        // Double check with raw pixels to avoid hash collisions or
                        // gradient-only similarities (like uniform color shifts)
                        if analyzed[segment_end - 1].frame.image
                            != analyzed[segment_end].frame.image
                        {
                            all_identical = false;
                        }
                    }

                    segment_end += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        let segment_frames = segment_end - segment_start;

        // Create segment if it meets minimum frame requirement
        if segment_frames >= config.min_segment_frames {
            // Calculate total duration
            let total_duration_cs: u16 = analyzed[segment_start..segment_end]
                .iter()
                .map(|f| f.delay_cs())
                .sum();

            // Calculate average distance
            let avg_distance = if distance_count > 0 {
                total_distance as f64 / distance_count as f64
            } else {
                0.0
            };

            // Assign segment ID to frames
            for frame in &mut analyzed[segment_start..segment_end] {
                frame.segment_id = Some(segment_id);
            }

            segments.push(Segment {
                id: segment_id,
                frame_range: segment_start..segment_end,
                total_duration_cs,
                avg_distance,
                is_static: config.detect_static && all_identical,
            });

            segment_id += 1;
        } else {
            // Single frame or doesn't meet minimum - still create a segment
            let total_duration_cs = analyzed[segment_start].delay_cs();

            analyzed[segment_start].segment_id = Some(segment_id);

            segments.push(Segment {
                id: segment_id,
                frame_range: segment_start..segment_start + 1,
                total_duration_cs,
                avg_distance: 0.0,
                is_static: false,
            });

            segment_id += 1;
            segment_end = segment_start + 1;
        }

        i = segment_end;
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analysis_config_defaults() {
        let config = AnalysisConfig::default();
        assert_eq!(config.similarity_threshold, 5);
        assert_eq!(config.min_segment_frames, 2);
        assert!(config.detect_static);
    }

    #[test]
    fn test_analysis_config_builder() {
        let config = AnalysisConfig::new()
            .with_similarity_threshold(10)
            .with_min_segment_frames(3)
            .with_static_detection(false);

        assert_eq!(config.similarity_threshold, 10);
        assert_eq!(config.min_segment_frames, 3);
        assert!(!config.detect_static);
    }
}
