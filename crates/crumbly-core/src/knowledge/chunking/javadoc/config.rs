//! Configuration types for Java source file indexing.

use crate::knowledge::domain::{DocLineCount, Visibility};
use crate::knowledge::indexing::JavaFilter;

/// Configuration for Java source file indexing.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
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
        let items: Vec<_> = self.items.iter().map(|i| i.to_indexing_type()).collect();
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

/// Categories of Java language items that can be filtered during indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    pub(super) fn to_indexing_type(self) -> crate::knowledge::indexing::JavaItemType {
        use crate::knowledge::indexing::JavaItemType as IndexingType;
        match self {
            Self::All => IndexingType::All,
            Self::Class => IndexingType::Class,
            Self::Interface => IndexingType::Interface,
            Self::Enum => IndexingType::Enum,
            Self::Record => IndexingType::Record,
            Self::Method => IndexingType::Method,
            Self::Field => IndexingType::Field,
            Self::Constructor => IndexingType::Constructor,
            Self::Annotation => IndexingType::Annotation,
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
