//! CLI for forester.

mod init;
mod seed;
mod grove;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "forester")]
#[command(about = "Generic forest management for multi-repo projects")]
#[command(version)]
pub struct Cli {
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

pub fn run() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init(args) => init::run(args),
        Command::Seed(args) => seed::run(args),
        Command::Grove(cmd) => grove::run(cmd),
    }
}
