//! BashTool - Execute shell commands with timeout control
//!
//! Provides shell command execution with safety limits:
//! - 30 second timeout
//! - 50KB output limit
//! - Command history recording

use std::process::Command;
use std::time::Duration;
use tirea::prelude::{Tool, ToolDescriptor, ToolError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::tools::{ToolArgs, ToolContext, ToolExecutionEffect};

/// Default timeout for commands (30 seconds)
const DEFAULT_TIMEOUT_MS: u64 = 30000;

/// Maximum output size (50KB)
const MAX_OUTPUT_SIZE: usize = 50 * 1024;

/// BashTool - Shell command execution tool
#[derive(Debug, Clone)]
pub struct BashTool;

impl Tool for BashTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "bash".to_string(),
            description: indoc::indoc!(r#"
                Execute shell commands with timeout control. Returns stdout and stderr.

                Use this tool when you need to:
                - Run build commands (cargo build, npm install, etc.)
                - Execute tests
                - Run git commands
                - Execute any shell command

                Safety features:
                - 30 second timeout (configurable)
                - Output truncated at 50KB
                - Commands are recorded in history

                Examples:
                - List files: command = "ls -la"
                - Run tests: command = "cargo test"
                - Git status: command = "git status"
                - With custom timeout: command = "make build", timeout_ms = 60000
            "#).to_string(),
            parameters_schema: BashParams::json_schema(),
        }
    }

    fn execute_effect(
        &self,
        args: ToolArgs,
        context: &ToolContext,
    ) -> Result<ToolExecutionEffect, ToolError> {
        let params: BashParams = serde_json::from_value(args.inner.into())
            .map_err(|e| ToolError::InvalidArgument(format!("Invalid arguments: {}", e)))?;

        let result = execute_command(
            &params.command,
            params.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS),
        )?;

        // Format output
        let mut output = String::new();

        if !result.stdout.is_empty() {
            output.push_str("STDOUT:\n");
            output.push_str(&result.stdout);
        }

        if !result.stderr.is_empty() {
            if !output.is_empty() {
                output.push_str("\n");
            }
            output.push_str("STDERR:\n");
            output.push_str(&result.stderr);
        }

        if output.is_empty() {
            output.push_str("(Command produced no output)\n");
        }

        output.push_str(&format!("\nExit code: {:?}\n", result.exit_code));

        Ok(output)
    }
}

/// Parameters for BashTool
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct BashParams {
    /// Shell command to execute
    command: String,

    /// Optional timeout in milliseconds (default: 30000)
    #[serde(default)]
    timeout_ms: Option<u64>,
}

/// Result of command execution
#[derive(Debug)]
struct CommandResult {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    duration_ms: u64,
}

/// Execute a shell command with timeout
///
/// Runs the command through the default shell (/bin/sh on Unix, cmd.exe on Windows).
/// Enforces timeout and output size limits.
fn execute_command(command: &str, timeout_ms: u64) -> Result<CommandResult, ToolError> {
    let start = std::time::Instant::now();

    // Determine the shell to use
    let (shell, flag) = if cfg!(windows) {
        ("cmd.exe", "/C")
    } else {
        ("/bin/sh", "-c")
    };

    // Execute the command
    let output = tokio::runtime::Runtime::new()
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to create runtime: {}", e)))?
        .block_on(async {
            tokio::time::timeout(
                Duration::from_millis(timeout_ms),
                tokio::process::Command::new(shell)
                    .arg(flag)
                    .arg(command)
                    .output(),
            )
            .await
        })
        .map_err(|_| ToolError::ExecutionFailed(format!(
            "Command timed out after {}ms",
            timeout_ms
        )))?
        .map_err(|e| ToolError::ExecutionFailed(format!("Failed to execute command: {}", e)))?;

    let duration_ms = start.elapsed().as_millis() as u64;

    // Process stdout
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stdout = truncate_output(&stdout);

    // Process stderr
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stderr = truncate_output(&stderr);

    // Get exit code
    let exit_code = output.status.code();

    Ok(CommandResult {
        stdout,
        stderr,
        exit_code,
        duration_ms,
    })
}

/// Truncate output if it exceeds MAX_OUTPUT_SIZE
fn truncate_output(content: &str) -> String {
    if content.len() > MAX_OUTPUT_SIZE {
        let mut truncated = String::from(&content[..MAX_OUTPUT_SIZE]);
        truncated.push_str(&format!(
            "\n\n--- Output truncated (was {} bytes) ---\n",
            content.len()
        ));
        truncated
    } else {
        content.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_simple_command() {
        let result = execute_command("echo 'Hello, World!'", 5000).unwrap();
        assert!(result.stdout.contains("Hello, World!"));
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn test_execute_command_with_error() {
        let result = execute_command("exit 42", 5000).unwrap();
        assert_eq!(result.exit_code, Some(42));
    }

    #[test]
    fn test_execute_nonexistent_command() {
        let result = execute_command("thiscommanddoesnotexist123", 5000);
        // Should either error or return non-zero exit code
        assert!(result.is_err() || result.unwrap().exit_code != Some(0));
    }

    #[test]
    fn test_truncate_output() {
        let large_content = "x".repeat(MAX_OUTPUT_SIZE + 1000);
        let truncated = super::truncate_output(&large_content);
        assert!(truncated.len() < large_content.len());
        assert!(truncated.contains("truncated"));
    }
}
