use crate::index::errors::IndexError;
use crate::theme;
use crumbly_core::knowledge::domain::SearchResult;
use crumbly_core::knowledge::domain::{
    FileSearchResult, IndexRelativePath, RelevanceScore, RepoName, SearchResults,
};

use snafu::ResultExt;
use std::collections::HashMap;
use std::path::Path;

/// Parses the output format string into an OutputFormat enum
pub(super) fn parse_output_format(format_str: Option<&str>) -> Result<OutputFormat, IndexError> {
    match format_str {
        Some("human") => Ok(OutputFormat::Human),
        Some("json") => Ok(OutputFormat::Json),
        None => Ok(OutputFormat::Human),
        Some(format) => Err(IndexError::InvalidOutputFormat {
            format: format.to_string(),
        }),
    }
}

/// Groups search results by file path, aggregating chunks and computing best scores
pub(super) fn group_results_by_file(results: &SearchResults) -> Vec<FileSearchResult> {
    let mut file_map: HashMap<IndexRelativePath, (RepoName, Vec<SearchResult>)> = HashMap::new();

    for result in &results.results {
        let path = result.chunk.source.file_path.clone();
        let repo = result.chunk.source.repo_name.clone();

        file_map
            .entry(path)
            .or_insert_with(|| (repo, Vec::new()))
            .1
            .push(result.clone());
    }

    let mut file_results: Vec<_> = file_map
        .into_iter()
        .map(|(path, (repo, chunks))| {
            let best_score = chunks
                .iter()
                .map(|r| r.score)
                .max_by(|a, b| a.cmp(b))
                .unwrap_or_else(RelevanceScore::zero);

            FileSearchResult::builder()
                .file_path(path)
                .repo_name(repo)
                .match_count(chunks.len())
                .best_score(best_score)
                .chunks(chunks)
                .build()
        })
        .collect();

    file_results.sort_by(|a, b| {
        b.best_score
            .cmp(&a.best_score)
            .then_with(|| b.match_count.cmp(&a.match_count))
    });

    file_results
}

/// Prints a formatted summary of build operation results
pub(super) fn format_build_result(result: &crumbly_core::knowledge::indexing::IndexResult) {
    println!("{} Index build complete!", theme::success("✓"));
    println!(
        "  Files processed: {}",
        theme::value(result.files_processed)
    );
    println!("  Chunks created: {}", theme::value(result.chunks_affected));
    println!(
        "  Duration: {}",
        theme::muted(format!("{:.2}s", result.duration.as_secs_f64()))
    );
}

/// Prints a formatted summary of update operation results
pub(super) fn format_update_result(result: &crumbly_core::knowledge::indexing::IndexResult) {
    println!("{} Index update complete!", theme::success("✓"));
    println!("  Files added: {}", theme::value(result.files_added));
    println!("  Files updated: {}", theme::value(result.files_updated));
    println!("  Files removed: {}", theme::value(result.files_removed));
    println!(
        "  Chunks affected: {}",
        theme::value(result.chunks_affected)
    );
    println!(
        "  Duration: {}",
        theme::muted(format!("{:.2}s", result.duration.as_secs_f64()))
    );
}

fn compute_display_path(index_root: &Path, file_path: &IndexRelativePath, cwd: &Path) -> String {
    let abs_path = index_root.join(file_path.to_string());
    pathdiff::diff_paths(&abs_path, cwd)
        .unwrap_or(abs_path)
        .display()
        .to_string()
}

/// Prints file search results in human-readable format with color-coded scores
#[expect(clippy::excessive_nesting)]
pub(super) fn format_file_results_human(
    file_results: &[FileSearchResult],
    show_chunks: bool,
    index_root: &Path,
    cwd: &Path,
) {
    if file_results.is_empty() {
        println!("No results");
        return;
    }

    println!("Found {} unique files:\n", theme::value(file_results.len()));

    for (i, file_result) in file_results.iter().enumerate() {
        let score_value = file_result.best_score.into_inner();
        let display_path = compute_display_path(index_root, &file_result.file_path, cwd);

        println!(
            "{}. [Matches: {}, Best Score: {}] {}",
            theme::value(i + 1),
            theme::value(file_result.match_count),
            theme::score(score_value),
            theme::label(display_path)
        );

        if show_chunks {
            for chunk_result in &file_result.chunks {
                let preview = if chunk_result.chunk.content.text.len() > 100 {
                    format!("{}...", &chunk_result.chunk.content.text[..100])
                } else {
                    chunk_result.chunk.content.text.to_string()
                };
                let chunk_score = chunk_result.score.into_inner();
                println!(
                    "   - [Score: {}] {}\n",
                    theme::score(chunk_score),
                    theme::muted(preview.replace('\n', " "))
                );
            }
        }
    }
}

/// Prints file search results as JSON
pub(super) fn format_file_results_json(
    file_results: &[FileSearchResult],
    index_root: &Path,
    cwd: &Path,
) -> Result<(), IndexError> {
    use super::errors::index_error::*;
    use serde_json::json;

    let results_with_display_paths: Vec<_> = file_results
        .iter()
        .map(|fr| {
            json!({
                "file_path": compute_display_path(index_root, &fr.file_path, cwd),
                "repo_name": fr.repo_name,
                "match_count": fr.match_count,
                "best_score": fr.best_score,
            })
        })
        .collect();

    let json = serde_json::to_string_pretty(&results_with_display_paths)
        .context(JsonSerializationFailedSnafu)?;
    println!("{}", json);
    Ok(())
}

/// Prints index status information including metadata and statistics
pub(super) fn format_status(status: &crumbly_core::knowledge::facade::IndexStatus) {
    println!("{}", theme::label("Knowledge Index Status"));
    println!("  Exists: {}", theme::value(status.exists));
    println!("  Chunks: {}", theme::value(status.chunk_count));
    println!("  Files: {}", theme::value(status.file_count));

    if let Some(Ok(elapsed)) = status.last_build.map(|t| t.elapsed()) {
        println!(
            "  Last build: {}",
            theme::muted(format!("{:.0} seconds ago", elapsed.as_secs_f64()))
        );
    }

    println!("  Model: {}", theme::value(&status.model_config.model_name));
    println!(
        "  Embedding dim: {}",
        theme::value(status.model_config.embedding_dim)
    );
    println!(
        "  Max tokens: {}",
        theme::value(status.model_config.max_tokens)
    );

    if let Some(size) = status.size_bytes {
        println!(
            "  Size: {}",
            theme::size(format!("{:.2} MB", size as f64 / 1_000_000.0))
        );
    }
}

/// Prompts the user for yes/no confirmation via stdin
///
/// Returns true if stdin is not a tty (non-interactive mode) to allow
/// scripted usage without the -y flag.
pub(super) fn prompt_confirmation(message: &str) -> Result<bool, IndexError> {
    use super::errors::index_error::*;
    use std::io::IsTerminal;

    // In non-interactive mode, assume yes
    if !std::io::stdin().is_terminal() {
        return Ok(true);
    }

    println!("{} (yes/no): ", message);

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context(InputReadFailedSnafu)?;

    let input = input.trim().to_lowercase();
    Ok(input == "yes" || input == "y")
}

/// Output format for search results
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum OutputFormat {
    Human,
    Json,
}
