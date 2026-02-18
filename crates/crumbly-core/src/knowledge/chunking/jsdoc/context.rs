//! JavaScript/TypeScript documentation context types.

use bon::Builder;
use serde::{Deserialize, Serialize};

use crate::knowledge::domain::{ItemName, PackageName, Signature};

/// Visibility of a JS/TS item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JsVisibility {
    /// Exported item visible outside module.
    Exported,
    /// Local item visible only within module.
    Local,
}

/// Type of JS/TS item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JsItemType {
    /// All item types.
    All,
    /// Function declaration.
    Function,
    /// Class declaration.
    Class,
    /// Variable declaration.
    Variable,
    /// Interface declaration (TS).
    Interface,
    /// Type alias declaration (TS).
    TypeAlias,
    /// Enum declaration (TS).
    Enum,
    /// Module-level documentation.
    Module,
}

/// JS/TS item metadata for doc comment context.
#[derive(Debug, Clone, PartialEq, Eq, Builder, Serialize, Deserialize)]
#[builder(on(_, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct JsDocContext {
    /// Name of the documented item.
    pub item_name: ItemName,
    /// Visibility level of the item.
    pub visibility: JsVisibility,
    /// Function or type signature if applicable.
    pub signature: Option<Signature>,
    /// Kind of JS/TS item.
    pub item_type: JsItemType,
    /// Module name for this item. None only during parsing errors or malformed files.
    pub module_name: Option<PackageName>,
}
