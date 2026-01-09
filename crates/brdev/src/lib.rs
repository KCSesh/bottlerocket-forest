//! brdev provides Bottlerocket-specific development tooling.
//!
//! This library provides functionality for managing local OCI registries,
//! coordinating builds, and managing development environments.

pub mod cli;
pub mod config;
pub mod grove;
pub mod registry;
