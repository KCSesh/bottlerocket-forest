use snafu::Snafu;

#[derive(Debug, Snafu, miette::Diagnostic)]
#[snafu(module, visibility(pub))]
pub enum IndexError {
    #[snafu(display("Knowledge index operation failed"))]
    #[diagnostic(code(sembly::cli::knowledge_index_failed), forward(source))]
    KnowledgeIndex {
        source: sembly_core::knowledge::IndexError,
    },

    #[snafu(display("Invalid context path: {path}"))]
    #[diagnostic(
        code(sembly::cli::invalid_context_path),
        help("Context path must be a valid relative path within the workspace")
    )]
    InvalidContextPath {
        path: String,
        source: sembly_core::knowledge::domain::ContextIdError,
    },

    #[snafu(display("Context path does not exist: {path}"))]
    #[diagnostic(
        code(sembly::cli::context_path_not_found),
        help("The specified context path must exist on the filesystem")
    )]
    ContextPathNotFound { path: String },

    #[snafu(display("Invalid output format: {format}"))]
    #[diagnostic(
        code(sembly::cli::invalid_output_format),
        help("Must be 'human' or 'json'")
    )]
    InvalidOutputFormat { format: String },

    #[snafu(display("Failed to read user input"))]
    #[diagnostic(
        code(sembly::cli::input_read_failed),
        help("Check that stdin is available and not closed")
    )]
    InputReadFailed { source: std::io::Error },

    #[snafu(display("Failed to serialize JSON output"))]
    #[diagnostic(
        code(sembly::cli::json_serialization_failed),
        help("The search results may contain invalid data")
    )]
    JsonSerializationFailed { source: serde_json::Error },
}
