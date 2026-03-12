use criterion::{criterion_group, criterion_main, Criterion};
use figif_core::hashers::{DHasher, PHasher, BlockHasher};
use figif_core::traits::FrameHasher;
use figif_core::types::{DecodedFrame, DisposalMethod};
use image::{Rgba, RgbaImage};

fn bench_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("Hashing");
    let width = 640;
    let height = 480;
    
    // Create a dummy image
    let mut img = RgbaImage::new(width, height);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let r = (x % 255) as u8;
        let g = (y % 255) as u8;
        *pixel = Rgba([r, g, 128, 255]);
    }

    let dhasher = DHasher::new();
    let phasher = PHasher::new();
    let blockhasher = BlockHasher::new();

    group.bench_function("dHash", |b| b.iter(|| dhasher.hash_frame(&img)));
    group.bench_function("pHash", |b| b.iter(|| phasher.hash_frame(&img)));
    group.bench_function("blockHash", |b| b.iter(|| blockhasher.hash_frame(&img)));
    
    group.finish();
}

fn bench_analysis(c: &mut Criterion) {
    use figif_core::prelude::*;
    
    let mut group = c.benchmark_group("Analysis");
    let frame_count = 50;
    let width = 256;
    let height = 256;
    
    let mut frames = Vec::new();
    for i in 0..frame_count {
        let mut img = RgbaImage::new(width, height);
        for pixel in img.pixels_mut() {
            *pixel = Rgba([i as u8, 100, 200, 255]);
        }
        frames.push(DecodedFrame {
            index: i,
            image: img,
            delay_centiseconds: 10,
            disposal: DisposalMethod::Keep,
            left: 0,
            top: 0,
        });
    }

    let figif = Figif::new();
    
    group.bench_function("Sequential Analysis (50 frames)", |b| {
        b.iter(|| figif.analyze_frames(frames.clone()))
    });

    #[cfg(feature = "parallel")]
    group.bench_function("Parallel Analysis (50 frames)", |b| {
        b.iter(|| figif.analyze_frames_parallel(frames.clone()))
    });

    group.finish();
}

criterion_group!(benches, bench_hashing, bench_analysis);
criterion_main!(benches);
