//! State management module for CodingAgent
//!
//! This module implements the state layer of the DDD architecture,
//! with CodingState and ConversationState as aggregate roots.

mod coding_state;
mod actions;
mod conversation;

pub use coding_state::{CodingState, TodoItem, TodoStatus, CommandRecord};
pub use actions::CodingAction;
pub use conversation::{ChatMessage, ConversationState, ToolCallStatus};
