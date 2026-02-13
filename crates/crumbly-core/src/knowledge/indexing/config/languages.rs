//! Language-specific configuration types for indexing
//!
//! Contains configuration structs for Rust source file indexing,
//! including visibility filters, item type selection, and documentation requirements.

use serde::{Deserialize, Serialize};

use crate::knowledge::chunking::rustdoc::RustItemType;
use crate::knowledge::domain::Visibility;

/// Configuration for indexing Rust source files.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RustConfig {
    /// Visibility levels to index.
    #[serde(default = "default_rust_visibility")]
    pub visibility: Vec<Visibility>,

    /// Item types to index ("all" or specific types).
    #[serde(
        default = "default_rust_items",
        deserialize_with = "deserialize_rust_items"
    )]
    pub items: Vec<RustItemType>,

    /// Minimum doc comment length in lines.
    #[serde(default)]
    pub min_doc_lines: usize,
}

fn deserialize_rust_items<'de, D>(deserializer: D) -> Result<Vec<RustItemType>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let strings: Vec<String> = Vec::deserialize(deserializer)?;

    if strings.len() == 1 && strings[0] == "all" {
        return Ok(default_rust_items());
    }

    strings
        .into_iter()
        .map(|s| match s.as_str() {
            "modules" => Ok(RustItemType::Module),
            "functions" => Ok(RustItemType::Function),
            "structs" => Ok(RustItemType::Struct),
            "enums" => Ok(RustItemType::Enum),
            "traits" => Ok(RustItemType::Trait),
            "impls" => Ok(RustItemType::Impl),
            "type-aliases" => Ok(RustItemType::TypeAlias),
            "constants" => Ok(RustItemType::Constant),
            _ => Err(D::Error::custom(format!("unknown item type: {}", s))),
        })
        .collect()
}

impl Default for RustConfig {
    fn default() -> Self {
        Self {
            visibility: default_rust_visibility(),
            items: default_rust_items(),
            min_doc_lines: 0,
        }
    }
}

fn default_rust_visibility() -> Vec<Visibility> {
    vec![Visibility::Public]
}

fn default_rust_items() -> Vec<RustItemType> {
    vec![
        RustItemType::Module,
        RustItemType::Function,
        RustItemType::Struct,
        RustItemType::Enum,
        RustItemType::Trait,
        RustItemType::Impl,
        RustItemType::TypeAlias,
        RustItemType::Constant,
    ]
}
