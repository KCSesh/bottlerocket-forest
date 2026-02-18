//! Shell documentation context types.

use bon::Builder;
use serde::{Deserialize, Serialize};

use crate::knowledge::domain::{ItemName, Signature};

/// Type of shell item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ShellItemType {
    /// All item types.
    All,
    /// Function definition.
    Function,
    /// Standalone comment block.
    StandaloneComment,
}

/// Shell item metadata for comment context.
#[derive(Debug, Clone, PartialEq, Eq, Builder, Serialize, Deserialize)]
#[builder(on(_, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct ShellDocContext {
    /// Name of the documented item (None for standalone comments).
    pub item_name: Option<ItemName>,
    /// Kind of shell item.
    pub item_type: ShellItemType,
    /// Function signature if applicable.
    pub signature: Option<Signature>,
}
