//! Context compaction for long conversations
//!
//! When the conversation history approaches the token limit,
//! older messages are summarized to free up context space.

use super::token_estimation::estimate_tokens;

/// Default maximum context tokens before compaction is needed.
pub const DEFAULT_MAX_CONTEXT_TOKENS: usize = 100_000;

/// Compaction threshold as a fraction of max tokens.
/// Compact when estimated usage exceeds this ratio.
pub const COMPACTION_THRESHOLD: f64 = 0.7;

/// Compaction configuration
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Maximum context tokens
    pub max_tokens: usize,
    /// Threshold ratio (0.0-1.0) at which to trigger compaction
    pub threshold: f64,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            max_tokens: DEFAULT_MAX_CONTEXT_TOKENS,
            threshold: COMPACTION_THRESHOLD,
        }
    }
}

/// Result of a compaction operation.
#[derive(Debug, Clone)]
pub struct CompactionResult {
    /// The summary text that replaces old messages
    pub summary: String,
    /// The index in the message array at which messages were replaced
    pub boundary_index: usize,
    /// How many messages were compacted
    pub compacted_count: usize,
    /// Estimated tokens before compaction
    pub tokens_before: usize,
    /// Estimated tokens after compaction
    pub tokens_after: usize,
}

impl CompactionConfig {
    /// Check if compaction is needed based on the estimated token count.
    pub fn should_compact(&self, estimated_tokens: usize) -> bool {
        let threshold_tokens = (self.max_tokens as f64 * self.threshold) as usize;
        estimated_tokens >= threshold_tokens
    }
}

/// Estimate the total tokens in a slice of message strings.
pub fn estimate_conversation_tokens(messages: &[String]) -> usize {
    messages.iter().map(|m| estimate_tokens(m)).sum()
}

/// Build a summary prompt for compaction.
///
/// This prompt instructs the LLM to summarize the conversation history,
/// preserving key facts, decisions, and file paths.
pub fn build_compaction_prompt(messages: &[String]) -> String {
    let mut prompt = String::from(
        "Please provide a concise summary of the following conversation history. \
        Preserve:\n\
        - Key decisions and conclusions\n\
        - File paths that were read, written, or modified\n\
        - Important error messages and their resolutions\n\
        - Any TODOs or pending tasks\n\n\
        Conversation history:\n\n",
    );

    for (i, msg) in messages.iter().enumerate() {
        // Truncate very long messages in the compaction prompt
        let truncated = if msg.len() > 2000 {
            format!("{}...\n[truncated, {} chars total]", &msg[..2000], msg.len())
        } else {
            msg.clone()
        };
        prompt.push_str(&format!("[{}]: {}\n\n", i, truncated));
    }

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_compact() {
        let config = CompactionConfig {
            max_tokens: 1000,
            threshold: 0.7,
        };

        assert!(!config.should_compact(500));
        assert!(config.should_compact(700));
        assert!(config.should_compact(1000));
    }

    #[test]
    fn test_estimate_conversation_tokens() {
        let messages = vec![
            "Hello world".to_string(),
            "This is a test".to_string(),
        ];
        let tokens = estimate_conversation_tokens(&messages);
        assert!(tokens > 0);
        assert!(tokens < 50);
    }

    #[test]
    fn test_build_compaction_prompt() {
        let messages = vec![
            "User: Read file foo.rs".to_string(),
            "Agent: Here is the content...".to_string(),
        ];
        let prompt = build_compaction_prompt(&messages);
        assert!(prompt.contains("summary"));
        assert!(prompt.contains("User: Read file foo.rs"));
    }

    #[test]
    fn test_build_compaction_prompt_truncates_long_messages() {
        let messages = vec!["a".repeat(5000)];
        let prompt = build_compaction_prompt(&messages);
        assert!(prompt.contains("truncated"));
    }

    #[test]
    fn test_default_config() {
        let config = CompactionConfig::default();
        assert_eq!(config.max_tokens, 100_000);
        assert!((config.threshold - 0.7).abs() < 0.001);
    }
}
