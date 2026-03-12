//! Fluent segment selector API for building operations.
//!
//! This module provides a chainable, ergonomic API for selecting and
//! manipulating segments in an analyzed GIF.
//!
//! # Example
//!
//! ```ignore
//! use figif_core::prelude::*;
//!
//! let analysis = Figif::new().analyze_file("demo.gif")?;
//!
//! // Cap all pauses to 300ms
//! let ops = analysis.pauses().cap(300);
//!
//! // Collapse only long pauses
//! let ops = analysis.pauses()
//!     .longer_than(500)
//!     .collapse(200);
//!
//! // Speed up motion segments
//! let ops = analysis.motion().speed_up(1.5);
//!
//! // Combine operations
//! let ops = analysis.pauses().cap(300)
//!     .merge(&analysis.motion().speed_up(1.2));
//! ```

use crate::types::{Segment, SegmentOp, SegmentOps};

/// A selector over a subset of segments, supporting filtering and operations.
///
/// Created via methods on [`Analysis`](crate::Analysis) like `pauses()`, `motion()`, etc.
/// Filters can be chained, and terminal operations produce [`SegmentOps`].
#[derive(Debug, Clone)]
pub struct SegmentSelector<'a> {
    segments: Vec<&'a Segment>,
}

impl<'a> SegmentSelector<'a> {
    /// Create a new selector from a list of segment references.
    pub(crate) fn new(segments: Vec<&'a Segment>) -> Self {
        Self { segments }
    }

    /// Get the number of selected segments.
    pub fn count(&self) -> usize {
        self.segments.len()
    }

    /// Check if any segments are selected.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Get the selected segments.
    pub fn segments(&self) -> &[&'a Segment] {
        &self.segments
    }

    /// Get total duration of selected segments in milliseconds.
    pub fn total_duration_ms(&self) -> u64 {
        self.segments.iter().map(|s| s.duration_ms() as u64).sum()
    }

    /// Get total frame count of selected segments.
    pub fn total_frames(&self) -> usize {
        self.segments.iter().map(|s| s.frame_count()).sum()
    }

    // =========================================================================
    // Filters - return Self for chaining
    // =========================================================================

    /// Filter to segments longer than the specified duration.
    pub fn longer_than(self, ms: u32) -> Self {
        let cs = (ms / 10) as u16;
        Self {
            segments: self
                .segments
                .into_iter()
                .filter(|s| s.total_duration_cs > cs)
                .collect(),
        }
    }

    /// Filter to segments shorter than the specified duration.
    pub fn shorter_than(self, ms: u32) -> Self {
        let cs = (ms / 10) as u16;
        Self {
            segments: self
                .segments
                .into_iter()
                .filter(|s| s.total_duration_cs < cs)
                .collect(),
        }
    }

    /// Filter to segments with duration in the specified range (inclusive).
    pub fn duration_between(self, min_ms: u32, max_ms: u32) -> Self {
        let min_cs = (min_ms / 10) as u16;
        let max_cs = (max_ms / 10) as u16;
        Self {
            segments: self
                .segments
                .into_iter()
                .filter(|s| s.total_duration_cs >= min_cs && s.total_duration_cs <= max_cs)
                .collect(),
        }
    }

    /// Filter to segments with more than N frames.
    pub fn frames_gt(self, count: usize) -> Self {
        Self {
            segments: self
                .segments
                .into_iter()
                .filter(|s| s.frame_count() > count)
                .collect(),
        }
    }

    /// Filter to segments with fewer than N frames.
    pub fn frames_lt(self, count: usize) -> Self {
        Self {
            segments: self
                .segments
                .into_iter()
                .filter(|s| s.frame_count() < count)
                .collect(),
        }
    }

    /// Filter to segments with exactly N frames.
    pub fn frames_eq(self, count: usize) -> Self {
        Self {
            segments: self
                .segments
                .into_iter()
                .filter(|s| s.frame_count() == count)
                .collect(),
        }
    }

    /// Filter using a custom predicate.
    pub fn filter<F>(self, predicate: F) -> Self
    where
        F: Fn(&Segment) -> bool,
    {
        Self {
            segments: self.segments.into_iter().filter(|s| predicate(s)).collect(),
        }
    }

    /// Take only the first N segments.
    pub fn take(self, n: usize) -> Self {
        Self {
            segments: self.segments.into_iter().take(n).collect(),
        }
    }

    /// Skip the first N segments.
    pub fn skip(self, n: usize) -> Self {
        Self {
            segments: self.segments.into_iter().skip(n).collect(),
        }
    }

    /// Take the first segment only.
    pub fn first(self) -> Self {
        self.take(1)
    }

    /// Take the last segment only.
    pub fn last(self) -> Self {
        Self {
            segments: self.segments.into_iter().last().into_iter().collect(),
        }
    }

    // =========================================================================
    // Terminal operations - return SegmentOps
    // =========================================================================

    /// Cap duration to a maximum value.
    ///
    /// Segments longer than `max_ms` are collapsed to a single frame
    /// with that duration. Shorter segments are unchanged.
    pub fn cap(&self, max_ms: u32) -> SegmentOps {
        let max_cs = (max_ms / 10) as u16;
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            if segment.total_duration_cs > max_cs {
                ops.insert(segment.id, SegmentOp::Collapse { delay_cs: max_cs });
            }
        }

        ops
    }

    /// Collapse each segment to a single frame with the specified duration.
    pub fn collapse(&self, duration_ms: u32) -> SegmentOps {
        let delay_cs = (duration_ms / 10) as u16;
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            ops.insert(segment.id, SegmentOp::Collapse { delay_cs });
        }

        ops
    }

    /// Remove all selected segments entirely.
    pub fn remove(&self) -> SegmentOps {
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            ops.insert(segment.id, SegmentOp::Remove);
        }

        ops
    }

    /// Speed up selected segments by a factor.
    ///
    /// A factor of 2.0 makes segments play 2x faster (half duration).
    pub fn speed_up(&self, factor: f64) -> SegmentOps {
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

    /// Slow down selected segments by a factor.
    ///
    /// A factor of 2.0 makes segments play 2x slower (double duration).
    pub fn slow_down(&self, factor: f64) -> SegmentOps {
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            ops.insert(segment.id, SegmentOp::Scale { factor });
        }

        ops
    }

    /// Set the total duration for each selected segment.
    ///
    /// The duration is distributed evenly across all frames in the segment.
    pub fn set_duration(&self, ms: u32) -> SegmentOps {
        let total_cs = (ms / 10) as u16;
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            ops.insert(segment.id, SegmentOp::SetDuration { total_cs });
        }

        ops
    }

    /// Set a fixed delay for each frame in the selected segments.
    pub fn set_frame_delay(&self, ms: u32) -> SegmentOps {
        let delay_cs = (ms / 10) as u16;
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            ops.insert(segment.id, SegmentOp::SetFrameDelay { delay_cs });
        }

        ops
    }

    /// Explicitly keep selected segments unchanged.
    ///
    /// This is useful when merging with other operations to ensure
    /// certain segments are not modified.
    pub fn keep(&self) -> SegmentOps {
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            ops.insert(segment.id, SegmentOp::Keep);
        }

        ops
    }

    /// Scale timing by a raw factor.
    ///
    /// Factor < 1.0 speeds up, factor > 1.0 slows down.
    pub fn scale(&self, factor: f64) -> SegmentOps {
        let mut ops = SegmentOps::new();

        for segment in &self.segments {
            ops.insert(segment.id, SegmentOp::Scale { factor });
        }

        ops
    }
}

// =========================================================================
// Extension trait for SegmentOps to enable fluent merging
// =========================================================================

/// Extension methods for [`SegmentOps`].
pub trait SegmentOpsExt {
    /// Merge with another set of operations.
    ///
    /// Operations from `other` override operations in `self` for the same segment.
    fn merge(&self, other: &SegmentOps) -> SegmentOps;

    /// Merge with another set, consuming self.
    fn and(self, other: SegmentOps) -> SegmentOps;

    /// Merge multiple operation sets.
    fn merge_all(sets: &[&SegmentOps]) -> SegmentOps;
}

impl SegmentOpsExt for SegmentOps {
    fn merge(&self, other: &SegmentOps) -> SegmentOps {
        let mut merged = self.clone();
        merged.extend(other.iter().map(|(k, v)| (*k, v.clone())));
        merged
    }

    fn and(mut self, other: SegmentOps) -> SegmentOps {
        self.extend(other);
        self
    }

    fn merge_all(sets: &[&SegmentOps]) -> SegmentOps {
        let mut merged = SegmentOps::new();
        for ops in sets {
            merged.extend(ops.iter().map(|(k, v)| (*k, v.clone())));
        }
        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Range;

    fn make_segment(id: usize, duration_cs: u16, frames: usize, is_static: bool) -> Segment {
        Segment {
            id,
            frame_range: Range {
                start: 0,
                end: frames,
            },
            total_duration_cs: duration_cs,
            avg_distance: if is_static { 0.0 } else { 5.0 },
            is_static,
        }
    }

    #[test]
    fn test_filter_longer_than() {
        let segments = [
            make_segment(0, 100, 10, true),
            make_segment(1, 50, 5, true),
            make_segment(2, 200, 20, true),
        ];
        let refs: Vec<_> = segments.iter().collect();
        let selector = SegmentSelector::new(refs);

        let filtered = selector.longer_than(600); // 60cs = 600ms
        assert_eq!(filtered.count(), 2);
    }

    #[test]
    fn test_cap_operation() {
        let segments = [
            make_segment(0, 100, 10, true), // 1000ms
            make_segment(1, 50, 5, true),   // 500ms
            make_segment(2, 200, 20, true), // 2000ms
        ];
        let refs: Vec<_> = segments.iter().collect();
        let selector = SegmentSelector::new(refs);

        let ops = selector.cap(800); // Cap to 800ms

        // Only segments 0 and 2 should be capped
        assert_eq!(ops.len(), 2);
        assert!(matches!(
            ops.get(&0),
            Some(SegmentOp::Collapse { delay_cs: 80 })
        ));
        assert!(matches!(
            ops.get(&2),
            Some(SegmentOp::Collapse { delay_cs: 80 })
        ));
    }

    #[test]
    fn test_merge_ops() {
        let mut ops1 = SegmentOps::new();
        ops1.insert(0, SegmentOp::Keep);
        ops1.insert(1, SegmentOp::Remove);

        let mut ops2 = SegmentOps::new();
        ops2.insert(1, SegmentOp::Collapse { delay_cs: 50 }); // Override
        ops2.insert(2, SegmentOp::Remove);

        let merged = ops1.merge(&ops2);

        assert_eq!(merged.len(), 3);
        assert!(matches!(merged.get(&0), Some(SegmentOp::Keep)));
        assert!(matches!(
            merged.get(&1),
            Some(SegmentOp::Collapse { delay_cs: 50 })
        ));
        assert!(matches!(merged.get(&2), Some(SegmentOp::Remove)));
    }
}
