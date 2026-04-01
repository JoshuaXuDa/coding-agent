//! Tools module for CodingAgent
//!
//! This module implements the core coding tools that form
//! the core domain of the CodingAgent bounded context.
//!
//! Tools are registered via the ToolProvider trait. To add a new tool:
//! 1. Create the tool file in `src/tools/application/`
//! 2. Add `pub mod your_tool;` to `src/tools/application/mod.rs`
//! 3. Implement `ToolProvider` for a unit struct in your tool file
//! 4. Add the provider to `all_providers()` in this file

// Domain layer
pub mod domain;

// Application layer
pub mod application;

// Schema compatibility
pub mod schema_fix;

use std::collections::HashMap;
use std::sync::Arc;
use tirea::prelude::Tool;
use crate::platform::{create_filesystem, create_command_executor};
use domain::provider::{DependencyType, ToolProvider};

/// Build the tool map by collecting all ToolProvider implementations.
///
/// Each provider declares its dependencies, and the registry
/// injects the appropriate services (FileSystem, CommandExecutor).
/// BatchTool is built last because it needs a reference to the full tool map.
pub fn build_tool_map() -> HashMap<String, Arc<dyn Tool>> {
    let fs = create_filesystem();
    let executor = create_command_executor();

    let providers: Vec<Box<dyn ToolProvider>> = all_providers();

    let mut tools = HashMap::new();

    for provider in providers {
        let tool_id = provider.tool_id().to_string();

        // BatchTool is handled separately after all tools are built
        if tool_id == "batch" {
            continue;
        }

        let dep = provider.dependency_type();

        let tool = match dep {
            DependencyType::FileSystem => {
                provider.build(Some(fs.clone()), None)
            }
            DependencyType::CommandExecutor => {
                provider.build(None, Some(executor.clone()))
            }
            DependencyType::None => {
                provider.build(None, None)
            }
        };

        tools.insert(tool_id, tool);
    }

    // Build BatchTool with a reference to the complete tool map
    let batch_tool = Arc::new(
        application::batch_tool::BatchTool::new(Arc::new(tools.clone()))
    ) as Arc<dyn Tool>;
    tools.insert("batch".to_string(), batch_tool);

    // Normalize tool parameter schemas for Anthropic API compatibility.
    for tool in tools.values_mut() {
        let fixed = schema_fix::SchemaFixingTool::new(Arc::clone(tool));
        *tool = Arc::new(fixed) as Arc<dyn Tool>;
    }

    tools
}

/// Collect all tool providers.
///
/// To add a new tool, add its provider here.
fn all_providers() -> Vec<Box<dyn ToolProvider>> {
    let mut providers: Vec<Box<dyn ToolProvider>> = vec![
        // FileSystem-dependent tools
        Box::new(application::list_tool::ListToolProvider),
        Box::new(application::read_tool::ReadToolProvider),
        Box::new(application::write_tool::WriteToolProvider),
        Box::new(application::stat_tool::StatToolProvider),
        Box::new(application::glob_tool::GlobToolProvider),
        Box::new(application::grep_tool::GrepToolProvider),
        Box::new(application::edit_tool::EditToolProvider),
        Box::new(application::head_tail_tool::HeadTailToolProvider),

        // CommandExecutor-dependent tools
        Box::new(application::bash_tool::BashToolProvider),

        // Stateless tools
        Box::new(application::todo_tool::TodoWriteToolProvider),
        Box::new(application::batch_tool::BatchToolProvider),

        // Feature-gated tools
        #[cfg(feature = "web-tools")]
        Box::new(application::webfetch_tool::WebFetchToolProvider),

        #[cfg(feature = "patch-tool")]
        Box::new(application::apply_patch_tool::ApplyPatchToolProvider),

        #[cfg(feature = "websearch")]
        Box::new(application::websearch_tool::WebSearchToolProvider),

        #[cfg(feature = "codesearch")]
        Box::new(application::codesearch_tool::CodeSearchToolProvider),
    ];

    providers
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
        let mut expected_count = 11; // Base tools

        // All features are enabled by default
        #[cfg(all(feature = "web-tools", feature = "patch-tool", feature = "websearch", feature = "codesearch"))]
        {
            expected_count += 4;
            assert!(tools.contains_key("webfetch"));
            assert!(tools.contains_key("apply_patch"));
            assert!(tools.contains_key("websearch"));
            assert!(tools.contains_key("codesearch"));
        }

        assert_eq!(tools.len(), expected_count);
        assert!(tools.contains_key("glob"));
        assert!(tools.contains_key("grep"));
        assert!(tools.contains_key("read"));
        assert!(tools.contains_key("write"));
        assert!(tools.contains_key("bash"));
        assert!(tools.contains_key("edit"));
        assert!(tools.contains_key("list"));
        assert!(tools.contains_key("stat"));
        assert!(tools.contains_key("head_tail"));
        assert!(tools.contains_key("todowrite"));
        assert!(tools.contains_key("batch"));
    }

    #[test]
    #[cfg(feature = "minimal")]
    fn test_build_tool_map_minimal() {
        let tools = build_tool_map();
        assert_eq!(tools.len(), 11);
        assert!(!tools.contains_key("webfetch"));
        assert!(!tools.contains_key("apply_patch"));
        assert!(!tools.contains_key("websearch"));
        assert!(!tools.contains_key("codesearch"));
    }
}
