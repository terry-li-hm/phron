use anyhow::Result;
use clap::{Parser, Subcommand};
use phron::commands;
use phron::config;
use phron::state;
use std::process::ExitCode;

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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{error::ErrorKind, Parser};

    #[test]
    fn parses_each_subcommand() {
        for (args, expected) in [
            (["comes", "health"], "Health"),
            (["comes", "brief"], "Brief"),
            (["comes", "nudge"], "Nudge"),
            (["comes", "overnight"], "Overnight"),
            (["comes", "status"], "Status"),
        ] {
            let cli = Cli::try_parse_from(args).unwrap();
            assert_eq!(format!("{:?}", cli.command), expected);
        }
    }

    #[test]
    fn missing_subcommand_errors() {
        let err = Cli::try_parse_from(["comes"]).unwrap_err();
        assert_eq!(
            err.kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }

    #[test]
    fn unknown_subcommand_errors() {
        let err = Cli::try_parse_from(["comes", "nope"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn help_flag_is_display_help() {
        let err = Cli::try_parse_from(["comes", "--help"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
    }

    #[test]
    fn version_flag_is_display_version() {
        let err = Cli::try_parse_from(["comes", "--version"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayVersion);
    }
}
