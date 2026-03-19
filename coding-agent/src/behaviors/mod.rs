//! Behaviors module for CodingAgent
//!
//! This module implements the cross-cutting concerns using the AOP pattern.
//! Behaviors are registered with the agent and hook into the inference loop.

mod system_prompt;
mod output_guard;

pub use system_prompt::SystemPromptBehavior;
pub use output_guard::OutputGuardBehavior;
