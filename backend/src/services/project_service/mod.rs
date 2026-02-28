//! Project service — split into commands (write) and queries (read) submodules.

pub mod commands;
pub mod queries;

// Re-export all public items for backward compatibility.
pub use commands::*;
pub use queries::*;
