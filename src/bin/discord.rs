//! `discord` binary — bundled archival CLI built on `discord-user-rs`.
//!
//! Built only when the `cli` feature is enabled:
//!   `cargo build --features cli --bin discord`

use clap::Parser;
use discord_user::cli::args::Cli;
use discord_user::cli::{commands, output};
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.no_color || std::env::var("NO_COLOR").is_ok() {
        colored::control::set_override(false);
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .init();

    match commands::dispatch(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            output::err(&format!("{:#}", e));
            ExitCode::FAILURE
        }
    }
}
