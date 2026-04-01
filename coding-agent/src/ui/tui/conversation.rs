//! Conversation display widget
//!
//! Renders chat messages in the TUI conversation area.
//! Message types are defined in the state module and re-exported here for convenience.

// Re-export from state module
pub use crate::state::{ChatMessage, ToolCallStatus};
