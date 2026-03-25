//! Event system for TUI
//!
//! Handles async events from both user input and agent responses.

use crossterm::event::{KeyEvent, MouseEvent};

/// TUI events
#[derive(Debug, Clone)]
pub enum TuiEvent {
    /// User keyboard input
    Input(KeyEvent),
    /// Mouse event
    Mouse(MouseEvent),
    /// Reasoning/thinking delta from agent (streaming)
    AgentReasoning(String),
    /// Text delta from agent (streaming formal output)
    AgentText(String),
    /// Agent is calling a tool
    AgentToolCall { name: String, input: serde_json::Value },
    /// Tool execution completed
    AgentToolDone { name: String },
    /// Agent error
    AgentError(String),
    /// Agent response completed
    AgentResponseComplete,
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
