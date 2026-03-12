//! Segment operations and manipulation.

use crate::types::{
    AnalyzedFrame, EncodableFrame, FrameOp, FrameOps, Segment, SegmentOp, SegmentOps,
};

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Apply segment operations to analyzed frames, producing encodable frames.
///
/// This function takes the analyzed frames and a map of operations to apply
/// to each segment, and produces a list of frames ready for encoding.
///
/// # Arguments
///
/// * `frames` - The analyzed frames
/// * `segments` - The detected segments
/// * `ops` - Operations to apply to each segment (by segment ID)
///
/// # Returns
///
/// A vector of encodable frames with timing adjustments applied.
pub fn apply_segment_operations<H: Sync + Send>(
    frames: &[AnalyzedFrame<H>],
    segments: &[Segment],
    ops: &SegmentOps,
) -> Vec<EncodableFrame> {
    #[cfg(feature = "parallel")]
    {
        segments
            .par_iter()
            .map(|segment| {
                let op = ops.get(&segment.id).unwrap_or(&SegmentOp::Keep);
                let segment_frames = &frames[segment.frame_range.clone()];
                let mut segment_output = Vec::new();

                match op {
                    SegmentOp::Keep => {
                        for frame in segment_frames {
                            segment_output.push(EncodableFrame::from_decoded(&frame.frame));
                        }
                    }
                    SegmentOp::Remove => {}
                    SegmentOp::Collapse { delay_cs } => {
                        if let Some(first) = segment_frames.first() {
                            segment_output.push(EncodableFrame::new(first.frame.image.clone(), *delay_cs));
                        }
                    }
                    SegmentOp::SetDuration { total_cs } => {
                        let frame_count = segment_frames.len();
                        if frame_count > 0 {
                            let per_frame_delay = *total_cs / frame_count as u16;
                            let remainder = *total_cs % frame_count as u16;

                            for (i, frame) in segment_frames.iter().enumerate() {
                                let delay = if (i as u16) < remainder {
                                    per_frame_delay + 1
                                } else {
                                    per_frame_delay
                                };
                                segment_output.push(EncodableFrame::new(frame.frame.image.clone(), delay));
                            }
                        }
                    }
                    SegmentOp::Scale { factor } => {
                        for frame in segment_frames {
                            let original_delay = frame.frame.delay_centiseconds as f64;
                            let new_delay = (original_delay * factor).round() as u16;
                            segment_output.push(EncodableFrame::new(
                                frame.frame.image.clone(),
                                new_delay.max(1),
                            ));
                        }
                    }
                    SegmentOp::SetFrameDelay { delay_cs } => {
                        for frame in segment_frames {
                            segment_output.push(EncodableFrame::new(frame.frame.image.clone(), *delay_cs));
                        }
                    }
                }
                segment_output
            })
            .flatten()
            .collect()
    }

    #[cfg(not(feature = "parallel"))]
    {
        let mut output = Vec::new();

        for segment in segments {
            let op = ops.get(&segment.id).unwrap_or(&SegmentOp::Keep);
            let segment_frames = &frames[segment.frame_range.clone()];

            match op {
                SegmentOp::Keep => {
                    for frame in segment_frames {
                        output.push(EncodableFrame::from_decoded(&frame.frame));
                    }
                }

                SegmentOp::Remove => {}

                SegmentOp::Collapse { delay_cs } => {
                    if let Some(first) = segment_frames.first() {
                        output.push(EncodableFrame::new(first.frame.image.clone(), *delay_cs));
                    }
                }

                SegmentOp::SetDuration { total_cs } => {
                    // Distribute duration evenly across all frames
                    let frame_count = segment_frames.len();
                    if frame_count > 0 {
                        let per_frame_delay = *total_cs / frame_count as u16;
                        let remainder = *total_cs % frame_count as u16;

                        for (i, frame) in segment_frames.iter().enumerate() {
                            // Add 1 to first `remainder` frames to distribute evenly
                            let delay = if (i as u16) < remainder {
                                per_frame_delay + 1
                            } else {
                                per_frame_delay
                            };
                            output.push(EncodableFrame::new(frame.frame.image.clone(), delay));
                        }
                    }
                }

                SegmentOp::Scale { factor } => {
                    // Scale each frame's delay by the factor
                    for frame in segment_frames {
                        let original_delay = frame.frame.delay_centiseconds as f64;
                        let new_delay = (original_delay * factor).round() as u16;
                        // Ensure minimum delay of 1 centisecond
                        let new_delay = new_delay.max(1);
                        output.push(EncodableFrame::new(frame.frame.image.clone(), new_delay));
                    }
                }

                SegmentOp::SetFrameDelay { delay_cs } => {
                    // Set the same delay for all frames
                    for frame in segment_frames {
                        output.push(EncodableFrame::new(frame.frame.image.clone(), *delay_cs));
                    }
                }
            }
        }

        output
    }
}

/// Calculate the impact of segment operations without cloning images.
///
/// Returns (total_frames, total_duration_cs).
pub fn dry_run_segment_operations<H: Sync + Send>(
    frames: &[AnalyzedFrame<H>],
    segments: &[Segment],
    ops: &SegmentOps,
) -> (usize, u64) {
    let mut total_frames = 0;
    let mut total_duration_cs: u64 = 0;

    for segment in segments {
        let op = ops.get(&segment.id).unwrap_or(&SegmentOp::Keep);
        let segment_frames = &frames[segment.frame_range.clone()];

        match op {
            SegmentOp::Keep => {
                total_frames += segment_frames.len();
                total_duration_cs += segment.total_duration_cs as u64;
            }
            SegmentOp::Remove => {}
            SegmentOp::Collapse { delay_cs } => {
                if !segment_frames.is_empty() {
                    total_frames += 1;
                    total_duration_cs += *delay_cs as u64;
                }
            }
            SegmentOp::SetDuration { total_cs } => {
                if !segment_frames.is_empty() {
                    total_frames += segment_frames.len();
                    total_duration_cs += *total_cs as u64;
                }
            }
            SegmentOp::Scale { factor } => {
                for frame in segment_frames {
                    let original_delay = frame.frame.delay_centiseconds as f64;
                    let new_delay = (original_delay * factor).round() as u16;
                    total_frames += 1;
                    total_duration_cs += new_delay.max(1) as u64;
                }
            }
            SegmentOp::SetFrameDelay { delay_cs } => {
                for _ in segment_frames {
                    total_frames += 1;
                    total_duration_cs += *delay_cs as u64;
                }
            }
        }
    }

    (total_frames, total_duration_cs)
}
///
/// This is the enhanced version that handles frame-level operations like
/// individual frame removal and segment splitting.
///
/// # Arguments
///
/// * `frames` - The analyzed frames
/// * `segments` - The detected segments
/// * `segment_ops` - Operations to apply to each segment (by segment ID)
/// * `frame_ops` - Operations to apply to individual frames (by segment ID + frame index)
///
/// # Returns
///
/// A vector of encodable frames with all operations applied.
pub fn apply_operations<H: Sync + Send>(
    frames: &[AnalyzedFrame<H>],
    segments: &[Segment],
    segment_ops: &SegmentOps,
    frame_ops: &FrameOps,
) -> Vec<EncodableFrame> {
    #[cfg(feature = "parallel")]
    {
        segments
            .par_iter()
            .map(|segment| {
                let seg_op = segment_ops.get(&segment.id).unwrap_or(&SegmentOp::Keep);
                let segment_frames = &frames[segment.frame_range.clone()];
                let mut segment_output = Vec::new();

                // If segment-level operation is Remove, skip the entire segment
                if matches!(seg_op, SegmentOp::Remove) {
                    return segment_output;
                }

                // If segment-level operation is not Keep, apply it without frame ops
                if !matches!(seg_op, SegmentOp::Keep) {
                    match seg_op {
                        SegmentOp::Collapse { delay_cs } => {
                            if let Some(first) = segment_frames.first() {
                                segment_output
                                    .push(EncodableFrame::new(first.frame.image.clone(), *delay_cs));
                            }
                        }
                        SegmentOp::SetDuration { total_cs } => {
                            let frame_count = segment_frames.len();
                            if frame_count > 0 {
                                let per_frame_delay = *total_cs / frame_count as u16;
                                let remainder = *total_cs % frame_count as u16;
                                for (i, frame) in segment_frames.iter().enumerate() {
                                    let delay = if (i as u16) < remainder {
                                        per_frame_delay + 1
                                    } else {
                                        per_frame_delay
                                    };
                                    segment_output
                                        .push(EncodableFrame::new(frame.frame.image.clone(), delay));
                                }
                            }
                        }
                        SegmentOp::Scale { factor } => {
                            for frame in segment_frames {
                                let original_delay = frame.frame.delay_centiseconds as f64;
                                let new_delay = (original_delay * factor).round() as u16;
                                let new_delay = new_delay.max(1);
                                segment_output
                                    .push(EncodableFrame::new(frame.frame.image.clone(), new_delay));
                            }
                        }
                        SegmentOp::SetFrameDelay { delay_cs } => {
                            for frame in segment_frames {
                                segment_output.push(EncodableFrame::new(
                                    frame.frame.image.clone(),
                                    *delay_cs,
                                ));
                            }
                        }
                        _ => {}
                    }
                } else {
                    // Segment operation is Keep - apply frame-level operations
                    for (i, frame) in segment_frames.iter().enumerate() {
                        let frame_op = frame_ops.get(&(segment.id, i)).unwrap_or(&FrameOp::Keep);

                        match frame_op {
                            FrameOp::Keep | FrameOp::SplitAfter => {
                                segment_output.push(EncodableFrame::from_decoded(&frame.frame));
                            }
                            FrameOp::Remove => {}
                        }
                    }
                }
                segment_output
            })
            .flatten()
            .collect()
    }

    #[cfg(not(feature = "parallel"))]
    {
        let mut output = Vec::new();

        for segment in segments {
            let seg_op = segment_ops.get(&segment.id).unwrap_or(&SegmentOp::Keep);
            let segment_frames = &frames[segment.frame_range.clone()];

            // If segment-level operation is Remove, skip the entire segment
            if matches!(seg_op, SegmentOp::Remove) {
                continue;
            }

            // If segment-level operation is not Keep, apply it without frame ops
            if !matches!(seg_op, SegmentOp::Keep) {
                match seg_op {
                    SegmentOp::Collapse { delay_cs } => {
                        if let Some(first) = segment_frames.first() {
                            output.push(EncodableFrame::new(first.frame.image.clone(), *delay_cs));
                        }
                    }
                    SegmentOp::SetDuration { total_cs } => {
                        let frame_count = segment_frames.len();
                        if frame_count > 0 {
                            let per_frame_delay = *total_cs / frame_count as u16;
                            let remainder = *total_cs % frame_count as u16;
                            for (i, frame) in segment_frames.iter().enumerate() {
                                let delay = if (i as u16) < remainder {
                                    per_frame_delay + 1
                                } else {
                                    per_frame_delay
                                };
                                output.push(EncodableFrame::new(frame.frame.image.clone(), delay));
                            }
                        }
                    }
                    SegmentOp::Scale { factor } => {
                        for frame in segment_frames {
                            let original_delay = frame.frame.delay_centiseconds as f64;
                            let new_delay = (original_delay * factor).round() as u16;
                            let new_delay = new_delay.max(1);
                            output.push(EncodableFrame::new(frame.frame.image.clone(), new_delay));
                        }
                    }
                    SegmentOp::SetFrameDelay { delay_cs } => {
                        for frame in segment_frames {
                            output.push(EncodableFrame::new(frame.frame.image.clone(), *delay_cs));
                        }
                    }
                    _ => {}
                }
                continue;
            }

            // Segment operation is Keep - apply frame-level operations
            for (i, frame) in segment_frames.iter().enumerate() {
                let frame_op = frame_ops.get(&(segment.id, i)).unwrap_or(&FrameOp::Keep);

                match frame_op {
                    FrameOp::Keep | FrameOp::SplitAfter => {
                        // Keep the frame (SplitAfter is just a marker for UI, doesn't affect output)
                        output.push(EncodableFrame::from_decoded(&frame.frame));
                    }
                    FrameOp::Remove => {
                        // Skip this frame
                    }
                }
            }
        }

        output
    }
}

/// Calculate the impact of all operations without cloning images.
///
/// Returns (total_frames, total_duration_cs).
pub fn dry_run_all_operations<H: Sync + Send>(
    frames: &[AnalyzedFrame<H>],
    segments: &[Segment],
    segment_ops: &SegmentOps,
    frame_ops: &FrameOps,
) -> (usize, u64) {
    let mut total_frames = 0;
    let mut total_duration_cs: u64 = 0;

    for segment in segments {
        let seg_op = segment_ops.get(&segment.id).unwrap_or(&SegmentOp::Keep);
        let segment_frames = &frames[segment.frame_range.clone()];

        // If segment-level operation is Remove, skip the entire segment
        if matches!(seg_op, SegmentOp::Remove) {
            continue;
        }

        // If segment-level operation is not Keep, apply it without frame ops
        if !matches!(seg_op, SegmentOp::Keep) {
            match seg_op {
                SegmentOp::Collapse { delay_cs } => {
                    if !segment_frames.is_empty() {
                        total_frames += 1;
                        total_duration_cs += *delay_cs as u64;
                    }
                }
                SegmentOp::SetDuration { total_cs } => {
                    let frame_count = segment_frames.len();
                    if frame_count > 0 {
                        total_frames += frame_count;
                        total_duration_cs += *total_cs as u64;
                    }
                }
                SegmentOp::Scale { factor } => {
                    for frame in segment_frames {
                        let original_delay = frame.frame.delay_centiseconds as f64;
                        let new_delay = (original_delay * factor).round() as u16;
                        total_frames += 1;
                        total_duration_cs += new_delay.max(1) as u64;
                    }
                }
                SegmentOp::SetFrameDelay { delay_cs } => {
                    for _ in segment_frames {
                        total_frames += 1;
                        total_duration_cs += *delay_cs as u64;
                    }
                }
                _ => {}
            }
            continue;
        }

        // Segment operation is Keep - apply frame-level operations
        for (i, frame) in segment_frames.iter().enumerate() {
            let frame_op = frame_ops.get(&(segment.id, i)).unwrap_or(&FrameOp::Keep);

            match frame_op {
                FrameOp::Keep | FrameOp::SplitAfter => {
                    total_frames += 1;
                    total_duration_cs += frame.frame.delay_centiseconds as u64;
                }
                FrameOp::Remove => {}
            }
        }
    }

    (total_frames, total_duration_cs)
}

/// Partition existing segments into new ones based on `FrameOp::SplitAfter` markers.
///
/// Returns (new_analyzed_frames, new_segments).
pub fn split_segments_at_points<H: Clone>(
    frames: &[AnalyzedFrame<H>],
    segments: &[Segment],
    frame_ops: &FrameOps,
) -> (Vec<AnalyzedFrame<H>>, Vec<Segment>) {
    let mut new_frames = frames.to_vec();
    let mut new_segments = Vec::new();
    let mut current_segment_id = 0;

    for segment in segments {
        let mut sub_segment_start = segment.frame_range.start;

        for i in 0..segment.frame_count() {
            let abs_idx = segment.frame_range.start + i;
            let op = frame_ops.get(&(segment.id, i)).unwrap_or(&FrameOp::Keep);
            let is_last_frame = i == segment.frame_count() - 1;

            if matches!(op, FrameOp::SplitAfter) || is_last_frame {
                // Split point or end of segment reached
                let sub_segment_end = abs_idx + 1;
                
                // Only create if we haven't already processed this sub-segment
                if sub_segment_start < sub_segment_end {
                    let sub_range = sub_segment_start..sub_segment_end;

                    // Re-calculate statistics for the new sub-segment
                    let sub_frames = &new_frames[sub_range.clone()];
                    let total_duration_cs: u16 = sub_frames.iter().map(|f| f.delay_cs()).sum();

                    // Inherit static status from parent segment
                    let is_static = segment.is_static;

                    // Create the new segment
                    new_segments.push(Segment {
                        id: current_segment_id,
                        frame_range: sub_range.clone(),
                        total_duration_cs,
                        avg_distance: segment.avg_distance,
                        is_static,
                    });

                    // Update segment ID for these frames
                    for f in &mut new_frames[sub_range] {
                        f.segment_id = Some(current_segment_id);
                    }

                    current_segment_id += 1;
                    sub_segment_start = sub_segment_end;
                }
            }
        }
    }

    (new_frames, new_segments)
}

/// Compute statistics for a set of segments.
#[derive(Debug, Clone, Default)]
pub struct SegmentStats {
    /// Total number of segments.
    pub total_segments: usize,
    /// Number of static segments (all identical frames).
    pub static_segments: usize,
    /// Total number of frames.
    pub total_frames: usize,
    /// Total duration in centiseconds.
    pub total_duration_cs: u64,
    /// Average frames per segment.
    pub avg_frames_per_segment: f64,
    /// Average segment duration in centiseconds.
    pub avg_duration_cs: f64,
}

impl SegmentStats {
    /// Compute statistics from a list of segments.
    pub fn from_segments(segments: &[Segment]) -> Self {
        if segments.is_empty() {
            return Self::default();
        }

        let total_segments = segments.len();
        let static_segments = segments.iter().filter(|s| s.is_static).count();
        let total_frames: usize = segments.iter().map(|s| s.frame_count()).sum();
        let total_duration_cs: u64 = segments.iter().map(|s| s.total_duration_cs as u64).sum();

        Self {
            total_segments,
            static_segments,
            total_frames,
            total_duration_cs,
            avg_frames_per_segment: total_frames as f64 / total_segments as f64,
            avg_duration_cs: total_duration_cs as f64 / total_segments as f64,
        }
    }

    /// Total duration in milliseconds.
    pub fn total_duration_ms(&self) -> u64 {
        self.total_duration_cs * 10
    }
}

/// Find segments that are likely "pause" segments (static with long duration).
///
/// These are good candidates for collapsing or removing.
pub fn find_pause_segments(segments: &[Segment], min_duration_cs: u16) -> Vec<usize> {
    segments
        .iter()
        .filter(|s| s.is_static && s.total_duration_cs >= min_duration_cs)
        .map(|s| s.id)
        .collect()
}

/// Find the longest segment.
pub fn find_longest_segment(segments: &[Segment]) -> Option<&Segment> {
    segments.iter().max_by_key(|s| s.total_duration_cs)
}

/// Suggest operations to reduce GIF duration by targeting static segments.
///
/// This creates operations that collapse long static segments to a shorter duration.
///
/// # Arguments
///
/// * `segments` - The detected segments
/// * `target_reduction` - Target reduction ratio (0.5 = reduce duration by half)
/// * `max_static_duration_cs` - Maximum duration for static segments after reduction
pub fn suggest_compression_ops(
    segments: &[Segment],
    target_reduction: f64,
    max_static_duration_cs: u16,
) -> SegmentOps {
    let mut ops = SegmentOps::new();

    for segment in segments {
        if segment.is_static && segment.total_duration_cs > max_static_duration_cs {
            // Collapse long static segments
            let new_duration = ((segment.total_duration_cs as f64 * target_reduction) as u16)
                .max(max_static_duration_cs.min(segment.total_duration_cs));
            ops.insert(
                segment.id,
                SegmentOp::Collapse {
                    delay_cs: new_duration,
                },
            );
        }
    }

    ops
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_stats_empty() {
        let stats = SegmentStats::from_segments(&[]);
        assert_eq!(stats.total_segments, 0);
        assert_eq!(stats.total_frames, 0);
    }

    #[test]
    fn test_find_pause_segments() {
        let segments = vec![
            Segment {
                id: 0,
                frame_range: 0..5,
                total_duration_cs: 50,
                avg_distance: 0.0,
                is_static: true,
            },
            Segment {
                id: 1,
                frame_range: 5..10,
                total_duration_cs: 100,
                avg_distance: 2.0,
                is_static: false,
            },
            Segment {
                id: 2,
                frame_range: 10..15,
                total_duration_cs: 200,
                avg_distance: 0.0,
                is_static: true,
            },
        ];

        let pauses = find_pause_segments(&segments, 100);
        assert_eq!(pauses, vec![2]);
    }

    #[test]
    fn test_split_segments_at_points() {
        use image::RgbaImage;
        use crate::types::DecodedFrame;

        let frames: Vec<AnalyzedFrame<()>> = (0..10)
            .map(|i| {
                AnalyzedFrame::new(
                    DecodedFrame {
                        index: i,
                        image: RgbaImage::new(1, 1),
                        delay_centiseconds: 10,
                        disposal: crate::types::DisposalMethod::Keep,
                        left: 0,
                        top: 0,
                    },
                    (),
                )
            })
            .collect();

        let segments = vec![Segment {
            id: 0,
            frame_range: 0..10,
            total_duration_cs: 100,
            avg_distance: 0.0,
            is_static: true,
        }];

        let mut frame_ops = FrameOps::new();
        // Split after frame 2 (making a 3-frame segment and a 7-frame segment)
        frame_ops.insert((0, 2), FrameOp::SplitAfter);

        let (new_frames, new_segments) = split_segments_at_points(&frames, &segments, &frame_ops);

        assert_eq!(new_segments.len(), 2);
        assert_eq!(new_segments[0].id, 0);
        assert_eq!(new_segments[0].frame_range, 0..3);
        assert_eq!(new_segments[0].total_duration_cs, 30);

        assert_eq!(new_segments[1].id, 1);
        assert_eq!(new_segments[1].frame_range, 3..10);
        assert_eq!(new_segments[1].total_duration_cs, 70);

        // Verify frame assignments
        assert_eq!(new_frames[0].segment_id, Some(0));
        assert_eq!(new_frames[2].segment_id, Some(0));
        assert_eq!(new_frames[3].segment_id, Some(1));
        assert_eq!(new_frames[9].segment_id, Some(1));
    }
}
