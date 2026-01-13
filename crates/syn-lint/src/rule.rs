//! Rule trait and violation reporting.

use std::path::Path;

/// A style violation found by a rule.
#[derive(Debug)]
pub struct Violation {
    /// File path where violation occurred.
    pub file: String,
    /// Line number of the violation.
    pub line: usize,
    /// Description of the violation.
    pub message: String,
    /// Optional documentation URL for the rule.
    pub doc_url: Option<&'static str>,
}

/// A lint rule that checks parsed Rust files.
pub trait Rule: Send + Sync {
    /// Rule identifier.
    fn name(&self) -> &'static str;

    /// Check a parsed file and return violations.
    fn check(&self, path: &Path, file: &syn::File) -> Vec<Violation>;
}

inventory::collect!(&'static dyn Rule);
