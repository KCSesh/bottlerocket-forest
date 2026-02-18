//! C documentation context types.

use bon::Builder;
use serde::{Deserialize, Serialize};

use crate::knowledge::domain::{ItemName, Signature};

/// Type of C item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CItemType {
    /// All item types.
    All,
    /// Function definition.
    Function,
    /// Struct definition.
    Struct,
    /// Enum definition.
    Enum,
    /// Typedef declaration.
    Typedef,
    /// Macro definition.
    Macro,
    /// Variable or other declaration.
    Declaration,
    /// Standalone comment not attached to a declaration.
    StandaloneComment,
}

/// C item metadata for doc comment context.
#[derive(Debug, Clone, PartialEq, Eq, Builder, Serialize, Deserialize)]
#[builder(on(_, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct CDocContext {
    /// Name of the documented item. None for standalone comments.
    pub item_name: Option<ItemName>,
    /// Kind of C item.
    pub item_type: CItemType,
    /// First line of declaration if applicable.
    pub signature: Option<Signature>,
}
