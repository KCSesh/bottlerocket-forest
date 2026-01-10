//! Grove subcommands.

use crate::forest::ForestConfig;
use crate::grove::{ForestManager, GroveContext};
use clap::{Args, Subcommand};
use miette::Diagnostic;
use owo_colors::OwoColorize;
use snafu::Snafu;
use tracing::instrument;

#[derive(Debug, Snafu, Diagnostic)]
#[snafu(module)]
pub enum GroveError {
    #[snafu(display("cannot remove grove '{name}' while inside it"))]
    #[diagnostic(help("Change to a directory outside the grove before removing it"))]
    RemoveCurrentGrove { name: String },
}

#[derive(Subcommand, Debug)]
pub enum GroveCommand {
    /// Create a new forest grove
    Create(CreateArgs),
    /// List existing forest groves
    List,
    /// Remove a forest grove
    Remove(RemoveArgs),
    /// Show current grove status
    Status,
    /// Print current grove name for scripts
    Current,
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

#[instrument(skip_all, err)]
pub fn run(cmd: GroveCommand) -> miette::Result<()> {
    let (forest_root, config) = ForestConfig::find()?;
    let manager = ForestManager::new(forest_root.clone(), config);

    match cmd {
        GroveCommand::Create(args) => {
            manager.create_grove(&args.name, args.branch.as_deref(), true)?;
            println!("{} Created grove '{}'", "✓".green(), args.name.cyan());
        }
        GroveCommand::List => {
            let groves = manager.list_groves()?;
            let current = GroveContext::detect().ok().flatten();
            if groves.is_empty() {
                println!("No groves found");
            } else {
                println!("Forest groves:");
                for g in groves {
                    if current.as_ref().is_some_and(|c| c.name() == g) {
                        println!("* {} {}", g.cyan(), "(current)".dimmed());
                    } else {
                        println!("  {}", g.cyan());
                    }
                }
            }
        }
        GroveCommand::Remove(args) => {
            use grove_error::*;
            if let Ok(Some(ctx)) = GroveContext::detect() && ctx.name() == args.name {
                return Err(RemoveCurrentGroveSnafu { name: args.name }.build().into());
            }
            manager.remove_grove(&args.name, args.force)?;
            println!("{} Removed grove '{}'", "✓".green(), args.name.cyan());
        }
        GroveCommand::Status => {
            if let Some(ctx) = GroveContext::detect().ok().flatten() {
                println!("Grove:       {}", ctx.name().cyan());
                println!("Grove root:  {}", ctx.grove_root().display());
                println!("Forest root: {}", ctx.forest_root().display());
            } else {
                println!("Not in a grove");
                println!("Forest root: {}", forest_root.display());
            }
        }
        GroveCommand::Current => {
            if let Some(ctx) = GroveContext::detect().ok().flatten() {
                println!("{}", ctx.name());
            } else {
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
