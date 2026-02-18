//! Configuration types for JavaScript/TypeScript source file indexing.

use serde::{Deserialize, Serialize};

use super::context::JsItemType;
use crate::knowledge::domain::{DocLineCount, Visibility};

/// Configuration for JavaScript source file indexing.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct JsFilterConfig {
    /// Visibility levels to index.
    #[serde(default = "default_visibility")]
    pub visibility: Vec<Visibility>,
    /// Item types to index.
    #[serde(default = "default_js_items")]
    pub items: Vec<JsFilterItemType>,
    /// Minimum doc comment length in lines.
    #[serde(default)]
    pub min_doc_lines: usize,
}

impl JsFilterConfig {
    /// Convert to a JsFilter for use during indexing.
    pub fn to_filter(&self) -> JsFilter {
        let items: Vec<_> = self.items.iter().map(|i| i.to_domain_type()).collect();
        JsFilter::new(
            self.visibility.clone(),
            items,
            DocLineCount::new(self.min_doc_lines),
        )
    }
}

impl Default for JsFilterConfig {
    fn default() -> Self {
        Self {
            visibility: default_visibility(),
            items: default_js_items(),
            min_doc_lines: 0,
        }
    }
}

/// Filtering rules for JS/TS source code indexing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JsFilter {
    visibility: Vec<Visibility>,
    items: Vec<JsItemType>,
    min_doc_lines: DocLineCount,
}

impl JsFilter {
    /// Create a filter with visibility, item types, and minimum documentation length.
    pub fn new(
        visibility: Vec<Visibility>,
        items: Vec<JsItemType>,
        min_doc_lines: DocLineCount,
    ) -> Self {
        Self {
            visibility,
            items,
            min_doc_lines,
        }
    }

    /// Determine whether a JS/TS item should be indexed based on filter criteria.
    pub fn should_index(
        &self,
        visibility: &Visibility,
        item_type: &JsItemType,
        doc_lines: DocLineCount,
    ) -> bool {
        doc_lines >= self.min_doc_lines
            && self.visibility.contains(visibility)
            && (self.items.contains(&JsItemType::All) || self.items.contains(item_type))
    }
}

/// Categories of JS/TS language items that can be filtered during indexing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JsFilterItemType {
    /// All item types.
    All,
    /// Function declarations.
    #[serde(rename = "functions")]
    Function,
    /// Class declarations.
    #[serde(rename = "classes")]
    Class,
    /// Variable declarations.
    #[serde(rename = "variables")]
    Variable,
    /// Interface declarations (TS).
    #[serde(rename = "interfaces")]
    Interface,
    /// Type alias declarations (TS).
    #[serde(rename = "type-aliases")]
    TypeAlias,
    /// Enum declarations (TS).
    #[serde(rename = "enums")]
    Enum,
}

impl JsFilterItemType {
    pub(super) fn to_domain_type(self) -> JsItemType {
        match self {
            Self::All => JsItemType::All,
            Self::Function => JsItemType::Function,
            Self::Class => JsItemType::Class,
            Self::Variable => JsItemType::Variable,
            Self::Interface => JsItemType::Interface,
            Self::TypeAlias => JsItemType::TypeAlias,
            Self::Enum => JsItemType::Enum,
        }
    }
}

fn default_visibility() -> Vec<Visibility> {
    vec![Visibility::Public]
}

fn default_js_items() -> Vec<JsFilterItemType> {
    vec![
        JsFilterItemType::Function,
        JsFilterItemType::Class,
        JsFilterItemType::Variable,
        JsFilterItemType::Interface,
        JsFilterItemType::TypeAlias,
        JsFilterItemType::Enum,
    ]
}
