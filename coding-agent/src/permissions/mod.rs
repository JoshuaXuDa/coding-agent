//! Permission system for tool execution
//!
//! Provides a multi-layer permission checking system inspired by Claude Code's
//! allow/deny/ask pattern. Tools can be auto-allowed, denied, or require
//! user confirmation before execution.

mod engine;
mod config;

pub use engine::{PermissionEngine, PermissionDecision, PermissionMode, ToolPermissionRule};
pub use config::PermissionConfig;
