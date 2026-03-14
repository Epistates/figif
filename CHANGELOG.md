# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-13

### Added

- **figif-core**: Core GIF analysis and optimization library
  - Perceptual frame hashing with dHash, pHash, and BlockHash algorithms
  - Automatic segment detection (static vs. motion) based on frame similarity
  - Fluent pipeline API for chaining analysis, selection, and encoding
  - Standard GIF encoder with delta-encoding optimization
  - Optional lossy encoding via `gifski` (behind `lossy` feature flag)
  - Parallel frame hashing via `rayon` (behind `parallel` feature flag)
  - Pluggable trait system: `FrameHasher`, `GifDecoder`, `GifEncoder`
- **figif**: Interactive Ratatui TUI for GIF exploration
  - Real-time frame preview with hardware-accelerated image rendering
  - Segment timeline navigation with keyboard controls
  - Per-segment operations: cap duration, collapse, speed adjust, set delay
  - Live playback preview with configurable speed
  - Export optimized GIFs with applied operations
- **figif-cli**: Headless CLI for batch GIF processing
  - `info` command for GIF metadata display
  - `analyze` command for segment breakdown analysis
  - `optimize` command with pause capping and speed adjustment
  - Shell completion generation (bash, zsh, fish, PowerShell)
  - Persistent configuration via `confy`
- Continuous integration via GitHub Actions (fmt, clippy, tests)
- Fuzz testing targets for decoder robustness

[0.1.0]: https://github.com/nickpaterno/figif/releases/tag/v0.1.0
