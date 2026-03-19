//! CodingAgent - An intelligent code editing agent built on Tirea framework
//!
//! This is the main entry point for the CodingAgent application.
//! It uses DDD architecture with clear bounded contexts for Tools, State, and Behaviors.

mod state;
mod tools;
mod behaviors;
mod prompt;

use std::sync::Arc;
use std::env;
use std::io::{self, Write};

use tools::build_tool_map;
use behaviors::{SystemPromptBehavior, OutputGuardBehavior};
use prompt::SYSTEM_PROMPT;

/// Default model to use if not specified by environment variable
const DEFAULT_MODEL: &str = "claude-sonnet-4-6";

/// Environment variable for model selection
const MODEL_ENV_VAR: &str = "AGENT_MODEL";

/// Environment variable for API key
const API_KEY_ENV_VAR: &str = "ANTHROPIC_API_KEY";

/// Maximum number of inference rounds
const MAX_ROUNDS: usize = 50;

/// Session storage directory
const SESSION_DIR: &str = "./sessions";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    env_logger::init();

    // Verify API key is present
    check_api_key()?;

    // Get model from environment or use default
    let model = env::var(MODEL_ENV_VAR).unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    println!("🤖 CodingAgent starting...");
    println!("📦 Model: {}", model);
    println!("📁 Session directory: {}", SESSION_DIR);
    println!();

    // Build the tool map - register all 6 core tools
    let tools = build_tool_map();
    println!("✅ Registered {} tools:", tools.len());
    let mut tool_names: Vec<_> = tools.keys().collect();
    tool_names.sort();
    for tool_name in tool_names {
        println!("   - {}", tool_name);
    }
    println!();

    println!("✅ AgentOS initialized successfully");
    println!();
    println!("═══════════════════════════════════════════════════════════");
    println!("  CodingAgent Ready - Type your message below");
    println!("═══════════════════════════════════════════════════════════");
    println!();

    // Run CLI mode
    run_cli_mode(tools).await
}

/// Check if API key is present in environment
fn check_api_key() -> anyhow::Result<()> {
    if env::var(API_KEY_ENV_VAR).is_err() {
        anyhow::bail!(
            "❌ {} environment variable not set!\n\
             Please set it with: export {}=\"your-key-here\"",
            API_KEY_ENV_VAR, API_KEY_ENV_VAR
        );
    }
    Ok(())
}

/// Run the agent in CLI mode (stdin/stdout)
async fn run_cli_mode(tools: std::collections::HashMap<String, Arc<dyn tirea::prelude::Tool>>) -> anyhow::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        // Display prompt
        print!("You> ");
        stdout.flush()?;

        // Read user input
        let mut input = String::new();
        stdin.read_line(&mut input)?;

        let input = input.trim();

        // Handle empty input
        if input.is_empty() {
            continue;
        }

        // Handle exit commands
        if matches!(input, "exit" | "quit" | "q") {
            println!("👋 Goodbye!");
            break;
        }

        // Process the user's message
        println!();
        println!("🔄 Processing...");

        match process_message(&tools, input.to_string()).await {
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
            }
        }

        println!();
    }

    Ok(())
}

/// Process a user message through the agent
async fn process_message(
    _tools: &std::collections::HashMap<String, Arc<dyn tirea::prelude::Tool>>,
    message: String,
) -> anyhow::Result<String> {
    // For now, just return a placeholder response
    // TODO: Implement actual agent processing
    Ok(format!("Received: {}", message))
}
