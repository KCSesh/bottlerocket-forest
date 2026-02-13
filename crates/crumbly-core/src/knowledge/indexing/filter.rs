//! Filtering rules for controlling indexing scope
//!
//! Provides [`IndexingFilter`] for file-level filtering and language-specific
//! configuration. Filters are typically constructed from configuration loaded via
//! [`CrumblyConfig`](super::config::CrumblyConfig).

use std::collections::{HashMap, HashSet};

use crate::knowledge::chunking::{LanguageConfig, LanguageSupport};
use crate::knowledge::domain::FileType;

// Re-export language-specific filters from their modules
pub use crate::knowledge::chunking::godoc::{GoConfig, GoFilter};
pub use crate::knowledge::chunking::javadoc::JavaFilter;
pub use crate::knowledge::chunking::rustdoc::{RustFilter, RustItemType};

/// Combined filtering rules for file types and language-specific criteria.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct IndexingFilter {
    enabled_types: HashSet<String>,
    language_configs: HashMap<String, LanguageConfig>,
}

impl IndexingFilter {
    /// Create a filter with enabled file types and language-specific configurations.
    pub fn new(
        enabled_types: HashSet<String>,
        language_configs: HashMap<String, LanguageConfig>,
    ) -> Self {
        Self {
            enabled_types,
            language_configs,
        }
    }

    /// Determine whether a file type should be indexed.
    pub fn should_index_file_type(&self, file_type: &FileType) -> bool {
        self.enabled_types.contains(file_type.context_type_name())
    }

    /// Access the language-specific configuration for a given type.
    pub fn language_config(&self, type_name: &str) -> Option<&LanguageConfig> {
        self.language_configs.get(type_name)
    }

    /// Check if a language type is enabled.
    pub fn is_enabled(&self, type_name: &str) -> bool {
        self.enabled_types.contains(type_name)
    }
}

impl Default for IndexingFilter {
    fn default() -> Self {
        // Default to all registered languages enabled
        let enabled_types: HashSet<String> = inventory::iter::<&dyn LanguageSupport>
            .into_iter()
            .map(|lang| lang.context_type_name().to_string())
            .collect();

        // Collect default configs from all registered languages
        let language_configs: HashMap<String, LanguageConfig> =
            inventory::iter::<&dyn LanguageSupport>
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
    use crate::knowledge::domain::{DocLineCount, Visibility};

    #[test]
    fn test_rust_filter_should_index_checks_doc_lines() {
        let filter = RustFilter::new(
            vec![Visibility::Public],
            vec![RustItemType::Function],
            DocLineCount::new(3),
        );

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

        assert!(!short_doc);
        assert!(long_doc);
    }

    #[test]
    fn test_rust_filter_should_index_checks_visibility() {
        let filter = RustFilter::new(
            vec![Visibility::Public],
            vec![RustItemType::Function],
            DocLineCount::new(0),
        );

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

        assert!(public);
        assert!(!private);
    }

    #[test]
    fn test_rust_filter_should_index_checks_item_type() {
        let filter = RustFilter::new(
            vec![Visibility::Public],
            vec![RustItemType::Struct],
            DocLineCount::new(0),
        );

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

        assert!(struct_item);
        assert!(!function_item);
    }

    #[test]
    fn test_indexing_filter_should_index_file_type() {
        let filter = IndexingFilter::new(
            ["markdown"].into_iter().map(String::from).collect(),
            HashMap::new(),
        );

        let markdown = filter.should_index_file_type(&FileType::new("markdown"));
        let rust = filter.should_index_file_type(&FileType::new("rust_doc"));

        assert!(markdown);
        assert!(!rust);
    }

    #[test]
    fn test_indexing_filter_language_config_returns_reference() {
        let rust_filter = RustFilter::new(vec![Visibility::Public], vec![], DocLineCount::new(0));
        let rust_config = LanguageConfig::new(&rust_filter).unwrap();
        let mut configs = HashMap::new();
        configs.insert("rust_doc".to_string(), rust_config);
        let filter = IndexingFilter::new(HashSet::new(), configs);

        let result = filter.language_config("rust_doc");

        assert!(result.is_some());
    }
}
