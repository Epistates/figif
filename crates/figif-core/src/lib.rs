//! # figif-core
//!
//! A Rust library for GIF frame analysis and manipulation with a plugin architecture.
//!
//! figif-core provides tools for:
//! - **Duplicate Detection**: Identify similar/duplicate frames using perceptual hashing
//! - **Segment Analysis**: Group consecutive similar frames into logical segments
//! - **Timing Control**: Modify frame delays to speed up, slow down, or collapse segments
//! - **Export**: Re-encode GIFs with applied modifications
//!
//! ## Quick Start
//!
//! ```ignore
//! use figif_core::prelude::*;
//!
//! // Create analyzer with default settings (uses dHash)
//! let figif = Figif::new();
//!
//! // Analyze a GIF file
//! let analysis = figif.analyze_file("demo.gif")?;
//!
//! // Inspect detected segments
//! for segment in &analysis.segments {
//!     println!(
//!         "Segment {}: {} frames, {}ms, static={}",
//!         segment.id,
//!         segment.frame_count(),
//!         segment.duration_ms(),
//!         segment.is_static
//!     );
//! }
//!
//! // Cap all pause segments to max 300ms
//! let ops = analysis.pauses().cap(300);
//!
//! // Export with a lossless encoder
//! let encoder = StandardEncoder::new();
//! analysis.export_to_file(&encoder, &ops, "output.gif", &EncodeConfig::default())?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Fluent Selector API
//!
//! The selector API provides chainable, ergonomic control over segment operations:
//!
//! ```ignore
//! use figif_core::prelude::*;
//!
//! let analysis = Figif::new().analyze_file("demo.gif")?;
//!
//! // Cap all pauses to 300ms
//! let ops = analysis.pauses().cap(300);
//!
//! // Collapse only long pauses (> 500ms) to 200ms
//! let ops = analysis.pauses().longer_than(500).collapse(200);
//!
//! // Remove pauses longer than 1 second
//! let ops = analysis.pauses().longer_than(1000).remove();
//!
//! // Speed up motion segments by 1.5x
//! let ops = analysis.motion().speed_up(1.5);
//!
//! // Combine operations: cap long pauses AND speed up motion
//! let ops = analysis.pauses().longer_than(500).cap(300)
//!     .merge(&analysis.motion().speed_up(1.2));
//!
//! // Get statistics
//! println!("Pause count: {}", analysis.pauses().count());
//! println!("Pause duration: {}ms", analysis.pauses().total_duration_ms());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Using Different Hashers
//!
//! ```ignore
//! use figif_core::{Figif, hashers::{PHasher, BlockHasher}};
//!
//! // Use pHash for more robust matching
//! let figif = Figif::new()
//!     .with_hasher(PHasher::new())
//!     .similarity_threshold(8);
//!
//! // Or use BlockHash
//! let figif = Figif::new()
//!     .with_hasher(BlockHasher::with_size(16, 16));
//! ```
//!
//! ## Lossy Encoding (requires `lossy` feature)
//!
//! ```ignore
//! #[cfg(feature = "lossy")]
//! {
//!     use figif_core::encoders::GifskiEncoder;
//!     use figif_core::EncodeConfig;
//!
//!     let encoder = GifskiEncoder::with_quality(85);
//!     let config = EncodeConfig::new()
//!         .with_width(480)
//!         .with_lossy_quality(80);
//!
//!     let bytes = analysis.export(&encoder, &ops, &config)?;
//! }
//! ```
//!
//! ## Feature Flags
//!
//! - `parallel` (default): Enable parallel frame hashing using rayon
//! - `lossy`: Enable GifskiEncoder for high-quality lossy compression
//!
//! ## Plugin Architecture
//!
//! figif-core is designed to be extensible. You can implement custom:
//!
//! - **Hashers**: Implement [`traits::FrameHasher`] for custom duplicate detection
//! - **Decoders**: Implement [`traits::GifDecoder`] for custom loading
//! - **Encoders**: Implement [`traits::GifEncoder`] for custom output formats
//! - **Similarity Metrics**: Implement [`traits::SimilarityMetric`] for custom comparison

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod analysis;
pub mod decoders;
pub mod encoders;
pub mod error;
pub mod hashers;
pub mod pipeline;
pub mod segment;
pub mod selector;
pub mod traits;
pub mod types;

// Re-export main types at crate root for convenience
pub use error::{FigifError, Result};
pub use pipeline::{Analysis, Figif, ProgressCallback};
pub use selector::{SegmentOpsExt, SegmentSelector};
pub use types::{
    AnalyzedFrame, DecodedFrame, DisposalMethod, EncodableFrame, EncodeConfig, FrameOp, FrameOps,
    GifMetadata, LoopCount, Segment, SegmentOp, SegmentOps,
};

/// Prelude module for convenient imports.
///
/// ```ignore
/// use figif_core::prelude::*;
/// ```
pub mod prelude {
    pub use crate::decoders::{BufferedDecoder, StreamingDecoder};
    pub use crate::encoders::StandardEncoder;
    pub use crate::error::{FigifError, Result};
    pub use crate::hashers::{BlockHasher, DHasher, PHasher};
    pub use crate::pipeline::{Analysis, Figif};
    pub use crate::selector::{SegmentOpsExt, SegmentSelector};
    pub use crate::traits::{FrameHasher, GifDecoder, GifEncoder};
    pub use crate::types::{
        EncodeConfig, FrameOp, FrameOps, LoopCount, Segment, SegmentOp, SegmentOps,
    };

    #[cfg(feature = "lossy")]
    pub use crate::encoders::GifskiEncoder;

    #[cfg(feature = "parallel")]
    pub use crate::traits::ParallelFrameHasher;
}
