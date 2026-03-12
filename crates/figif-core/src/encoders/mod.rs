//! Built-in GIF encoders.
//!
//! This module provides encoder implementations for writing GIF files:
//!
//! - [`StandardEncoder`]: Lossless GIF encoding using the `gif` crate
//! - [`GifskiEncoder`]: High-quality lossy encoding (requires `lossy` feature)
//!
//! # Choosing an Encoder
//!
//! | Encoder | File Size | Quality | Speed |
//! |---------|-----------|---------|-------|
//! | StandardEncoder | Larger | Lossless | Fast |
//! | GifskiEncoder | Smaller | Excellent (lossy) | Slower |
//!
//! For most use cases, GifskiEncoder produces significantly smaller files
//! with minimal quality loss.

mod standard;

pub use standard::{ResizeFilter, StandardEncoder};

#[cfg(feature = "lossy")]
mod gifski;

#[cfg(feature = "lossy")]
pub use self::gifski::GifskiEncoder;
