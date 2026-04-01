//! Conversation state management
//!
//! Defines the core conversation message types used by both
//! the query engine and the TUI, decoupled from any specific UI framework.

/// Tool call execution status
#[derive(Debug, Clone, PartialEq)]
pub enum ToolCallStatus {
    Running,
    Done,
    Error(String),
}

/// Chat message types for the conversation history.
///
/// These represent the canonical message types shared between
/// the QueryEngine (producer) and the TUI (consumer).
#[derive(Debug, Clone)]
pub enum ChatMessage {
    /// User input
    User { content: String },
    /// Thinking/reasoning content (collapsible)
    Thinking { content: String, expanded: bool },
    /// Assistant response (formal output)
    Assistant { content: String },
    /// Tool call
    ToolCall { name: String, status: ToolCallStatus },
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

/// Conversation state tracking message history and metadata.
#[derive(Debug, Clone, Default)]
pub struct ConversationState {
    /// All messages in the conversation
    pub messages: Vec<ChatMessage>,
    /// Estimated total tokens used
    pub total_tokens_used: usize,
}

impl ConversationState {
    /// Create a new empty conversation state
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a message to the conversation
    pub fn push(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// Get the number of messages
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if conversation is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Get a reference to the messages
    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    /// Get a mutable reference to the messages
    pub fn messages_mut(&mut self) -> &mut [ChatMessage] {
        &mut self.messages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_types() {
        let user = ChatMessage::User { content: "hello".into() };
        assert!(user.is_user());
        assert!(!user.is_assistant());

        let thinking = ChatMessage::Thinking { content: "hmm".into(), expanded: false };
        assert!(thinking.is_thinking());
        assert!(!thinking.is_thinking_expanded());

        let tool = ChatMessage::ToolCall { name: "bash".into(), status: ToolCallStatus::Running };
        assert!(tool.is_tool_call());
    }

    #[test]
    fn test_toggle_thinking() {
        let mut msg = ChatMessage::Thinking { content: "test".into(), expanded: false };
        assert!(!msg.is_thinking_expanded());
        msg.toggle_thinking();
        assert!(msg.is_thinking_expanded());
        msg.toggle_thinking();
        assert!(!msg.is_thinking_expanded());
    }

    #[test]
    fn test_conversation_state() {
        let mut state = ConversationState::new();
        assert!(state.is_empty());

        state.push(ChatMessage::User { content: "hello".into() });
        assert_eq!(state.len(), 1);
        assert_eq!(state.messages()[0].content(), "hello");
    }

    #[test]
    fn test_tool_call_status_equality() {
        assert_eq!(ToolCallStatus::Running, ToolCallStatus::Running);
        assert_eq!(ToolCallStatus::Done, ToolCallStatus::Done);
        assert_ne!(ToolCallStatus::Running, ToolCallStatus::Done);
    }
}
