//! figif - GIF frame analysis and optimization CLI
//!
//! A command-line tool for analyzing GIF segments and optimizing timing.

use clap::Parser;
use color_eyre::eyre::Result;

mod commands;
mod config;
mod output;
mod progress;

use commands::{Cli, Commands};

fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Initialize tracing based on verbosity
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    // Handle no-color flag
    if cli.no_color {
        console::set_colors_enabled(false);
        console::set_colors_enabled_stderr(false);
    }

    // Create output context
    let output = output::OutputContext::new(cli.json, cli.quiet);

    // Dispatch to command handlers
    match cli.command {
        Commands::Info(args) => commands::info::run(args, &output),
        Commands::Analyze(args) => commands::analyze::run(args, &output),
        Commands::Optimize(args) => commands::optimize::run(args, &output),
        Commands::Completions(args) => commands::completions::run(args),
    }
}

fn init_tracing(verbosity: u8) {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let filter = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(verbosity >= 2))
        .with(env_filter)
        .init();
}
