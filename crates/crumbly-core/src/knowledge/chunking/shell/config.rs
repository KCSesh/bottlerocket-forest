//! Configuration types for shell script indexing.

use serde::{Deserialize, Serialize};

use super::context::ShellItemType;
use crate::knowledge::domain::DocLineCount;

/// Configuration for indexing shell scripts.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ShellConfig {
    /// Item types to index.
    #[serde(default = "default_shell_items")]
    pub items: Vec<ShellItemType>,

    /// Minimum doc comment length in lines.
    #[serde(default)]
    pub min_doc_lines: usize,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            items: default_shell_items(),
            min_doc_lines: 0,
        }
    }
}

/// Filtering rules for shell script indexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellFilter {
    items: Vec<ShellItemType>,
    min_doc_lines: DocLineCount,
}

impl ShellFilter {
    /// Create a filter with item types and minimum documentation length.
    pub fn new(items: Vec<ShellItemType>, min_doc_lines: DocLineCount) -> Self {
        Self {
            items,
            min_doc_lines,
        }
    }

    /// Determine whether a shell item should be indexed based on filter criteria.
    pub fn should_index(&self, item_type: &ShellItemType, doc_lines: DocLineCount) -> bool {
        doc_lines >= self.min_doc_lines
            && (self.items.contains(&ShellItemType::All) || self.items.contains(item_type))
    }
}

fn default_shell_items() -> Vec<ShellItemType> {
    vec![ShellItemType::Function, ShellItemType::StandaloneComment]
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_shell_filter_should_index_checks_doc_lines() {
        // Given a filter requiring minimum 3 doc lines
        let filter = ShellFilter::new(vec![ShellItemType::Function], DocLineCount::new(3));

        // When checking items with different doc line counts
        let short_doc = filter.should_index(&ShellItemType::Function, DocLineCount::new(2));
        let long_doc = filter.should_index(&ShellItemType::Function, DocLineCount::new(5));

        // Then short docs are rejected and long docs are accepted
        assert!(!short_doc);
        assert!(long_doc);
    }

    #[test]
    fn test_shell_filter_should_index_checks_item_type() {
        // Given a filter for functions only
        let filter = ShellFilter::new(vec![ShellItemType::Function], DocLineCount::new(0));

        // When checking different item types
        let function_item = filter.should_index(&ShellItemType::Function, DocLineCount::new(100));
        let standalone_item =
            filter.should_index(&ShellItemType::StandaloneComment, DocLineCount::new(100));

        // Then functions are accepted and standalone comments are rejected
        assert!(function_item);
        assert!(!standalone_item);
    }
}
