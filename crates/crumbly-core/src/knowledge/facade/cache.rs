//! Cache operations for the knowledge index.

use super::KnowledgeIndex;
use super::inner;
use super::types::{CacheResult, CacheSource, IndexError};
use std::sync::Arc;

use crate::knowledge::indexing::ProgressReporter;

#[bon::bon]
impl KnowledgeIndex {
    /// Cache chunks and embeddings from a content source.
    ///
    /// Populates the chunk store without context association, enabling
    /// pre-warming for faster subsequent builds.
    #[builder]
    pub fn cache(
        &self,
        source: CacheSource,
        progress: Option<Arc<dyn ProgressReporter>>,
    ) -> Result<CacheResult, IndexError> {
        inner::cache(self, source, progress)
    }
}
