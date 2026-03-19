//! State management module for CodingAgent
//!
//! This module implements the state layer of the DDD architecture,
//! with CodingState as the aggregate root.

mod coding_state;
mod actions;

pub use coding_state::{CodingState, TodoItem, TodoStatus, CommandRecord};
pub use actions::CodingAction;
