//! CLI for forester.

mod seed;
mod worktree;

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
    /// Clone all member repositories and set up the forest
    Seed(seed::SeedArgs),
    /// Manage forest worktrees
    #[command(subcommand)]
    Worktree(worktree::WorktreeCommand),
}

pub fn run() -> miette::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Seed(args) => seed::run(args),
        Command::Worktree(cmd) => worktree::run(cmd),
    }
}
