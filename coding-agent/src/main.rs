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

use tools::build_tool_map;
use tirea_agentos::AgentOs;
use ui::TuiApp;
use log::{info, debug};


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
    info!("Starting TUI Mode...");

    // Run TUI mode
    run_tui_mode(agent_os).await
}

/// Run the agent in TUI mode
async fn run_tui_mode(agent_os: AgentOs) -> anyhow::Result<()> {
    let mut app = TuiApp::new(agent_os)?;
    app.run().await
}
