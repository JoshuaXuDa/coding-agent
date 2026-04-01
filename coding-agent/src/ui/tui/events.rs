//! Event system for TUI
//!
//! Handles async events from both user input and agent responses.

/// TUI events
#[derive(Debug, Clone)]
pub enum TuiEvent {
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

impl TuiEvent {
    /// Convert from a QueryEvent (produced by the QueryEngine).
    pub fn from_query_event(event: crate::query::QueryEvent) -> Self {
        match event {
            crate::query::QueryEvent::ReasoningDelta(delta) => TuiEvent::AgentReasoning(delta),
            crate::query::QueryEvent::TextDelta(delta) => TuiEvent::AgentText(delta),
            crate::query::QueryEvent::ToolCallStart { name, input } => {
                TuiEvent::AgentToolCall { name, input }
            }
            crate::query::QueryEvent::ToolCallDone { name } => TuiEvent::AgentToolDone { name },
            crate::query::QueryEvent::Error(msg) => TuiEvent::AgentError(msg),
            crate::query::QueryEvent::Complete => TuiEvent::AgentResponseComplete,
        }
    }
}
