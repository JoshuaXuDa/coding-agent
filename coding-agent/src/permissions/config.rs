//! Permission configuration
//!
//! Parses permission rules from the agent configuration file.

use serde::{Deserialize, Serialize};
use super::engine::{PermissionMode, ToolPermissionRule, PermissionEngine, RuleDecision};

/// Permission configuration section in agent.json.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionConfig {
    /// Default permission mode
    #[serde(default)]
    pub mode: Option<PermissionMode>,
    /// Permission rules
    #[serde(default)]
    pub rules: Vec<ToolPermissionRule>,
}

impl PermissionConfig {
    /// Build a PermissionEngine from this configuration.
    pub fn build_engine(&self) -> PermissionEngine {
        let mode = self.mode.unwrap_or_default();
        PermissionEngine::new(mode, self.rules.clone())
    }

    /// Create a default permission config with sensible defaults for a coding assistant.
    pub fn coding_defaults() -> Self {
        Self {
            mode: Some(PermissionMode::Ask),
            rules: vec![
                // Read-only tools are safe
                ToolPermissionRule {
                    tool: "read".to_string(),
                    pattern: None,
                    decision: RuleDecision::Allow,
                },
                ToolPermissionRule {
                    tool: "list".to_string(),
                    pattern: None,
                    decision: RuleDecision::Allow,
                },
                ToolPermissionRule {
                    tool: "stat".to_string(),
                    pattern: None,
                    decision: RuleDecision::Allow,
                },
                ToolPermissionRule {
                    tool: "glob".to_string(),
                    pattern: None,
                    decision: RuleDecision::Allow,
                },
                ToolPermissionRule {
                    tool: "grep".to_string(),
                    pattern: None,
                    decision: RuleDecision::Allow,
                },
                ToolPermissionRule {
                    tool: "head_tail".to_string(),
                    pattern: None,
                    decision: RuleDecision::Allow,
                },
                // Common safe bash commands
                ToolPermissionRule {
                    tool: "bash".to_string(),
                    pattern: Some("cargo *".to_string()),
                    decision: RuleDecision::Allow,
                },
                ToolPermissionRule {
                    tool: "bash".to_string(),
                    pattern: Some("git status".to_string()),
                    decision: RuleDecision::Allow,
                },
                ToolPermissionRule {
                    tool: "bash".to_string(),
                    pattern: Some("git diff".to_string()),
                    decision: RuleDecision::Allow,
                },
                ToolPermissionRule {
                    tool: "bash".to_string(),
                    pattern: Some("git log".to_string()),
                    decision: RuleDecision::Allow,
                },
                ToolPermissionRule {
                    tool: "bash".to_string(),
                    pattern: Some("ls".to_string()),
                    decision: RuleDecision::Allow,
                },
                ToolPermissionRule {
                    tool: "bash".to_string(),
                    pattern: Some("pwd".to_string()),
                    decision: RuleDecision::Allow,
                },
                ToolPermissionRule {
                    tool: "bash".to_string(),
                    pattern: Some("echo *".to_string()),
                    decision: RuleDecision::Allow,
                },
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PermissionConfig::default();
        assert!(config.mode.is_none());
        assert!(config.rules.is_empty());
    }

    #[test]
    fn test_coding_defaults() {
        let config = PermissionConfig::coding_defaults();
        assert_eq!(config.mode, Some(PermissionMode::Ask));
        assert!(!config.rules.is_empty());

        let engine = config.build_engine();
        // Read tools should be allowed
        assert!(matches!(engine.check("read", &serde_json::json!({})), super::super::PermissionDecision::Allow));
        assert!(matches!(engine.check("glob", &serde_json::json!({})), super::super::PermissionDecision::Allow));
        // Write tools should ask
        assert!(matches!(engine.check("write", &serde_json::json!({})), super::super::PermissionDecision::Ask));
        // Safe cargo commands should be allowed
        assert!(matches!(engine.check("bash", &serde_json::json!({"command": "cargo test"})), super::super::PermissionDecision::Allow));
    }

    #[test]
    fn test_parse_from_json() {
        let json = serde_json::json!({
            "mode": "auto",
            "rules": [
                {"tool": "read", "decision": "allow"},
                {"tool": "bash", "pattern": "cargo *", "decision": "allow"}
            ]
        });

        let config: PermissionConfig = serde_json::from_value(json).unwrap();
        assert_eq!(config.mode, Some(PermissionMode::Auto));
        assert_eq!(config.rules.len(), 2);
    }
}
