# figif 🎞️

**figif** is a state-of-the-art (SOTA) GIF manipulation suite designed for intelligent frame analysis and optimization. Built in Rust with a focus on performance, perceptual correctness, and high-quality encoding.

[![CI](https://github.com/nickpaterno/figif/actions/workflows/ci.yml/badge.svg)](https://github.com/nickpaterno/figif/actions/workflows/ci.yml)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/License-MIT%2FApache--2.0-blue.svg)](LICENSE)

## ✨ Features

- **🧠 Perceptual Analysis**: Uses image hashing (dHash, pHash, BlockHash) to group frames into semantic "segments" (pauses vs. motion).
- **📉 SOTA Compression**: Integrated with `gifski` for industry-leading lossy LZW compression.
- **⚡ Parallel Processing**: High-performance frame hashing and processing powered by `rayon`.
- **🖥️ Premium TUI**: A hardware-accelerated terminal interface for interactive GIF exploration and optimization.
- **🛠️ Fluent API**: A chainable Rust DSL for complex filtering and optimization logic.
- **🛡️ Google-Grade Quality**: Continuous fuzzing, exhaustive integration testing, and strict CI enforcement.

## 📦 Project Structure

- `figif-core`: The heart of the suite. Handles decoding, analysis, segment detection, and re-encoding.
- `figif-cli`: Headless CLI for automation and batch processing.
- `figif`: Interactive terminal UI for granular control.

## 🚀 Getting Started

### Installation

```bash
cargo install --path crates/figif-cli
cargo install --path crates/figif
```

### CLI Usage

Analyze a GIF and show segment breakdown:
```bash
figif-cli analyze demo.gif
```

Optimize a GIF by capping pauses to 300ms and speeding up motion by 1.2x:
```bash
figif-cli optimize demo.gif --cap 300 --speed 1.2 -o optimized.gif
```

### TUI Usage

Launch the interactive suite:
```bash
figif demo.gif
```

**Keybindings:**
- `j`/`k`: Navigate segments
- `Enter`: Zoom into frames of a segment
- `p`: Toggle playback preview
- `c`: Cap segment duration
- `x`: Clear operations
- `s`: Save/Export optimized GIF

## 🛠️ Development

### Testing

Run the full suite of unit and integration tests:
```bash
cargo test --all-targets --all-features
```

### Fuzzing

Ensure decoder robustness against malformed inputs:
```bash
cargo +nightly fuzz run fuzz_decoder
```

## 📜 License

This project is licensed under either the [MIT License](LICENSE-MIT) or the [Apache License 2.0](LICENSE-APACHE).
