//! Semantic search implementation using embeddings
//!
//! Implements the SearchEngine trait using semantic similarity search with
//! embeddings. This engine generates embeddings for query text and finds
//! chunks with similar semantic meaning using vector similarity.
//!
//! For embedding generation, see [`crate::knowledge::search::embeddings`].
//! For vector similarity search implementation, see [`crate::knowledge::storage::sqlite::search::search_semantic`].

use snafu::ResultExt;

use crate::knowledge::constants::DEFAULT_FILE_SEARCH_CHUNK_MULTIPLIER;
use crate::knowledge::domain::{FileSearchResults, SearchQuery};
use crate::knowledge::scoring::ScoreBooster;
use crate::knowledge::storage::ChunkRepository;

use super::{EmbeddingProvider, SearchEngine, SearchError};

/// Semantic search engine using embedding-based similarity
///
/// Converts query text into semantic vectors and searches for chunks with similar
/// embeddings using cosine similarity. Applies score boosting based on file characteristics.
pub struct SemanticSearchEngine<R: ChunkRepository> {
    repository: R,
    embedding_provider: Box<dyn EmbeddingProvider>,
    score_booster: ScoreBooster,
}

impl<R: ChunkRepository> SemanticSearchEngine<R> {
    /// Create a semantic search engine
    pub fn new(
        repository: R,
        embedding_provider: Box<dyn EmbeddingProvider>,
        score_booster: ScoreBooster,
    ) -> Self {
        Self {
            repository,
            embedding_provider,
            score_booster,
        }
    }
}

impl<R: ChunkRepository> SearchEngine for SemanticSearchEngine<R> {
    fn search(&self, query: &SearchQuery) -> Result<FileSearchResults, SearchError> {
        use super::engine::search_error::*;
        use std::time::Instant;

        let start = Instant::now();

        let query_embedding = self
            .embedding_provider
            .embed(query.text.clone().into_inner().as_str())
            .map_err(crate::knowledge::error::box_err)
            .context(EmbeddingFailedSnafu)?;

        let limit = query.limit;
        let mut multiplier = DEFAULT_FILE_SEARCH_CHUNK_MULTIPLIER;

        let mut file_results = self
            .repository
            .search_files(
                query_embedding.as_ref(),
                limit,
                multiplier,
                query.context_id.clone(),
            )
            .context(StorageSnafu)?;

        // Retry with doubled multiplier if insufficient results
        if file_results.len() < limit.into_inner() {
            multiplier *= 2;
            file_results = self
                .repository
                .search_files(
                    query_embedding.as_ref(),
                    limit,
                    multiplier,
                    query.context_id.clone(),
                )
                .context(StorageSnafu)?;
        }

        // Apply score boosting to each file's best_score
        let mut boosted_results: Vec<_> = file_results
            .into_iter()
            .map(|mut file_result| {
                let boosted_score = self
                    .score_booster
                    .apply_boost_to_score(file_result.best_score, &file_result.file_path)
                    .map_err(crate::knowledge::error::box_err)
                    .context(InvalidScoreSnafu {
                        score: file_result.best_score.into_inner(),
                    })?;
                file_result.best_score = boosted_score;
                Ok(file_result)
            })
            .collect::<Result<Vec<_>, SearchError>>()?;

        // Re-sort by boosted best_score descending
        boosted_results.sort_by_key(|r| std::cmp::Reverse(r.best_score));

        let search_duration = start.elapsed();

        Ok(FileSearchResults::builder()
            .query(query.clone())
            .results(boosted_results)
            .search_duration(search_duration)
            .build())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::knowledge::domain::{
        Embedding, FileSearchResult, IndexRelativePath, QueryText, RelevanceScore, RepoName,
        ResultLimit,
    };
    use crate::knowledge::search::embeddings::model::MockEmbeddingProvider;
    use crate::knowledge::storage::repository::MockChunkRepository;
    use test_case::test_case;

    fn create_test_embedding(values: Vec<f32>) -> Embedding {
        Embedding::try_new(values).unwrap()
    }

    fn create_file_result(path: &str, score: f32) -> FileSearchResult {
        FileSearchResult::builder()
            .file_path(IndexRelativePath::try_new(path).unwrap())
            .repo_name(RepoName::try_new("test").unwrap())
            .match_count(1usize)
            .best_score(RelevanceScore::try_new(score).unwrap())
            .chunks(vec![])
            .build()
    }

    #[test]
    fn test_semantic_search_generates_query_embedding() {
        // Given A semantic search engine with embedding provider
        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_search_files()
            .returning(|_, _, _, _| Ok(vec![]));

        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider
            .expect_embed()
            .times(1)
            .returning(|_| Ok(create_test_embedding(vec![0.1; 384])));

        let engine =
            SemanticSearchEngine::new(mock_repo, Box::new(mock_provider), ScoreBooster::default());
        let query = SearchQuery::builder()
            .text(QueryText::try_new("test query").unwrap())
            .limit(ResultLimit::try_new(10).unwrap())
            .context_id(Default::default())
            .build();

        // When Searching
        let result = engine.search(&query);

        // Then Query embedding should be generated
        assert!(result.is_ok());
    }

    #[test]
    fn test_semantic_search_with_no_results() {
        // Given A repository with no matching files
        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_search_files()
            .returning(|_, _, _, _| Ok(vec![]));

        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider
            .expect_embed()
            .returning(|_| Ok(create_test_embedding(vec![0.1; 384])));

        let engine =
            SemanticSearchEngine::new(mock_repo, Box::new(mock_provider), ScoreBooster::default());
        let query = SearchQuery::builder()
            .text(QueryText::try_new("nonexistent concept").unwrap())
            .limit(ResultLimit::try_new(10).unwrap())
            .context_id(Default::default())
            .build();

        // When Searching
        let results = engine.search(&query).unwrap();

        // Then Results should be empty
        assert_eq!(results.results.len(), 0);
    }

    #[test]
    fn test_semantic_search_returns_ranked_results() {
        // Given A repository with matching files
        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_search_files()
            .returning(move |_, _, _, _| {
                Ok(vec![
                    create_file_result("test/2.md", 0.95),
                    create_file_result("test/1.md", 0.75),
                ])
            });

        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider
            .expect_embed()
            .returning(|_| Ok(create_test_embedding(vec![0.8; 384])));

        // Use empty boost rules to test raw score ordering
        let score_booster = crate::knowledge::scoring::ScoreBooster::new(vec![]);
        let engine = SemanticSearchEngine {
            repository: mock_repo,
            embedding_provider: Box::new(mock_provider),
            score_booster,
        };
        let query = SearchQuery::builder()
            .text(QueryText::try_new("rust").unwrap())
            .limit(ResultLimit::try_new(10).unwrap())
            .context_id(Default::default())
            .build();

        // When Searching
        let results = engine.search(&query).unwrap();

        // Then Results should be ranked by best_score
        assert_eq!(results.results.len(), 2);
        assert!(
            results.results[0].best_score.into_inner() > results.results[1].best_score.into_inner()
        );
    }

    #[test]
    fn test_semantic_search_respects_limit() {
        // Given A repository with many matching files
        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_search_files()
            .returning(move |_, limit, _, _| {
                let all_results = vec![
                    create_file_result("test/1.md", 0.9),
                    create_file_result("test/2.md", 0.8),
                    create_file_result("test/3.md", 0.7),
                ];
                Ok(all_results.into_iter().take(limit.into_inner()).collect())
            });

        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider
            .expect_embed()
            .returning(|_| Ok(create_test_embedding(vec![0.5; 384])));

        let engine =
            SemanticSearchEngine::new(mock_repo, Box::new(mock_provider), ScoreBooster::default());
        let query = SearchQuery::builder()
            .text(QueryText::try_new("test").unwrap())
            .limit(ResultLimit::try_new(2).unwrap())
            .context_id(Default::default())
            .build();

        // When Searching with limit of 2
        let results = engine.search(&query).unwrap();

        // Then Only 2 results should be returned
        assert_eq!(results.results.len(), 2);
    }

    #[test]
    fn test_semantic_search_propagates_embedding_errors() {
        // Given An embedding provider that returns an error
        let mock_repo = MockChunkRepository::new();
        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider.expect_embed().returning(|_| {
            Err(
                crate::knowledge::search::EmbeddingError::EmbeddingGenerationFailed {
                    source: Box::new(std::io::Error::other("test error")),
                },
            )
        });
        let engine =
            SemanticSearchEngine::new(mock_repo, Box::new(mock_provider), ScoreBooster::default());
        let query = SearchQuery::builder()
            .text(QueryText::try_new("test").unwrap())
            .limit(ResultLimit::try_new(10).unwrap())
            .context_id(Default::default())
            .build();
        // When Searching
        let result = engine.search(&query);
        // Then Embedding error should be propagated
        assert!(matches!(
            result.unwrap_err(),
            SearchError::EmbeddingFailed { .. }
        ));
    }

    #[test]
    fn test_semantic_search_propagates_storage_errors() {
        // Given A repository that returns an error
        let mut mock_repo = MockChunkRepository::new();
        mock_repo.expect_search_files().returning(|_, _, _, _| {
            Err(crate::knowledge::storage::StorageError::InvalidData {
                message: "test error".to_string(),
            })
        });
        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider
            .expect_embed()
            .returning(|_| Ok(create_test_embedding(vec![0.1; 384])));
        let engine =
            SemanticSearchEngine::new(mock_repo, Box::new(mock_provider), ScoreBooster::default());
        let query = SearchQuery::builder()
            .text(QueryText::try_new("test").unwrap())
            .limit(ResultLimit::try_new(10).unwrap())
            .context_id(Default::default())
            .build();
        // When Searching
        let result = engine.search(&query);
        // Then Storage error should be propagated
        assert!(matches!(result.unwrap_err(), SearchError::Storage { .. }));
    }

    #[test]
    fn test_semantic_search_passes_embedding_to_repository() {
        // Given A semantic search engine
        let query_embedding = vec![0.42; 384];
        let expected_embedding = query_embedding.clone();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_search_files()
            .withf(move |embedding, _, _, _| embedding == expected_embedding.as_slice())
            .returning(|_, _, _, _| Ok(vec![]));

        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider
            .expect_embed()
            .returning(move |_| Ok(create_test_embedding(query_embedding.clone())));

        let engine =
            SemanticSearchEngine::new(mock_repo, Box::new(mock_provider), ScoreBooster::default());
        let query = SearchQuery::builder()
            .text(QueryText::try_new("test").unwrap())
            .limit(ResultLimit::try_new(10).unwrap())
            .context_id(Default::default())
            .build();

        // When Searching
        let result = engine.search(&query);

        // Then Query embedding should be passed to repository
        assert!(result.is_ok());
    }

    #[test_case("semantic search" ; "multi-word query")]
    #[test_case("rust" ; "single word")]
    #[test_case("How does Bottlerocket boot?" ; "question")]
    fn test_semantic_search_handles_various_queries(query_text: &str) {
        // Given A semantic search engine
        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_search_files()
            .returning(move |_, _, _, _| Ok(vec![create_file_result("test/1.md", 0.8)]));

        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider
            .expect_embed()
            .returning(|_| Ok(create_test_embedding(vec![0.6; 384])));

        let engine =
            SemanticSearchEngine::new(mock_repo, Box::new(mock_provider), ScoreBooster::default());
        let query = SearchQuery::builder()
            .text(QueryText::try_new(query_text).unwrap())
            .limit(ResultLimit::try_new(10).unwrap())
            .context_id(Default::default())
            .build();

        // When Searching
        let result = engine.search(&query);

        // Then Search should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_semantic_search_returns_results() {
        // Given A repository with matching files
        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_search_files()
            .returning(move |_, _, _, _| Ok(vec![create_file_result("test/1.md", 0.85)]));

        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider
            .expect_embed()
            .returning(|_| Ok(create_test_embedding(vec![0.6; 384])));

        let engine =
            SemanticSearchEngine::new(mock_repo, Box::new(mock_provider), ScoreBooster::default());
        let query = SearchQuery::builder()
            .text(QueryText::try_new("rust").unwrap())
            .limit(ResultLimit::try_new(10).unwrap())
            .context_id(Default::default())
            .build();

        // When Searching
        let results = engine.search(&query).unwrap();

        // Then Results should be returned
        assert_eq!(results.results.len(), 1);
    }

    #[test]
    fn test_semantic_search_reorders_after_boosting() {
        // Given Two files where boosting will reverse their order
        // file1: .rs file with higher raw score (0.9)
        // file2: .md file with lower raw score (0.7) but gets 1.5x boost
        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_search_files()
            .returning(move |_, _, _, _| {
                // Repository returns in raw score order (rs file first)
                Ok(vec![
                    create_file_result("src/main.rs", 0.9),
                    create_file_result("docs/guide.md", 0.7),
                ])
            });

        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider
            .expect_embed()
            .returning(|_| Ok(create_test_embedding(vec![0.8; 384])));

        // Boost markdown files by 1.5x
        let boost_rules = vec![
            crate::knowledge::scoring::BoostRule::builder()
                .description("Markdown files")
                .pattern(crate::knowledge::scoring::BoostPattern::new("**/*.md").unwrap())
                .multiplier(crate::knowledge::scoring::BoostMultiplier::try_new(1.5).unwrap())
                .build(),
        ];
        let score_booster = crate::knowledge::scoring::ScoreBooster::new(boost_rules);
        let engine = SemanticSearchEngine {
            repository: mock_repo,
            embedding_provider: Box::new(mock_provider),
            score_booster,
        };

        let query = SearchQuery::builder()
            .text(QueryText::try_new("test").unwrap())
            .limit(ResultLimit::try_new(10).unwrap())
            .context_id(Default::default())
            .build();

        // When Searching
        let results = engine.search(&query).unwrap();

        // Then Results should be reordered by boosted score
        // .md file: 0.7 * 1.5 = 1.0 (clamped)
        // .rs file: 0.9 * 1.0 = 0.9
        assert_eq!(results.results.len(), 2);
        assert!(results.results[0].file_path.to_string().ends_with(".md"));
        assert!(results.results[1].file_path.to_string().ends_with(".rs"));
        assert_eq!(results.results[0].best_score.into_inner(), 1.0);
        assert_eq!(results.results[1].best_score.into_inner(), 0.9);
    }

    #[test]
    fn test_semantic_search_retries_with_doubled_multiplier() {
        // Given A repository that returns fewer files than limit on first call
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_search_files()
            .times(2)
            .returning(move |_, _limit, multiplier, _| {
                let count = call_count_clone.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    // First call: return fewer than limit
                    assert_eq!(multiplier, 10); // DEFAULT_FILE_SEARCH_CHUNK_MULTIPLIER
                    Ok(vec![create_file_result("test/1.md", 0.9)])
                } else {
                    // Second call: doubled multiplier
                    assert_eq!(multiplier, 20);
                    Ok(vec![
                        create_file_result("test/1.md", 0.9),
                        create_file_result("test/2.md", 0.8),
                    ])
                }
            });

        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider
            .expect_embed()
            .returning(|_| Ok(create_test_embedding(vec![0.5; 384])));

        let engine =
            SemanticSearchEngine::new(mock_repo, Box::new(mock_provider), ScoreBooster::default());
        let query = SearchQuery::builder()
            .text(QueryText::try_new("test").unwrap())
            .limit(ResultLimit::try_new(5).unwrap())
            .context_id(Default::default())
            .build();

        // When Searching with limit > initial results
        let results = engine.search(&query).unwrap();

        // Then Retry should have been called
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
        assert_eq!(results.results.len(), 2);
    }

    #[test]
    fn test_semantic_search_no_retry_when_sufficient_results() {
        // Given A repository that returns enough files on first call
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let mut mock_repo = MockChunkRepository::new();
        mock_repo
            .expect_search_files()
            .times(1)
            .returning(move |_, _, _, _| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                Ok(vec![
                    create_file_result("test/1.md", 0.9),
                    create_file_result("test/2.md", 0.8),
                    create_file_result("test/3.md", 0.7),
                ])
            });

        let mut mock_provider = MockEmbeddingProvider::new();
        mock_provider
            .expect_embed()
            .returning(|_| Ok(create_test_embedding(vec![0.5; 384])));

        let engine =
            SemanticSearchEngine::new(mock_repo, Box::new(mock_provider), ScoreBooster::default());
        let query = SearchQuery::builder()
            .text(QueryText::try_new("test").unwrap())
            .limit(ResultLimit::try_new(3).unwrap())
            .context_id(Default::default())
            .build();

        // When Searching with limit <= initial results
        let results = engine.search(&query).unwrap();

        // Then No retry should occur
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        assert_eq!(results.results.len(), 3);
    }
}
