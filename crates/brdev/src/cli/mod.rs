//! Command-line interface for brdev.
//!
//! This module provides the top-level CLI structure and dispatches to subcommands:
//! * [`registry`] - Manages the local OCI registry
//! * [`twoliter`] - Manages Twoliter configuration

mod registry;
mod theme;
mod twoliter;

use std::io::IsTerminal;

use clap::{Parser, Subcommand, ValueEnum};
use snafu::{ResultExt, Snafu};

/// Bottlerocket development orchestration tool
#[derive(Parser)]
#[command(version, about, styles = clap_cargo::style::CLAP_STYLING)]
pub struct Args {
    /// When to use colors (auto, always, never)
    #[arg(long, global = true, default_value = "auto")]
    color: ColorChoice,

    #[command(subcommand)]
    command: Command,
}

/// When to use colored output.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum ColorChoice {
    /// Use colors only when output is a terminal
    #[default]
    Auto,
    /// Always use colors
    Always,
    /// Never use colors
    Never,
}

#[derive(Subcommand)]
enum Command {
    Registry(registry::RegistryCommand),
    Twoliter(twoliter::TwoliterCommand),
}

/// Parses CLI arguments and executes the requested command.
pub fn run() -> Result<(), CliError> {
    use cli_error::*;

    let args = Args::parse();

    match args.color {
        ColorChoice::Always => owo_colors::set_override(true),
        ColorChoice::Never => owo_colors::set_override(false),
        ColorChoice::Auto => {
            if !std::io::stdout().is_terminal() {
                owo_colors::set_override(false);
            }
        }
    }

    match args.command {
        Command::Registry(cmd) => registry::run(cmd).context(RegistrySnafu)?,
        Command::Twoliter(cmd) => twoliter::run(cmd).context(TwoliterSnafu)?,
    }

    Ok(())
}

/// Errors from CLI command execution.
#[derive(Debug, Snafu, miette::Diagnostic)]
#[snafu(module)]
pub enum CliError {
    /// Registry subcommand failed.
    #[snafu(display("Registry command failed"))]
    #[diagnostic(
        code(forester::cli::registry_command_failed),
        help("Check the error details above for specific guidance")
    )]
    Registry {
        /// Underlying registry error.
        source: registry::RegistryError,
    },

    /// Twoliter subcommand failed.
    #[snafu(display("Twoliter command failed"))]
    #[diagnostic(
        code(brdev::cli::twoliter_command_failed),
        help("Check the error details above for specific guidance")
    )]
    Twoliter {
        /// Underlying twoliter error.
        source: twoliter::TwoliterError,
    },
}
