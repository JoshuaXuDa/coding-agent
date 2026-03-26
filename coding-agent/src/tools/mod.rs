//! Tools module for CodingAgent
//!
//! This module implements the core coding tools that form
//! the core domain of the CodingAgent bounded context.
//!
//! Tools are automatically registered using the macro-based registration system.
//! To add a new tool:
//! 1. Create the tool file in `src/tools/application/`
//! 2. Add `pub mod your_tool;` to `src/tools/application/mod.rs`
//! 3. Add the registration macro at the end of your tool file:
//!    `register_tool_fs!(YourTool, "your_tool_id");`
//! 4. Add the tool to the `collect_tools!` macro in `build_tool_map()`

// Domain layer
pub mod domain;

// Application layer
pub mod application;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tirea::prelude::Tool;
use crate::platform::{create_filesystem, create_command_executor};

/// Global tool registry (for tools that need access to other tools)
static TOOL_REGISTRY: OnceLock<Arc<HashMap<String, Arc<dyn Tool>>>> = OnceLock::new();

/// Macro to collect all tool registrations
///
/// This macro expands to register all tools with their respective dependencies.
/// To add a new tool, add a line following the existing pattern.
macro_rules! collect_tools {
    ($tools:ident, $fs:expr, $executor:expr) => {
        {
            // FileSystem-dependent tools
            $tools.insert("list".to_string(), Arc::new(
                crate::tools::application::list_tool::ListTool::new($fs.clone())
            ) as Arc<dyn Tool>);

            $tools.insert("read".to_string(), Arc::new(
                crate::tools::application::read_tool::ReadTool::new($fs.clone())
            ) as Arc<dyn Tool>);

            $tools.insert("write".to_string(), Arc::new(
                crate::tools::application::write_tool::WriteTool::new($fs.clone())
            ) as Arc<dyn Tool>);

            $tools.insert("stat".to_string(), Arc::new(
                crate::tools::application::stat_tool::StatTool::new($fs.clone())
            ) as Arc<dyn Tool>);

            $tools.insert("glob".to_string(), Arc::new(
                crate::tools::application::glob_tool::GlobTool::new($fs.clone())
            ) as Arc<dyn Tool>);

            $tools.insert("grep".to_string(), Arc::new(
                crate::tools::application::grep_tool::GrepTool::new($fs.clone())
            ) as Arc<dyn Tool>);

            $tools.insert("edit".to_string(), Arc::new(
                crate::tools::application::edit_tool::EditTool::new($fs.clone())
            ) as Arc<dyn Tool>);

            $tools.insert("head_tail".to_string(), Arc::new(
                crate::tools::application::head_tail_tool::HeadTailTool::new($fs)
            ) as Arc<dyn Tool>);

            // CommandExecutor-dependent tools
            $tools.insert("bash".to_string(), Arc::new(
                crate::tools::application::bash_tool::BashTool::new($executor)
            ) as Arc<dyn Tool>);

            // Standalone tools (no dependencies)
            $tools.insert("todowrite".to_string(), Arc::new(
                crate::tools::application::todo_tool::TodoWriteTool::new()
            ) as Arc<dyn Tool>);

            $tools.insert("batch".to_string(), Arc::new(
                crate::tools::application::batch_tool::BatchTool::new()
            ) as Arc<dyn Tool>);

            // Web tools (behind feature flags)
            #[cfg(feature = "web-tools")]
            $tools.insert("webfetch".to_string(), Arc::new(
                crate::tools::application::webfetch_tool::WebFetchTool::new()
            ) as Arc<dyn Tool>);

            // Patch tools (behind feature flags)
            #[cfg(feature = "patch-tool")]
            $tools.insert("apply_patch".to_string(), Arc::new(
                crate::tools::application::apply_patch_tool::ApplyPatchTool::new()
            ) as Arc<dyn Tool>);

            // Search tools (behind feature flags)
            #[cfg(feature = "websearch")]
            $tools.insert("websearch".to_string(), Arc::new(
                crate::tools::application::websearch_tool::WebSearchTool::new()
            ) as Arc<dyn Tool>);

            #[cfg(feature = "codesearch")]
            $tools.insert("codesearch".to_string(), Arc::new(
                crate::tools::application::codesearch_tool::CodeSearchTool::new()
            ) as Arc<dyn Tool>);
        }
    };
}

/// Build the tool map for the CodingAgent
///
/// This is the tool registry that registers all available tools.
/// Tools are automatically registered via the `register_tool_*!` macros.
///
/// # Adding a new tool
///
/// To add a new tool:
/// 1. Create the tool implementation in `src/tools/application/your_tool.rs`
/// 2. Add the module declaration to `src/tools/application/mod.rs`
/// 3. Add a registration macro call at the end of your tool file:
///    ```rust,ignore
///    register_tool_fs!(YourTool, "your_tool_id");
///    ```
/// 4. Add the tool to the `collect_tools!` macro above
pub fn build_tool_map() -> HashMap<String, Arc<dyn Tool>> {
    let mut tools = HashMap::new();

    // Create platform services
    let fs = create_filesystem();
    let executor = create_command_executor();

    // Register all tools using the collect_tools macro
    collect_tools!(tools, fs, executor);

    // Set the global tool registry for tools that need access to other tools
    let _ = TOOL_REGISTRY.set(Arc::new(tools.clone()));

    tools
}

/// Get the global tool registry (for tools that need access to other tools)
pub fn get_tool_registry() -> Option<Arc<HashMap<String, Arc<dyn Tool>>>> {
    TOOL_REGISTRY.get().cloned()
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
            expected_count += 4; // webfetch, apply_patch, websearch, codesearch
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
        // Minimal build should only have base tools (11)
        assert_eq!(tools.len(), 11);
        assert!(!tools.contains_key("webfetch"));
        assert!(!tools.contains_key("apply_patch"));
        assert!(!tools.contains_key("websearch"));
        assert!(!tools.contains_key("codesearch"));
    }
}
