//! Java documentation context types.

use bon::Builder;
use serde::{Deserialize, Serialize};

use crate::knowledge::domain::{ItemName, PackageName, Signature};

/// Visibility of a Java item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JavaVisibility {
    /// Public visibility.
    Public,
    /// Protected visibility.
    Protected,
    /// Package-private (default) visibility.
    PackagePrivate,
    /// Private visibility.
    Private,
}

/// Type of Java item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum JavaItemType {
    /// All item types.
    All,
    /// Class definition.
    Class,
    /// Interface definition.
    Interface,
    /// Enum definition.
    Enum,
    /// Record definition.
    Record,
    /// Method definition.
    Method,
    /// Field definition.
    Field,
    /// Constructor definition.
    Constructor,
    /// Annotation definition.
    Annotation,
}

/// Java item metadata for doc comment context.
#[derive(Debug, Clone, PartialEq, Eq, Builder, Serialize, Deserialize)]
#[builder(on(_, into))]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct JavaDocContext {
    /// Name of the documented item.
    pub item_name: ItemName,
    /// Visibility level of the item.
    pub visibility: JavaVisibility,
    /// Method or type signature if applicable.
    pub signature: Option<Signature>,
    /// Kind of Java item.
    pub item_type: JavaItemType,
    /// Package name for this Java item. None only during parsing errors or malformed files.
    pub package_name: Option<PackageName>,
}
