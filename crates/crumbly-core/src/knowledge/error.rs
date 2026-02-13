//! Error handling utilities
//!
//! Provides helpers for common error handling patterns.

/// Boxes an error for use with snafu context selectors.
///
/// This is a convenience function for the common pattern of boxing an error
/// before passing it to a snafu context selector that expects a boxed error.
///
/// # Example
///
/// Instead of:
/// ```text
/// value.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
///     .context(SomeSnafu)?;
/// ```
///
/// Write:
/// ```text
/// value.map_err(box_err).context(SomeSnafu)?;
/// ```
pub fn box_err<E: std::error::Error + Send + Sync + 'static>(
    e: E,
) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(e)
}
