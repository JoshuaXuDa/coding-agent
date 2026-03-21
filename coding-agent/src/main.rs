//! CodingAgent - An intelligent code editing agent built on Tirea framework
//!
//! This is the main entry point for the CodingAgent application.
//! It uses DDD architecture with clear bounded contexts for Tools, State, and Behaviors.

mod state;
mod tools;
mod behaviors;
mod prompt;
mod client_resolver;

use std::sync::Arc;
use std::env;
use std::io::{self, Write};

use tools::build_tool_map;
use prompt::SYSTEM_PROMPT;
use tirea_agent_loop::runtime::loop_runner::{BaseAgent, run_loop_stream, GenaiLlmExecutor};
use tirea::contracts::{AgentEvent, RunContext, RunPolicy};
use futures::StreamExt;

/// Default model to use if not specified by environment variable
const DEFAULT_MODEL: &str = "claude-3-5-sonnet-20240620";

/// Environment variable for model selection
const MODEL_ENV_VAR: &str = "AGENT_MODEL";

/// Environment variable for API key
const API_KEY_ENV_VAR: &str = "ANTHROPIC_API_KEY";
const OPENAI_API_KEY_VAR: &str = "OPENAI_API_KEY";

/// Environment variable for base URL
const BASE_URL_ENV_VAR: &str = "ANTHROPIC_BASE_URL";
const OPENAI_BASE_URL_VAR: &str = "OPENAI_BASE_URL";

/// Get API key from environment (try OpenAI first, then Anthropic)
fn get_api_key() -> Result<String, env::VarError> {
    env::var(OPENAI_API_KEY_VAR).or_else(|_| env::var(API_KEY_ENV_VAR))
}

/// Get base URL from environment (try OpenAI first, then Anthropic)
fn get_base_url() -> Option<String> {
    env::var(OPENAI_BASE_URL_VAR).ok().or_else(|| env::var(BASE_URL_ENV_VAR).ok())
}

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

    // Verify API key is present
    check_api_key()?;

    // Get model from environment or use default
    let model = env::var(MODEL_ENV_VAR).unwrap_or_else(|_| DEFAULT_MODEL.to_string());

    // Get base URL from environment for debugging
    let base_url = get_base_url();

    println!("🤖 CodingAgent starting...");
    println!("📦 Model: {}", model);
    if let Some(ref url) = base_url {
        println!("🌐 Base URL: {}", url);
    }
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
    if get_api_key().is_err() {
        anyhow::bail!(
            "❌ API key not set!\n\
             Please set OPENAI_API_KEY or ANTHROPIC_API_KEY environment variable."
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
    tools: &std::collections::HashMap<String, Arc<dyn tirea::prelude::Tool>>,
    message: String,
) -> anyhow::Result<String> {
    use tirea::prelude::{Message, Thread};

    let model = env::var(MODEL_ENV_VAR).unwrap_or_else(|_| DEFAULT_MODEL.to_string());
    let base_url = get_base_url();
    eprintln!("DEBUG: Using model: {} (raw from env)", model);
    if let Some(ref url) = base_url {
        eprintln!("DEBUG: Base URL: {}", url);
    }

    // Create custom client with BigModel Coding endpoint resolver
    eprintln!("DEBUG: Creating custom client with BigModel Coding endpoint resolver");
    let custom_client = client_resolver::create_bigmodel_coding_client();
    eprintln!("DEBUG: Custom client created successfully");

    // Build agent using BaseAgent with custom client
    let agent = Arc::new(BaseAgent {
        id: "coding-agent".to_string(),
        model,
        system_prompt: SYSTEM_PROMPT.to_string(),
        max_rounds: MAX_ROUNDS,
        llm_executor: Some(Arc::new(GenaiLlmExecutor::new(custom_client))),
        ..BaseAgent::default()
    });

    // Create a fresh thread with the user message
    let mut thread = Thread::new("session");
    thread.messages = vec![Arc::new(Message::user(message))];

    let run_ctx = RunContext::from_thread(&thread, RunPolicy::default())
        .map_err(|e| anyhow::anyhow!("Failed to create run context: {}", e))?;

    eprintln!("DEBUG: Starting agent loop...");

    // Run the agent loop with streaming output (no state store needed)
    let mut stream = run_loop_stream(
        agent,
        tools.clone(),
        run_ctx,
        None, // no cancellation token
        None, // no state committer
        None, // no decision receiver
    );

    let mut final_response = String::new();
    let mut event_count = 0;
    while let Some(event) = stream.next().await {
        event_count += 1;
        match event {
            AgentEvent::TextDelta { delta, .. } => {
                print!("{}", delta);
                let _ = io::stdout().flush();
                final_response.push_str(&delta);
            }
            AgentEvent::ToolCallStart { name, .. } => {
                println!("\n[Calling tool: {}]", name);
            }
            AgentEvent::ToolCallDone { .. } => {
                println!("[Tool done]");
            }
            AgentEvent::Error { message, .. } => {
                eprintln!("ERROR: {}", message);
                return Err(anyhow::anyhow!("Agent error: {}", message));
            }
            _ => {
                if event_count <= 10 {
                    eprintln!("DEBUG: Event {}: {:?}", event_count, std::mem::discriminant(&event));
                }
            }
        }
    }
    eprintln!("DEBUG: Processed {} events, final_response length: {}", event_count, final_response.len());
    println!(); // newline after streaming output
    Ok(final_response)
}
