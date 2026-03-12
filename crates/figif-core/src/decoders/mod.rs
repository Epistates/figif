//! Built-in GIF decoders.
//!
//! This module provides decoder implementations for loading GIF files:
//!
//! - [`BufferedDecoder`]: Loads all frames into memory (simpler, random access)
//! - [`StreamingDecoder`]: Decodes frames lazily (memory efficient)
//!
//! Both decoders properly handle GIF disposal methods and transparency,
//! producing fully composited RGBA frames.

mod buffered;
mod streaming;

pub use buffered::{BufferedDecoder, BufferedFrameIter};
pub use streaming::{StreamingDecoder, StreamingIterWrapper};
