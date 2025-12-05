//! Worktree subcommands.

use crate::forest::ForestConfig;
use crate::worktree::ForestManager;
use clap::{Args, Subcommand};
use owo_colors::OwoColorize;

#[derive(Subcommand)]
pub enum WorktreeCommand {
    /// Create a new forest worktree
    Create(CreateArgs),
    /// List existing forest worktrees
    List,
    /// Remove a forest worktree
    Remove(RemoveArgs),
}

#[derive(Args)]
pub struct CreateArgs {
    /// Name for the worktree
    name: String,

    /// Branch to check out (creates if doesn't exist)
    #[arg(short, long)]
    branch: Option<String>,
}

#[derive(Args)]
pub struct RemoveArgs {
    /// Name of the worktree to remove
    name: String,

    /// Force removal even if there are uncommitted changes
    #[arg(short, long)]
    force: bool,
}

pub fn run(cmd: WorktreeCommand) -> miette::Result<()> {
    let (forest_root, config) = ForestConfig::find()?;
    let manager = ForestManager::new(forest_root, config);

    match cmd {
        WorktreeCommand::Create(args) => {
            manager.create_worktree(&args.name, args.branch.as_deref(), true)?;
            println!("{} Created worktree '{}'", "✓".green(), args.name.cyan());
        }
        WorktreeCommand::List => {
            let worktrees = manager.list_worktrees()?;
            if worktrees.is_empty() {
                println!("No worktrees found");
            } else {
                println!("Forest worktrees:");
                for wt in worktrees {
                    println!("  {}", wt.cyan());
                }
            }
        }
        WorktreeCommand::Remove(args) => {
            manager.remove_worktree(&args.name, args.force)?;
            println!("{} Removed worktree '{}'", "✓".green(), args.name.cyan());
        }
    }

    Ok(())
}
