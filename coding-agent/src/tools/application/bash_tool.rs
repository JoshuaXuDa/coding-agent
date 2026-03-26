//! Bash tool - Application layer
//!
//! Orchestrates command execution to provide shell command capabilities.

use anyhow::Result;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;
use std::time::Duration;
use tirea::prelude::{Tool, ToolDescriptor, ToolError, ToolResult};
use tirea_contract::ToolCallContext;
use crate::platform::domain::command::{CommandExecutor, CommandRequest};
use crate::tools::domain::validation::{validate_command, validate_command_args, validate_path, validate_timeout};
use crate::tools::domain::json_builder::JsonBuilder;

/// Bash tool
///
/// Provides shell command execution capabilities.
pub struct BashTool {
    /// Command executor service
    executor: Arc<dyn CommandExecutor>,
}

impl BashTool {
    /// Create a new bash tool
    pub fn new(executor: Arc<dyn CommandExecutor>) -> Self {
        Self { executor }
    }

    /// Parse tool arguments
    fn parse_args(args: &serde_json::Value) -> Result<BashArgs> {
        let command_str = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' argument"))?;

        // Smart parsing: if command contains spaces and args array is empty/missing,
        // split the command into command and args
        let (command, args_list) = if command_str.contains(' ') {
            let parts: Vec<&str> = command_str.split_whitespace().collect();
            let cmd = parts.first()
                .ok_or_else(|| anyhow::anyhow!("Command cannot be empty"))?;

            // Check if args were explicitly provided
            let has_explicit_args = args.get("args")
                .and_then(|v| v.as_array())
                .map(|arr| !arr.is_empty())
                .unwrap_or(false);

            if has_explicit_args {
                // User provided both command with spaces AND explicit args
                // Use explicit args
                let explicit_args: Vec<String> = args.get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                (*cmd, explicit_args)
            } else {
                // Auto-split: "git status" -> command="git", args=["status"]
                let auto_args: Vec<String> = parts.iter().skip(1).map(|s| s.to_string()).collect();
                (*cmd, auto_args)
            }
        } else {
            // No spaces in command, use as-is
            let explicit_args: Vec<String> = args.get("args")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();
            (command_str, explicit_args)
        };

        // Validate command for security
        validate_command(command)?;

        // Validate command arguments
        validate_command_args(&args_list)?;

        // Validate command arguments
        validate_command_args(&args_list)?;

        let working_dir = args
            .get("working_dir")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Validate working directory if provided
        if let Some(dir) = &working_dir {
            validate_path(dir)?;
        }

        let timeout_secs = args
            .get("timeout")
            .and_then(|v| v.as_u64());

        // Validate timeout if provided
        if let Some(secs) = timeout_secs {
            validate_timeout(secs)?;
        }

        let timeout = timeout_secs.map(|v| Duration::from_secs(v));

        Ok(BashArgs {
            command: command.to_string(),
            args: args_list,
            working_dir,
            timeout,
        })
    }

    /// Build command request from args
    fn build_command_request(&self, args: &BashArgs) -> CommandRequest {
        let mut request = CommandRequest::new(&args.command)
            .with_args(args.args.clone());

        if let Some(dir) = &args.working_dir {
            request = request.with_working_dir(dir);
        }

        if let Some(timeout) = args.timeout {
            request = request.with_timeout(timeout);
        }

        request
    }
}

/// Bash tool arguments
#[derive(Debug, Clone)]
struct BashArgs {
    command: String,
    args: Vec<String>,
    working_dir: Option<String>,
    timeout: Option<Duration>,
}

impl Tool for BashTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: "bash".to_string(),
            name: "bash".to_string(),
            description: "Execute shell commands and return the output".to_string(),
            category: Some("execution".to_string()),
            parameters: serde_json::json!({
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                },
                "args": {
                    "type": "array",
                    "description": "Command arguments",
                    "items": {"type": "string"}
                },
                "working_dir": {
                    "type": "string",
                    "description": "Working directory for command execution"
                },
                "timeout": {
                    "type": "number",
                    "description": "Maximum time to wait in seconds (default: no timeout)"
                }
            }),
            metadata: Default::default(),
        }
    }

    fn execute<'life0: 'async_trait, 'life1: 'async_trait, 'life2: 'async_trait, 'async_trait>(
        &'life0 self,
        args: serde_json::Value,
        _context: &'life1 ToolCallContext<'life2>,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResult, ToolError>> + Send + 'async_trait>> {
        Box::pin(async move {
            // Parse arguments
            let bash_args = Self::parse_args(&args)
                ;

            // Check if command is available
            let is_avail = self.executor.is_available(&bash_args.command);

            if !is_avail {
                let json = JsonBuilder::build_error(
                    "bash",
                    "COMMAND_NOT_FOUND",
                    &format!("Command not found: {}", bash_args.command),
                    &format!("The command '{}' is not available", bash_args.command),
                );

                return Ok(ToolResult::success("bash", json));
            }

            // Build command request
            let request = self.build_command_request(&bash_args);

            // Execute command
            let result = self.executor.execute(request).await
                ;

            // Build XML response
            let command_display = if bash_args.args.is_empty() {
                bash_args.command.clone()
            } else {
                format!("{} {}", bash_args.command, bash_args.args.join(" "))
            };

            let json = JsonBuilder::build_command_result(&command_display, &result)
                ;

            Ok(ToolResult::success("bash", json))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_args() {
        let args = serde_json::json!({"command": "ls"});
        let parsed = BashTool::parse_args(&args).unwrap();
        assert_eq!(parsed.command, "ls");
        assert!(parsed.args.is_empty());
        assert!(parsed.working_dir.is_none());
        assert!(parsed.timeout.is_none());

        let args = serde_json::json!({
            "command": "git",
            "args": ["status"],
            "working_dir": "/tmp",
            "timeout": 30
        });
        let parsed = BashTool::parse_args(&args).unwrap();
        assert_eq!(parsed.command, "git");
        assert_eq!(parsed.args, vec!["status"]);
        assert_eq!(parsed.working_dir, Some("/tmp".to_string()));
        assert_eq!(parsed.timeout, Some(Duration::from_secs(30)));
    }

    #[test]
    fn test_parse_args_empty_command() {
        let args = serde_json::json!({"command": ""});
        assert!(BashTool::parse_args(&args).is_err());

        let args = serde_json::json!({"command": "   "});
        assert!(BashTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_command_injection() {
        // Test command chaining attempts
        let args = serde_json::json!({"command": "ls && rm -rf /"});
        assert!(BashTool::parse_args(&args).is_err());

        let args = serde_json::json!({"command": "cat /etc/passwd; echo done"});
        assert!(BashTool::parse_args(&args).is_err());

        let args = serde_json::json!({"command": "echo $(whoami)"});
        assert!(BashTool::parse_args(&args).is_err());

        let args = serde_json::json!({"command": "ls `whoami`"});
        assert!(BashTool::parse_args(&args).is_err());

        let args = serde_json::json!({"command": "ls > /tmp/output"});
        assert!(BashTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_path_traversal() {
        let args = serde_json::json!({
            "command": "ls",
            "working_dir": "../../etc"
        });
        assert!(BashTool::parse_args(&args).is_err());

        let args = serde_json::json!({
            "command": "ls",
            "working_dir": "/tmp/../etc"
        });
        assert!(BashTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_timeout_validation() {
        // Timeout too short
        let args = serde_json::json!({
            "command": "ls",
            "timeout": 0
        });
        assert!(BashTool::parse_args(&args).is_err());

        // Timeout too long
        let args = serde_json::json!({
            "command": "ls",
            "timeout": 700
        });
        assert!(BashTool::parse_args(&args).is_err());

        // Valid timeout
        let args = serde_json::json!({
            "command": "ls",
            "timeout": 30
        });
        assert!(BashTool::parse_args(&args).is_ok());
    }

    #[test]
    fn test_parse_args_dangerous_arguments() {
        let args = serde_json::json!({
            "command": "echo",
            "args": ["$(whoami)"]
        });
        assert!(BashTool::parse_args(&args).is_err());

        let args = serde_json::json!({
            "command": "echo",
            "args": ["test;rm -rf /"]
        });
        assert!(BashTool::parse_args(&args).is_err());
    }

    #[test]
    fn test_parse_args_valid_complex_commands() {
        // Valid multi-part command
        let args = serde_json::json!({
            "command": "cargo",
            "args": ["build", "--release"]
        });
        assert!(BashTool::parse_args(&args).is_ok());

        // Valid command with path
        let args = serde_json::json!({
            "command": "/usr/bin/git",
            "args": ["status"]
        });
        assert!(BashTool::parse_args(&args).is_ok());
    }
}
