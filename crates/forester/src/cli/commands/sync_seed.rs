//! Sync-seed command handler.

use crate::cli::args::SyncSeedArgs;
use crate::domain::{ForestConfig, ForestRoot};
use crate::events::{ConsoleEmitter, EventEmitter};
use crate::ops::SyncSeedOperation;
use std::path::Path;
use std::sync::Arc;

pub fn run(args: SyncSeedArgs) -> miette::Result<()> {
    let (forest_path, config) = if let Some(config_path) = args.config {
        let cfg = load_config(&config_path)?;
        let root = config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        (root, cfg)
    } else {
        find_config()?
    };

    let forest_root = ForestRoot::builder().path(&forest_path).build();
    let emitter: Arc<dyn EventEmitter> = Arc::new(ConsoleEmitter::new(args.verbose));

    let op = SyncSeedOperation::new(&forest_root, &config, Arc::clone(&emitter), args.verbose);
    op.execute().map_err(|e| miette::miette!("{}", e))?;

    Ok(())
}

fn load_config(path: &Path) -> miette::Result<ForestConfig> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| miette::miette!("Failed to read {}: {}", path.display(), e))?;
    toml::from_str(&content)
        .map_err(|e| miette::miette!("Failed to parse {}: {}", path.display(), e))
}

fn find_config() -> miette::Result<(std::path::PathBuf, ForestConfig)> {
    let cwd = std::env::current_dir()
        .map_err(|e| miette::miette!("Failed to get current directory: {}", e))?;
    let mut dir = cwd;
    loop {
        let config_path = dir.join("forester.toml");
        if config_path.exists() {
            let config = load_config(&config_path)?;
            return Ok((dir, config));
        }
        if !dir.pop() {
            return Err(miette::miette!("No forester.toml found"));
        }
    }
}
