//! Event system for TUI
//!
//! Handles async events from both user input and agent responses.

use crossterm::event::KeyEvent;

/// TUI events
#[derive(Debug, Clone)]
pub enum TuiEvent {
    /// User keyboard input
    Input(KeyEvent),
    /// Text delta from agent (streaming)
    AgentText(String),
    /// Agent is calling a tool
    AgentToolCall { name: String, input: serde_json::Value },
    /// Tool execution completed
    AgentToolDone { name: String },
    /// Agent error
    AgentError(String),
    /// Tick event for periodic updates
    Tick,
}

/// Tool execution status
#[derive(Debug, Clone, PartialEq)]
pub enum ToolStatus {
    Running,
    Done,
    Error(String),
}
