//! Frame hashing trait for duplicate detection.

use image::RgbaImage;
use std::fmt::Debug;
use std::hash::Hash;

/// Trait for computing perceptual hashes of frames.
///
/// Implementations of this trait provide different algorithms for computing
/// perceptual hashes that can be compared to detect similar or duplicate frames.
///
/// # Example
///
/// ```ignore
/// use figif_core::traits::FrameHasher;
/// use figif_core::hashers::DHasher;
///
/// let hasher = DHasher::new();
/// let hash1 = hasher.hash_frame(&frame1);
/// let hash2 = hasher.hash_frame(&frame2);
/// let distance = hasher.distance(&hash1, &hash2);
///
/// if distance < 5 {
///     println!("Frames are likely duplicates");
/// }
/// ```
pub trait FrameHasher: Send + Sync {
    /// The hash type produced by this hasher.
    ///
    /// Must be comparable via Hamming distance or similar metric.
    type Hash: Clone + Debug + Send + Sync + Eq + Hash;

    /// Compute the perceptual hash for a single frame.
    ///
    /// The image is expected to be in RGBA format. Implementations
    /// may internally convert or resize as needed.
    fn hash_frame(&self, image: &RgbaImage) -> Self::Hash;

    /// Compute the distance between two hashes.
    ///
    /// Lower values indicate more similar images:
    /// - 0 = identical
    /// - 1-5 = likely duplicates
    /// - 5-10 = possibly similar
    /// - >10 = different
    ///
    /// The exact thresholds depend on the algorithm and use case.
    fn distance(&self, a: &Self::Hash, b: &Self::Hash) -> u32;

    /// Get the name of this hasher for logging/debugging.
    fn name(&self) -> &'static str;

    /// Suggested threshold for considering frames as duplicates.
    ///
    /// Returns the recommended maximum distance for two frames
    /// to be considered duplicates. Defaults to 5.
    fn suggested_threshold(&self) -> u32 {
        5
    }
}

/// Extension trait for batch hashing operations.
pub trait FrameHasherExt: FrameHasher {
    /// Hash multiple frames, returning a vector of hashes.
    fn hash_frames(&self, images: &[RgbaImage]) -> Vec<Self::Hash> {
        images.iter().map(|img| self.hash_frame(img)).collect()
    }

    /// Check if two frames are duplicates using the suggested threshold.
    fn are_duplicates(&self, a: &RgbaImage, b: &RgbaImage) -> bool {
        let hash_a = self.hash_frame(a);
        let hash_b = self.hash_frame(b);
        self.distance(&hash_a, &hash_b) <= self.suggested_threshold()
    }
}

// Blanket implementation for all FrameHasher types
impl<T: FrameHasher> FrameHasherExt for T {}

#[cfg(feature = "parallel")]
mod parallel {
    use super::*;
    use rayon::prelude::*;

    /// Extension trait for parallel hashing operations.
    pub trait ParallelFrameHasher: FrameHasher {
        /// Hash multiple frames in parallel.
        fn hash_frames_parallel(&self, images: &[RgbaImage]) -> Vec<Self::Hash>
        where
            Self::Hash: Send,
        {
            images.par_iter().map(|img| self.hash_frame(img)).collect()
        }
    }

    // Blanket implementation
    impl<T: FrameHasher> ParallelFrameHasher for T {}
}

#[cfg(feature = "parallel")]
pub use parallel::ParallelFrameHasher;
