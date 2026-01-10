//! Build and rebuild operations for the knowledge index.

use super::KnowledgeIndex;
use super::types::IndexError;
use snafu::ResultExt;
use std::sync::Arc;

use crate::knowledge::domain::{Context, ContextId, IndexMetadata};
use crate::knowledge::indexing::{
    BatchConfig, IndexDataProvider, IndexResult, IndexStrategy, Indexer, ProgressReporter,
};
use crate::knowledge::storage::sqlite::SqliteChunkRepository;
use crate::knowledge::storage::{ChunkRepository, ContextRepository};

#[bon::bon]
impl KnowledgeIndex {
    #[builder]
    pub fn build(
        &self,
        progress: Option<Arc<dyn ProgressReporter>>,
        #[builder(default = 100)] batch_size: usize,
        #[builder(default)] context_id: ContextId,
    ) -> Result<IndexResult, IndexError> {
        use super::types::index_error::*;

        snafu::ensure!(
            !self.db_path.exists(),
            IndexAlreadyExistsSnafu {
                path: self.db_path.display().to_string()
            }
        );

        let mut repository = SqliteChunkRepository::open(&self.db_path, &self.config)
            .context(DatabaseAccessFailedSnafu)?;

        let metadata = IndexMetadata::builder()
            .last_build(std::time::SystemTime::now())
            .chunk_count(0)
            .file_count(0)
            .model_config(self.config.clone())
            .build();
        repository
            .set_metadata(&metadata)
            .context(DatabaseAccessFailedSnafu)?;

        let context = Context::builder().context_id(context_id.clone()).build();
        repository
            .context_repository()
            .insert_context(&context)
            .context(ContextRegistrationFailedSnafu)?;

        let provider = Box::new(self.create_provider()?) as Box<dyn IndexDataProvider>;
        let scan_config = self.load_scan_config_for_context(&context_id)?;
        let filter = self.load_indexing_filter()?;
        let repository = self.repository()?;

        let batch_config = BatchConfig { batch_size };

        let mut indexer = Indexer::builder()
            .index_root(&self.index_root)
            .repository(repository)
            .config(&self.config)
            .provider(provider)
            .scan_config(scan_config)
            .filter(filter)
            .maybe_progress(progress)
            .batch_config(batch_config)
            .context_id(context_id)
            .build()
            .context(IndexingFailedSnafu)?;

        let result = indexer
            .index(IndexStrategy::Build)
            .context(IndexingFailedSnafu)?;

        self.update_last_build_timestamp()?;

        Ok(result)
    }

    /// Delete the index and rebuild from scratch
    ///
    /// Deletes the existing index database and creates a new one by scanning all files.
    #[builder]
    pub fn rebuild(
        &self,
        progress: Option<Arc<dyn ProgressReporter>>,
        #[builder(default = 100)] batch_size: usize,
        #[builder(default)] context_id: ContextId,
    ) -> Result<IndexResult, IndexError> {
        use super::types::index_error::*;

        if self.db_path.exists() {
            std::fs::remove_file(&self.db_path).context(IndexDeletionFailedSnafu)?;
        }

        let mut repository = SqliteChunkRepository::open(&self.db_path, &self.config)
            .context(DatabaseAccessFailedSnafu)?;

        let metadata = IndexMetadata::builder()
            .last_build(std::time::SystemTime::now())
            .chunk_count(0)
            .file_count(0)
            .model_config(self.config.clone())
            .build();
        repository
            .set_metadata(&metadata)
            .context(DatabaseAccessFailedSnafu)?;

        let default_context = Context::builder()
            .context_id(ContextId::from_path(".").expect("'.' is valid context id"))
            .build();
        repository
            .context_repository()
            .insert_context(&default_context)
            .context(ContextRegistrationFailedSnafu)?;

        if context_id.as_str() != "." {
            let context = Context::builder().context_id(context_id.clone()).build();
            repository
                .context_repository()
                .insert_context(&context)
                .context(ContextRegistrationFailedSnafu)?;
        }

        let provider = Box::new(self.create_provider()?) as Box<dyn IndexDataProvider>;
        let scan_config = self.load_scan_config_for_context(&context_id)?;
        let filter = self.load_indexing_filter()?;
        let repository = self.repository()?;

        let batch_config = BatchConfig { batch_size };

        let mut indexer = Indexer::builder()
            .index_root(&self.index_root)
            .repository(repository)
            .config(&self.config)
            .provider(provider)
            .scan_config(scan_config)
            .filter(filter)
            .maybe_progress(progress)
            .batch_config(batch_config)
            .context_id(context_id)
            .build()
            .context(IndexingFailedSnafu)?;

        let result = indexer
            .index(IndexStrategy::Build)
            .context(IndexingFailedSnafu)?;

        self.update_last_build_timestamp()?;

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::facade::test_helpers::test_helpers::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_build_indexes_files_in_forest() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "README.md",
            "# Test\n\nContent here",
        );

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.build().call();

        assert!(result.is_ok());
        let index_result = result.unwrap();
        assert!(index_result.files_processed > 0);
        assert!(index_result.chunks_affected > 0);
    }

    #[test]
    fn test_build_handles_empty_forest() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        let result = index.build().call().unwrap();

        assert_eq!(result.files_processed, 0);
        assert_eq!(result.chunks_affected, 0);
    }

    #[test]
    fn test_build_uses_configured_targets() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join(".git")).unwrap();
        create_test_file(
            temp_dir.path(),
            "docs",
            "guide.md",
            "# Guide\n\nDocumentation content",
        );
        create_test_file(
            temp_dir.path(),
            "bottlerocket",
            "README.md",
            "# Bottlerocket\n\nProject info",
        );
        create_test_file(
            temp_dir.path(),
            "ignored",
            "secret.md",
            "# Secret\n\nSecret content",
        );

        let config_content = r#"
targets = ["docs", "bottlerocket"]
"#;
        fs::write(temp_dir.path().join("crumbly.toml"), config_content).unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.build().call().unwrap();

        assert_eq!(result.files_processed, 2);
        let status = index.status().unwrap();
        assert_eq!(status.file_count, 2);
    }

    #[test]
    fn test_rebuild_clears_existing_chunks() {
        let temp_dir = TempDir::new().unwrap();
        let _index = test_index_with_content(&temp_dir);

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.rebuild().call();

        assert!(result.is_ok());
    }

    #[test]
    fn test_rebuild_reindexes_all_files() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(temp_dir.path(), "test-repo", "test.md", "# Test\n\nContent");

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.rebuild().call().unwrap();

        assert!(result.files_processed > 0);
        assert!(result.chunks_affected > 0);
    }

    #[test]
    fn test_rebuild_uses_configured_targets() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir(temp_dir.path().join(".git")).unwrap();
        create_test_file(
            temp_dir.path(),
            "docs",
            "guide.md",
            "# Guide\n\nDocumentation",
        );
        create_test_file(
            temp_dir.path(),
            "other",
            "other.md",
            "# Other\n\nOther content",
        );

        let config_content = r#"
targets = ["docs"]
"#;
        fs::write(temp_dir.path().join("crumbly.toml"), config_content).unwrap();

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        let result = index.rebuild().call().unwrap();

        assert_eq!(result.files_processed, 1);
    }
}
