//! SystemPromptBehavior - Inject system prompt before inference
//!
//! This behavior hooks into the before_inference phase to inject
//! the system prompt that defines the agent's role and behavior.

use tirea::prelude::AgentBehavior;
use std::path::Path;
use anyhow::Context;

/// SystemPromptBehavior - Inject system prompt
#[derive(Debug, Clone)]
pub struct SystemPromptBehavior {
    prompt: String,
}

impl SystemPromptBehavior {
    /// Create a new SystemPromptBehavior
    pub fn new() -> Self {
        Self {
            prompt: Self::load_prompt().unwrap_or_else(|_| "You are a helpful coding assistant.".to_string()),
        }
    }

    /// Load the system prompt from the external file
    fn load_prompt() -> anyhow::Result<String> {
        let prompt_path = Path::new("config/prompt.txt");
        std::fs::read_to_string(prompt_path)
            .with_context(|| format!("Failed to read system prompt file: {}", prompt_path.display()))
    }

    /// Create a behavior with a custom prompt
    pub fn with_prompt(prompt: String) -> Self {
        Self { prompt }
    }

    /// Get the system prompt
    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_creation() {
        let behavior = SystemPromptBehavior::new();
        assert!(!behavior.prompt.is_empty());
    }

    #[test]
    fn test_system_prompt_with_custom() {
        let custom = "You are a custom assistant.";
        let behavior = SystemPromptBehavior::with_prompt(custom.to_string());
        assert_eq!(behavior.prompt, custom);
    }
}
