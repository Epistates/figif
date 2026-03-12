//! Output formatting utilities for CLI output.

#![allow(dead_code)]

use console::{Style, style};
use serde::Serialize;
use std::fmt::Display;

/// Context for output formatting decisions.
#[derive(Clone)]
pub struct OutputContext {
    /// Output as JSON
    pub json: bool,
    /// Suppress non-essential output
    pub quiet: bool,
}

impl OutputContext {
    pub fn new(json: bool, quiet: bool) -> Self {
        Self { json, quiet }
    }

    /// Print a status message (suppressed in quiet mode).
    pub fn status(&self, message: impl Display) {
        if !self.quiet && !self.json {
            eprintln!("{}", message);
        }
    }

    /// Print an info message with a label.
    pub fn info(&self, label: &str, value: impl Display) {
        if !self.json {
            println!("  {}: {}", style(label).dim(), value);
        }
    }

    /// Print a header line.
    pub fn header(&self, text: impl Display) {
        if !self.json {
            println!("\n{}", style(text).bold());
        }
    }

    /// Print a success message.
    pub fn success(&self, message: impl Display) {
        if !self.json {
            println!("{} {}", style("✓").green().bold(), message);
        }
    }

    /// Print a warning message.
    pub fn warn(&self, message: impl Display) {
        if !self.json {
            eprintln!("{} {}", style("⚠").yellow().bold(), message);
        }
    }

    /// Print an error message.
    pub fn error(&self, message: impl Display) {
        eprintln!("{} {}", style("✗").red().bold(), message);
    }

    /// Print JSON output if in JSON mode, otherwise do nothing.
    pub fn json<T: Serialize>(&self, data: &T) -> color_eyre::Result<()> {
        if self.json {
            println!("{}", serde_json::to_string_pretty(data)?);
        }
        Ok(())
    }

    /// Print raw JSON (already serialized).
    pub fn json_raw(&self, json: &str) {
        if self.json {
            println!("{}", json);
        }
    }
}

/// Styles for segment types.
pub struct SegmentStyles {
    pub static_style: Style,
    pub motion_style: Style,
    pub id_style: Style,
    pub duration_style: Style,
    pub frames_style: Style,
}

impl Default for SegmentStyles {
    fn default() -> Self {
        Self {
            static_style: Style::new().yellow(),
            motion_style: Style::new().blue(),
            id_style: Style::new().dim(),
            duration_style: Style::new().cyan(),
            frames_style: Style::new().dim(),
        }
    }
}

/// Format a duration in milliseconds to human-readable form.
pub fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) as f64 / 1000.0;
        format!("{}m {:.1}s", mins, secs)
    }
}

/// Format a file size in bytes to human-readable form.
pub fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Format a percentage.
pub fn format_percent(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(1500), "1.5s");
        assert_eq!(format_duration(65000), "1m 5.0s");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1536), "1.5 KB");
        assert_eq!(format_size(1_572_864), "1.5 MB");
    }
}
