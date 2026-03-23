//! CodingAgent - An intelligent code editing agent built on Tirea framework
//!
//! This is the main entry point for the CodingAgent application.
//! It uses DDD architecture with clear bounded contexts for Tools, State, and Behaviors.

mod state;
mod tools;
mod behaviors;
mod prompt;
mod config;
mod llm_logger;
mod platform;

use std::io::{self, BufRead, Write};

use tools::build_tool_map;
use tirea_agentos::AgentOs;
use tirea::contracts::AgentEvent;
use llm_logger::LlmLogger;
use std::time::Instant;


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

/// Run the agent in CLI mode (stdin/stdout)
async fn run_cli_mode(agent_os: AgentOs) -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Initialize LLM logger
    let mut logger = LlmLogger::new()?;
    println!("📝 LLM interaction logging enabled (logs/llm_interactions.log)");
    println!();

    loop {
        // Display prompt
        print!("You> ");
        stdout.flush()?;

        // Read user input - handle invalid UTF-8 gracefully
        let mut input_bytes = Vec::new();
        stdin.lock().read_until(b'\n', &mut input_bytes)?;
        let input = String::from_utf8_lossy(&input_bytes).trim_end().to_string();

        // Handle empty input
        if input.is_empty() {
            continue;
        }

        // Handle exit commands
        if matches!(input.as_str(), "exit" | "quit" | "q") {
            println!("👋 Goodbye!");
            break;
        }

        // Process the user's message
        println!();
        println!("🔄 Processing...");

        match process_message(&agent_os, input.to_string(), &mut logger).await {
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

    Ok(())
}

/// Process a user message through the agent
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

    // Start timing
    let start_time = Instant::now();

    // Run the agent with streaming output
    let mut stream = agent_os.run_stream(run_request).await?;

    let mut final_response = String::new();
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
                // Log tool call (arguments not available in ToolCallStart event)
                let _ = logger.log_tool_call(&name, &serde_json::json!({}));
            }
            AgentEvent::ToolCallDone { .. } => {
                println!("[Tool done]");
            }
            AgentEvent::Error { message, .. } => {
                eprintln!("ERROR: {}", message);
                let _ = logger.log_error(&message);
                return Err(anyhow::anyhow!("Agent error: {}", message));
            }
            _ => {
                // Ignore other events
            }
        }
    }

    let duration = start_time.elapsed().as_millis() as u64;

    println!(); // newline after streaming output

    // Log the response
    logger.log_response(&final_response, duration)?;

    Ok(final_response)
}
