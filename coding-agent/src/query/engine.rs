//! Query engine trait and event types
//!
//! Defines the interface for LLM interaction lifecycle management.

use serde_json::Value;
use tokio::sync::mpsc;

use crate::query::CancellationToken;

/// Events emitted by the QueryEngine for the UI to consume.
///
/// These represent the high-level events that the TUI cares about,
/// abstracting away tirea-specific AgentEvent details.
#[derive(Debug, Clone)]
pub enum QueryEvent {
    /// Reasoning/thinking delta from agent (streaming)
    ReasoningDelta(String),
    /// Text delta from agent (streaming formal output)
    TextDelta(String),
    /// Agent is calling a tool
    ToolCallStart {
        name: String,
        input: Value,
    },
    /// Tool execution completed
    ToolCallDone {
        name: String,
    },
    /// Agent error
    Error(String),
    /// Agent response completed successfully
    Complete,
}

/// A request to the QueryEngine to start processing a user message.
pub struct QueryRequest {
    /// The user's message text
    pub message: String,
    /// Channel to send log entries for the debug panel
    pub log_tx: tokio::sync::mpsc::UnboundedSender<crate::logging::LogEntry>,
    /// Cancellation token for aborting this query
    pub cancel_token: CancellationToken,
}

/// The QueryEngine manages the LLM interaction lifecycle:
/// message submission, streaming, tool execution, error handling.
///
/// The TUI holds a reference to this trait and delegates all
/// LLM communication through it, keeping the TUI layer clean.
pub trait QueryEngine: Send + Sync {
    /// Submit a user message for processing.
    /// Returns immediately; events are emitted via the channel.
    fn submit(
        &self,
        request: QueryRequest,
        event_tx: mpsc::UnboundedSender<QueryEvent>,
    );

    /// Cancel the currently running query (if any).
    fn cancel(&self);

    /// Check if a query is currently running.
    fn is_busy(&self) -> bool;
}
