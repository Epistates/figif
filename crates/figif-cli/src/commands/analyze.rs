//! `figif analyze` command - analyze GIF and show segment breakdown.

use crate::output::{OutputContext, SegmentStyles, format_duration, format_percent, format_size};
use crate::progress;
use color_eyre::eyre::{Result, WrapErr};
use console::style;
use figif_core::hashers::{BlockHasher, DHasher, PHasher};
use figif_core::prelude::*;
use serde::Serialize;
use std::fs;
use std::io::Write;

use super::{AnalyzeArgs, HasherType};

#[derive(Serialize)]
struct AnalysisOutput {
    file: String,
    metadata: MetadataOutput,
    segments: Vec<SegmentOutput>,
    summary: SummaryOutput,
}

#[derive(Serialize)]
struct MetadataOutput {
    width: u16,
    height: u16,
    frames: usize,
    duration_ms: u64,
    file_size: u64,
}

#[derive(Serialize)]
struct SegmentOutput {
    id: usize,
    segment_type: String,
    frame_start: usize,
    frame_end: usize,
    frame_count: usize,
    duration_ms: u32,
}

#[derive(Serialize)]
struct SummaryOutput {
    total_segments: usize,
    static_segments: usize,
    motion_segments: usize,
    static_duration_ms: u64,
    motion_duration_ms: u64,
    static_percent: f64,
    motion_percent: f64,
}

pub fn run(args: AnalyzeArgs, output: &OutputContext) -> Result<()> {
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

    // Show analyzing spinner
    let spinner = if !output.quiet && !output.json {
        Some(progress::spinner(&format!(
            "Analyzing {}...",
            path.display()
        )))
    } else {
        None
    };

    // Run analysis with selected hasher
    let analysis = match args.hasher {
        HasherType::Dhash => {
            let figif = Figif::new()
                .with_hasher(DHasher::new())
                .similarity_threshold(args.threshold);
            figif.analyze_file(path)?
        }
        HasherType::Phash => {
            let figif = Figif::new()
                .with_hasher(PHasher::new())
                .similarity_threshold(args.threshold);
            figif.analyze_file(path)?
        }
        HasherType::Blockhash => {
            let figif = Figif::new()
                .with_hasher(BlockHasher::new())
                .similarity_threshold(args.threshold);
            figif.analyze_file(path)?
        }
    };

    // Stop spinner
    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    // Calculate summary stats
    let static_segments: Vec<_> = analysis.segments.iter().filter(|s| s.is_static).collect();
    let motion_segments: Vec<_> = analysis.segments.iter().filter(|s| !s.is_static).collect();

    let static_duration_ms: u64 = static_segments.iter().map(|s| s.duration_ms() as u64).sum();
    let motion_duration_ms: u64 = motion_segments.iter().map(|s| s.duration_ms() as u64).sum();

    let total_duration = analysis.total_duration_ms() as f64;
    let static_percent = if total_duration > 0.0 {
        static_duration_ms as f64 / total_duration
    } else {
        0.0
    };
    let motion_percent = if total_duration > 0.0 {
        motion_duration_ms as f64 / total_duration
    } else {
        0.0
    };

    if output.json {
        let analysis_output = AnalysisOutput {
            file: path.display().to_string(),
            metadata: MetadataOutput {
                width: analysis.metadata.width,
                height: analysis.metadata.height,
                frames: analysis.frame_count(),
                duration_ms: analysis.total_duration_ms(),
                file_size,
            },
            segments: analysis
                .segments
                .iter()
                .map(|s| SegmentOutput {
                    id: s.id,
                    segment_type: if s.is_static {
                        "static".to_string()
                    } else {
                        "motion".to_string()
                    },
                    frame_start: s.frame_range.start,
                    frame_end: s.frame_range.end,
                    frame_count: s.frame_count(),
                    duration_ms: s.duration_ms(),
                })
                .collect(),
            summary: SummaryOutput {
                total_segments: analysis.segments.len(),
                static_segments: static_segments.len(),
                motion_segments: motion_segments.len(),
                static_duration_ms,
                motion_duration_ms,
                static_percent,
                motion_percent,
            },
        };

        // Write to file or stdout
        if let Some(output_path) = &args.output {
            let json = serde_json::to_string_pretty(&analysis_output)?;
            let mut file = fs::File::create(output_path)?;
            file.write_all(json.as_bytes())?;
            if !output.quiet {
                eprintln!("Analysis written to: {}", output_path.display());
            }
        } else {
            output.json(&analysis_output)?;
        }
    } else {
        // Text output
        output.header(format!("Analysis: {}", path.display()));

        // Metadata section
        println!();
        println!("{}", style("Metadata:").bold());
        output.info(
            "Dimensions",
            format!("{}x{}", analysis.metadata.width, analysis.metadata.height),
        );
        output.info("Frames", analysis.frame_count().to_string());
        output.info("Duration", format_duration(analysis.total_duration_ms()));
        output.info("File size", format_size(file_size));

        // Segments section (unless segments_only is false and we don't want full output)
        println!();
        println!(
            "{} ({} total, {} static):",
            style("Segments").bold(),
            analysis.segments.len(),
            static_segments.len()
        );

        let styles = SegmentStyles::default();

        for segment in &analysis.segments {
            let type_str = if segment.is_static {
                styles.static_style.apply_to("Static")
            } else {
                styles.motion_style.apply_to("Motion")
            };

            let pause_marker = if segment.is_static { " <- pause" } else { "" };

            println!(
                "  #{:<3} [{:^6}]  frames {:<4}-{:<4}  ({:>7})  {:>3} frames{}",
                styles.id_style.apply_to(segment.id),
                type_str,
                segment.frame_range.start,
                segment.frame_range.end.saturating_sub(1),
                styles
                    .duration_style
                    .apply_to(format_duration(segment.duration_ms() as u64)),
                segment.frame_count(),
                style(pause_marker).dim(),
            );
        }

        // Summary section
        println!();
        println!("{}", style("Summary:").bold());
        output.info(
            "Static (pauses)",
            format!(
                "{} ({} segments)",
                format_duration(static_duration_ms),
                static_segments.len()
            ),
        );
        output.info(
            "Motion",
            format!(
                "{} ({} segments)",
                format_duration(motion_duration_ms),
                motion_segments.len()
            ),
        );
        output.info(
            "Ratio",
            format!(
                "{} static / {} motion",
                format_percent(static_percent),
                format_percent(motion_percent)
            ),
        );

        // Write to file if requested
        if let Some(output_path) = &args.output {
            let analysis_output = AnalysisOutput {
                file: path.display().to_string(),
                metadata: MetadataOutput {
                    width: analysis.metadata.width,
                    height: analysis.metadata.height,
                    frames: analysis.frame_count(),
                    duration_ms: analysis.total_duration_ms(),
                    file_size,
                },
                segments: analysis
                    .segments
                    .iter()
                    .map(|s| SegmentOutput {
                        id: s.id,
                        segment_type: if s.is_static {
                            "static".to_string()
                        } else {
                            "motion".to_string()
                        },
                        frame_start: s.frame_range.start,
                        frame_end: s.frame_range.end,
                        frame_count: s.frame_count(),
                        duration_ms: s.duration_ms(),
                    })
                    .collect(),
                summary: SummaryOutput {
                    total_segments: analysis.segments.len(),
                    static_segments: static_segments.len(),
                    motion_segments: motion_segments.len(),
                    static_duration_ms,
                    motion_duration_ms,
                    static_percent,
                    motion_percent,
                },
            };

            let json = serde_json::to_string_pretty(&analysis_output)?;
            let mut file = fs::File::create(output_path)?;
            file.write_all(json.as_bytes())?;
            println!();
            output.success(format!("Analysis written to: {}", output_path.display()));
        }
    }

    Ok(())
}
