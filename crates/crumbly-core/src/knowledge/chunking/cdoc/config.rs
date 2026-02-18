//! Configuration types for C source file indexing.

use serde::{Deserialize, Serialize};

use super::context::CItemType;
use crate::knowledge::domain::DocLineCount;

/// Configuration for indexing C source files.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CConfig {
    /// Item types to index.
    #[serde(default = "default_c_items")]
    pub items: Vec<CItemType>,

    /// Minimum doc comment length in lines.
    #[serde(default)]
    pub min_doc_lines: usize,
}

impl Default for CConfig {
    fn default() -> Self {
        Self {
            items: default_c_items(),
            min_doc_lines: 0,
        }
    }
}

/// Filtering rules for C source code indexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CFilter {
    items: Vec<CItemType>,
    min_doc_lines: DocLineCount,
}

impl CFilter {
    /// Create a filter with item types and minimum documentation length.
    pub fn new(items: Vec<CItemType>, min_doc_lines: DocLineCount) -> Self {
        Self {
            items,
            min_doc_lines,
        }
    }

    /// Determine whether a C item should be indexed based on filter criteria.
    pub fn should_index(&self, item_type: &CItemType, doc_lines: DocLineCount) -> bool {
        doc_lines >= self.min_doc_lines
            && (self.items.contains(&CItemType::All) || self.items.contains(item_type))
    }
}

fn default_c_items() -> Vec<CItemType> {
    vec![
        CItemType::Function,
        CItemType::Struct,
        CItemType::Enum,
        CItemType::Typedef,
        CItemType::Macro,
        CItemType::Declaration,
        CItemType::StandaloneComment,
    ]
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_c_filter_should_index_checks_doc_lines() {
        // Given a filter requiring minimum 3 doc lines
        let filter = CFilter::new(vec![CItemType::Function], DocLineCount::new(3));

        // When checking items with different doc line counts
        let short_doc = filter.should_index(&CItemType::Function, DocLineCount::new(2));
        let long_doc = filter.should_index(&CItemType::Function, DocLineCount::new(5));

        // Then short docs are rejected and long docs are accepted
        assert!(!short_doc);
        assert!(long_doc);
    }

    #[test]
    fn test_c_filter_should_index_checks_item_type() {
        // Given a filter for structs only
        let filter = CFilter::new(vec![CItemType::Struct], DocLineCount::new(0));

        // When checking different item types
        let struct_item = filter.should_index(&CItemType::Struct, DocLineCount::new(100));
        let function_item = filter.should_index(&CItemType::Function, DocLineCount::new(100));

        // Then structs are accepted and functions are rejected
        assert!(struct_item);
        assert!(!function_item);
    }
}
