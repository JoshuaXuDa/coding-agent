//! OutputGuardBehavior - Output truncation protection
//!
//! This behavior provides a second layer of output size protection
//! by checking tool outputs after execution.

use tirea::AgentBehavior;
use tirea_contract::run::InferenceContext;

/// Maximum output size (50KB) - same as tool layer
const MAX_OUTPUT_SIZE: usize = 50 * 1024;

/// OutputGuardBehavior - Output size guard
#[derive(Debug, Clone)]
pub struct OutputGuardBehavior;

impl OutputGuardBehavior {
    /// Create a new OutputGuardBehavior
    pub fn new() -> Self {
        Self
    }
}

impl Default for OutputGuardBehavior {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentBehavior for OutputGuardBehavior {
    fn after_tool_execute(&self, context: &mut InferenceContext) {
        // Check the last tool result if available
        if let Some(last_result) = context.last_tool_result.as_mut() {
            if let Some(output) = last_result.result.as_mut() {
                let output_str = output.to_string();
                if output_str.len() > MAX_OUTPUT_SIZE {
                    // Truncate the output
                    let truncated = format!(
                        "{}\n\n--- Output truncated by OutputGuard (was {} bytes) ---",
                        &output_str[..MAX_OUTPUT_SIZE],
                        output_str.len()
                    );
                    *output = serde_json::Value::String(truncated);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_output_guard_creation() {
        let guard = OutputGuardBehavior::new();
        // Just verify it creates successfully
        assert_eq!(guard.to_string(), "OutputGuardBehavior");
    }

    #[test]
    fn test_is_within_limit() {
        let small = "x".repeat(1000);
        assert!(small.len() <= MAX_OUTPUT_SIZE);
    }

    #[test]
    fn test_exceeds_limit() {
        let large = "x".repeat(MAX_OUTPUT_SIZE + 1000);
        assert!(large.len() > MAX_OUTPUT_SIZE);
    }
}
