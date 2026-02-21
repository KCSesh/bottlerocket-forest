//! Kotlin documentation context types.

use bon::Builder;
use serde::{Deserialize, Serialize};

use crate::knowledge::domain::{ItemName, PackageName, Signature};

/// Visibility of a Kotlin item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KotlinVisibility {
    /// Public visibility.
    Public,
    /// Internal visibility.
    Internal,
    /// Protected visibility.
    Protected,
    /// Private visibility.
    Private,
}

/// Type of Kotlin item.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KotlinItemType {
    /// All item types.
    All,
    /// Class definition.
    Class,
    /// Object definition.
    Object,
    /// Interface definition.
    Interface,
    /// Function definition.
    Function,
    /// Property definition.
    Property,
    /// Enum definition.
    Enum,
}

/// Kotlin item metadata for doc comment context.
#[derive(Debug, Clone, PartialEq, Eq, Builder, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct KotlinDocContext {
    /// Name of the documented item.
    #[builder(into)]
    pub item_name: ItemName,
    /// Visibility level of the item.
    pub visibility: KotlinVisibility,
    /// Function or type signature if applicable.
    #[builder(into)]
    pub signature: Option<Signature>,
    /// Kind of Kotlin item.
    pub item_type: KotlinItemType,
    /// Package name for this Kotlin item.
    #[builder(into)]
    pub package_name: Option<PackageName>,
}
