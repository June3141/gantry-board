//! Agent session output service — split into query (read) and buffer submodules.

pub mod buffer;
pub mod query;

// Re-export all public items for backward compatibility.
pub use buffer::*;
pub use query::*;

/// Maximum size of a single output content in bytes (64 KB).
pub(crate) const MAX_CONTENT_SIZE: usize = 64 * 1024;

/// Maximum number of output records per session.
pub(crate) const MAX_OUTPUTS_PER_SESSION: i64 = 10_000;
