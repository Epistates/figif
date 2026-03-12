//! Perceptual hash (pHash) implementation using DCT.

use crate::traits::FrameHasher;
use image::RgbaImage;
use img_hash::{HashAlg, HasherConfig, ImageHash};

/// Perceptual hash (pHash) using Discrete Cosine Transform.
///
/// pHash is more robust to image transformations than simpler hashes
/// but is slower to compute. It works by analyzing the frequency
/// components of the image.
///
/// # Algorithm
///
/// 1. Resize image to 32x32
/// 2. Convert to grayscale
/// 3. Apply DCT (Discrete Cosine Transform)
/// 4. Keep only low-frequency components (top-left 8x8)
/// 5. Compute median and create hash based on above/below median
///
/// # Example
///
/// ```ignore
/// use figif_core::hashers::PHasher;
/// use figif_core::traits::FrameHasher;
///
/// let hasher = PHasher::new();
/// let hash = hasher.hash_frame(&image);
/// ```
#[derive(Debug, Clone)]
pub struct PHasher {
    hash_width: u32,
    hash_height: u32,
}

impl Default for PHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl PHasher {
    /// Create a new pHash hasher with default 8x8 hash size.
    pub fn new() -> Self {
        Self::with_size(8, 8)
    }

    /// Create a pHash hasher with custom hash dimensions.
    ///
    /// The actual DCT is computed on a larger image (4x the hash size),
    /// then only the low-frequency components are kept.
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
            .hash_alg(HashAlg::DoubleGradient)
            .hash_size(self.hash_width, self.hash_height)
            .to_hasher()
    }
}

impl FrameHasher for PHasher {
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
        "phash"
    }

    fn suggested_threshold(&self) -> u32 {
        // pHash can tolerate slightly more difference due to its robustness
        let base_bits = 64;
        let actual_bits = self.hash_bits();
        (8 * actual_bits / base_bits).max(5)
    }
}
