use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;
use anyhow::Result;

#[derive(Debug, Clone)]
struct CommandRequest {
    command: String,
    args: Vec<String>,
    working_dir: Option<String>,
    env: HashMap<String, String>,
    timeout: Option<Duration>,
    capture_stdout: bool,
    capture_stderr: bool,
}

impl CommandRequest {
    fn new(command: &str) -> Self {
        Self {
            command: command.to_string(),
            args: Vec::new(),
            working_dir: None,
            env: HashMap::new(),
            timeout: None,
            capture_stdout: true,
            capture_stderr: true,
        }
    }

    fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }
}

struct UnixCommandExecutor;

impl UnixCommandExecutor {
    fn new() -> Self {
        Self
    }

    fn command_exists(command: &str) -> bool {
        let result = Command::new("sh")
            .arg("-c")
            .arg(format!("which {}", command))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();

        result.map(|status| status.success()).unwrap_or(false)
    }

    fn is_available(&self, command: &str) -> bool {
        if matches!(command, "sh" | "bash" | "zsh" | "dash") {
            return true;
        }
        Self::command_exists(command)
    }

    async fn execute(&self, request: CommandRequest) -> Result<CommandResult> {
        let start = std::time::Instant::now();

        // Use /usr/bin/env to ensure command is found in PATH
        let mut cmd = tokio::process::Command::new("/usr/bin/env");
        cmd.arg(&request.command);
        cmd.args(&request.args);

        if let Some(dir) = &request.working_dir {
            cmd.current_dir(dir);
        }

        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        if request.capture_stdout {
            cmd.stdout(std::process::Stdio::piped());
        } else {
            cmd.stdout(std::process::Stdio::inherit());
        }

        if request.capture_stderr {
            cmd.stderr(std::process::Stdio::piped());
        } else {
            cmd.stderr(std::process::Stdio::inherit());
        }

        let output = cmd.output().await?;
        let duration_ms = start.elapsed().as_millis() as u64;

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

        let exit_code = output.status.code();

        let result = match exit_code {
            Some(0) => CommandResult::success(request.command.clone(), 0, stdout, stderr, duration_ms),
            Some(code) => CommandResult::failure(request.command.clone(), code, stdout, stderr, duration_ms),
            None => CommandResult::terminated(request.command.clone(), stdout, stderr, duration_ms),
        };

        Ok(result)
    }
}

#[derive(Debug)]
struct CommandResult {
    command: String,
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    duration_ms: u64,
}

impl CommandResult {
    fn success(command: String, exit_code: i32, stdout: String, stderr: String, duration_ms: u64) -> Self {
        Self {
            command,
            success: true,
            exit_code: Some(exit_code),
            stdout,
            stderr,
            duration_ms,
        }
    }

    fn failure(command: String, exit_code: i32, stdout: String, stderr: String, duration_ms: u64) -> Self {
        Self {
            command,
            success: false,
            exit_code: Some(exit_code),
            stdout,
            stderr,
            duration_ms,
        }
    }

    fn terminated(command: String, stdout: String, stderr: String, duration_ms: u64) -> Self {
        Self {
            command,
            success: false,
            exit_code: None,
            stdout,
            stderr,
            duration_ms,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== BashTool Integration Test ===\n");

    let executor = UnixCommandExecutor::new();

    // Test 1: Check if git is available
    println!("Test 1: Check if git is available");
    let available = executor.is_available("git");
    println!("   is_available(\"git\") = {}\n", available);

    if !available {
        println!("   ❌ ERROR: git is not available! Cannot continue tests.");
        return Ok(());
    }

    // Test 2: Execute git --version
    println!("Test 2: Execute git --version");
    let request = CommandRequest::new("git").with_args(vec!["--version".to_string()]);
    match executor.execute(request).await {
        Ok(result) => {
            if result.success {
                println!("   ✅ Success");
                println!("   Command: {}", result.command);
                println!("   Output: {}", result.stdout.trim());
                println!("   Duration: {}ms\n", result.duration_ms);
            } else {
                println!("   ❌ Failed with exit code {:?}", result.exit_code);
                println!("   Stderr: {}\n", result.stderr);
            }
        }
        Err(e) => {
            println!("   ❌ Error: {}\n", e);
        }
    }

    // Test 3: Execute git status
    println!("Test 3: Execute git status");
    let request = CommandRequest::new("git").with_args(vec!["status".to_string()]);
    match executor.execute(request).await {
        Ok(result) => {
            if result.success {
                println!("   ✅ Success");
                println!("   Output preview:\n{}", result.stdout.chars().take(200).collect::<String>());
                println!("   Duration: {}ms\n", result.duration_ms);
            } else {
                println!("   ⚠️  Exit code {:?} (may be expected if not in git repo)", result.exit_code);
                println!("   Stderr: {}\n", result.stderr);
            }
        }
        Err(e) => {
            println!("   ❌ Error: {}\n", e);
        }
    }

    // Test 4: Execute ls
    println!("Test 4: Execute ls (should always work)");
    let request = CommandRequest::new("ls").with_args(vec!["-la".to_string(), "/tmp".to_string()]);
    match executor.execute(request).await {
        Ok(result) => {
            if result.success {
                println!("   ✅ Success");
                println!("   Output preview:\n{}", result.stdout.chars().take(200).collect::<String>());
            } else {
                println!("   ❌ Failed with exit code {:?}", result.exit_code);
            }
        }
        Err(e) => {
            println!("   ❌ Error: {}", e);
        }
    }

    println!("\n=== All Tests Completed ===");
    Ok(())
}
