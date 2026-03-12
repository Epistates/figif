//! Built-in perceptual hash implementations.
//!
//! This module provides various hash algorithms for duplicate frame detection:
//!
//! - [`DHasher`]: Difference hash - fast, good for near-duplicates
//! - [`PHasher`]: Perceptual hash (DCT) - robust to transformations
//! - [`BlockHasher`]: Block average hash - balanced approach
//!
//! All hashers implement the [`FrameHasher`](crate::traits::FrameHasher) trait
//! and can be used interchangeably.
//!
//! # Choosing a Hasher
//!
//! | Algorithm | Speed | Robustness | Best For |
//! |-----------|-------|------------|----------|
//! | dHash | Fast | Moderate | Frame-to-frame comparison |
//! | pHash | Slow | High | Content matching across transforms |
//! | BlockHash | Medium | Medium | General purpose |
//!
//! For GIF frame analysis, **dHash** is recommended as the default due to
//! its speed and effectiveness for detecting consecutive duplicate frames.

mod blockhash;
mod dhash;
mod phash;

pub use blockhash::BlockHasher;
pub use dhash::DHasher;
pub use phash::PHasher;

/// Re-export the ImageHash type from img_hash for convenience.
pub use img_hash::ImageHash;
