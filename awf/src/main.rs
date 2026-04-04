// TODO: Remove once implementation is complete.
#![allow(dead_code)]

mod cli;
mod engine;
mod model;
mod result;
mod store;

use clap::Parser as _;
use color_eyre::Result;

fn main() -> Result<()> {
    // Install color_eyre's panic and error hooks for rich error reporting.
    color_eyre::install()?;

    // Initialise the tracing subscriber, respecting the RUST_LOG env var.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = cli::Cli::parse();
    cli::dispatch(cli)
}
