//! figif-tui - Interactive terminal UI for GIF manipulation

use clap::Parser;
use color_eyre::eyre::Result;
use std::path::PathBuf;

mod actions;
mod app;
mod theme;
mod ui;

use app::App;

/// Interactive TUI for GIF segment manipulation
#[derive(Parser)]
#[command(name = "figif-tui")]
#[command(author, version, about)]
struct Args {
    /// GIF file to open
    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    /// Similarity threshold for analysis
    #[arg(short, long, default_value = "5")]
    threshold: u32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Parse args
    let args = Args::parse();

    // Create app (fast non-blocking initialization)
    let mut app = App::new(args.threshold);

    // Start loading file if provided (asynchronously)
    if let Some(path) = args.file {
        app.load_file(path);
    }

    // Initialize terminal
    let terminal = ratatui::init();

    // Run the app
    let result = app.run(terminal).await;

    // Restore terminal
    ratatui::restore();

    result
}
