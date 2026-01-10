//! Search operations for the knowledge index

use snafu::ResultExt;

use super::KnowledgeIndex;
use crate::knowledge::domain::{ContextId, QueryText, ResultLimit, SearchQuery, SearchResults};
use crate::knowledge::facade::types::IndexError;
use crate::knowledge::search::SearchEngine;

pub(in crate::knowledge::facade) fn search(
    index: &KnowledgeIndex,
    query: impl AsRef<str>,
    limit: usize,
) -> Result<SearchResults, IndexError> {
    use crate::knowledge::facade::types::index_error::*;

    let context_id = ContextId::from_path(".").expect("'.' is valid context id");

    let search_query = SearchQuery::builder()
        .text(QueryText::try_new(query.as_ref()).context(InvalidQuerySnafu)?)
        .limit(ResultLimit::try_new(limit).context(InvalidResultLimitSnafu)?)
        .context_id(context_id)
        .build();

    let engine = super::create_search_engine(index)?;
    engine.search(&search_query).context(SearchFailedSnafu)
}

pub(in crate::knowledge::facade) fn search_in_context(
    index: &KnowledgeIndex,
    query: impl AsRef<str>,
    limit: usize,
    context_id: ContextId,
) -> Result<SearchResults, IndexError> {
    use crate::knowledge::facade::types::index_error::*;

    snafu::ensure!(
        index.db_path.exists(),
        IndexNotFoundSnafu {
            path: index.db_path.display().to_string()
        }
    );

    let text = QueryText::try_new(query.as_ref()).context(InvalidQuerySnafu)?;
    let limit = ResultLimit::try_new(limit).context(InvalidResultLimitSnafu)?;

    let search_query = SearchQuery::builder()
        .text(text)
        .limit(limit)
        .context_id(context_id)
        .build();

    let engine = super::create_search_engine(index)?;
    engine.search(&search_query).context(SearchFailedSnafu)
}

#[cfg(test)]
mod test {
    use crate::knowledge::KnowledgeIndex;
    use crate::knowledge::facade::IndexError;
    use crate::knowledge::facade::test_helpers::test_helpers::*;
    use tempfile::TempDir;

    #[test]
    fn test_search_executes_query() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "test.md",
            "# Boot Process\n\nHow boot works",
        );

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        let result = index.search("boot", 10);

        assert!(result.is_ok());
        let search_results = result.unwrap();
        assert!(!search_results.results.is_empty());
    }

    #[test]
    fn test_search_respects_limit() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(
            temp_dir.path(),
            "test-repo",
            "test.md",
            "# Test\n\ntest test test\n\n## Section\n\ntest test",
        );

        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();
        index.build().call().unwrap();

        let result = index.search("test", 2).unwrap();

        assert!(result.results.len() <= 2);
    }

    #[test]
    fn test_search_validates_limit_minimum() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        let result = index.search("test", 0);

        assert!(matches!(result, Err(IndexError::InvalidResultLimit { .. })));
    }

    #[test]
    fn test_search_validates_limit_maximum() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        let result = index.search("test", 101);

        assert!(matches!(result, Err(IndexError::InvalidResultLimit { .. })));
    }

    #[test]
    fn test_search_validates_empty_query() {
        let temp_dir = TempDir::new().unwrap();
        let index = KnowledgeIndex::open(temp_dir.path()).unwrap();

        let result = index.search("", 10);

        assert!(matches!(result, Err(IndexError::InvalidQuery { .. })));
    }
}
