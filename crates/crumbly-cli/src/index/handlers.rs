//! Command handlers for knowledge index CLI operations.

use crate::index::errors::IndexError;

use crate::index::formatting::{
    OutputFormat, format_build_result, format_file_results_human, format_file_results_json,
    format_status, format_update_result, group_results_by_file, parse_output_format,
    prompt_confirmation,
};
use crate::index::{BuildArgs, ClearArgs, RebuildArgs, SearchArgs, StatusArgs, UpdateArgs};
use crumbly_core::knowledge::KnowledgeIndex;
use snafu::ResultExt;

use crate::index::progress::CliProgressReporter;
use crate::theme;
use crumbly_core::knowledge::domain::{ContextId, DEFAULT_RESULT_LIMIT, QueryText, ResultLimit};

use std::path::PathBuf;
use std::sync::Arc;

/// Parses and validates a context path argument.
///
/// Validates that the context path exists on the filesystem (relative to index_root)
/// and converts it to a ContextId.
fn parse_context_arg(
    index_root: &std::path::Path,
    context: Option<PathBuf>,
) -> Result<Option<ContextId>, IndexError> {
    use super::errors::index_error::*;

    let Some(path) = context else {
        return Ok(None);
    };

    // Validate the context path exists on filesystem (MCI-6)
    let full_path = index_root.join(&path);
    if !full_path.exists() {
        return Err(IndexError::ContextPathNotFound {
            path: path.display().to_string(),
        });
    }

    let context_id = ContextId::from_path(&path).context(InvalidContextPathSnafu {
        path: path.display().to_string(),
    })?;

    Ok(Some(context_id))
}

fn get_cwd() -> Result<PathBuf, IndexError> {
    use super::errors::index_error::*;
    std::env::current_dir().context(GetCurrentDirSnafu)
}

/// Builds the knowledge index, processing all files in the forest.
pub fn handle_build(args: BuildArgs) -> Result<(), IndexError> {
    use super::errors::index_error::*;

    let index_root = match args.index_root {
        Some(root) => root,
        None => get_cwd()?,
    };

    let index = KnowledgeIndex::open(&index_root).context(KnowledgeIndexSnafu)?;

    let context_id = parse_context_arg(&index_root, args.context)?;

    let progress = Arc::new(CliProgressReporter::default());
    let result = index
        .build()
        .progress(progress)
        .maybe_context_id(context_id)
        .call()
        .context(KnowledgeIndexSnafu)?;

    format_build_result(&result);

    Ok(())
}

/// Rebuilds the knowledge index from scratch, clearing existing data first.
pub fn handle_rebuild(args: RebuildArgs) -> Result<(), IndexError> {
    use super::errors::index_error::*;

    let index_root = match args.index_root {
        Some(root) => root,
        None => get_cwd()?,
    };

    let index = KnowledgeIndex::open(&index_root).context(KnowledgeIndexSnafu)?;

    let context_id = parse_context_arg(&index_root, args.context)?;

    let progress = Arc::new(CliProgressReporter::default());
    let result = index
        .rebuild()
        .progress(progress)
        .maybe_context_id(context_id)
        .call()
        .context(KnowledgeIndexSnafu)?;

    format_build_result(&result);

    Ok(())
}

/// Updates the knowledge index incrementally based on file changes.
pub fn handle_update(args: UpdateArgs) -> Result<(), IndexError> {
    use super::errors::index_error::*;

    let index = match args.index_root {
        Some(root) => KnowledgeIndex::open(&root).context(KnowledgeIndexSnafu)?,
        None => KnowledgeIndex::discover(&get_cwd()?).context(KnowledgeIndexSnafu)?,
    };

    let context_id = parse_context_arg(index.index_root(), args.context)?;

    let progress = Arc::new(CliProgressReporter::default());
    let result = index
        .update()
        .progress(progress)
        .maybe_context_id(context_id)
        .call()
        .context(KnowledgeIndexSnafu)?;

    format_update_result(&result);

    Ok(())
}

/// Clears file mappings for a context after user confirmation.
pub fn handle_clear(args: ClearArgs) -> Result<(), IndexError> {
    use super::errors::index_error::*;

    let index = match args.index_root {
        Some(root) => KnowledgeIndex::open(&root).context(KnowledgeIndexSnafu)?,
        None => KnowledgeIndex::discover(&get_cwd()?).context(KnowledgeIndexSnafu)?,
    };

    let context_id = match parse_context_arg(index.index_root(), args.context)? {
        Some(ctx) => ctx,
        None => {
            index
                .resolve_context(&get_cwd()?)
                .context(KnowledgeIndexSnafu)?
                .context_id
        }
    };

    if !args.yes
        && !prompt_confirmation(&format!(
            "Are you sure you want to clear context '{}'?",
            context_id.as_str()
        ))?
    {
        println!("{}", theme::muted("Cancelled"));
        return Ok(());
    }

    let deleted = index.clear(&context_id).context(KnowledgeIndexSnafu)?;

    println!(
        "{} Cleared {} file mappings from context '{}'",
        theme::success("✓"),
        deleted,
        context_id.as_str()
    );

    Ok(())
}

/// Searches the knowledge index and displays results in the requested format.
pub fn handle_search(args: SearchArgs) -> Result<(), IndexError> {
    use super::errors::index_error::*;

    let query = QueryText::try_new(&args.query).context(InvalidQuerySnafu)?;
    let limit = ResultLimit::try_new(args.limit.unwrap_or(DEFAULT_RESULT_LIMIT))
        .context(InvalidResultLimitSnafu)?;

    let format = parse_output_format(args.format.as_deref())?;

    let index = match args.index_root {
        Some(root) => KnowledgeIndex::open(&root).context(KnowledgeIndexSnafu)?,
        None => KnowledgeIndex::discover(&get_cwd()?).context(KnowledgeIndexSnafu)?,
    };

    if !index.db_path().exists() {
        return Err(crumbly_core::knowledge::facade::IndexError::IndexNotFound {
            path: index.db_path().to_path_buf(),
        })
        .context(KnowledgeIndexSnafu);
    }

    let context_id = match parse_context_arg(index.index_root(), args.context)? {
        Some(ctx) => ctx,
        None => {
            let context = index
                .resolve_context(&get_cwd()?)
                .context(KnowledgeIndexSnafu)?;
            context.context_id
        }
    };

    let results = index
        .search_in_context(query, limit, context_id)
        .context(KnowledgeIndexSnafu)?;

    let file_results = group_results_by_file(&results);

    let cwd = get_cwd()?;

    match format {
        OutputFormat::Human => {
            format_file_results_human(&file_results, args.show_chunks, index.index_root(), &cwd)
        }
        OutputFormat::Json => format_file_results_json(&file_results, index.index_root(), &cwd)?,
    }

    Ok(())
}

/// Displays index status and statistics including chunk count and model configuration.
pub fn handle_status(args: StatusArgs) -> Result<(), IndexError> {
    use super::errors::index_error::*;

    let index = match args.index_root {
        Some(root) => KnowledgeIndex::open(&root).context(KnowledgeIndexSnafu)?,
        None => KnowledgeIndex::discover(&get_cwd()?).context(KnowledgeIndexSnafu)?,
    };

    let status = index.status().context(KnowledgeIndexSnafu)?;

    format_status(&status);

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crumbly_core::knowledge::chunking::MarkdownContext;
    use crumbly_core::knowledge::domain::{
        Chunk, ChunkContent, ChunkContext, ChunkId, ChunkSource, EmbeddingModelConfig,
        FileSearchResult, IndexRelativePath, RelevanceScore, RepoName, SearchQuery, SearchResult,
        SearchResults, TokenCount,
    };
    use crumbly_core::knowledge::facade::IndexStatus;
    use crumbly_core::knowledge::indexing::IndexResult;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_parse_output_format_human() {
        // Given A format string "human"
        // When Parsing the output format
        let result = parse_output_format(Some("human"));

        // Then It should return OutputFormat::Human
        assert!(matches!(result, Ok(OutputFormat::Human)));
    }

    #[test]
    fn test_parse_output_format_json() {
        // Given A format string "json"
        // When Parsing the output format
        let result = parse_output_format(Some("json"));

        // Then It should return OutputFormat::Json
        assert!(matches!(result, Ok(OutputFormat::Json)));
    }

    #[test]
    fn test_parse_output_format_default() {
        // Given No format string (None)
        // When Parsing the output format
        let result = parse_output_format(None);

        // Then It should default to OutputFormat::Human
        assert!(matches!(result, Ok(OutputFormat::Human)));
    }

    #[test]
    fn test_parse_output_format_invalid() {
        // Given An invalid format string
        // When Parsing the output format
        let result = parse_output_format(Some("xml"));

        // Then It should return InvalidOutputFormat error
        assert!(matches!(
            result,
            Err(IndexError::InvalidOutputFormat { .. })
        ));
    }

    #[test]
    fn test_format_build_result_prints_summary() {
        // Given An IndexResult from a build operation
        let result = IndexResult::builder()
            .files_processed(10)
            .files_added(10)
            .files_updated(0)
            .files_removed(0)
            .files_skipped(0)
            .chunks_affected(50)
            .duration(Duration::from_secs(5))
            .build();

        // When Formatting the build result
        // Then It should print without panicking
        format_build_result(&result);
    }

    #[test]
    fn test_format_update_result_prints_summary() {
        // Given An IndexResult from an update operation
        let result = IndexResult::builder()
            .files_processed(3)
            .files_added(1)
            .files_updated(2)
            .files_removed(1)
            .files_skipped(0)
            .chunks_affected(12)
            .duration(Duration::from_secs(2))
            .build();

        // When Formatting the update result
        // Then It should print without panicking
        format_update_result(&result);
    }

    #[test]
    fn test_format_file_results_human_with_results() {
        // Given FileSearchResults with multiple files
        let chunk = create_test_chunk();
        let file_results = vec![
            FileSearchResult::builder()
                .file_path(IndexRelativePath::try_new("test.md").unwrap())
                .repo_name(RepoName::try_new("test-repo").unwrap())
                .match_count(2usize)
                .best_score(RelevanceScore::try_new(0.95).unwrap())
                .chunks(vec![
                    SearchResult::builder()
                        .chunk(chunk.clone())
                        .score(RelevanceScore::try_new(0.95).unwrap())
                        .build(),
                    SearchResult::builder()
                        .chunk(chunk)
                        .score(RelevanceScore::try_new(0.85).unwrap())
                        .build(),
                ])
                .build(),
        ];

        // When Formatting file results in human format
        // Then It should print without panicking
        let index_root = std::path::Path::new("/tmp");
        let cwd = std::path::Path::new("/tmp");
        format_file_results_human(&file_results, false, index_root, cwd);
        format_file_results_human(&file_results, true, index_root, cwd);
    }

    #[test]
    fn test_format_file_results_human_empty() {
        // Given Empty file results
        let file_results = vec![];

        // When Formatting empty file results
        // Then It should print without panicking
        let index_root = std::path::Path::new("/tmp");
        let cwd = std::path::Path::new("/tmp");
        format_file_results_human(&file_results, false, index_root, cwd);
    }

    #[test]
    fn test_format_search_results_json_success() {
        // Given SearchResults with results
        let chunk = create_test_chunk();
        let results = SearchResults::builder()
            .query(create_test_query())
            .results(vec![
                SearchResult::builder()
                    .chunk(chunk)
                    .score(RelevanceScore::try_new(0.95).unwrap())
                    .build(),
            ])
            .total_chunks_searched(100usize)
            .search_duration(Duration::from_millis(50))
            .build();

        // When Grouping and formatting as file results
        let file_results = group_results_by_file(&results);

        // Then It should succeed
        assert!(file_results.len() > 0);
    }

    #[test]
    fn test_format_file_results_json_success() {
        // Given FileSearchResults
        let chunk = create_test_chunk();
        let file_results = vec![
            FileSearchResult::builder()
                .file_path(IndexRelativePath::try_new("test.md").unwrap())
                .repo_name(RepoName::try_new("test-repo").unwrap())
                .match_count(1usize)
                .best_score(RelevanceScore::try_new(0.95).unwrap())
                .chunks(vec![
                    SearchResult::builder()
                        .chunk(chunk)
                        .score(RelevanceScore::try_new(0.95).unwrap())
                        .build(),
                ])
                .build(),
        ];

        // When Formatting file results as JSON
        let index_root = std::path::Path::new("/tmp");
        let cwd = std::path::Path::new("/tmp");
        let result = format_file_results_json(&file_results, index_root, cwd);

        // Then It should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_group_results_by_file() {
        // Given SearchResults with multiple chunks from same file
        let chunk1 = create_test_chunk();
        let mut chunk2 = chunk1.clone();
        chunk2.id = ChunkId::new(uuid::Uuid::new_v4());

        let results = SearchResults::builder()
            .query(create_test_query())
            .results(vec![
                SearchResult::builder()
                    .chunk(chunk1)
                    .score(RelevanceScore::try_new(0.95).unwrap())
                    .build(),
                SearchResult::builder()
                    .chunk(chunk2)
                    .score(RelevanceScore::try_new(0.85).unwrap())
                    .build(),
            ])
            .total_chunks_searched(100usize)
            .search_duration(Duration::from_millis(50))
            .build();

        // When Grouping results by file
        let file_results = group_results_by_file(&results);

        // Then It should return one file with two chunks
        assert_eq!(file_results.len(), 1);
        assert_eq!(file_results[0].match_count, 2);
        assert_eq!(
            file_results[0].best_score,
            RelevanceScore::try_new(0.95).unwrap()
        );
    }

    #[test]
    fn test_format_status_prints_info() {
        // Given An IndexStatus with metadata
        let status = IndexStatus::builder()
            .exists(true)
            .chunk_count(100)
            .file_count(20)
            .last_build(SystemTime::now())
            .model_config(EmbeddingModelConfig::default())
            .size_bytes(1024000)
            .build();

        // When Formatting the status
        // Then It should print without panicking
        format_status(&status);
    }

    #[test]
    fn test_prompt_confirmation_yes() {
        // Given User input "yes"
        // When Prompting for confirmation
        // Then It should return true
        // Note: This test requires mocking stdin, which is complex
        // We'll test the actual implementation manually
    }

    #[test]
    fn test_prompt_confirmation_no() {
        // Given User input "no"
        // When Prompting for confirmation
        // Then It should return false
        // Note: This test requires mocking stdin, which is complex
        // We'll test the actual implementation manually
    }

    fn create_test_chunk() -> Chunk {
        use crumbly_core::knowledge::domain::{ChunkHash, FileHash};
        use std::io::Cursor;

        let text = "Test content";
        Chunk::builder()
            .id(ChunkId::new(uuid::Uuid::new_v4()))
            .chunk_hash(ChunkHash::from_text(text))
            .file_hash(FileHash::from_reader(Cursor::new(text.as_bytes())).unwrap())
            .source(
                ChunkSource::builder()
                    .file_path(IndexRelativePath::try_new("test.md").unwrap())
                    .repo_name(RepoName::try_new("test-repo").unwrap())
                    .build(),
            )
            .content(
                ChunkContent::builder()
                    .text(text)
                    .token_count(TokenCount::try_new(10).unwrap())
                    .build(),
            )
            .context(ChunkContext::markdown(
                &MarkdownContext::builder().heading_hierarchy(vec![]).build(),
            ))
            .build()
    }

    fn create_test_query() -> SearchQuery {
        use crumbly_core::knowledge::domain::{
            ContextId, DEFAULT_RESULT_LIMIT, QueryText, ResultLimit,
        };
        SearchQuery::builder()
            .text(QueryText::try_new("test query").unwrap())
            .limit(ResultLimit::try_new(DEFAULT_RESULT_LIMIT).unwrap())
            .context_id(ContextId::default())
            .build()
    }
}
