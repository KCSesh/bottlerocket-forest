//! Configuration types for Java source file indexing.

use serde::{Deserialize, Serialize};

use crate::knowledge::domain::{DocLineCount, JavaItemType, Visibility};

/// Configuration for Java source file indexing.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct JavaFilterConfig {
    /// Visibility levels to index.
    #[serde(default = "default_visibility")]
    pub visibility: Vec<Visibility>,
    /// Item types to index.
    #[serde(default = "default_java_items")]
    pub items: Vec<JavaFilterItemType>,
    /// Minimum doc comment length in lines.
    #[serde(default)]
    pub min_doc_lines: usize,
}

impl JavaFilterConfig {
    /// Convert to a JavaFilter for use during indexing.
    pub fn to_filter(&self) -> JavaFilter {
        let items: Vec<_> = self.items.iter().map(|i| i.to_domain_type()).collect();
        JavaFilter::new(
            self.visibility.clone(),
            items,
            DocLineCount::new(self.min_doc_lines),
        )
    }
}

impl Default for JavaFilterConfig {
    fn default() -> Self {
        Self {
            visibility: default_visibility(),
            items: default_java_items(),
            min_doc_lines: 0,
        }
    }
}

/// Filtering rules for Java source code indexing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JavaFilter {
    visibility: Vec<Visibility>,
    items: Vec<JavaItemType>,
    min_doc_lines: DocLineCount,
}

impl JavaFilter {
    /// Create a filter with visibility, item types, and minimum documentation length.
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

    /// Determine whether a Java item should be indexed based on filter criteria.
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

/// Categories of Java language items that can be filtered during indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JavaFilterItemType {
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

impl JavaFilterItemType {
    pub(super) fn to_domain_type(self) -> JavaItemType {
        match self {
            Self::All => JavaItemType::All,
            Self::Class => JavaItemType::Class,
            Self::Interface => JavaItemType::Interface,
            Self::Enum => JavaItemType::Enum,
            Self::Record => JavaItemType::Record,
            Self::Method => JavaItemType::Method,
            Self::Field => JavaItemType::Field,
            Self::Constructor => JavaItemType::Constructor,
            Self::Annotation => JavaItemType::Annotation,
        }
    }
}

fn default_visibility() -> Vec<Visibility> {
    vec![Visibility::Public]
}

fn default_java_items() -> Vec<JavaFilterItemType> {
    vec![
        JavaFilterItemType::Class,
        JavaFilterItemType::Interface,
        JavaFilterItemType::Enum,
        JavaFilterItemType::Record,
        JavaFilterItemType::Method,
        JavaFilterItemType::Field,
        JavaFilterItemType::Constructor,
        JavaFilterItemType::Annotation,
    ]
}
