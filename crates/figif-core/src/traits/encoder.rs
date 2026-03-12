//! GIF encoder trait for writing GIF files.

use crate::error::Result;
use crate::types::{EncodableFrame, EncodeConfig};
use std::io::Write;
use std::path::Path;

/// Trait for encoding frames into GIF format.
///
/// Implementations can provide different encoding strategies:
/// - Standard lossless GIF encoding
/// - Lossy encoding for smaller file sizes
/// - High-quality encoding with dithering
///
/// # Example
///
/// ```ignore
/// use figif_core::traits::GifEncoder;
/// use figif_core::encoders::StandardEncoder;
///
/// let encoder = StandardEncoder::new();
/// let bytes = encoder.encode(&frames, &EncodeConfig::default())?;
/// ```
pub trait GifEncoder: Send + Sync {
    /// Encode frames to a byte vector.
    fn encode(&self, frames: &[EncodableFrame], config: &EncodeConfig) -> Result<Vec<u8>>;

    /// Encode frames to a file.
    fn encode_to_file(
        &self,
        frames: &[EncodableFrame],
        path: impl AsRef<Path>,
        config: &EncodeConfig,
    ) -> Result<()>;

    /// Encode frames to any writer.
    fn encode_to_writer<W: Write>(
        &self,
        frames: &[EncodableFrame],
        writer: W,
        config: &EncodeConfig,
    ) -> Result<()>;

    /// Whether this encoder supports lossy compression.
    fn supports_lossy(&self) -> bool;

    /// Get the name of this encoder for logging/debugging.
    fn name(&self) -> &'static str;
}

/// Extension trait for encoder utilities.
pub trait GifEncoderExt: GifEncoder {
    /// Encode with default configuration.
    fn encode_default(&self, frames: &[EncodableFrame]) -> Result<Vec<u8>> {
        self.encode(frames, &EncodeConfig::default())
    }

    /// Check if this encoder can handle the given configuration.
    fn can_encode(&self, config: &EncodeConfig) -> bool {
        // If lossy quality is requested, encoder must support it
        if config.lossy_quality.is_some() && !self.supports_lossy() {
            return false;
        }
        true
    }
}

// Blanket implementation
impl<T: GifEncoder> GifEncoderExt for T {}
