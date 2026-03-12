use figif_core::prelude::*;
use figif_core::types::{EncodableFrame, EncodeConfig};
use image::{Rgba, RgbaImage};

/// Helper to create a test GIF with specific properties.
fn create_test_gif(width: u16, height: u16, static_frames: usize, motion_frames: usize) -> Vec<u8> {
    let mut frames = Vec::new();

    // Add static frames (all identical)
    for _ in 0..static_frames {
        let mut img = RgbaImage::new(width as u32, height as u32);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([255, 0, 0, 255]); // Red
        }
        frames.push(EncodableFrame::new(img, 10)); // 100ms
    }

    // Add motion frames (changing colors)
    for i in 0..motion_frames {
        let mut img = RgbaImage::new(width as u32, height as u32);
        for (x, y, pixel) in img.enumerate_pixels_mut() {
            let color = (i * 20).min(255) as u8;
            *pixel = Rgba([color, x as u8, y as u8, 255]);
        }
        frames.push(EncodableFrame::new(img, 10)); // 100ms
    }

    let encoder = StandardEncoder::new();
    let config = EncodeConfig::default();
    encoder.encode(&frames, &config).unwrap()
}

#[test]
fn test_full_pipeline_analysis() {
    let bytes = create_test_gif(32, 32, 5, 5);

    let figif = Figif::new().similarity_threshold(5).min_segment_frames(2);
    let analysis = figif.analyze_bytes(&bytes).expect("Failed to analyze GIF");

    // Should have 10 frames total
    assert_eq!(analysis.frame_count(), 10);

    // Should have detected segments
    // 5 static frames -> 1 static segment
    // 5 motion frames -> 1 or more motion segments (depending on hash distance)
    assert!(analysis.segment_count() >= 2);

    let static_segments: Vec<_> = analysis.segments.iter().filter(|s| s.is_static).collect();
    assert!(
        !static_segments.is_empty(),
        "Should have detected at least one static segment"
    );
    assert_eq!(static_segments[0].frame_count(), 5);
}

#[test]
fn test_optimization_removals() {
    let bytes = create_test_gif(32, 32, 5, 5);

    let figif = Figif::new().similarity_threshold(5).min_segment_frames(2);
    let analysis = figif.analyze_bytes(&bytes).expect("Failed to analyze GIF");

    // Remove all static segments
    let ops = analysis.pauses().remove();
    let optimized_frames = analysis.apply_operations(&ops);

    // Should have removed the 5 static frames
    assert_eq!(optimized_frames.len(), 5);
}

#[test]
fn test_optimization_capping() {
    let bytes = create_test_gif(32, 32, 10, 0); // 10 static frames, 100ms each = 1000ms

    let figif = Figif::new().similarity_threshold(5).min_segment_frames(2);
    let analysis = figif.analyze_bytes(&bytes).expect("Failed to analyze GIF");

    // Cap to 200ms
    let ops = analysis.pauses().cap(200);
    let optimized_frames = analysis.apply_operations(&ops);

    // Capping a segment results in a single frame with the capped duration
    assert_eq!(optimized_frames.len(), 1);
    assert_eq!(optimized_frames[0].delay_centiseconds, 20);
}

#[test]
fn test_metadata_extraction() {
    let bytes = create_test_gif(16, 24, 1, 0);

    let decoder = BufferedDecoder::new();
    let metadata = decoder
        .metadata_from_bytes(&bytes)
        .expect("Failed to get metadata");

    assert_eq!(metadata.width, 16);
    assert_eq!(metadata.height, 24);
    assert_eq!(metadata.frame_count, 1);
}
