//! Configuration types for Go source file indexing.

use serde::{Deserialize, Serialize};

use crate::knowledge::domain::{DocLineCount, GoItemType, Visibility};

/// Configuration for indexing Go source files.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct GoConfig {
    /// Visibility levels to index.
    #[serde(default = "default_visibility")]
    pub visibility: Vec<Visibility>,

    /// Item types to index.
    #[serde(default = "default_go_items")]
    pub items: Vec<GoItemType>,

    /// Minimum doc comment length in lines.
    #[serde(default)]
    pub min_doc_lines: usize,
}

impl Default for GoConfig {
    fn default() -> Self {
        Self {
            visibility: default_visibility(),
            items: default_go_items(),
            min_doc_lines: 0,
        }
    }
}

/// Filtering rules for Go source code indexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoFilter {
    visibility: Vec<Visibility>,
    items: Vec<GoItemType>,
    min_doc_lines: DocLineCount,
}

impl GoFilter {
    /// Create a filter with visibility, item types, and minimum documentation length.
    pub fn new(
        visibility: Vec<Visibility>,
        items: Vec<GoItemType>,
        min_doc_lines: DocLineCount,
    ) -> Self {
        Self {
            visibility,
            items,
            min_doc_lines,
        }
    }

    /// Determine whether a Go item should be indexed based on filter criteria.
    pub fn should_index(
        &self,
        visibility: &Visibility,
        item_type: &GoItemType,
        doc_lines: DocLineCount,
    ) -> bool {
        doc_lines >= self.min_doc_lines
            && self.visibility.contains(visibility)
            && (self.items.contains(&GoItemType::All) || self.items.contains(item_type))
    }
}

fn default_visibility() -> Vec<Visibility> {
    vec![Visibility::Public]
}

fn default_go_items() -> Vec<GoItemType> {
    vec![
        GoItemType::Function,
        GoItemType::Method,
        GoItemType::Struct,
        GoItemType::Interface,
        GoItemType::Type,
        GoItemType::Const,
        GoItemType::Var,
    ]
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_go_filter_should_index_checks_doc_lines() {
        let filter = GoFilter::new(
            vec![Visibility::Public],
            vec![GoItemType::Function],
            DocLineCount::new(3),
        );

        let short_doc = filter.should_index(
            &Visibility::Public,
            &GoItemType::Function,
            DocLineCount::new(2),
        );
        let long_doc = filter.should_index(
            &Visibility::Public,
            &GoItemType::Function,
            DocLineCount::new(5),
        );

        assert!(!short_doc);
        assert!(long_doc);
    }

    #[test]
    fn test_go_filter_should_index_checks_visibility() {
        let filter = GoFilter::new(
            vec![Visibility::Public],
            vec![GoItemType::Function],
            DocLineCount::new(0),
        );

        let public = filter.should_index(
            &Visibility::Public,
            &GoItemType::Function,
            DocLineCount::new(100),
        );
        let private = filter.should_index(
            &Visibility::Private,
            &GoItemType::Function,
            DocLineCount::new(100),
        );

        assert!(public);
        assert!(!private);
    }

    #[test]
    fn test_go_filter_should_index_checks_item_type() {
        let filter = GoFilter::new(
            vec![Visibility::Public],
            vec![GoItemType::Struct],
            DocLineCount::new(0),
        );

        let struct_item = filter.should_index(
            &Visibility::Public,
            &GoItemType::Struct,
            DocLineCount::new(100),
        );
        let function_item = filter.should_index(
            &Visibility::Public,
            &GoItemType::Function,
            DocLineCount::new(100),
        );

        assert!(struct_item);
        assert!(!function_item);
    }
}
