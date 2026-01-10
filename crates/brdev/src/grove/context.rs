//! Grove context detection and access.

use bon::Builder;
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;

/// Represents a detected grove with its root path and name.
#[derive(Debug, Clone, PartialEq, Eq, Builder)]
#[builder(on(_, into))]
#[non_exhaustive]
pub struct GroveContext {
    root: PathBuf,
    name: String,
}

impl GroveContext {
    /// Detects the current grove by walking up from the current directory.
    pub fn detect() -> Result<Self, GroveContextError> {
        use grove_context_error::*;
        
        let mut current = std::env::current_dir().context(CurrentDirSnafu)?;
        loop {
            if current.join(".grove").is_dir() {
                let name = current
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(String::from)
                    .ok_or(GroveContextError::InvalidName)?;
                return Ok(Self { root: current, name });
            }
            if !current.pop() {
                return Err(GroveContextError::NotInGrove);
            }
        }
    }

    /// Returns the grove root directory path.
    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    /// Returns the grove name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum GroveContextError {
    #[snafu(display("Not in a grove (no .grove/ directory found)"))]
    NotInGrove,

    #[snafu(display("Failed to get current directory"))]
    CurrentDir { source: std::io::Error },

    #[snafu(display("Grove directory has invalid name"))]
    InvalidName,
}
