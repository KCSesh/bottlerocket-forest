//! Search domain types
//!
//! Search flow:
//! * [`SearchQuery`] specifies search parameters
//! * [`SearchResult`] represents a single matched chunk with relevance score
//! * [`SearchResults`] aggregates all results with search metadata
//! * [`FileSearchResult`] groups chunk matches by source file

use bon::Builder;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::{
    Chunk, ContextId, IndexRelativePath, QueryText, RelevanceScore, RepoName, ResultLimit,
};

/// Default number of chunks to return from search.
pub const DEFAULT_RESULT_LIMIT: usize = 20;

/// Search parameters combining query text and result limit.
#[derive(Debug, Clone, PartialEq, Eq, Builder, Serialize, Deserialize)]
#[builder(on(QueryText, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
#[allow(clippy::unwrap_used)]
pub struct SearchQuery {
    /// Query text for semantic or keyword matching.
    pub text: QueryText,
    /// Maximum number of results to return.
    #[builder(default = ResultLimit::try_new(DEFAULT_RESULT_LIMIT).unwrap())]
    pub limit: ResultLimit,
    /// Context to search within.
    #[serde(default)]
    pub context_id: ContextId,
}

/// Single chunk match with relevance score.
#[derive(Debug, Clone, PartialEq, Builder, Serialize, Deserialize)]
#[builder(on(_, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct SearchResult {
    /// Matched chunk content and metadata.
    pub chunk: Chunk,
    /// Relevance score for this match.
    pub score: RelevanceScore,
}

/// Complete search operation results with performance metrics.
#[derive(Debug, Clone, PartialEq, Builder, Serialize, Deserialize)]
#[builder(on(_, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct SearchResults {
    /// Original search query.
    pub query: SearchQuery,
    /// Matched chunks ordered by relevance.
    pub results: Vec<SearchResult>,
    /// Total number of chunks considered.
    pub total_chunks_searched: usize,
    /// Time taken to execute the search.
    pub search_duration: Duration,
}

/// Chunk matches grouped by source file.
#[derive(Debug, Clone, PartialEq, Builder, Serialize, Deserialize)]
#[builder(on(_, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct FileSearchResult {
    /// Path to the source file relative to index root.
    pub file_path: IndexRelativePath,
    /// Name of the repository containing this file.
    pub repo_name: RepoName,
    /// Number of matching chunks in this file.
    pub match_count: usize,
    /// Highest relevance score among matches.
    pub best_score: RelevanceScore,
    /// Individual chunk matches from this file.
    pub chunks: Vec<SearchResult>,
}
