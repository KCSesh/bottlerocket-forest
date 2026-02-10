//! Git abstraction layer for forester.
//!
//! Provides operations for bare repositories and clones.

mod bare;
mod command;

pub use bare::BareRepository;
pub use command::GitCommandError;
