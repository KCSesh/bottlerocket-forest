//! Search operations for the knowledge index

use snafu::ResultExt;

use super::KnowledgeIndex;
use crate::knowledge::domain::{ContextId, QueryText, ResultLimit, SearchQuery, SearchResults};
use crate::knowledge::facade::types::IndexError;
use crate::knowledge::search::SearchEngine;

pub(in crate::knowledge::facade) fn search(
    index: &KnowledgeIndex,
    query: QueryText,
    limit: ResultLimit,
) -> Result<SearchResults, IndexError> {
    use crate::knowledge::facade::types::index_error::*;

    let context_id = ContextId::from_path(".").expect("'.' is valid context id");

    let search_query = SearchQuery::builder()
        .text(query)
        .limit(limit)
        .context_id(context_id)
        .build();

    let engine = super::create_search_engine(index)?;
    engine.search(&search_query).context(SearchFailedSnafu)
}

pub(in crate::knowledge::facade) fn search_in_context(
    index: &KnowledgeIndex,
    query: QueryText,
    limit: ResultLimit,
    context_id: ContextId,
) -> Result<SearchResults, IndexError> {
    use crate::knowledge::facade::types::index_error::*;

    snafu::ensure!(
        index.db_path.exists(),
        IndexNotFoundSnafu {
            path: index.db_path.display().to_string()
        }
    );

    let search_query = SearchQuery::builder()
        .text(query)
        .limit(limit)
        .context_id(context_id)
        .build();

    let engine = super::create_search_engine(index)?;
    engine.search(&search_query).context(SearchFailedSnafu)
}

#[cfg(test)]
mod test {
    use crate::knowledge::KnowledgeIndex;
    use crate::knowledge::domain::{QueryText, ResultLimit};
    use crate::knowledge::facade::test_helpers::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_search_executes_query() {
        // Given an indexed document containing "boot"
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "test.md",
            "# Boot Process\n\nHow boot works",
        );

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        // When searching for "boot"
        let query = QueryText::try_new("boot").unwrap();
        let limit = ResultLimit::try_new(10).unwrap();
        let result = index.search(query, limit);

        // Then results are returned
        assert!(result.is_ok());
        let search_results = result.unwrap();
        assert!(!search_results.results.is_empty());
    }

    #[test]
    fn test_search_respects_limit() {
        // Given an indexed document with multiple matches
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "test.md",
            "# Test\n\ntest test test\n\n## Section\n\ntest test",
        );

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        // When searching with a limit of 2
        let query = QueryText::try_new("test").unwrap();
        let limit = ResultLimit::try_new(2).unwrap();
        let result = index.search(query, limit).unwrap();

        // Then at most 2 results are returned
        assert!(result.results.len() <= 2);
    }
}
