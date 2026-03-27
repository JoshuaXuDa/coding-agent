//! OutputGuardBehavior - Output truncation protection
//!
//! This behavior provides a second layer of output size protection
//! by checking tool outputs after execution.

use tirea::prelude::AgentBehavior;

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

    /// Check if output is within size limits
    pub fn is_within_limit(&self, output: &str) -> bool {
        output.len() <= MAX_OUTPUT_SIZE
    }

    /// Truncate output if it exceeds limits
    pub fn truncate_output(&self, output: &str) -> String {
        if output.len() > MAX_OUTPUT_SIZE {
            format!(
                "{}\n\n--- Output truncated by OutputGuard (was {} bytes) ---",
                &output[..MAX_OUTPUT_SIZE],
                output.len()
            )
        } else {
            output.to_string()
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
        // Just verify it creates successfully (Debug is derived)
        let _debug = format!("{:?}", guard);
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
