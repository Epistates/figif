//! Similarity metric trait for custom frame comparison.

use image::RgbaImage;

/// Trait for computing similarity between frames.
///
/// This provides an alternative to hash-based comparison for cases
/// where more precise similarity measurement is needed.
///
/// # Example
///
/// ```ignore
/// use figif_core::traits::SimilarityMetric;
///
/// struct MSEMetric;
///
/// impl SimilarityMetric for MSEMetric {
///     fn similarity(&self, a: &RgbaImage, b: &RgbaImage) -> f64 {
///         // Compute mean squared error and convert to similarity
///         let mse = compute_mse(a, b);
///         1.0 / (1.0 + mse)
///     }
///
///     fn duplicate_threshold(&self) -> f64 {
///         0.95 // 95% similarity = duplicate
///     }
/// }
/// ```
pub trait SimilarityMetric: Send + Sync {
    /// Compute similarity between two frames.
    ///
    /// Returns a value between 0.0 and 1.0:
    /// - 1.0 = identical
    /// - 0.0 = completely different
    fn similarity(&self, a: &RgbaImage, b: &RgbaImage) -> f64;

    /// The threshold above which frames are considered duplicates.
    ///
    /// Frames with `similarity >= duplicate_threshold()` are considered duplicates.
    fn duplicate_threshold(&self) -> f64;

    /// Get the name of this metric for logging/debugging.
    fn name(&self) -> &'static str;

    /// Check if two frames are duplicates.
    fn are_duplicates(&self, a: &RgbaImage, b: &RgbaImage) -> bool {
        self.similarity(a, b) >= self.duplicate_threshold()
    }
}

/// A similarity metric based on a hash distance threshold.
///
/// This adapter allows using a `FrameHasher` as a `SimilarityMetric`.
pub struct HashBasedSimilarity<H> {
    hasher: H,
    max_distance: u32,
}

impl<H> HashBasedSimilarity<H> {
    /// Create a new hash-based similarity metric.
    ///
    /// `max_distance` is the maximum hash distance that maps to similarity > 0.
    pub fn new(hasher: H, max_distance: u32) -> Self {
        Self {
            hasher,
            max_distance,
        }
    }
}

impl<H> SimilarityMetric for HashBasedSimilarity<H>
where
    H: crate::traits::FrameHasher,
{
    fn similarity(&self, a: &RgbaImage, b: &RgbaImage) -> f64 {
        let hash_a = self.hasher.hash_frame(a);
        let hash_b = self.hasher.hash_frame(b);
        let distance = self.hasher.distance(&hash_a, &hash_b);

        // Convert distance to similarity (0 distance = 1.0 similarity)
        if distance >= self.max_distance {
            0.0
        } else {
            1.0 - (distance as f64 / self.max_distance as f64)
        }
    }

    fn duplicate_threshold(&self) -> f64 {
        // Map suggested threshold to similarity
        let threshold = self.hasher.suggested_threshold();
        1.0 - (threshold as f64 / self.max_distance as f64)
    }

    fn name(&self) -> &'static str {
        "hash-based"
    }
}
