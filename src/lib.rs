//! pg_glimpse - A TUI for monitoring PostgreSQL databases.

pub mod app;
pub mod cli;
pub mod config;
pub mod connection;
pub mod db;
pub mod event;
pub mod history;
pub mod recorder;
pub mod replay;
pub mod runtime;
pub mod ui;

use clap::Parser;
use cli::Cli;
use color_eyre::eyre::Result;

/// Main entry point - parses CLI args and runs the application.
///
/// This is the primary entry point for the pg_glimpse application.
/// It handles argument parsing, sets up the tokio runtime, and delegates
/// to either live mode or replay mode based on the arguments.
pub fn run_cli() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(runtime::run(cli))
}
