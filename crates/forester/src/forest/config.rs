//! Forest configuration types.

use bon::Builder;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Forest configuration loaded from `forester.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct ForestConfig {
    pub forest: ForestMeta,
    #[serde(default)]
    pub worktree: Option<WorktreeConfig>,
}

/// Forest metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForestMeta {
    pub name: String,
    #[serde(default)]
    pub member: Vec<Member>,
}

/// Worktree configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    #[serde(default)]
    pub symlink: Vec<SymlinkEntry>,
}

/// A symlink to create in worktrees.
#[derive(Debug, Clone, Serialize, Deserialize, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct SymlinkEntry {
    pub source: PathBuf,
    pub target: PathBuf,
}

/// A member repository in the forest.
#[derive(Debug, Clone, Serialize, Deserialize, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct Member {
    pub name: String,
    pub remote: String,
    pub path: PathBuf,
    #[serde(default)]
    pub default_branch: Option<String>,
}

impl ForestConfig {
    /// Load forest config from a file.
    pub fn load(path: &Path) -> Result<Self, crate::Error> {
        let content = std::fs::read_to_string(path).map_err(|e| crate::Error::ConfigRead {
            path: path.to_path_buf(),
            source: e,
        })?;
        toml::from_str(&content).map_err(|e| crate::Error::ConfigParse {
            path: path.to_path_buf(),
            source: e,
        })
    }

    /// Find forester.toml by walking up from current directory.
    pub fn find() -> Result<(PathBuf, Self), crate::Error> {
        let cwd = std::env::current_dir().map_err(|e| crate::Error::CurrentDir { source: e })?;
        Self::find_from(&cwd)
    }

    /// Find forester.toml by walking up from given directory.
    pub fn find_from(start: &Path) -> Result<(PathBuf, Self), crate::Error> {
        let mut dir = start.to_path_buf();
        loop {
            let config_path = dir.join("forester.toml");
            if config_path.exists() {
                let config = Self::load(&config_path)?;
                return Ok((dir, config));
            }
            if !dir.pop() {
                return Err(crate::Error::NoForestFound);
            }
        }
    }
}

impl Member {
    /// Get the default branch, falling back to "main".
    pub fn branch(&self) -> &str {
        self.default_branch.as_deref().unwrap_or("main")
    }
}
