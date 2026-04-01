//! Token estimation for conversation history
//!
//! Provides rough token count estimation using character-based heuristics.
//! This avoids requiring a full tokenizer while being accurate enough for
//! deciding when to compact.

/// Estimate the number of tokens in a text string.
///
/// Uses a simple heuristic: ~4 characters per token for mixed CJK/Latin text.
/// This is intentionally conservative (overestimates slightly) to trigger
/// compaction before hitting actual limits.
pub fn estimate_tokens(text: &str) -> usize {
    // Count CJK characters (each CJK character is typically 1-2 tokens)
    let cjk_count = text
        .chars()
        .filter(|c| matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3040}'..='\u{30FF}' | '\u{AC00}'..='\u{D7AF}'))
        .count();

    let byte_len = text.len();

    // CJK chars ~1.5 tokens each, rest ~4 chars per token
    let cjk_tokens = (cjk_count as f64 * 1.5) as usize;
    let non_cjk_bytes = byte_len.saturating_sub(cjk_count * 3); // CJK chars are ~3 bytes UTF-8
    let non_cjk_tokens = non_cjk_bytes / 4;

    cjk_tokens + non_cjk_tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_string() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_ascii_text() {
        let text = "Hello, world! This is a test message.";
        // ~45 chars / 4 = ~11 tokens
        let tokens = estimate_tokens(text);
        assert!(tokens >= 8 && tokens <= 15);
    }

    #[test]
    fn test_cjk_text() {
        let text = "你好世界测试";
        // 6 CJK chars * 1.5 = ~9 tokens
        let tokens = estimate_tokens(text);
        assert!(tokens >= 6 && tokens <= 15);
    }

    #[test]
    fn test_mixed_text() {
        let text = "Hello 你好 World 世界";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
    }

    #[test]
    fn test_long_text() {
        let text = "a".repeat(10000);
        // ~10000 chars / 4 = ~2500 tokens
        let tokens = estimate_tokens(&text);
        assert!(tokens >= 2000 && tokens <= 3000);
    }
}
