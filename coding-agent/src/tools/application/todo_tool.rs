//! TodoWrite tool - Application layer
//!
//! Manages todo lists for task tracking.

use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use std::pin::Pin;
use std::future::Future;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::tools::domain::xml_builder::XmlBuilder;
use serde::{Deserialize, Serialize};

/// Todo item status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

/// Todo item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: TodoStatus,
    pub active_form: String,
}

/// TodoWrite tool
///
/// Manages todo lists for task tracking during agent sessions.
pub struct TodoWriteTool {
    /// Base directory for storing todo files
    storage_dir: PathBuf,
}

impl TodoWriteTool {
    /// Create a new todo write tool
    pub fn new() -> Self {
        let storage_dir = std::path::PathBuf::from(".coding-agent/todos");
        // Create storage directory if it doesn't exist
        if let Err(e) = fs::create_dir_all(&storage_dir) {
            eprintln!("Warning: Failed to create todo storage directory: {}", e);
        }
        Self { storage_dir }
    }

    /// Get the storage file path for a session
    fn get_storage_path(&self, session_id: &str) -> PathBuf {
        self.storage_dir.join(format!("session-{}.json", session_id))
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<Vec<TodoItem>> {
        let todos_array = args
            .get("todos")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing 'todos' argument"))?;

        let mut todos = Vec::new();
        for (idx, item) in todos_array.iter().enumerate() {
            let content = item
                .get("content")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Todo item {} missing 'content' field", idx))?
                .to_string();

            let status_str = item
                .get("status")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Todo item {} missing 'status' field", idx))?;

            let status = match status_str {
                "pending" => TodoStatus::Pending,
                "in_progress" => TodoStatus::InProgress,
                "completed" => TodoStatus::Completed,
                _ => return Err(anyhow::anyhow!("Invalid status '{}' for todo item {}", status_str, idx)),
            };

            let active_form = item
                .get("activeForm")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow::anyhow!("Todo item {} missing 'activeForm' field", idx))?
                .to_string();

            todos.push(TodoItem {
                content,
                status,
                active_form,
            });
        }

        Ok(todos)
    }

    /// Save todos to storage
    fn save_todos(&self, session_id: &str, todos: &[TodoItem]) -> Result<()> {
        let path = self.get_storage_path(session_id);
        let json = serde_json::to_string_pretty(todos)?;
        fs::write(path, json)?;
        Ok(())
    }
}

impl Default for TodoWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Tool for TodoWriteTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "todowrite".to_string(),
            name: "todowrite".to_string(),
            description: "Create and track todo lists for multi-step tasks".to_string(),
            category: Some("task_management".to_string()),
            parameters: serde_json::json!({
                "todos": {
                    "type": "array",
                    "description": "The updated todo list",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "The task description in imperative form (e.g., 'Run tests')"
                            },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed"],
                                "description": "The current status of the todo item"
                            },
                            "activeForm": {
                                "type": "string",
                                "description": "The task in present continuous form (e.g., 'Running tests')"
                            }
                        },
                        "required": ["content", "status", "activeForm"]
                    }
                }
            }),
            metadata: Default::default(),
        }
    }

    fn execute<'life0: 'async_trait, 'life1: 'async_trait, 'life2: 'async_trait, 'async_trait>(
        &'life0 self,
        args: serde_json::Value,
        context: &'life1 ToolCallContext<'life2>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'async_trait>> {
        Box::pin(async move {
            // Parse arguments
            let todos = Self::parse_args(&args)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Get call ID from context to use as session identifier
            let call_id = context.call_id();

            // Save todos to storage
            self.save_todos(call_id, &todos)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Count active todos
            let active_count = todos.iter()
                .filter(|t| t.status != TodoStatus::Completed)
                .count();

            // Build JSON output
            let json_output = serde_json::to_string_pretty(&todos)
                .unwrap_or_else(|_| "Failed to serialize todos".to_string());

            // Build XML response
            let xml = XmlBuilder::build_success(
                "todowrite",
                &format!("{} todos", active_count),
                &json_output,
            ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            Ok(ToolResult::success("todowrite", xml))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args_valid() {
        let args = serde_json::json!({
            "todos": [
                {
                    "content": "Test task",
                    "status": "pending",
                    "activeForm": "Testing"
                }
            ]
        });

        let result = TodoWriteTool::parse_args(&args).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "Test task");
        assert_eq!(result[0].status, TodoStatus::Pending);
        assert_eq!(result[0].active_form, "Testing");
    }

    #[test]
    fn test_parse_args_missing_todos() {
        let args = serde_json::json!({});
        let result = TodoWriteTool::parse_args(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_args_invalid_status() {
        let args = serde_json::json!({
            "todos": [
                {
                    "content": "Test task",
                    "status": "invalid",
                    "activeForm": "Testing"
                }
            ]
        });

        let result = TodoWriteTool::parse_args(&args);
        assert!(result.is_err());
    }

    #[test]
    fn test_todo_status_serialization() {
        let item = TodoItem {
            content: "Test".to_string(),
            status: TodoStatus::InProgress,
            active_form: "Testing".to_string(),
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"status\":\"in_progress\""));
    }
}
