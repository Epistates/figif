//! Error types for figif-core.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using [`FigifError`].
pub type Result<T> = std::result::Result<T, FigifError>;

/// Errors that can occur during GIF processing.
#[derive(Debug, Error)]
pub enum FigifError {
    /// Failed to read or open a file.
    #[error("failed to read file: {path}")]
    FileRead {
        /// Path to the file that could not be read.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to write a file.
    #[error("failed to write file: {path}")]
    FileWrite {
        /// Path to the file that could not be written.
        path: PathBuf,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// GIF decoding failed.
    #[error("failed to decode GIF: {reason}")]
    DecodeError {
        /// Description of the decoding failure.
        reason: String,
    },

    /// GIF encoding failed.
    #[error("failed to encode GIF: {reason}")]
    EncodeError {
        /// Description of the encoding failure.
        reason: String,
    },

    /// Invalid frame index.
    #[error("invalid frame index: {index} (total frames: {total})")]
    InvalidFrameIndex {
        /// The requested frame index.
        index: usize,
        /// Total number of frames available.
        total: usize,
    },

    /// Invalid segment ID.
    #[error("invalid segment ID: {id} (total segments: {total})")]
    InvalidSegmentId {
        /// The requested segment ID.
        id: usize,
        /// Total number of segments available.
        total: usize,
    },

    /// Invalid configuration parameter.
    #[error("invalid configuration: {message}")]
    InvalidConfig {
        /// Description of the configuration error.
        message: String,
    },

    /// Frame dimensions mismatch.
    #[error(
        "frame dimension mismatch: expected {expected_width}x{expected_height}, got {actual_width}x{actual_height}"
    )]
    DimensionMismatch {
        /// Expected frame width.
        expected_width: u32,
        /// Expected frame height.
        expected_height: u32,
        /// Actual frame width encountered.
        actual_width: u32,
        /// Actual frame height encountered.
        actual_height: u32,
    },

    /// No frames in the GIF.
    #[error("GIF contains no frames")]
    NoFrames,

    /// Image processing error.
    #[error("image processing error: {reason}")]
    ImageError {
        /// Description of the image processing failure.
        reason: String,
    },

    /// Hashing error.
    #[error("hashing error: {reason}")]
    HashError {
        /// Description of the hashing failure.
        reason: String,
    },

    /// The GIF data is empty or invalid.
    #[error("empty or invalid GIF data")]
    EmptyData,
}

impl From<gif::DecodingError> for FigifError {
    fn from(err: gif::DecodingError) -> Self {
        FigifError::DecodeError {
            reason: err.to_string(),
        }
    }
}

impl From<image::ImageError> for FigifError {
    fn from(err: image::ImageError) -> Self {
        FigifError::ImageError {
            reason: err.to_string(),
        }
    }
}
