//! File discovery for indexable documentation in the forest
//!
//! The [`FileScanner`] walks the forest directory structure to discover files
//! that can be indexed (.md and .rs files). It respects .gitignore and
//! .crumblyignore patterns, and can be configured to scan specific target
//! directories or the entire forest.

mod entry;
mod types;
mod walk;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{IndexingFilter, ProgressReporter};
use crate::knowledge::domain::{RepoName, ScanConfig};

/// Discovers indexable files in the forest directory structure
pub struct FileScanner {
    index_root: PathBuf,
    config: ScanConfig,
    filter: IndexingFilter,
    progress: Option<Arc<dyn ProgressReporter>>,
}

impl std::fmt::Debug for FileScanner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileScanner")
            .field("index_root", &self.index_root)
            .field("config", &self.config)
            .field("filter", &self.filter)
            .field(
                "progress",
                &self.progress.as_ref().map(|_| "Some(ProgressReporter)"),
            )
            .finish()
    }
}

impl FileScanner {
    /// Create a scanner for the given forest root directory
    pub fn new(index_root: impl AsRef<Path>) -> Result<Self, ScanError> {
        Self::with_progress(
            index_root,
            ScanConfig::default(),
            IndexingFilter::default(),
            None,
        )
    }

    /// Create a scanner with custom configuration
    pub fn with_config(
        index_root: impl AsRef<Path>,
        config: ScanConfig,
    ) -> Result<Self, ScanError> {
        Self::with_progress(index_root, config, IndexingFilter::default(), None)
    }

    /// Create a scanner with custom configuration and filter
    pub fn with_config_and_filter(
        index_root: impl AsRef<Path>,
        config: ScanConfig,
        filter: IndexingFilter,
    ) -> Result<Self, ScanError> {
        Self::with_progress(index_root, config, filter, None)
    }

    /// Create a scanner with optional progress reporting
    pub fn with_progress(
        index_root: impl AsRef<Path>,
        config: ScanConfig,
        filter: IndexingFilter,
        progress: Option<Arc<dyn ProgressReporter>>,
    ) -> Result<Self, ScanError> {
        use types::scan_error::*;

        let index_root = index_root.as_ref();
        snafu::ensure!(
            index_root.exists(),
            IndexRootNotFoundSnafu {
                path: index_root.display().to_string()
            }
        );
        snafu::ensure!(
            index_root.is_dir(),
            IndexRootNotDirectorySnafu {
                path: index_root.display().to_string()
            }
        );

        Ok(Self {
            index_root: index_root.to_path_buf(),
            config,
            filter,
            progress,
        })
    }

    /// Scan for all indexable files in the forest
    pub fn scan(&self) -> Result<Vec<IndexableFile>, ScanError> {
        if let Some(progress) = &self.progress {
            progress.scanning_started();
        }

        let result = self.scan_internal(None);

        if result.is_ok()
            && let Some(progress) = &self.progress
        {
            progress.scanning_completed();
        }

        result
    }

    /// Scan a specific repository directory
    pub fn scan_repo(&self, repo_name: &RepoName) -> Result<Vec<IndexableFile>, ScanError> {
        if let Some(progress) = &self.progress {
            progress.scanning_started();
        }

        let result = self.scan_internal(Some(repo_name));

        if result.is_ok()
            && let Some(progress) = &self.progress
        {
            progress.scanning_completed();
        }

        result
    }
}

pub use types::{IndexableFile, ScanError};
