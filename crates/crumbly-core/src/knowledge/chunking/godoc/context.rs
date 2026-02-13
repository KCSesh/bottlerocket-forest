//! Go documentation context types.

use bon::Builder;
use serde::{Deserialize, Serialize};

use crate::knowledge::domain::{ItemName, PackageName, Signature};

/// Visibility of a Go item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GoVisibility {
    /// Exported (capitalized) item visible outside package.
    Exported,
    /// Unexported (lowercase) item visible only within package.
    Unexported,
}

/// Type of Go item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GoItemType {
    /// All item types.
    All,
    /// Standalone function.
    Function,
    /// Method on a type.
    Method,
    /// Struct type definition.
    Struct,
    /// Interface type definition.
    Interface,
    /// Type alias or definition.
    Type,
    /// Constant declaration.
    Const,
    /// Variable declaration.
    Var,
    /// Package-level documentation.
    Package,
}

/// Go item metadata for doc comment context.
#[derive(Debug, Clone, PartialEq, Eq, Builder, Serialize, Deserialize)]
#[builder(on(_, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct GoDocContext {
    /// Name of the documented item.
    pub item_name: ItemName,
    /// Visibility level of the item.
    pub visibility: GoVisibility,
    /// Function or type signature if applicable.
    pub signature: Option<Signature>,
    /// Kind of Go item.
    pub item_type: GoItemType,
    /// Package name for this Go item. None only during parsing errors or malformed files.
    pub package_name: Option<PackageName>,
}
