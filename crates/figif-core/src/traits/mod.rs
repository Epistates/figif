//! Plugin traits for extending figif functionality.
//!
//! This module contains the core traits that define the plugin architecture:
//!
//! - [`FrameHasher`]: Compute perceptual hashes for duplicate detection
//! - [`GifDecoder`]: Decode GIF files into frames
//! - [`GifEncoder`]: Encode frames into GIF format
//! - [`SimilarityMetric`]: Custom similarity comparison
//!
//! Users can implement these traits to provide custom algorithms while
//! using the built-in implementations for common cases.

mod decoder;
mod encoder;
mod hasher;
mod similarity;

pub use decoder::{BufferedGifDecoder, GifDecoder};
pub use encoder::{GifEncoder, GifEncoderExt};
pub use hasher::FrameHasher;
pub use hasher::FrameHasherExt;
pub use similarity::{HashBasedSimilarity, SimilarityMetric};

#[cfg(feature = "parallel")]
pub use hasher::ParallelFrameHasher;
