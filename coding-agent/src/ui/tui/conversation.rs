//! Conversation display widget
//!
//! Renders chat messages in the TUI conversation area.

use crate::ui::tui::events::ToolStatus;

/// Chat message types
#[derive(Debug, Clone)]
pub enum ChatMessage {
    /// User input
    User { content: String },
    /// Assistant response
    Assistant { content: String },
    /// Tool call
    ToolCall { name: String, status: ToolStatus },
    /// System message
    System { content: String },
}

impl ChatMessage {
    /// Get the display content of the message
    pub fn content(&self) -> &str {
        match self {
            ChatMessage::User { content } => content,
            ChatMessage::Assistant { content } => content,
            ChatMessage::ToolCall { .. } => "[Tool Call]",
            ChatMessage::System { content } => content,
        }
    }

    /// Check if this is a user message
    pub fn is_user(&self) -> bool {
        matches!(self, ChatMessage::User { .. })
    }

    /// Check if this is an assistant message
    pub fn is_assistant(&self) -> bool {
        matches!(self, ChatMessage::Assistant { .. })
    }

    /// Check if this is a tool call
    pub fn is_tool_call(&self) -> bool {
        matches!(self, ChatMessage::ToolCall { .. })
    }
}
