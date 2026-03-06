use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process::ExitCode;

mod clients;
mod commands;
mod config;
mod state;

#[derive(Parser, Debug)]
#[command(name = "comes", about = "Personal AI life coach", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// On-demand health check
    Health,
    /// Morning brief synthesis
    Brief,
    /// Proactive alert check
    Nudge,
    /// Overnight research runner
    Overnight,
    /// Status dashboard
    Status,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("Error: {}", err);
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = config::load_config()?;
    let mut state = state::load_state()?;

    match cli.command {
        Commands::Health => commands::health::run(&config, &state)?,
        Commands::Brief => commands::brief::run(&config, &mut state)?,
        Commands::Nudge => commands::nudge::run(&config)?,
        Commands::Overnight => commands::overnight::run(&config, &mut state)?,
        Commands::Status => commands::status::run()?,
    }

    Ok(())
}
