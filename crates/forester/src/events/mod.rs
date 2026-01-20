//! Event emission system for forester operations.
//!
//! Provides structured events for all forester operations, enabling
//! consistent user feedback and potential integration with external systems.

mod emitter;
mod rich;
mod types;

pub use emitter::{ConsoleEmitter, EventEmitter, create_emitter};
pub use rich::RichEmitter;
pub use types::ForesterEvent;
