//! `discord` binary — bundled Discord CLI client built on `discord-user-rs`.
//!
//! Built only when the `cli` feature is enabled:
//!   `cargo build --features cli --bin discord`

use clap::Parser;
use discord_user::cli::args::Cli;
use discord_user::cli::{commands, output};
use std::process::ExitCode;

/// Roomy stack for the worker thread (see `main`).
const WORKER_STACK_SIZE: usize = 32 * 1024 * 1024;

fn main() -> ExitCode {
    // clap's command graph is large (the `dc` group nests ~15 write/client
    // sub-groups). In debug builds clap runs `Command::debug_assert`, whose
    // recursive validation is stack-hungry and overflows Windows' default
    // 1 MiB main-thread stack. Run everything on a thread with a roomy stack
    // instead — the same approach rustc uses for its own driver.
    let worker = std::thread::Builder::new()
        .name("discord-cli".into())
        .stack_size(WORKER_STACK_SIZE)
        .spawn(run)
        .expect("failed to spawn worker thread");
    match worker.join() {
        Ok(code) => code,
        Err(_) => {
            output::err("worker thread panicked");
            ExitCode::FAILURE
        }
    }
}

/// Parse args and dispatch the command on its own Tokio runtime.
fn run() -> ExitCode {
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

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            output::err(&format!("failed to start async runtime: {}", e));
            return ExitCode::FAILURE;
        }
    };

    match runtime.block_on(commands::dispatch(cli)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            output::err(&format!("{:#}", e));
            ExitCode::FAILURE
        }
    }
}
