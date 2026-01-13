//! brdev provides Bottlerocket-specific development tooling.
//!
//! This library provides functionality for managing local OCI registries,
//! coordinating builds, and managing development environments.

pub mod cli;
pub mod grove;
/// Local OCI registry lifecycle and catalog operations.
pub mod registry;
