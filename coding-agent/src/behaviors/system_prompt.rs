//! SystemPromptBehavior - Inject system prompt before inference
//!
//! This behavior hooks into the before_inference phase to inject
//! the system prompt that defines the agent's role and behavior.

use tirea::prelude::AgentBehavior;

/// SystemPromptBehavior - Inject system prompt
#[derive(Debug, Clone)]
pub struct SystemPromptBehavior {
    prompt: String,
}

impl SystemPromptBehavior {
    /// Create a new SystemPromptBehavior
    pub fn new() -> Self {
        Self {
            prompt: crate::prompt::SYSTEM_PROMPT.to_string(),
        }
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
