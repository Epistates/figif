//! Difference hash (dHash) implementation.

use crate::traits::FrameHasher;
use image::RgbaImage;
use img_hash::{HashAlg, HasherConfig, ImageHash};

/// Difference hash (dHash) for fast duplicate detection.
///
/// dHash computes a gradient-based hash by comparing adjacent pixels.
/// It's very fast and works well for detecting near-duplicate frames.
///
/// # Algorithm
///
/// 1. Resize image to (hash_width + 1, hash_height)
/// 2. Convert to grayscale
/// 3. Compare each pixel to its right neighbor
/// 4. Set bit to 1 if left > right, else 0
///
/// # Example
///
/// ```ignore
/// use figif_core::hashers::DHasher;
/// use figif_core::traits::FrameHasher;
///
/// let hasher = DHasher::new();
/// let hash = hasher.hash_frame(&image);
/// ```
#[derive(Debug, Clone)]
pub struct DHasher {
    hash_width: u32,
    hash_height: u32,
}

impl Default for DHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl DHasher {
    /// Create a new dHash hasher with default 8x8 hash size.
    pub fn new() -> Self {
        Self::with_size(8, 8)
    }

    /// Create a dHash hasher with custom hash dimensions.
    ///
    /// Larger sizes provide more precision but slower comparison.
    /// Common sizes: 8x8 (64-bit), 16x16 (256-bit).
    pub fn with_size(width: u32, height: u32) -> Self {
        Self {
            hash_width: width,
            hash_height: height,
        }
    }

    /// Get the hash width.
    pub fn hash_width(&self) -> u32 {
        self.hash_width
    }

    /// Get the hash height.
    pub fn hash_height(&self) -> u32 {
        self.hash_height
    }

    /// Get the total hash bits.
    pub fn hash_bits(&self) -> u32 {
        self.hash_width * self.hash_height
    }

    fn build_hasher(&self) -> img_hash::Hasher {
        HasherConfig::new()
            .hash_alg(HashAlg::Gradient)
            .hash_size(self.hash_width, self.hash_height)
            .to_hasher()
    }
}

impl FrameHasher for DHasher {
    type Hash = ImageHash;

    fn hash_frame(&self, image: &RgbaImage) -> Self::Hash {
        let hasher = self.build_hasher();
        // Convert to img_hash's image type via raw pixel conversion
        let (width, height) = image.dimensions();
        let raw = image.as_raw().clone();
        let img_hash_image: img_hash::image::RgbaImage =
            img_hash::image::ImageBuffer::from_raw(width, height, raw)
                .expect("image dimensions should match");
        let dynamic = img_hash::image::DynamicImage::ImageRgba8(img_hash_image);
        hasher.hash_image(&dynamic)
    }

    fn distance(&self, a: &Self::Hash, b: &Self::Hash) -> u32 {
        a.dist(b)
    }

    fn name(&self) -> &'static str {
        "dhash"
    }

    fn suggested_threshold(&self) -> u32 {
        // For 8x8 (64-bit) hash, 5 is a good threshold
        // Scale proportionally for other sizes
        let base_bits = 64;
        let actual_bits = self.hash_bits();
        (5 * actual_bits / base_bits).max(3)
    }
}
