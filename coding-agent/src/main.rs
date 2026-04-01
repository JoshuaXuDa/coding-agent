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
mod logging;
mod query;
mod permissions;

use std::sync::Arc;
use tools::build_tool_map;
use ui::TuiApp;
use log::{info, debug};
use query::{QueryEngine, TireaQueryEngine};


/// Maximum number of inference rounds
const MAX_ROUNDS: usize = 50;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file if present
    dotenv::dotenv().ok();

    // Initialize logging
    logging::init_logging()?;

    info!("CodingAgent starting...");

    // Build the tool map - register all core tools
    let tools = build_tool_map();
    info!("Registered {} tools:", tools.len());
    let mut tool_names: Vec<_> = tools.keys().collect();
    tool_names.sort();
    for tool_name in tool_names {
        info!("  - {}", tool_name);
    }

    // Build AgentOs from configuration
    debug!("Loading configuration...");
    let agent_os = config::load_and_build_agent_os(tools)?;

    // Display model information
    if let Some(_agent) = agent_os.agent("coding-agent") {
        info!("Agent: coding-agent");
    }

    info!("AgentOS initialized successfully");

    // Build QueryEngine wrapping the AgentOs
    let query_engine: Arc<dyn QueryEngine> = Arc::new(TireaQueryEngine::new(agent_os));

    info!("Starting TUI Mode...");

    // Run TUI mode
    run_tui_mode(query_engine).await
}

/// Run the agent in TUI mode
async fn run_tui_mode(query_engine: Arc<dyn QueryEngine>) -> anyhow::Result<()> {
    let mut app = TuiApp::new(query_engine)?;
    app.run().await
}
