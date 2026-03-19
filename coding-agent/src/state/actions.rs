//! Domain events (actions) for CodingState
//!
//! These actions represent intents that cause state transitions.
//! Each action is processed by the reduce function to produce a new state.

use crate::state::{CodingState, TodoItem, TodoStatus, CommandRecord};

/// Domain actions that can transition the CodingState
///
/// Each variant represents a business intent that will cause
/// a state transition when processed by the reduce function.
#[derive(Debug, Clone)]
pub enum CodingAction {
    /// Set or update the working directory
    SetWorkingDir(String),

    /// Add a new todo item
    AddTodo(TodoItem),

    /// Update the status of an existing todo
    UpdateTodoStatus { id: String, status: TodoStatus },

    /// Add a command execution record to history
    AddCommandRecord(CommandRecord),

    /// Clear all completed todos
    ClearCompletedTodos,
}

impl CodingAction {
    /// Apply this action to produce a new state
    ///
    /// This is the reduce function that implements the state machine.
    /// It follows functional programming principles - pure function,
    /// no side effects, easy to test.
    pub fn reduce(self, state: CodingState) -> CodingState {
        match self {
            CodingAction::SetWorkingDir(dir) => {
                CodingState {
                    working_dir: Some(dir),
                    ..state
                }
            }

            CodingAction::AddTodo(todo) => {
                let mut todos = state.todos;
                todos.push(todo);
                CodingState { todos, ..state }
            }

            CodingAction::UpdateTodoStatus { id, status } => {
                let todos = state.todos
                    .into_iter()
                    .map(|mut t| {
                        if t.id == id {
                            t.status = status;
                        }
                        t
                    })
                    .collect();
                CodingState { todos, ..state }
            }

            CodingAction::AddCommandRecord(record) => {
                state.add_command_record(record)
            }

            CodingAction::ClearCompletedTodos => {
                let todos = state.todos
                    .into_iter()
                    .filter(|t| t.status != TodoStatus::Completed)
                    .collect();
                CodingState { todos, ..state }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{TodoItem, TodoStatus, CommandRecord};

    #[test]
    fn test_set_working_dir() {
        let state = CodingState::default();
        let action = CodingAction::SetWorkingDir("/tmp".to_string());
        let new_state = action.reduce(state);

        assert_eq!(new_state.working_dir, Some("/tmp".to_string()));
    }

    #[test]
    fn test_add_todo() {
        let state = CodingState::default();
        let todo = TodoItem {
            id: "1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            status: TodoStatus::Pending,
            parent_id: None,
            blocked_by: vec![],
        };

        let action = CodingAction::AddTodo(todo);
        let new_state = action.reduce(state);

        assert_eq!(new_state.todos.len(), 1);
        assert_eq!(new_state.todos[0].id, "1");
    }

    #[test]
    fn test_update_todo_status() {
        let mut state = CodingState::default();
        state.todos.push(TodoItem {
            id: "1".to_string(),
            title: "Test".to_string(),
            description: "Desc".to_string(),
            status: TodoStatus::Pending,
            parent_id: None,
            blocked_by: vec![],
        });

        let action = CodingAction::UpdateTodoStatus {
            id: "1".to_string(),
            status: TodoStatus::Completed,
        };
        let new_state = action.reduce(state);

        assert_eq!(new_state.todos[0].status, TodoStatus::Completed);
    }

    #[test]
    fn test_clear_completed_todos() {
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

        let action = CodingAction::ClearCompletedTodos;
        let new_state = action.reduce(state);

        assert_eq!(new_state.todos.len(), 1);
        assert_eq!(new_state.todos[0].id, "1");
    }
}
