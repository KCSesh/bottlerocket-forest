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
/// ```ignore
/// use snafu::ResultExt;
/// use crate::knowledge::error::box_err;
///
/// // Instead of:
/// value.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
///     .context(SomeSnafu)?;
///
/// // Write:
/// value.map_err(box_err).context(SomeSnafu)?;
/// ```
pub fn box_err<E: std::error::Error + Send + Sync + 'static>(
    e: E,
) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(e)
}
