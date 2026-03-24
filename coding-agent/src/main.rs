//! CodingAgent - An intelligent code editing agent built on Tirea framework
//!
//! This is the main entry point for the CodingAgent application.
//! It uses DDD architecture with clear bounded contexts for Tools, State, and Behaviors.

mod state;
mod tools;
mod behaviors;
mod config;
mod llm_logger;
mod platform;
mod context;
mod ui;

use std::io::{self, Write};

use tools::build_tool_map;
use tirea_agentos::AgentOs;
use tirea::contracts::AgentEvent;
use llm_logger::LlmLogger;
use std::time::Instant;
use rustyline::{Editor, error::ReadlineError};


/// Maximum number of inference rounds
const MAX_ROUNDS: usize = 50;

/// Session storage directory
const SESSION_DIR: &str = "./sessions";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenv::dotenv().ok();

    // Initialize logging
    env_logger::init();

    println!("🤖 CodingAgent starting...");

    // Build the tool map - register all 6 core tools
    let tools = build_tool_map();
    println!("✅ Registered {} tools:", tools.len());
    let mut tool_names: Vec<_> = tools.keys().collect();
    tool_names.sort();
    for tool_name in tool_names {
        println!("   - {}", tool_name);
    }
    println!();

    // Build AgentOs from configuration
    println!("📝 Loading configuration...");
    let agent_os = config::load_and_build_agent_os(tools)?;

    // Display model information
    if let Some(agent) = agent_os.agent("coding-agent") {
        println!("✅ Agent: coding-agent");
    }
    println!();

    println!("✅ AgentOS initialized successfully");
    println!();
    println!("═══════════════════════════════════════════════════════════");
    println!("  CodingAgent Ready - Type your message below");
    println!("═══════════════════════════════════════════════════════════");
    println!();

    // Run CLI mode
    run_cli_mode(agent_os).await
}

/// Run the agent in CLI mode with readline support
async fn run_cli_mode(agent_os: AgentOs) -> anyhow::Result<()> {
    // Initialize readline editor with file reference completion
    let fs = crate::platform::create_filesystem();
    let helper = ui::FileReferenceHelper::new(fs);
    let mut rl = Editor::new()?;
    rl.set_helper(Some(helper));

    // Load command history
    let history_path = ".coding_agent_history";
    if let Err(_) = rl.load_history(history_path) {
        println!("📝 No previous history found, starting fresh");
    }

    // Initialize LLM logger
    let mut logger = LlmLogger::new()?;
    println!("📝 LLM interaction logging enabled (logs/llm_interactions.log)");
    println!();
    println!("💡 提示:");
    println!("   - 输入 @ 进入 TUI 文件选择器");
    println!("   - 或输入 @ 后按 Tab 键自动补全");
    println!();

    loop {
        // Read user input with readline support
        let input = match rl.readline("You> ") {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("👋 Goodbye!");
                break;
            }
            Err(err) => {
                println!("❌ Error reading input: {}", err);
                break;
            }
        };

        let input = input.trim();

        // Handle empty input
        if input.is_empty() {
            continue;
        }

        // Check if input ends with @ to trigger TUI file selector
        let final_input = if input.ends_with('@') {
            let fs = crate::platform::create_filesystem();
            let mut selector = ui::TuiFileSelector::new(fs);

            // Search for files (empty pattern to show all)
            if let Err(e) = selector.search("") {
                eprintln!("⚠️  文件搜索失败: {}", e);
                input.to_string()
            } else {
                match selector.run() {
                    Ok(Some(selected_file)) => {
                        // User selected a file, append it to @
                        format!("{}{}", input, selected_file.display())
                    }
                    Ok(None) => {
                        // User cancelled, remove the @
                        input[..input.len()-1].to_string()
                    }
                    Err(e) => {
                        eprintln!("⚠️  TUI 错误: {}", e);
                        input.to_string()
                    }
                }
            }
        } else {
            input.to_string()
        };

        // Save original input to history
        rl.add_history_entry(input);

        // Handle exit commands
        if matches!(final_input.as_str(), "exit" | "quit" | "q") {
            println!("👋 Goodbye!");
            break;
        }

        // Process the user's message
        println!();
        println!("🔄 Processing...");

        // Preprocess: Expand @ file references
        // Non-interactive mode since TUI provides selection during input
        let enhanced_message = match context::ContextBuilder::new()
            .with_filesystem(crate::platform::create_filesystem())
            .with_interactive_mode(false)
            .build_context(&final_input)
            .await
        {
            Ok(enhanced) => enhanced,
            Err(e) => {
                println!("⚠️  上下文构建失败: {}", e);
                println!("使用原始消息继续...");
                final_input.clone()
            }
        };

        match process_message(&agent_os, enhanced_message, &mut logger).await {
            Ok(response) => {
                println!();
                println!("═══════════════════════════════════════════════════════════");
                println!("Agent Response:");
                println!("═══════════════════════════════════════════════════════════");
                println!("{}", response);
                println!("═══════════════════════════════════════════════════════════");
            }
            Err(e) => {
                println!();
                println!("❌ Error: {}", e);
                let _ = logger.log_error(&e.to_string());
            }
        }

        // Flush logger after each message
        let _ = logger.flush();
        println!();
    }

    // Save history before exiting
    if let Err(err) = rl.save_history(history_path) {
        eprintln!("⚠️  Failed to save history: {}", err);
    }

    Ok(())
}

/// Process a user message through the agent with retry on stream errors
async fn process_message(
    agent_os: &AgentOs,
    message: String,
    logger: &mut LlmLogger,
) -> anyhow::Result<String> {
    use tirea::prelude::Message;
    use tirea_contract::RunRequest;

    // Log the user request
    logger.log_request(&message)?;

    // Create run request with the user message
    let run_request = RunRequest {
        agent_id: "coding-agent".to_string(),
        thread_id: None, // auto-generate
        run_id: None,    // auto-generate
        parent_run_id: None,
        parent_thread_id: None,
        resource_id: None,
        origin: Default::default(),
        state: None,
        messages: vec![Message::user(message)],
        initial_decisions: vec![],
    };

    // Retry logic for stream errors
    let max_retries = 3;
    let mut retry_count = 0;

    loop {
        // Start timing
        let start_time = Instant::now();

        // Run the agent with streaming output
        let mut stream = match agent_os.run_stream(run_request.clone()).await {
            Ok(s) => s,
            Err(e) => {
                if retry_count < max_retries && e.to_string().contains("utf-8") {
                    retry_count += 1;
                    eprintln!("⚠️  UTF-8 stream error, retrying ({}/{})...", retry_count, max_retries);
                    std::thread::sleep(std::time::Duration::from_millis(500 * retry_count as u64));
                    continue;
                }
                return Err(e.into());
            }
        };

        let mut final_response = String::new();
        let mut stream_error = None;
        use futures::StreamExt;

        while let Some(event) = stream.events.next().await {
            match event {
                AgentEvent::TextDelta { delta, .. } => {
                    print!("{}", delta);
                    let _ = io::stdout().flush();
                    final_response.push_str(&delta);
                }
                AgentEvent::ToolCallStart { name, .. } => {
                    println!("\n[Calling tool: {}]", name);
                    let _ = logger.log_tool_call(&name, &serde_json::json!({}));
                }
                AgentEvent::ToolCallDone { .. } => {
                    println!("[Tool done]");
                }
                AgentEvent::Error { message, .. } => {
                    if message.contains("incomplete utf-8") || message.contains("utf-8") {
                        stream_error = Some(message);
                        break; // Exit the event loop
                    }

                    eprintln!("ERROR: {}", message);
                    let _ = logger.log_error(&message);
                    return Err(anyhow::anyhow!("Agent error: {}", message));
                }
                _ => {
                    // Ignore other events
                }
            }
        }

        // If we had a UTF-8 stream error and haven't exceeded retries, try again
        if let Some(err_msg) = stream_error {
            if retry_count < max_retries {
                retry_count += 1;
                eprintln!("⚠️  UTF-8 stream error, retrying ({}/{})...", retry_count, max_retries);
                std::thread::sleep(std::time::Duration::from_millis(500 * retry_count as u64));

                // Clear any partial output before retry
                if !final_response.is_empty() {
                    eprintln!("\n[Partial response received, retrying...]");
                }
                continue;
            } else {
                eprintln!("⚠️  Max retries reached, returning partial response");
                // Return what we have instead of error
            }
        }

        let duration = start_time.elapsed().as_millis() as u64;
        println!(); // newline after streaming output

        // Log the response
        logger.log_response(&final_response, duration)?;

        return Ok(final_response);
    }
}
