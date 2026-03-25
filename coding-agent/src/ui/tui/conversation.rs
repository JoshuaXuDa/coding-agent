//! Conversation display widget
//!
//! Renders chat messages in the TUI conversation area.

use crate::ui::tui::events::ToolStatus;

/// Chat message types
#[derive(Debug, Clone)]
pub enum ChatMessage {
    /// User input
    User { content: String },
    /// Thinking/reasoning content (collapsible)
    Thinking { content: String, expanded: bool },
    /// Assistant response (formal output)
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
            ChatMessage::Thinking { content, .. } => content,
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

    /// Check if this is a thinking message
    pub fn is_thinking(&self) -> bool {
        matches!(self, ChatMessage::Thinking { .. })
    }

    /// Toggle expanded state for thinking messages
    pub fn toggle_thinking(&mut self) {
        if let ChatMessage::Thinking { expanded, .. } = self {
            *expanded = !*expanded;
        }
    }

    /// Get expanded state for thinking messages
    pub fn is_thinking_expanded(&self) -> bool {
        match self {
            ChatMessage::Thinking { expanded, .. } => *expanded,
            _ => false,
        }
    }
}
