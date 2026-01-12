use snafu::Snafu;

#[derive(Debug, Snafu, miette::Diagnostic)]
#[snafu(module, visibility(pub))]
pub enum IndexError {
    #[snafu(display("Knowledge index operation failed"))]
    #[diagnostic(code(crumbly::cli::knowledge_index_failed), forward(source))]
    KnowledgeIndex {
        source: crumbly_core::knowledge::IndexError,
    },

    #[snafu(display("Invalid search query"))]
    #[diagnostic(
        code(crumbly::cli::invalid_query),
        help("Provide a non-empty query string")
    )]
    InvalidQuery {
        source: crumbly_core::knowledge::domain::QueryTextError,
    },

    #[snafu(display("Invalid result limit"))]
    #[diagnostic(
        code(crumbly::cli::invalid_result_limit),
        help("Limit must be between 1 and 100")
    )]
    InvalidResultLimit {
        source: crumbly_core::knowledge::domain::ResultLimitError,
    },

    #[snafu(display("Invalid context path: {path}"))]
    #[diagnostic(
        code(crumbly::cli::invalid_context_path),
        help("Context path must be a valid relative path within the workspace")
    )]
    InvalidContextPath {
        path: String,
        source: crumbly_core::knowledge::domain::ContextIdError,
    },

    #[snafu(display("Context path does not exist: {path}"))]
    #[diagnostic(
        code(crumbly::cli::context_path_not_found),
        help("The specified context path must exist on the filesystem")
    )]
    ContextPathNotFound { path: String },

    #[snafu(display("Invalid output format: {format}"))]
    #[diagnostic(
        code(crumbly::cli::invalid_output_format),
        help("Must be 'human' or 'json'")
    )]
    InvalidOutputFormat { format: String },

    #[snafu(display("Failed to read user input"))]
    #[diagnostic(
        code(crumbly::cli::input_read_failed),
        help("Check that stdin is available and not closed")
    )]
    InputReadFailed { source: std::io::Error },

    #[snafu(display("Failed to serialize JSON output"))]
    #[diagnostic(
        code(crumbly::cli::json_serialization_failed),
        help("The search results may contain invalid data")
    )]
    JsonSerializationFailed { source: serde_json::Error },

    #[snafu(display("Failed to get current directory"))]
    #[diagnostic(
        code(crumbly::cli::get_current_dir_failed),
        help("Check that the current directory exists and is accessible")
    )]
    GetCurrentDir { source: std::io::Error },
}
