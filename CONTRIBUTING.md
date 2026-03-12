# Contributing to figif

Thank you for your interest in improving `figif`! This project aims to provide a state-of-the-art GIF manipulation suite.

## Development Workflow

1. **Fork and Clone**:
   ```bash
   git clone https://github.com/nickpaterno/figif.git
   cd figif
   ```

2. **Run Tests**:
   Ensure everything is working correctly:
   ```bash
   cargo test --all-targets --all-features
   ```

3. **Check Code Quality**:
   We maintain high standards using `clippy` and `rustfmt`:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --all-targets --all-features -- -D warnings
   ```

4. **Benchmarking**:
   If you make performance-related changes, run the benchmarks:
   ```bash
   cargo bench -p figif-core
   ```

5. **Fuzzing**:
   Any changes to decoding or analysis should be validated with fuzzing:
   ```bash
   cargo +nightly fuzz run fuzz_decoder
   ```

## Coding Standards

- **Rust Edition 2024**: We use the latest stable Rust features.
- **Traits over Implementation**: Prefer implementing functionality via existing traits (`GifDecoder`, `GifEncoder`, `FrameHasher`) to maintain modularity.
- **Zero-Panic Policy**: Code should handle errors gracefully via `FigifError`. Avoid `unwrap()` and `expect()` except in tests.
- **Parallelism**: Utilize `rayon` for CPU-intensive operations when appropriate.
- **Documentation**: All public APIs in `figif-core` should be documented.

## Submitting Changes

1. Create a descriptive branch: `git checkout -b feature/my-new-hasher`.
2. Commit your changes with clear, concise messages.
3. Push and open a Pull Request against the `main` branch.
4. Ensure the CI passes all checks.

## Questions?

Feel free to open an issue or start a discussion if you have questions about the architecture or contribution process.
