//! Progress bar utilities.

#![allow(dead_code)]

use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Create a spinner for indeterminate operations.
pub fn spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏")
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a progress bar for determinate operations.
pub fn progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/dim}] {pos}/{len} ({eta})")
            .expect("valid template")
            .progress_chars("█▓░"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Create a progress bar for byte-based operations (file I/O).
pub fn bytes_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:40.cyan/dim}] {bytes}/{total_bytes} ({bytes_per_sec})")
            .expect("valid template")
            .progress_chars("█▓░"),
    );
    pb.set_message(message.to_string());
    pb
}
