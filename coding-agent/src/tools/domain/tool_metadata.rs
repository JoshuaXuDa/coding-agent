//! Enhanced tool metadata for progressive disclosure and auto-documentation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Extended metadata for tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Tool priority for progressive disclosure (1-10, lower = always show)
    pub priority: u8,

    /// When to show this tool
    pub disclosure: DisclosurePolicy,

    /// Usage examples for auto-documentation
    pub examples: Vec<ToolExample>,

    /// Related tools (for contextual suggestions)
    pub related_tools: Vec<String>,

    /// Tool tags for categorization
    pub tags: Vec<String>,

    /// Custom prompt hints
    pub prompt_hints: Option<String>,
}

/// When to disclose a tool to the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisclosurePolicy {
    /// Always show (default for basic tools)
    Always,

    /// Show only in specific contexts
    Conditional { context: Vec<String> },

    /// Show after certain tools are used
    Sequential { after: Vec<String> },

    /// Show only when explicitly requested
    Explicit,
}

/// Tool usage example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub outcome: String,
}

impl Default for ToolMetadata {
    fn default() -> Self {
        Self {
            priority: 5,
            disclosure: DisclosurePolicy::Always,
            examples: Vec::new(),
            related_tools: Vec::new(),
            tags: Vec::new(),
            prompt_hints: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_default() {
        let metadata = ToolMetadata::default();
        assert_eq!(metadata.priority, 5);
        assert!(matches!(metadata.disclosure, DisclosurePolicy::Always));
        assert!(metadata.examples.is_empty());
        assert!(metadata.related_tools.is_empty());
        assert!(metadata.tags.is_empty());
        assert!(metadata.prompt_hints.is_none());
    }

    #[test]
    fn test_disclosure_policy() {
        let always = DisclosurePolicy::Always;
        let conditional = DisclosurePolicy::Conditional {
            context: vec!["file_operations".to_string()],
        };
        let sequential = DisclosurePolicy::Sequential {
            after: vec!["read".to_string()],
        };
        let explicit = DisclosurePolicy::Explicit;

        // Verify all variants can be created
        assert!(matches!(always, DisclosurePolicy::Always));
        assert!(matches!(conditional, DisclosurePolicy::Conditional { .. }));
        assert!(matches!(sequential, DisclosurePolicy::Sequential { .. }));
        assert!(matches!(explicit, DisclosurePolicy::Explicit));
    }

    #[test]
    fn test_tool_example() {
        let mut params = HashMap::new();
        params.insert("path".to_string(), serde_json::json!("src/main.rs"));

        let example = ToolExample {
            description: "Read a file".to_string(),
            parameters: params,
            outcome: "Returns file contents".to_string(),
        };

        assert_eq!(example.description, "Read a file");
        assert_eq!(example.parameters.len(), 1);
        assert_eq!(example.outcome, "Returns file contents");
    }
}
