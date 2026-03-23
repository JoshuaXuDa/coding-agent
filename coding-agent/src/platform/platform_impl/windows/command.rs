//! Windows command execution implementation
//!
//! Provides command execution capabilities for Windows using cmd.exe
/// and PowerShell.

use anyhow::{Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use crate::platform::domain::command::{CommandExecutor, CommandRequest, CommandResult};

/// Windows command executor
///
/// Executes shell commands on Windows using cmd.exe or PowerShell.
pub struct WindowsCommandExecutor;

impl WindowsCommandExecutor {
    /// Create a new Windows command executor
    pub fn new() -> Self {
        Self
    }

    /// Check if a command exists on Windows
    fn command_exists(command: &str) -> bool {
        // Try to execute with /? to check if it exists
        let result = std::process::Command::new(&command)
            .arg("/?")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        result.is_ok()
    }
}

impl Default for WindowsCommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandExecutor for WindowsCommandExecutor {
    async fn execute(&self, request: CommandRequest) -> Result<CommandResult> {
        let start = std::time::Instant::now();

        // Build the command
        let mut cmd = TokioCommand::new(&request.command);
        cmd.args(&request.args);

        // Set working directory if specified
        if let Some(dir) = &request.working_dir {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        // Configure output capture
        if request.capture_stdout {
            cmd.stdout(Stdio::piped());
        } else {
            cmd.stdout(Stdio::inherit());
        }

        if request.capture_stderr {
            cmd.stderr(Stdio::piped());
        } else {
            cmd.stderr(Stdio::inherit());
        }

        // Execute the command
        let output = if let Some(timeout) = request.timeout {
            // Execute with timeout
            tokio::select! {
                result = cmd.output() => result,
                _ = tokio::time::sleep(timeout) => {
                    return Ok(CommandResult::terminated(
                        request.full_command(),
                        String::new(),
                        format!("Command timed out after {:?}", timeout),
                        start.elapsed().as_millis() as u64,
                    ));
                }
            }
        } else {
            cmd.output().await
        };

        let output = output.context(format!("Failed to execute command: {}", request.command))?;
        let duration_ms = start.elapsed().as_millis() as u64;

        // Extract stdout and stderr
        let stdout = if request.capture_stdout {
            String::from_utf8_lossy(&output.stdout).to_string()
        } else {
            String::new()
        };

        let stderr = if request.capture_stderr {
            String::from_utf8_lossy(&output.stderr).to_string()
        } else {
            String::new()
        };

        // Get exit code
        let exit_code = output.status.code();

        // Build result
        let result = match exit_code {
            Some(0) => CommandResult::success(
                request.full_command(),
                0,
                stdout,
                stderr,
                duration_ms,
            ),
            Some(code) => CommandResult::failure(
                request.full_command(),
                code,
                stdout,
                stderr,
                duration_ms,
            ),
            None => CommandResult::terminated(
                request.full_command(),
                stdout,
                stderr,
                duration_ms,
            ),
        };

        Ok(result)
    }

    async fn execute_command_string(&self, command: &str) -> Result<CommandResult> {
        // For Windows, we use cmd.exe /c to execute the command string
        let request = CommandRequest {
            command: "cmd".to_string(),
            args: vec!["/C".to_string(), command.to_string()],
            working_dir: None,
            env: HashMap::new(),
            timeout: None,
            capture_stdout: true,
            capture_stderr: true,
        };

        self.execute(request).await
    }

    fn is_available(&self, command: &str) -> bool {
        // Check for built-in commands
        if matches!(command, "cmd" | "cmd.exe" | "powershell" | "powershell.exe" | "pwsh" | "pwsh.exe") {
            return true;
        }

        // Check if command exists
        Self::command_exists(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_simple_command() {
        let executor = WindowsCommandExecutor::new();
        let request = CommandRequest::new("cmd").with_args(vec!["/C".to_string(), "echo".to_string(), "Hello".to_string()]);

        let result = executor.execute(request).await.unwrap();
        assert!(result.success);
        assert!(result.stdout.contains("Hello"));
    }

    #[tokio::test]
    async fn test_execute_command_string() {
        let executor = WindowsCommandExecutor::new();
        let result = executor.execute_command_string("echo Hello, World!").await.unwrap();

        assert!(result.success);
        assert!(result.stdout.contains("Hello, World!"));
    }

    #[test]
    fn test_is_available() {
        let executor = WindowsCommandExecutor::new();
        assert!(executor.is_available("cmd"));
        assert!(executor.is_available("cmd.exe"));
        assert!(!executor.is_available("nonexistent_command_xyz123"));
    }
}
