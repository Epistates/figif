//! Example: Optimize a GIF by collapsing static segments.
//!
//! Usage: cargo run --example optimize_gif <input.gif> [output.gif]

use figif_core::prelude::*;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let input = args.get(1).map(|s| s.as_str()).unwrap_or("output.gif");
    let output = args.get(2).map(|s| s.as_str()).unwrap_or("optimized.gif");

    println!("Input: {}", input);
    println!("Output: {}", output);
    println!();

    // Analyze the GIF
    let figif = Figif::new().similarity_threshold(5);
    let analysis = figif.analyze_file(input)?;

    println!(
        "Original: {} frames, {}ms",
        analysis.frame_count(),
        analysis.total_duration_ms()
    );
    println!("Static segments: {}", analysis.static_segments().len());
    println!();

    // =========================================================================
    // Fluent Selector API - Maximum developer control with chainable operations
    // =========================================================================

    // Option 1: Cap all pauses to max 300ms (using selector API)
    println!("=== Option 1: Cap pauses to 300ms ===");
    let ops = analysis.pauses().cap(300);
    preview_ops(&analysis, &ops);

    // Option 2: Collapse ALL pauses to exactly 200ms
    println!("=== Option 2: Collapse all pauses to 200ms ===");
    let ops = analysis.pauses().collapse(200);
    preview_ops(&analysis, &ops);

    // Option 3: Remove pauses longer than 1 second (chained filter)
    println!("=== Option 3: Remove pauses > 1s ===");
    let ops = analysis.pauses().longer_than(1000).remove();
    preview_ops(&analysis, &ops);

    // Option 4: Speed up pauses by 3x
    println!("=== Option 4: Speed up pauses 3x ===");
    let ops = analysis.pauses().speed_up(3.0);
    preview_ops(&analysis, &ops);

    // Option 5: Target a specific duration (e.g., 45 seconds)
    println!("=== Option 5: Target 45s duration ===");
    if let Some(ops) = analysis.target_duration(45_000) {
        preview_ops(&analysis, &ops);
    } else {
        println!("  Cannot achieve target - non-static content too long");
    }

    // =========================================================================
    // Advanced: Combining multiple operations with merge
    // =========================================================================
    println!("=== Advanced: Combined operations ===");
    let ops = analysis
        .pauses()
        .longer_than(500)
        .cap(300) // Cap long pauses
        .merge(&analysis.motion().speed_up(1.2)); // Speed up motion
    preview_ops(&analysis, &ops);

    // =========================================================================
    // More selector examples
    // =========================================================================
    println!("=== Selector statistics ===");
    println!("  Total pauses: {}", analysis.pauses().count());
    println!(
        "  Pauses > 500ms: {}",
        analysis.pauses().longer_than(500).count()
    );
    println!(
        "  Pause duration: {}ms",
        analysis.pauses().total_duration_ms()
    );
    println!("  Motion segments: {}", analysis.motion().count());
    println!(
        "  Motion duration: {}ms",
        analysis.motion().total_duration_ms()
    );
    println!();

    // Actually export using Option 1 (cap pauses to 300ms)
    println!("=== Exporting with capped pauses (300ms max) ===");
    let ops = analysis.pauses().cap(300);
    let encoder = StandardEncoder::new();
    let config = EncodeConfig::default();

    analysis.export_to_file(&encoder, &ops, output, &config)?;

    let input_size = std::fs::metadata(input).map(|m| m.len()).unwrap_or(0);
    let output_size = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);
    println!("Saved to: {}", output);
    println!(
        "File size: {} -> {}",
        format_size(input_size),
        format_size(output_size)
    );

    Ok(())
}

fn preview_ops(analysis: &figif_core::Analysis<img_hash::ImageHash>, ops: &figif_core::SegmentOps) {
    let output_frames = analysis.apply_operations(ops);
    let output_duration: u32 = output_frames
        .iter()
        .map(|f| f.delay_centiseconds as u32 * 10)
        .sum();

    println!(
        "  Result: {} frames, {}ms (saved {}ms)",
        output_frames.len(),
        output_duration,
        analysis.total_duration_ms() as i64 - output_duration as i64
    );
    println!();
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
