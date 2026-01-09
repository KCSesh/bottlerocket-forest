//! Grove subcommands.

use crate::forest::ForestConfig;
use crate::grove::ForestManager;
use clap::{Args, Subcommand};
use owo_colors::OwoColorize;
use tracing::instrument;

#[derive(Subcommand, Debug)]
pub enum GroveCommand {
    /// Create a new forest grove
    Create(CreateArgs),
    /// List existing forest groves
    List,
    /// Remove a forest grove
    Remove(RemoveArgs),
}

#[derive(Args, Debug)]
pub struct CreateArgs {
    /// Name for the grove
    name: String,

    /// Branch to check out (creates if doesn't exist)
    #[arg(short, long)]
    branch: Option<String>,
}

#[derive(Args, Debug)]
pub struct RemoveArgs {
    /// Name of the grove to remove
    name: String,

    /// Force removal even if there are uncommitted changes
    #[arg(short, long)]
    force: bool,
}

#[instrument(err)]
pub fn run(cmd: GroveCommand) -> miette::Result<()> {
    let (forest_root, config) = ForestConfig::find()?;
    let manager = ForestManager::new(forest_root, config);

    match cmd {
        GroveCommand::Create(args) => {
            manager.create_grove(&args.name, args.branch.as_deref(), true)?;
            println!("{} Created grove '{}'", "✓".green(), args.name.cyan());
        }
        GroveCommand::List => {
            let groves = manager.list_groves()?;
            if groves.is_empty() {
                println!("No groves found");
            } else {
                println!("Forest groves:");
                for g in groves {
                    println!("  {}", g.cyan());
                }
            }
        }
        GroveCommand::Remove(args) => {
            manager.remove_grove(&args.name, args.force)?;
            println!("{} Removed grove '{}'", "✓".green(), args.name.cyan());
        }
    }

    Ok(())
}
