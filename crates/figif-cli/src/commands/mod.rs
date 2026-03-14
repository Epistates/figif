//! CLI command definitions and handlers.

use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

pub mod analyze;
pub mod completions;
pub mod info;
pub mod optimize;

/// figif - GIF frame analysis and optimization
#[derive(Parser)]
#[command(name = "figif-cli")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
#[command(after_help = "Examples:
  figif-cli info demo.gif                         Show GIF metadata
  figif-cli analyze demo.gif                      Analyze segments
  figif-cli optimize demo.gif out.gif --cap-pauses 300
  figif-cli completions fish > ~/.config/fish/completions/figif-cli.fish")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Custom config file path
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Show GIF metadata (quick, no analysis)
    Info(InfoArgs),

    /// Analyze a GIF and show segment breakdown
    Analyze(AnalyzeArgs),

    /// Optimize a GIF with preset or custom operations
    Optimize(OptimizeArgs),

    /// Generate shell completions
    Completions(CompletionsArgs),
}

// ============================================================================
// Info Command
// ============================================================================

#[derive(Parser)]
pub struct InfoArgs {
    /// GIF file to inspect
    #[arg(value_name = "FILE")]
    pub input: PathBuf,
}

// ============================================================================
// Analyze Command
// ============================================================================

#[derive(Parser)]
pub struct AnalyzeArgs {
    /// GIF file to analyze
    #[arg(value_name = "FILE")]
    pub input: PathBuf,

    /// Similarity threshold for segment detection (lower = more sensitive)
    #[arg(short, long, default_value = "5", value_name = "N")]
    pub threshold: u32,

    /// Hash algorithm to use
    #[arg(long, default_value = "dhash", value_name = "TYPE")]
    pub hasher: HasherType,

    /// Write analysis to JSON file
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Only show segment summary
    #[arg(long)]
    pub segments_only: bool,

    /// Include per-frame details
    #[arg(long)]
    pub frames: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug, Default)]
pub enum HasherType {
    #[default]
    Dhash,
    Phash,
    Blockhash,
}

// ============================================================================
// Optimize Command
// ============================================================================

#[derive(Parser)]
pub struct OptimizeArgs {
    /// Source GIF file
    #[arg(value_name = "INPUT")]
    pub input: PathBuf,

    /// Output GIF path
    #[arg(value_name = "OUTPUT")]
    pub output: PathBuf,

    /// Apply a preset configuration
    #[arg(long, value_name = "NAME")]
    pub preset: Option<Preset>,

    /// Cap pause segments to max duration (ms)
    #[arg(long, value_name = "MS")]
    pub cap_pauses: Option<u32>,

    /// Collapse all pauses to fixed duration (ms)
    #[arg(long, value_name = "MS")]
    pub collapse_pauses: Option<u32>,

    /// Remove pauses longer than threshold (ms)
    #[arg(long, value_name = "MS")]
    pub remove_long: Option<u32>,

    /// Speed up pause segments by factor
    #[arg(long, value_name = "X")]
    pub speed_up_pauses: Option<f64>,

    /// Speed up entire GIF by factor
    #[arg(long, value_name = "X")]
    pub speed_up_all: Option<f64>,

    /// Target total duration (ms) - adjusts pauses to reach target
    #[arg(long, value_name = "MS")]
    pub target_duration: Option<u64>,

    /// Use lossy encoder (gifski) with optional quality 1-100
    #[arg(long, value_name = "QUALITY", num_args = 0..=1, default_missing_value = "80")]
    pub lossy: Option<u8>,

    /// Resize width in pixels (maintains aspect ratio)
    #[arg(long, value_name = "PX")]
    pub width: Option<u16>,

    /// Resize height in pixels (maintains aspect ratio)
    #[arg(long, value_name = "PX")]
    pub height: Option<u16>,

    /// Similarity threshold for analysis
    #[arg(short, long, default_value = "5", value_name = "N")]
    pub threshold: u32,

    /// Hash algorithm to use
    #[arg(long, default_value = "dhash")]
    pub hasher: HasherType,

    /// Overwrite output without confirmation
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Preview changes without writing
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum Preset {
    /// Minimal optimization - cap pauses to 200ms
    Fast,
    /// Moderate optimization - cap pauses to 300ms, speed up pauses 1.5x
    Balanced,
    /// Aggressive optimization - collapse pauses to 100ms, speed up 1.5x
    Aggressive,
}

// ============================================================================
// Completions Command
// ============================================================================

#[derive(Parser)]
pub struct CompletionsArgs {
    /// Target shell
    #[arg(value_name = "SHELL")]
    pub shell: clap_complete::Shell,
}
