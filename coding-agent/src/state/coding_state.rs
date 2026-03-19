//! CodingState - Aggregate Root for the State bounded context
//!
//! This module defines the core state management for the CodingAgent,
//! following DDD patterns with clear aggregate boundaries.

use serde::{Deserialize, Serialize};

/// Aggregate root for the CodingAgent state
///
/// This state maintains:
/// - Current working directory context
/// - TodoList for task tracking
/// - Command execution history (bounded to 20 entries)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodingState {
    /// Current working directory for the agent
    pub working_dir: Option<String>,

    /// TodoList for tracking agent tasks
    pub todos: Vec<TodoItem>,

    /// Recent command history (max 20 entries)
    pub command_history: Vec<CommandRecord>,
}

impl CodingState {
    /// Maximum number of command records to keep
    const MAX_COMMAND_HISTORY: usize = 20;

    /// Add a command record to history, maintaining the max size
    pub fn add_command_record(mut self, record: CommandRecord) -> Self {
        self.command_history.push(record);
        // Keep only the most recent MAX_COMMAND_HISTORY entries
        if self.command_history.len() > Self::MAX_COMMAND_HISTORY {
            self.command_history = self.command_history
                .into_iter()
                .rev()
                .take(Self::MAX_COMMAND_HISTORY)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
        }
        self
    }

    /// Find a todo item by ID
    pub fn find_todo(&self, id: &str) -> Option<&TodoItem> {
        self.todos.iter().find(|t| t.id == id)
    }

    /// Get all pending todos
    pub fn pending_todos(&self) -> Vec<&TodoItem> {
        self.todos.iter()
            .filter(|t| t.status == TodoStatus::Pending)
            .collect()
    }
}

/// TodoItem - Entity within the CodingState aggregate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// Unique identifier for the todo
    pub id: String,

    /// Brief title of the task
    pub title: String,

    /// Detailed description of what needs to be done
    pub description: String,

    /// Current status of the todo
    pub status: TodoStatus,

    /// Optional parent todo ID (for subtasks)
    pub parent_id: Option<String>,

    /// IDs of todos that block this one
    pub blocked_by: Vec<String>,
}

/// Status of a TodoItem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoStatus {
    /// Task is not yet started
    Pending,

    /// Task is currently being worked on
    InProgress,

    /// Task is completed
    Completed,
}

impl Default for TodoStatus {
    fn default() -> Self {
        TodoStatus::Pending
    }
}

/// Record of a command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandRecord {
    /// The command that was executed
    pub command: String,

    /// Exit code of the command
    pub exit_code: Option<i32>,

    /// Output from the command (may be truncated)
    pub output: String,

    /// Error output from the command (may be truncated)
    pub error: String,

    /// Duration of the command execution
    pub duration_ms: u64,

    /// Timestamp when the command was executed
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = CodingState::default();
        assert!(state.working_dir.is_none());
        assert!(state.todos.is_empty());
        assert!(state.command_history.is_empty());
    }

    #[test]
    fn test_command_history_limit() {
        let mut state = CodingState::default();

        // Add 25 commands
        for i in 0..25 {
            let record = CommandRecord {
                command: format!("echo {}", i),
                exit_code: Some(0),
                output: format!("{}", i),
                error: String::new(),
                duration_ms: 10,
                timestamp: chrono::Utc::now(),
            };
            state = state.add_command_record(record);
        }

        // Should only have 20
        assert_eq!(state.command_history.len(), 20);
        // Should have the last 20 (5-24, not 0-4)
        assert_eq!(state.command_history[0].command, "echo 5");
        assert_eq!(state.command_history[19].command, "echo 24");
    }

    #[test]
    fn test_find_todo() {
        let mut state = CodingState::default();

        let todo = TodoItem {
            id: "test-1".to_string(),
            title: "Test Todo".to_string(),
            description: "Description".to_string(),
            status: TodoStatus::Pending,
            parent_id: None,
            blocked_by: vec![],
        };

        state.todos.push(todo.clone());

        let found = state.find_todo("test-1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "test-1");

        let not_found = state.find_todo("non-existent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_pending_todos() {
        let mut state = CodingState::default();

        state.todos.push(TodoItem {
            id: "1".to_string(),
            title: "Pending".to_string(),
            description: "A".to_string(),
            status: TodoStatus::Pending,
            parent_id: None,
            blocked_by: vec![],
        });

        state.todos.push(TodoItem {
            id: "2".to_string(),
            title: "Completed".to_string(),
            description: "B".to_string(),
            status: TodoStatus::Completed,
            parent_id: None,
            blocked_by: vec![],
        });

        let pending = state.pending_todos();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, "1");
    }
}
