//! CLI for forester.

mod grove;
mod init;
mod seed;

use clap::{Parser, Subcommand, ValueEnum};
use std::io::IsTerminal;

/// Controls terminal color output behavior.
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ColorChoice {
    /// Detect terminal capability automatically.
    #[default]
    Auto,
    /// Always emit color codes.
    Always,
    /// Never emit color codes.
    Never,
}

/// Command-line interface for forester.
#[derive(Parser)]
#[command(name = "forester")]
#[command(about = "Generic forest management for multi-repo projects")]
#[command(version)]
pub struct Cli {
    #[arg(long, global = true, default_value = "auto")]
    color: ColorChoice,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new forest in the current directory
    Init(init::InitArgs),
    /// Clone all member repositories and set up the forest
    Seed(seed::SeedArgs),
    /// Manage forest groves
    #[command(subcommand)]
    Grove(grove::GroveCommand),
}

/// Parses CLI arguments and executes the requested command.
pub fn run() -> miette::Result<()> {
    let cli = Cli::parse();

    match cli.color {
        ColorChoice::Always => owo_colors::set_override(true),
        ColorChoice::Never => owo_colors::set_override(false),
        ColorChoice::Auto => {
            if !std::io::stdout().is_terminal() {
                owo_colors::set_override(false);
            }
        }
    }

    match cli.command {
        Command::Init(args) => init::run(args),
        Command::Seed(args) => seed::run(args),
        Command::Grove(cmd) => grove::run(cmd),
    }
}
