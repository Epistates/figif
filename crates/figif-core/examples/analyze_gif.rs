//! Example: Analyze a GIF and print segment information.
//!
//! Usage: cargo run --example analyze_gif <path-to-gif>

use figif_core::prelude::*;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("output.gif");

    println!("Analyzing: {}", path);
    println!();

    // Create analyzer with default settings (dHash)
    let figif = Figif::new();

    // Analyze the GIF
    let analysis = figif.analyze_file(path)?;

    // Print metadata
    println!("=== GIF Metadata ===");
    println!(
        "Dimensions: {}x{}",
        analysis.metadata.width, analysis.metadata.height
    );
    println!("Frames: {}", analysis.frame_count());
    println!("Duration: {}ms", analysis.total_duration_ms());
    println!("Segments: {}", analysis.segment_count());
    println!();

    // Print segment details
    println!("=== Segments ===");
    for segment in &analysis.segments {
        println!(
            "Segment {}: frames {}-{} ({} frames), {}ms, static={}, avg_dist={:.2}",
            segment.id,
            segment.frame_range.start,
            segment.frame_range.end - 1,
            segment.frame_count(),
            segment.duration_ms(),
            segment.is_static,
            segment.avg_distance
        );
    }
    println!();

    // Find static segments (potential pause points)
    let static_segments: Vec<_> = analysis.static_segments();
    if !static_segments.is_empty() {
        println!("=== Static Segments (good candidates for collapsing) ===");
        for segment in static_segments {
            println!(
                "  Segment {}: {} frames, {}ms",
                segment.id,
                segment.frame_count(),
                segment.duration_ms()
            );
        }
        println!();
    }

    // Show example operations
    println!("=== Example: Apply operations ===");
    let mut ops = std::collections::HashMap::new();

    // Keep first segment, collapse any static segments > 500ms
    for segment in &analysis.segments {
        if segment.is_static && segment.duration_ms() > 500 {
            ops.insert(segment.id, SegmentOp::Collapse { delay_cs: 30 }); // Collapse to 300ms
            println!(
                "  Would collapse segment {} ({}ms -> 300ms)",
                segment.id,
                segment.duration_ms()
            );
        }
    }

    if ops.is_empty() {
        println!("  No long static segments found to optimize.");
    }

    // Show what the output would look like
    let output_frames = analysis.apply_operations(&ops);
    let output_duration: u32 = output_frames
        .iter()
        .map(|f| f.delay_centiseconds as u32 * 10)
        .sum();
    println!();
    println!(
        "Output would have {} frames, {}ms duration",
        output_frames.len(),
        output_duration
    );
    println!(
        "Size reduction: {} frames removed, {}ms shorter",
        analysis.frame_count() - output_frames.len(),
        analysis.total_duration_ms() as i64 - output_duration as i64
    );

    Ok(())
}
