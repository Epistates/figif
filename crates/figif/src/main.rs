//! figif - Interactive terminal UI for GIF manipulation

use clap::Parser;
use color_eyre::eyre::Result;
use std::path::PathBuf;

mod actions;
mod app;
mod theme;
mod ui;

use app::App;
use ratatui_image::picker::{Picker, ProtocolType};

/// Interactive TUI for GIF segment manipulation
#[derive(Parser)]
#[command(name = "figif")]
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

    // Initialize terminal (enters alternate screen + raw mode)
    let terminal = ratatui::init();

    // Query terminal for graphics protocol support synchronously.
    // Must happen after alternate screen but before event stream starts.
    let mut picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());

    // iTerm2 falsely reports Kitty graphics capability but doesn't support
    // Kitty's unicode placeholder rendering. Override to iTerm2 protocol,
    // or fall back to Sixel which iTerm2 also supports.
    if picker.protocol_type() == ProtocolType::Kitty {
        let is_iterm = std::env::var("TERM_PROGRAM").is_ok_and(|tp| tp.contains("iTerm"));
        if is_iterm {
            picker.set_protocol_type(ProtocolType::Iterm2);
        }
    }

    app.set_picker(picker);

    // Run the app
    let result = app.run(terminal).await;

    // Restore terminal
    ratatui::restore();

    result
}
