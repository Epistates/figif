//! `figif optimize` command - optimize GIF with operations.

use crate::output::{OutputContext, format_duration, format_percent, format_size};
use crate::progress;
use color_eyre::eyre::{Result, WrapErr};
use console::style;
use dialoguer::Confirm;
use figif_core::hashers::{BlockHasher, DHasher, PHasher};
use figif_core::prelude::*;
use serde::Serialize;
use std::fs;

use super::{HasherType, OptimizeArgs, Preset};

#[derive(Serialize)]
struct OptimizeOutput {
    input: String,
    output: String,
    original_frames: usize,
    original_duration_ms: u64,
    result_frames: usize,
    result_duration_ms: u64,
    saved_duration_ms: i64,
    saved_percent: f64,
    input_size: u64,
    output_size: u64,
}

pub fn run(args: OptimizeArgs, output: &OutputContext) -> Result<()> {
    let input_path = &args.input;
    let output_path = &args.output;

    // Check input file exists
    if !input_path.exists() {
        return Err(color_eyre::eyre::eyre!(
            "Input file not found: {}",
            input_path.display()
        ));
    }

    // Check output file doesn't exist (unless --yes)
    if output_path.exists() && !args.yes && !args.dry_run {
        if output.json {
            return Err(color_eyre::eyre::eyre!(
                "Output file already exists: {}. Use --yes to overwrite.",
                output_path.display()
            ));
        }

        let confirm = Confirm::new()
            .with_prompt(format!(
                "Output file {} already exists. Overwrite?",
                output_path.display()
            ))
            .default(false)
            .interact()?;

        if !confirm {
            output.status("Aborted.");
            return Ok(());
        }
    }

    // Get input file size
    let input_size = fs::metadata(input_path)
        .wrap_err_with(|| format!("Failed to read file: {}", input_path.display()))?
        .len();

    // Show analyzing spinner
    let spinner = if !output.quiet && !output.json {
        Some(progress::spinner(&format!(
            "Analyzing {}...",
            input_path.display()
        )))
    } else {
        None
    };

    // Run analysis with selected hasher
    let analysis: figif_core::Analysis<img_hash::ImageHash> = match args.hasher {
        HasherType::Dhash => {
            let figif = Figif::new()
                .with_hasher(DHasher::new())
                .similarity_threshold(args.threshold);
            figif.analyze_file(input_path)?
        }
        HasherType::Phash => {
            let figif = Figif::new()
                .with_hasher(PHasher::new())
                .similarity_threshold(args.threshold);
            figif.analyze_file(input_path)?
        }
        HasherType::Blockhash => {
            let figif = Figif::new()
                .with_hasher(BlockHasher::new())
                .similarity_threshold(args.threshold);
            figif.analyze_file(input_path)?
        }
    };

    // Stop spinner
    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    // Build operations based on args
    let ops = build_operations(&args, &analysis)?;

    // Preview the result
    let output_frames = analysis.apply_operations(&ops);
    let result_duration_ms: u64 = output_frames
        .iter()
        .map(|f| f.delay_centiseconds as u64 * 10)
        .sum();
    let saved_duration_ms = analysis.total_duration_ms() as i64 - result_duration_ms as i64;
    let saved_percent = if analysis.total_duration_ms() > 0 {
        saved_duration_ms as f64 / analysis.total_duration_ms() as f64
    } else {
        0.0
    };

    if args.dry_run {
        // Dry run - just show preview
        if output.json {
            let result = OptimizeOutput {
                input: input_path.display().to_string(),
                output: output_path.display().to_string(),
                original_frames: analysis.frame_count(),
                original_duration_ms: analysis.total_duration_ms(),
                result_frames: output_frames.len(),
                result_duration_ms,
                saved_duration_ms,
                saved_percent,
                input_size,
                output_size: 0, // Unknown for dry run
            };
            output.json(&result)?;
        } else {
            output.header("Dry Run Preview");
            println!();
            println!("{}", style("Original:").bold());
            output.info("Frames", analysis.frame_count().to_string());
            output.info("Duration", format_duration(analysis.total_duration_ms()));
            output.info("Size", format_size(input_size));

            println!();
            println!("{}", style("After optimization:").bold());
            output.info("Frames", output_frames.len().to_string());
            output.info("Duration", format_duration(result_duration_ms));

            println!();
            println!("{}", style("Savings:").bold());
            output.info(
                "Duration saved",
                format!(
                    "{} ({})",
                    format_duration(saved_duration_ms.unsigned_abs()),
                    format_percent(saved_percent.abs())
                ),
            );

            println!();
            output.warn("Dry run - no file written. Remove --dry-run to export.");
        }

        return Ok(());
    }

    // Export
    let export_spinner = if !output.quiet && !output.json {
        Some(progress::spinner("Encoding..."))
    } else {
        None
    };

    let encoder = StandardEncoder::new();
    let mut config = EncodeConfig::default();

    if let Some(width) = args.width {
        config = config.with_width(width);
    }
    if let Some(height) = args.height {
        config = config.with_height(height);
    }

    analysis.export_to_file(&encoder, &ops, output_path, &config)?;

    if let Some(pb) = export_spinner {
        pb.finish_and_clear();
    }

    // Get output file size
    let output_size = fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    // Output result
    if output.json {
        let result = OptimizeOutput {
            input: input_path.display().to_string(),
            output: output_path.display().to_string(),
            original_frames: analysis.frame_count(),
            original_duration_ms: analysis.total_duration_ms(),
            result_frames: output_frames.len(),
            result_duration_ms,
            saved_duration_ms,
            saved_percent,
            input_size,
            output_size,
        };
        output.json(&result)?;
    } else {
        output.header(format!("Optimized: {}", output_path.display()));

        println!();
        println!(
            "  {} -> {}",
            style(format!(
                "{} frames, {}",
                analysis.frame_count(),
                format_duration(analysis.total_duration_ms())
            ))
            .dim(),
            style(format!(
                "{} frames, {}",
                output_frames.len(),
                format_duration(result_duration_ms)
            ))
            .green()
            .bold(),
        );

        println!(
            "  {} -> {}",
            style(format_size(input_size)).dim(),
            style(format_size(output_size)).cyan(),
        );

        if saved_duration_ms > 0 {
            println!();
            output.success(format!(
                "Saved {} ({})",
                format_duration(saved_duration_ms as u64),
                format_percent(saved_percent)
            ));
        }
    }

    Ok(())
}

fn build_operations(
    args: &OptimizeArgs,
    analysis: &figif_core::Analysis<img_hash::ImageHash>,
) -> Result<SegmentOps> {
    let mut ops = SegmentOps::new();

    // Apply preset first if specified
    if let Some(preset) = &args.preset {
        match preset {
            Preset::Fast => {
                ops = analysis.pauses().cap(200);
            }
            Preset::Balanced => {
                ops = analysis
                    .pauses()
                    .cap(300)
                    .merge(&analysis.pauses().speed_up(1.5));
            }
            Preset::Aggressive => {
                ops = analysis
                    .pauses()
                    .collapse(100)
                    .merge(&analysis.all().speed_up(1.5));
            }
        }
    }

    // Apply individual operations (override preset)
    if let Some(cap_ms) = args.cap_pauses {
        ops = ops.merge(&analysis.pauses().cap(cap_ms));
    }

    if let Some(collapse_ms) = args.collapse_pauses {
        ops = ops.merge(&analysis.pauses().collapse(collapse_ms));
    }

    if let Some(threshold_ms) = args.remove_long {
        ops = ops.merge(&analysis.pauses().longer_than(threshold_ms).remove());
    }

    if let Some(factor) = args.speed_up_pauses {
        ops = ops.merge(&analysis.pauses().speed_up(factor));
    }

    if let Some(factor) = args.speed_up_all {
        ops = ops.merge(&analysis.all().speed_up(factor));
    }

    if let Some(target_ms) = args.target_duration {
        if let Some(target_ops) = analysis.target_duration(target_ms) {
            ops = ops.merge(&target_ops);
        } else {
            return Err(color_eyre::eyre::eyre!(
                "Cannot achieve target duration of {}ms - non-static content is too long",
                target_ms
            ));
        }
    }

    Ok(ops)
}
