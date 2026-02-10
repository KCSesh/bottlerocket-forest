//! Forest sync-seed operation.

use std::sync::Arc;

use snafu::{ResultExt, Snafu};

use crate::domain::{ForestConfig, ForestRoot};
use crate::events::{EventEmitter, ForesterEvent};
use crate::git::BareRepository;

/// Syncs a forest by fetching latest changes into bare repositories.
pub struct SyncSeedOperation<'a> {
    forest_root: &'a ForestRoot,
    config: &'a ForestConfig,
    emitter: Arc<dyn EventEmitter>,
    verbose: bool,
}

impl<'a> SyncSeedOperation<'a> {
    /// Creates a new sync-seed operation.
    pub fn new(
        forest_root: &'a ForestRoot,
        config: &'a ForestConfig,
        emitter: Arc<dyn EventEmitter>,
        verbose: bool,
    ) -> Self {
        Self {
            forest_root,
            config,
            emitter,
            verbose,
        }
    }

    /// Executes the sync-seed operation.
    pub fn execute(&self) -> Result<(), SyncSeedError> {
        self.emitter.emit(&ForesterEvent::SyncSeedStarted);

        for member in &self.config.forest.member {
            self.fetch_member(member)?;
        }

        self.emitter.emit(&ForesterEvent::SyncSeedCompleted);
        Ok(())
    }

    fn fetch_member(&self, member: &crate::domain::Member) -> Result<(), SyncSeedError> {
        use sync_seed_error::*;

        let bare_path = self
            .forest_root
            .bare_dir()
            .join(format!("{}.git", member.name));

        let bare = BareRepository::new(&bare_path, &member.name);
        if !bare.exists() {
            self.emitter.emit(&ForesterEvent::MemberSkipped {
                name: member.name.clone(),
                reason: "bare repository not found (run 'forester seed' first)".to_string(),
            });
            return Ok(());
        }

        self.emitter.emit(&ForesterEvent::MemberFetching {
            name: member.name.clone(),
        });

        bare.fetch("origin", !self.verbose)
            .context(FetchSnafu { name: &member.name })?;

        self.emitter.emit(&ForesterEvent::MemberFetched {
            name: member.name.clone(),
        });
        Ok(())
    }
}

/// Errors from syncing a forest.
#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum SyncSeedError {
    /// Failed to fetch a member repository.
    #[snafu(display("Failed to fetch member '{name}'"))]
    Fetch {
        /// Member name.
        name: String,
        /// Underlying fetch error.
        source: crate::git::GitCommandError,
    },
}
