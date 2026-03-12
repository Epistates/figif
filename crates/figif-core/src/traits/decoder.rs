//! GIF decoder trait for loading GIF files.

use crate::error::Result;
use crate::types::{DecodedFrame, GifMetadata};
use std::io::Read;
use std::path::Path;

/// Trait for decoding GIF files into frames.
///
/// Implementations can choose different strategies:
/// - Buffered: Load all frames into memory at once
/// - Streaming: Decode frames lazily on-demand
///
/// # Example
///
/// ```ignore
/// use figif_core::traits::GifDecoder;
/// use figif_core::decoders::BufferedDecoder;
///
/// let decoder = BufferedDecoder::new();
/// let frames = decoder.decode_file("animation.gif")?;
/// ```
pub trait GifDecoder: Send + Sync {
    /// The type of iterator returned by decode operations.
    type FrameIter: Iterator<Item = Result<DecodedFrame>>;

    /// Decode a GIF from a file path.
    ///
    /// Returns an iterator over decoded frames.
    fn decode_file(&self, path: impl AsRef<Path>) -> Result<Self::FrameIter>;

    /// Decode a GIF from a byte slice.
    ///
    /// Returns an iterator over decoded frames.
    fn decode_bytes(&self, data: &[u8]) -> Result<Self::FrameIter>;

    /// Decode a GIF from any reader.
    ///
    /// Returns an iterator over decoded frames.
    fn decode_reader<R: Read + Send>(&self, reader: R) -> Result<Self::FrameIter>;

    /// Extract metadata without fully decoding all frames.
    ///
    /// This is useful for getting dimensions, frame count, and duration
    /// without the overhead of decoding all pixel data.
    fn metadata_from_bytes(&self, data: &[u8]) -> Result<GifMetadata>;

    /// Extract metadata from a file.
    fn metadata_from_file(&self, path: impl AsRef<Path>) -> Result<GifMetadata>;

    /// Get the name of this decoder for logging/debugging.
    fn name(&self) -> &'static str;
}

/// A decoder that collects all frames into a vector.
///
/// This is useful when you need random access to frames
/// or when the streaming approach is not suitable.
pub trait BufferedGifDecoder: GifDecoder {
    /// Decode all frames into a vector.
    fn decode_all(&self, data: &[u8]) -> Result<Vec<DecodedFrame>> {
        self.decode_bytes(data)?.collect()
    }

    /// Decode all frames from a file into a vector.
    fn decode_all_from_file(&self, path: impl AsRef<Path>) -> Result<Vec<DecodedFrame>> {
        self.decode_file(path)?.collect()
    }
}
