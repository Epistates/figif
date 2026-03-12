//! `figif info` command - show GIF metadata without full analysis.

use crate::output::{OutputContext, format_duration, format_size};
use color_eyre::eyre::{Result, WrapErr};
use figif_core::prelude::*;
use serde::Serialize;
use std::fs;

use super::InfoArgs;

#[derive(Serialize)]
struct InfoOutput {
    file: String,
    width: u16,
    height: u16,
    frames: usize,
    duration_ms: u64,
    loop_count: String,
    file_size: u64,
    has_transparency: bool,
}

pub fn run(args: InfoArgs, output: &OutputContext) -> Result<()> {
    let path = &args.input;

    // Check file exists
    if !path.exists() {
        return Err(color_eyre::eyre::eyre!(
            "File not found: {}",
            path.display()
        ));
    }

    // Get file size
    let file_size = fs::metadata(path)
        .wrap_err_with(|| format!("Failed to read file: {}", path.display()))?
        .len();

    // Read GIF metadata
    let decoder = BufferedDecoder::new();
    let data = fs::read(path).wrap_err("Failed to read GIF file")?;
    let metadata = decoder
        .metadata_from_bytes(&data)
        .wrap_err("Failed to parse GIF metadata")?;

    // Format loop count
    let loop_count = match metadata.loop_count {
        figif_core::LoopCount::Infinite => "Infinite".to_string(),
        figif_core::LoopCount::Finite(n) => format!("{}", n),
        figif_core::LoopCount::Once => "Once (no loop)".to_string(),
    };

    if output.json {
        let info = InfoOutput {
            file: path.display().to_string(),
            width: metadata.width,
            height: metadata.height,
            frames: metadata.frame_count,
            duration_ms: metadata.total_duration_ms,
            loop_count,
            file_size,
            has_transparency: metadata.has_transparency,
        };
        output.json(&info)?;
    } else {
        output.header(format!("GIF Info: {}", path.display()));
        output.info(
            "Dimensions",
            format!("{}x{}", metadata.width, metadata.height),
        );
        output.info("Frames", metadata.frame_count.to_string());
        output.info("Duration", format_duration(metadata.total_duration_ms));
        output.info("Loop count", &loop_count);
        output.info("File size", format_size(file_size));
        output.info(
            "Transparency",
            if metadata.has_transparency {
                "Yes"
            } else {
                "No"
            },
        );
    }

    Ok(())
}
