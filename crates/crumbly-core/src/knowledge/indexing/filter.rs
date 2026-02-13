//! Filtering rules for controlling indexing scope
//!
//! Provides [`IndexingFilter`] for file-level filtering and [`RustFilter`] for
//! Rust-specific filtering based on visibility, item type, and documentation
//! length. Filters are typically constructed from configuration loaded via
//! [`CrumblyConfig`](super::config::CrumblyConfig).

use serde::{Deserialize, Serialize};

use crate::knowledge::domain::{DocLineCount, Visibility};

// Re-export RustItemType and RustFilter from the rustdoc chunking module
pub use crate::knowledge::chunking::rustdoc::{RustFilter, RustItemType};

/// Categories of Java language items that can be filtered during indexing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JavaItemType {
    /// All item types.
    All,
    /// Class definitions.
    #[serde(rename = "classes")]
    Class,
    /// Interface definitions.
    #[serde(rename = "interfaces")]
    Interface,
    /// Enum definitions.
    #[serde(rename = "enums")]
    Enum,
    /// Record definitions.
    #[serde(rename = "records")]
    Record,
    /// Method definitions.
    #[serde(rename = "methods")]
    Method,
    /// Field definitions.
    #[serde(rename = "fields")]
    Field,
    /// Constructor definitions.
    #[serde(rename = "constructors")]
    Constructor,
    /// Annotation definitions.
    #[serde(rename = "annotations")]
    Annotation,
}

/// Categories of Go language items that can be filtered during indexing
///
/// Unlike RustItemType, includes an `All` variant for convenience in configuration.
/// This allows `items = ["all"]` in config rather than listing all types explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GoItemType {
    /// All item types.
    All,
    /// Function definitions.
    #[serde(rename = "functions")]
    Function,
    /// Method definitions.
    #[serde(rename = "methods")]
    Method,
    /// Struct definitions.
    #[serde(rename = "structs")]
    Struct,
    /// Interface definitions.
    #[serde(rename = "interfaces")]
    Interface,
    /// Type definitions.
    #[serde(rename = "types")]
    Type,
    /// Constant definitions.
    #[serde(rename = "constants")]
    Const,
    /// Variable definitions.
    #[serde(rename = "variables")]
    Var,
}

/// Filtering rules for Java source code indexing
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JavaFilter {
    visibility: Vec<Visibility>,
    items: Vec<JavaItemType>,
    min_doc_lines: DocLineCount,
}

impl JavaFilter {
    /// Create a filter with visibility, item types, and minimum documentation length
    pub fn new(
        visibility: Vec<Visibility>,
        items: Vec<JavaItemType>,
        min_doc_lines: DocLineCount,
    ) -> Self {
        Self {
            visibility,
            items,
            min_doc_lines,
        }
    }

    /// Determine whether a Java item should be indexed based on filter criteria
    pub fn should_index(
        &self,
        visibility: &Visibility,
        item_type: &JavaItemType,
        doc_lines: DocLineCount,
    ) -> bool {
        doc_lines >= self.min_doc_lines
            && self.visibility.contains(visibility)
            && (self.items.contains(&JavaItemType::All) || self.items.contains(item_type))
    }
}

/// Filtering rules for Go source code indexing
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoFilter {
    visibility: Vec<Visibility>,
    items: Vec<GoItemType>,
    min_doc_lines: DocLineCount,
}

impl GoFilter {
    /// Create a filter with visibility, item types, and minimum documentation length
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

    /// Determine whether a Go item should be indexed based on filter criteria
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

/// Combined filtering rules for file types and language-specific criteria
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct IndexingFilter {
    enabled_types: std::collections::HashSet<String>,
    language_configs: std::collections::HashMap<String, crate::knowledge::chunking::LanguageConfig>,
}

impl IndexingFilter {
    /// Create a filter with enabled file types and language-specific configurations
    pub fn new(
        enabled_types: std::collections::HashSet<String>,
        language_configs: std::collections::HashMap<
            String,
            crate::knowledge::chunking::LanguageConfig,
        >,
    ) -> Self {
        Self {
            enabled_types,
            language_configs,
        }
    }

    /// Determine whether a file type should be indexed
    pub fn should_index_file_type(&self, file_type: &crate::knowledge::domain::FileType) -> bool {
        self.enabled_types.contains(file_type.context_type_name())
    }

    /// Access the language-specific configuration for a given type
    pub fn language_config(
        &self,
        type_name: &str,
    ) -> Option<&crate::knowledge::chunking::LanguageConfig> {
        self.language_configs.get(type_name)
    }

    /// Check if a language type is enabled
    pub fn is_enabled(&self, type_name: &str) -> bool {
        self.enabled_types.contains(type_name)
    }
}

impl Default for IndexingFilter {
    fn default() -> Self {
        use crate::knowledge::chunking::LanguageSupport;

        // Default to all registered languages enabled
        let enabled_types: std::collections::HashSet<String> =
            inventory::iter::<&dyn LanguageSupport>
                .into_iter()
                .map(|lang| lang.context_type_name().to_string())
                .collect();

        // Collect default configs from all registered languages
        let language_configs: std::collections::HashMap<
            String,
            crate::knowledge::chunking::LanguageConfig,
        > = inventory::iter::<&dyn LanguageSupport>
            .into_iter()
            .filter_map(|lang| {
                lang.default_config()
                    .map(|cfg| (lang.context_type_name().to_string(), cfg))
            })
            .collect();

        Self {
            enabled_types,
            language_configs,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rust_filter_should_index_checks_doc_lines() {
        // Given A filter with minimum doc lines
        let filter = RustFilter::new(
            vec![Visibility::Public],
            vec![RustItemType::Function],
            DocLineCount::new(3),
        );

        // When Checking items with different doc line counts
        let short_doc = filter.should_index(
            &Visibility::Public,
            &RustItemType::Function,
            DocLineCount::new(2),
        );
        let long_doc = filter.should_index(
            &Visibility::Public,
            &RustItemType::Function,
            DocLineCount::new(5),
        );

        // Then Only long docs should pass
        assert!(!short_doc);
        assert!(long_doc);
    }

    #[test]
    fn test_rust_filter_should_index_checks_visibility() {
        // Given A filter for public items only
        let filter = RustFilter::new(
            vec![Visibility::Public],
            vec![RustItemType::Function],
            DocLineCount::new(0),
        );

        // When Checking items with different visibility
        let public = filter.should_index(
            &Visibility::Public,
            &RustItemType::Function,
            DocLineCount::new(100),
        );
        let private = filter.should_index(
            &Visibility::Private,
            &RustItemType::Function,
            DocLineCount::new(100),
        );

        // Then Only public items should pass
        assert!(public);
        assert!(!private);
    }

    #[test]
    fn test_rust_filter_should_index_checks_item_type() {
        // Given A filter for structs only
        let filter = RustFilter::new(
            vec![Visibility::Public],
            vec![RustItemType::Struct],
            DocLineCount::new(0),
        );

        // When Checking different item types
        let struct_item = filter.should_index(
            &Visibility::Public,
            &RustItemType::Struct,
            DocLineCount::new(100),
        );
        let function_item = filter.should_index(
            &Visibility::Public,
            &RustItemType::Function,
            DocLineCount::new(100),
        );

        // Then Only structs should pass
        assert!(struct_item);
        assert!(!function_item);
    }

    #[test]
    fn test_indexing_filter_should_index_file_type() {
        // Given A filter with only markdown enabled
        use crate::knowledge::domain::FileType;
        let filter = IndexingFilter::new(
            ["markdown"].into_iter().map(String::from).collect(),
            std::collections::HashMap::new(),
        );

        // When Checking different file types
        let markdown = filter.should_index_file_type(&FileType::new("markdown"));
        let rust = filter.should_index_file_type(&FileType::new("rust_doc"));

        // Then Only markdown should pass
        assert!(markdown);
        assert!(!rust);
    }

    #[test]
    fn test_indexing_filter_language_config_returns_reference() {
        // Given A filter with Rust config
        use crate::knowledge::chunking::LanguageConfig;
        let rust_filter = RustFilter::new(vec![Visibility::Public], vec![], DocLineCount::new(0));
        let rust_config = LanguageConfig::new(&rust_filter).unwrap();
        let mut configs = std::collections::HashMap::new();
        configs.insert("rust_doc".to_string(), rust_config);
        let filter = IndexingFilter::new(std::collections::HashSet::new(), configs);

        // When Getting the language config
        let result = filter.language_config("rust_doc");

        // Then It should return a reference
        assert!(result.is_some());
    }
}

#[test]
fn test_go_filter_should_index_checks_doc_lines() {
    // Given a filter requiring minimum 3 doc lines
    let filter = GoFilter::new(
        vec![Visibility::Public],
        vec![GoItemType::Function],
        DocLineCount::new(3),
    );

    // When checking items with different doc line counts
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

    // Then only items meeting the threshold should be indexed
    assert!(!short_doc);
    assert!(long_doc);
}

#[test]
fn test_go_filter_should_index_checks_visibility() {
    // Given a filter accepting only public items
    let filter = GoFilter::new(
        vec![Visibility::Public],
        vec![GoItemType::Function],
        DocLineCount::new(0),
    );

    // When checking items with different visibilities
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

    // Then only public items should be indexed
    assert!(public);
    assert!(!private);
}

#[test]
fn test_go_filter_should_index_checks_item_type() {
    // Given a filter accepting only struct items
    let filter = GoFilter::new(
        vec![Visibility::Public],
        vec![GoItemType::Struct],
        DocLineCount::new(0),
    );

    // When checking items with different types
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

    // Then only struct items should be indexed
    assert!(struct_item);
    assert!(!function_item);
}
