//! High-quality lossy GIF encoder using gifski.
//!
//! This module is only available when the `lossy` feature is enabled.

#[cfg(feature = "lossy")]
mod inner {
    use crate::error::{FigifError, Result};
    use crate::traits::GifEncoder;
    use crate::types::{EncodableFrame, EncodeConfig, LoopCount};
    use gifski::Settings;
    use image::imageops::FilterType;
    use imgref::ImgVec;
    use rgb::RGBA8;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use std::thread;

    /// High-quality lossy GIF encoder using gifski.
    ///
    /// gifski produces much smaller files than standard GIF encoding
    /// by using lossy LZW compression while maintaining excellent quality.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use figif_core::encoders::GifskiEncoder;
    /// use figif_core::traits::GifEncoder;
    ///
    /// let encoder = GifskiEncoder::with_quality(85);
    /// let bytes = encoder.encode(&frames, &EncodeConfig::default())?;
    /// ```
    #[derive(Debug, Clone)]
    pub struct GifskiEncoder {
        /// Quality setting (1-100, higher = better quality, larger file).
        quality: u8,
        /// Motion quality (1-100, affects temporal smoothing).
        motion_quality: u8,
        /// Whether to use fast encoding mode.
        fast: bool,
    }

    impl Default for GifskiEncoder {
        fn default() -> Self {
            Self {
                quality: 90,
                motion_quality: 90,
                fast: false,
            }
        }
    }

    impl GifskiEncoder {
        /// Create a new gifski encoder with default settings.
        pub fn new() -> Self {
            Self::default()
        }

        /// Create a gifski encoder with the specified quality.
        ///
        /// Quality ranges from 1 (worst) to 100 (best).
        /// Values around 80-90 provide a good balance.
        pub fn with_quality(quality: u8) -> Self {
            Self {
                quality: quality.clamp(1, 100),
                ..Self::default()
            }
        }

        /// Set the motion quality.
        ///
        /// Lower values cause more smearing/banding in motion
        /// but can reduce file size significantly.
        pub fn motion_quality(mut self, quality: u8) -> Self {
            self.motion_quality = quality.clamp(1, 100);
            self
        }

        /// Enable fast encoding mode.
        ///
        /// Faster but may produce slightly larger files.
        pub fn fast(mut self, fast: bool) -> Self {
            self.fast = fast;
            self
        }

        /// Encode frames using gifski.
        fn encode_with_gifski(
            &self,
            frames: &[EncodableFrame],
            config: &EncodeConfig,
        ) -> Result<Vec<u8>> {
            if frames.is_empty() {
                return Err(FigifError::NoFrames);
            }

            // Determine output dimensions
            let first_frame = &frames[0];
            let (src_width, src_height) = first_frame.image.dimensions();

            let (out_width, out_height) = match (config.width, config.height) {
                (Some(w), Some(h)) => (w as u32, h as u32),
                (Some(w), None) => {
                    let ratio = w as f64 / src_width as f64;
                    (w as u32, (src_height as f64 * ratio).round() as u32)
                }
                (None, Some(h)) => {
                    let ratio = h as f64 / src_height as f64;
                    ((src_width as f64 * ratio).round() as u32, h as u32)
                }
                (None, None) => (src_width, src_height),
            };

            // Use lossy quality from config if provided, otherwise use encoder's quality
            let quality = config.lossy_quality.unwrap_or(self.quality);

            // Create gifski settings
            let settings = Settings {
                width: Some(out_width),
                height: Some(out_height),
                quality,
                fast: self.fast,
                repeat: match config.loop_count {
                    LoopCount::Infinite => gifski::Repeat::Infinite,
                    LoopCount::Once => gifski::Repeat::Finite(0),
                    LoopCount::Finite(n) => gifski::Repeat::Finite(n),
                },
            };

            let (collector, writer) =
                gifski::new(settings).map_err(|e| FigifError::EncodeError {
                    reason: e.to_string(),
                })?;

            // Prepare frames
            let needs_resize = out_width != src_width || out_height != src_height;
            let prepared_frames: Vec<_> = frames
                .iter()
                .enumerate()
                .map(|(i, f)| {
                    let image = if needs_resize {
                        image::imageops::resize(
                            &f.image,
                            out_width,
                            out_height,
                            FilterType::Lanczos3,
                        )
                    } else {
                        f.image.clone()
                    };
                    (i, image, f.delay_centiseconds)
                })
                .collect();

            // Output buffer
            let output = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
            let output_clone = output.clone();

            // Writer thread
            let writer_handle = thread::spawn(move || -> Result<()> {
                let mut buffer = output_clone.lock().unwrap();
                writer
                    .write(&mut *buffer, &mut gifski::progress::NoProgress {})
                    .map_err(|e| FigifError::EncodeError {
                        reason: e.to_string(),
                    })
            });

            // Add frames to collector
            let mut timestamp = 0.0;
            for (index, image, delay_cs) in prepared_frames {
                // Convert RGBA image to the format gifski expects
                let rgba_data: Vec<u8> = image.as_raw().to_vec();

                collector
                    .add_frame_rgba(
                        index,
                        ImgVec::new(
                            rgba_data
                                .chunks(4)
                                .map(|c| RGBA8::new(c[0], c[1], c[2], c[3]))
                                .collect(),
                            out_width as usize,
                            out_height as usize,
                        ),
                        timestamp,
                    )
                    .map_err(|e| FigifError::EncodeError {
                        reason: e.to_string(),
                    })?;

                // Convert centiseconds to seconds
                timestamp += delay_cs as f64 / 100.0;
            }

            // Signal completion
            drop(collector);

            // Wait for writer
            writer_handle.join().map_err(|_| FigifError::EncodeError {
                reason: "writer thread panicked".to_string(),
            })??;

            // Extract result
            let result = std::sync::Arc::try_unwrap(output)
                .map_err(|_| FigifError::EncodeError {
                    reason: "failed to unwrap output".to_string(),
                })?
                .into_inner()
                .unwrap();

            Ok(result)
        }
    }

    impl GifEncoder for GifskiEncoder {
        fn encode(&self, frames: &[EncodableFrame], config: &EncodeConfig) -> Result<Vec<u8>> {
            self.encode_with_gifski(frames, config)
        }

        fn encode_to_file(
            &self,
            frames: &[EncodableFrame],
            path: impl AsRef<Path>,
            config: &EncodeConfig,
        ) -> Result<()> {
            let path = path.as_ref();
            let bytes = self.encode(frames, config)?;
            let mut file = File::create(path).map_err(|e| FigifError::FileWrite {
                path: path.to_path_buf(),
                source: e,
            })?;
            file.write_all(&bytes).map_err(|e| FigifError::FileWrite {
                path: path.to_path_buf(),
                source: e,
            })?;
            Ok(())
        }

        fn encode_to_writer<W: Write>(
            &self,
            frames: &[EncodableFrame],
            mut writer: W,
            config: &EncodeConfig,
        ) -> Result<()> {
            let bytes = self.encode(frames, config)?;
            writer
                .write_all(&bytes)
                .map_err(|e| FigifError::EncodeError {
                    reason: format!("failed to write to output: {}", e),
                })?;
            Ok(())
        }

        fn supports_lossy(&self) -> bool {
            true
        }

        fn name(&self) -> &'static str {
            "gifski"
        }
    }
}

#[cfg(feature = "lossy")]
pub use inner::GifskiEncoder;
