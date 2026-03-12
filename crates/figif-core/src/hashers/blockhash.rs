//! Block hash implementation.

use crate::traits::FrameHasher;
use image::RgbaImage;
use img_hash::{HashAlg, HasherConfig, ImageHash};

/// Block hash for perceptual image comparison.
///
/// Block hash divides the image into blocks and computes a hash
/// based on the average brightness of each block compared to
/// the overall image average.
///
/// # Algorithm
///
/// 1. Divide image into NxN blocks
/// 2. Compute average brightness for each block
/// 3. Compute overall image average
/// 4. Set bit to 1 if block average > image average, else 0
///
/// # Example
///
/// ```ignore
/// use figif_core::hashers::BlockHasher;
/// use figif_core::traits::FrameHasher;
///
/// let hasher = BlockHasher::new();
/// let hash = hasher.hash_frame(&image);
/// ```
#[derive(Debug, Clone)]
pub struct BlockHasher {
    hash_width: u32,
    hash_height: u32,
}

impl Default for BlockHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockHasher {
    /// Create a new block hasher with default 8x8 hash size.
    pub fn new() -> Self {
        Self::with_size(8, 8)
    }

    /// Create a block hasher with custom hash dimensions.
    ///
    /// The hash size determines the number of blocks (width * height bits).
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
            .hash_alg(HashAlg::Blockhash)
            .hash_size(self.hash_width, self.hash_height)
            .to_hasher()
    }
}

impl FrameHasher for BlockHasher {
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
        "blockhash"
    }

    fn suggested_threshold(&self) -> u32 {
        // Block hash is fairly tolerant
        let base_bits = 64;
        let actual_bits = self.hash_bits();
        (6 * actual_bits / base_bits).max(4)
    }
}
