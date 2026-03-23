//! Tools module for CodingAgent
//!
//! This module implements the core coding tools that form
//! the core domain of the CodingAgent bounded context.

// Domain layer
pub mod domain;

// Application layer
pub mod application;

// Legacy placeholders (will be removed)
pub mod read_tool;
pub mod write_tool;
pub mod edit_tool;
pub mod glob_tool;
pub mod grep_tool;
pub mod bash_tool;
pub mod list_tool;
pub mod stat_tool;
pub mod head_tail_tool;

use std::collections::HashMap;
use std::sync::Arc;
use tirea::prelude::Tool;
use crate::platform::{create_filesystem, create_command_executor};

/// Build the tool map for the CodingAgent
///
/// This is the tool registry that registers all available tools.
/// Each tool is a domain service implementing the Tool trait.
pub fn build_tool_map() -> HashMap<String, Arc<dyn Tool>> {
    let mut tools = HashMap::new();

    // Create platform services
    let fs = create_filesystem();
    let executor = create_command_executor();

    // Register all implemented tools
    tools.insert("list".to_string(), Arc::new(
        crate::tools::application::list_tool::ListTool::new(fs.clone())
    ) as Arc<dyn Tool>);

    tools.insert("read".to_string(), Arc::new(
        crate::tools::application::read_tool::ReadTool::new(fs.clone())
    ) as Arc<dyn Tool>);

    tools.insert("write".to_string(), Arc::new(
        crate::tools::application::write_tool::WriteTool::new(fs.clone())
    ) as Arc<dyn Tool>);

    tools.insert("stat".to_string(), Arc::new(
        crate::tools::application::stat_tool::StatTool::new(fs.clone())
    ) as Arc<dyn Tool>);

    tools.insert("glob".to_string(), Arc::new(
        crate::tools::application::glob_tool::GlobTool::new(fs.clone())
    ) as Arc<dyn Tool>);

    tools.insert("grep".to_string(), Arc::new(
        crate::tools::application::grep_tool::GrepTool::new(fs.clone())
    ) as Arc<dyn Tool>);

    tools.insert("bash".to_string(), Arc::new(
        crate::tools::application::bash_tool::BashTool::new(executor)
    ) as Arc<dyn Tool>);

    tools.insert("edit".to_string(), Arc::new(
        crate::tools::application::edit_tool::EditTool::new(fs.clone())
    ) as Arc<dyn Tool>);

    tools.insert("head_tail".to_string(), Arc::new(
        crate::tools::application::head_tail_tool::HeadTailTool::new(fs)
    ) as Arc<dyn Tool>);

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
        assert_eq!(tools.len(), 9);
        assert!(tools.contains_key("glob"));
        assert!(tools.contains_key("grep"));
        assert!(tools.contains_key("read"));
        assert!(tools.contains_key("write"));
        assert!(tools.contains_key("bash"));
        assert!(tools.contains_key("edit"));
        assert!(tools.contains_key("list"));
        assert!(tools.contains_key("stat"));
        assert!(tools.contains_key("head_tail"));
    }
}
