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
use crate::tools::domain::xml_builder::XmlBuilder;

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
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' argument"))?;

        let args_list = args
            .get("args")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let working_dir = args
            .get("working_dir")
            .and_then(|v| v.as_str())
            .map(String::from);

        let timeout_secs = args
            .get("timeout")
            .and_then(|v| v.as_u64())
            .map(|v| Duration::from_secs(v));

        Ok(BashArgs {
            command: command.to_string(),
            args: args_list,
            working_dir,
            timeout: timeout_secs,
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
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Check if command is available
            if !self.executor.is_available(&bash_args.command) {
                let xml = XmlBuilder::build_error(
                    "bash",
                    "COMMAND_NOT_FOUND",
                    &format!("Command not found: {}", bash_args.command),
                    &format!("The command '{}' is not available", bash_args.command),
                ).map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

                return Ok(ToolResult::success("bash", xml));
            }

            // Build command request
            let request = self.build_command_request(&bash_args);

            // Execute command
            let result = self.executor.execute(request).await
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            // Build XML response
            let command_display = if bash_args.args.is_empty() {
                bash_args.command.clone()
            } else {
                format!("{} {}", bash_args.command, bash_args.args.join(" "))
            };

            let xml = XmlBuilder::build_command_result_xml(&command_display, &result)
                .map_err(|e: anyhow::Error| ToolError::ExecutionFailed(e.to_string()))?;

            Ok(ToolResult::success("bash", xml))
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
}
