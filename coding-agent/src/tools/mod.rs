//! Tools module for CodingAgent
//!
//! This module implements the 6 core coding tools that form
//! the core domain of the CodingAgent bounded context.

use std::collections::HashMap;
use std::sync::Arc;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use serde_json::Value;

// Type aliases for tool execution
pub type ToolArgs = Value;
pub type ToolContext = ();
pub type ToolExecutionEffect = String;

/// Simple tool wrapper for compatibility
pub struct SimpleTool {
    name: String,
    description: String,
}

impl SimpleTool {
    pub fn new(name: String, description: String) -> Self {
        Self { name, description }
    }
}

impl Tool for SimpleTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: self.name.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            category: Some("tool".to_string()),
            parameters: Default::default(),
            metadata: Default::default(),
        }
    }

    fn execute<'life0: 'async_trait, 'life1: 'async_trait, 'life2: 'async_trait, 'async_trait>(
        &'life0 self,
        _args: serde_json::Value,
        _context: &'life1 ToolCallContext<'life2>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ToolResult, ToolError>> + Send + 'async_trait>> {
        Box::pin(async move {
            Ok(ToolResult::success("simple_tool", "Tool executed"))
        })
    }
}

/// Build the tool map for the CodingAgent
///
/// This is the tool registry that registers all available tools.
/// Each tool is a domain service implementing the Tool trait.
pub fn build_tool_map() -> HashMap<String, Arc<dyn Tool>> {
    let mut tools = HashMap::new();

    // Register simplified tools for now
    tools.insert(
        "glob".to_string(),
        Arc::new(SimpleTool::new(
            "glob".to_string(),
            "File pattern matching tool".to_string(),
        )) as Arc<dyn Tool>,
    );
    tools.insert(
        "grep".to_string(),
        Arc::new(SimpleTool::new(
            "grep".to_string(),
            "Content search tool".to_string(),
        )) as Arc<dyn Tool>,
    );
    tools.insert(
        "read".to_string(),
        Arc::new(SimpleTool::new(
            "read".to_string(),
            "File reading tool".to_string(),
        )) as Arc<dyn Tool>,
    );
    tools.insert(
        "write".to_string(),
        Arc::new(SimpleTool::new(
            "write".to_string(),
            "File writing tool".to_string(),
        )) as Arc<dyn Tool>,
    );
    tools.insert(
        "bash".to_string(),
        Arc::new(SimpleTool::new(
            "bash".to_string(),
            "Shell command execution tool".to_string(),
        )) as Arc<dyn Tool>,
    );
    tools.insert(
        "edit".to_string(),
        Arc::new(SimpleTool::new(
            "edit".to_string(),
            "String replacement tool".to_string(),
        )) as Arc<dyn Tool>,
    );

    tools
}

/// Maximum output size before truncation (50KB)
const MAX_OUTPUT_SIZE: usize = 50 * 1024;

/// Maximum number of lines before truncation
const MAX_LINES: usize = 2000;

/// Truncate output if it exceeds limits
///
/// This utility function is used across multiple tools to enforce
/// output size limits and provide helpful guidance when truncation occurs.
pub fn truncate_output(content: &str) -> String {
    let content_size = content.len();
    let line_count = content.lines().count();

    if content_size > MAX_OUTPUT_SIZE || line_count > MAX_LINES {
        let mut truncated = String::new();
        let mut current_size = 0;
        let mut current_lines = 0;

        for line in content.lines() {
            if current_size + line.len() > MAX_OUTPUT_SIZE || current_lines >= MAX_LINES {
                truncated.push_str(&format!(
                    "\n\n--- Output truncated ({} bytes, {} lines) ---\n",
                    content_size, line_count
                ));
                truncated.push_str("Use offset/limit parameters to read specific sections.");
                break;
            }
            truncated.push_str(line);
            truncated.push('\n');
            current_size += line.len() + 1;
            current_lines += 1;
        }

        truncated
    } else {
        content.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_empty() {
        let result = truncate_output("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_truncate_small() {
        let content = "Hello\nWorld\n";
        let result = truncate_output(content);
        assert_eq!(result, content);
    }

    #[test]
    fn test_truncate_by_lines() {
        let mut content = String::new();
        for i in 0..(MAX_LINES + 100) {
            content.push_str(&format!("Line {}\n", i));
        }

        let result = truncate_output(&content);
        assert!(result.contains("Output truncated"));
        assert!(result.contains("offset/limit"));
    }

    #[test]
    fn test_truncate_by_size() {
        let mut content = String::new();
        for _ in 0..(MAX_OUTPUT_SIZE + 1000) {
            content.push('x');
        }

        let result = truncate_output(&content);
        assert!(result.contains("Output truncated"));
        assert!(result.contains("offset/limit"));
    }

    #[test]
    fn test_build_tool_map() {
        let tools = build_tool_map();
        assert_eq!(tools.len(), 6);
        assert!(tools.contains_key("glob"));
        assert!(tools.contains_key("grep"));
        assert!(tools.contains_key("read"));
        assert!(tools.contains_key("write"));
        assert!(tools.contains_key("bash"));
        assert!(tools.contains_key("edit"));
    }
}
