# figif Architecture

This document describes the high-level architecture of the `figif` suite.

## Core Concepts

### 1. Frame Analysis & Segmentation
The central idea of `figif` is to treat a GIF not just as a sequence of frames, but as a sequence of **semantic segments**.
- **Static Segments**: Consecutive frames with little to no visual change (pauses).
- **Motion Segments**: Consecutive frames with significant visual changes.

Analysis is performed using perceptual hashing (dHash, pHash, BlockHash) via the `FrameHasher` trait.

### 2. Pipeline Workflow
1. **Decode**: `GifDecoder` reads a GIF file and produces fully composited `DecodedFrame`s (handling disposal methods correctly).
2. **Analyze**: `Figif` computes hashes for each frame and groups them into `Segment`s.
3. **Select/Transform**: `SegmentSelector` (Fluent API) or the TUI allows users to define `SegmentOps` (Keep, Remove, Scale, Collapse, etc.).
4. **Split**: Logical segments can be further split into smaller ones using `FrameOp::SplitAfter`.
5. **Encode**: `GifEncoder` (Standard or Gifski) takes the transformed frames and produces the final GIF.

## Crate Structure

- **`figif-core`**: The library containing the traits, encoders, decoders, and the analysis engine.
- **`figif-cli`**: A command-line wrapper around the core library for headless optimization and analysis.
- **`figif-tui`**: A terminal user interface for interactive exploration and granular trimming.

## Extension Points
The system is designed to be extensible via traits:
- `FrameHasher`: Add new similarity algorithms.
- `GifDecoder`: Add support for other animation formats (e.g., APNG, WebP).
- `GifEncoder`: Add new output encoders or optimization techniques.

## Performance
- **Parallelism**: Rayon is used for batch hashing during analysis.
- **Memory**: Both streaming and buffered decoders are available to balance speed vs. memory usage.
- **Optimization**: Standard lossless encoder uses delta-encoding to minimize file size by only encoding changed regions.
