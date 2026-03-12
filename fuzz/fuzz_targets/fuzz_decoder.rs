#![no_main]

use figif_core::decoders::BufferedDecoder;
use figif_core::prelude::*;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // 1. Test metadata extraction
    let decoder = BufferedDecoder::new();
    let _ = decoder.metadata_from_bytes(data);

    // 2. Test full decoding
    if let Ok(frames_iter) = decoder.decode_bytes(data) {
        let frames: Vec<_> = frames_iter.filter_map(|r| r.ok()).collect();

        if !frames.is_empty() {
            // 3. Test analysis if decoding succeeded
            let figif = Figif::new().similarity_threshold(5).min_segment_frames(2);

            if let Ok(analysis) = figif.analyze_frames(frames) {
                // 4. Test optimization logic
                let _ = analysis.pauses().cap(200);
                let _ = analysis.motion().speed_up(1.5);
                let _ = analysis.all().remove();
            }
        }
    }
});
