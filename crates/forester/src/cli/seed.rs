//! Seed command - clone all member repos and set up the forest.

use crate::forest::ForestConfig;
use crate::worktree::ForestManager;
use clap::Args;
use owo_colors::OwoColorize;
use std::path::PathBuf;
use tracing::instrument;

#[derive(Args, Debug)]
pub struct SeedArgs {
    /// Path to forester.toml (default: current directory)
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[instrument(err)]
pub fn run(args: SeedArgs) -> miette::Result<()> {
    let (forest_root, config) = if let Some(config_path) = args.config {
        let config = ForestConfig::load(&config_path)?;
        let root = config_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        (root, config)
    } else {
        ForestConfig::find()?
    };

    if args.verbose {
        println!(
            "Seeding forest '{}' at {}",
            config.forest.name.cyan(),
            forest_root.display()
        );
    }

    let manager = ForestManager::new(forest_root, config);
    manager.seed(args.verbose)?;

    if args.verbose {
        println!("{}", "✓ Forest seeded successfully".green());
    }

    Ok(())
}
