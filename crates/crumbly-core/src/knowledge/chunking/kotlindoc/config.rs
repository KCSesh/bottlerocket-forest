//! Configuration types for Kotlin source file indexing.

use serde::{Deserialize, Serialize};

use super::context::KotlinItemType;
use crate::knowledge::domain::{DocLineCount, Visibility};

/// Configuration for Kotlin source file indexing.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct KotlinFilterConfig {
    /// Visibility levels to index.
    #[serde(default = "default_visibility")]
    pub visibility: Vec<Visibility>,
    /// Item types to index.
    #[serde(default = "default_kotlin_items")]
    pub items: Vec<KotlinFilterItemType>,
    /// Minimum doc comment length in lines.
    #[serde(default)]
    pub min_doc_lines: usize,
}

impl KotlinFilterConfig {
    /// Convert to a KotlinFilter for use during indexing.
    pub fn to_filter(&self) -> KotlinFilter {
        let items: Vec<_> = self.items.iter().map(|i| i.to_domain_type()).collect();
        KotlinFilter::new(
            self.visibility.clone(),
            items,
            DocLineCount::new(self.min_doc_lines),
        )
    }
}

impl Default for KotlinFilterConfig {
    fn default() -> Self {
        Self {
            visibility: default_visibility(),
            items: default_kotlin_items(),
            min_doc_lines: 0,
        }
    }
}

/// Filtering rules for Kotlin source code indexing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KotlinFilter {
    visibility: Vec<Visibility>,
    items: Vec<KotlinItemType>,
    min_doc_lines: DocLineCount,
}

impl KotlinFilter {
    /// Create a filter with visibility, item types, and minimum documentation length.
    pub fn new(
        visibility: Vec<Visibility>,
        items: Vec<KotlinItemType>,
        min_doc_lines: DocLineCount,
    ) -> Self {
        Self {
            visibility,
            items,
            min_doc_lines,
        }
    }

    /// Determine whether a Kotlin item should be indexed based on filter criteria.
    pub fn should_index(
        &self,
        visibility: &Visibility,
        item_type: &KotlinItemType,
        doc_lines: DocLineCount,
    ) -> bool {
        doc_lines >= self.min_doc_lines
            && self.visibility.contains(visibility)
            && (self.items.contains(&KotlinItemType::All) || self.items.contains(item_type))
    }
}

/// Categories of Kotlin language items that can be filtered during indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KotlinFilterItemType {
    /// All item types.
    All,
    /// Class definitions.
    #[serde(rename = "classes")]
    Class,
    /// Object definitions.
    #[serde(rename = "objects")]
    Object,
    /// Interface definitions.
    #[serde(rename = "interfaces")]
    Interface,
    /// Function definitions.
    #[serde(rename = "functions")]
    Function,
    /// Property definitions.
    #[serde(rename = "properties")]
    Property,
    /// Enum definitions.
    #[serde(rename = "enums")]
    Enum,
}

impl KotlinFilterItemType {
    pub(super) fn to_domain_type(self) -> KotlinItemType {
        match self {
            Self::All => KotlinItemType::All,
            Self::Class => KotlinItemType::Class,
            Self::Object => KotlinItemType::Object,
            Self::Interface => KotlinItemType::Interface,
            Self::Function => KotlinItemType::Function,
            Self::Property => KotlinItemType::Property,
            Self::Enum => KotlinItemType::Enum,
        }
    }
}

fn default_visibility() -> Vec<Visibility> {
    vec![Visibility::Public]
}

fn default_kotlin_items() -> Vec<KotlinFilterItemType> {
    vec![
        KotlinFilterItemType::Class,
        KotlinFilterItemType::Object,
        KotlinFilterItemType::Interface,
        KotlinFilterItemType::Function,
        KotlinFilterItemType::Property,
        KotlinFilterItemType::Enum,
    ]
}
