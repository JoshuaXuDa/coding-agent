//! Permission engine
//!
//! Checks tool calls against configured rules and determines
//! whether to allow, deny, or ask the user.

use serde::{Deserialize, Serialize};

/// Permission mode that controls default behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionMode {
    /// All tool calls are allowed without confirmation
    Auto,
    /// Read-only tools are allowed, write tools require confirmation
    Plan,
    /// All tool calls require user confirmation (except explicitly allowed)
    #[default]
    Ask,
}

/// Decision returned by the permission engine.
#[derive(Debug, Clone)]
pub enum PermissionDecision {
    /// Tool call is allowed
    Allow,
    /// Tool call is denied
    Deny { reason: String },
    /// Tool call requires user confirmation
    Ask,
}

/// A single permission rule that matches tool calls by name and optional pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermissionRule {
    /// Tool name to match ("*" matches all tools)
    pub tool: String,
    /// Optional glob pattern for tool arguments (e.g., "/tmp/*" for file paths)
    #[serde(default)]
    pub pattern: Option<String>,
    /// The decision to apply when this rule matches
    pub decision: RuleDecision,
}

/// Decision type for a permission rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RuleDecision {
    Allow,
    Deny,
    Ask,
}

/// Read-only tools that are safe to auto-allow in Plan mode.
const READ_ONLY_TOOLS: &[&str] = &[
    "read", "list", "stat", "glob", "grep", "head_tail", "webfetch", "websearch", "codesearch",
];

/// The permission engine checks tool calls against rules and mode.
pub struct PermissionEngine {
    mode: PermissionMode,
    rules: Vec<ToolPermissionRule>,
}

impl PermissionEngine {
    /// Create a new permission engine with the given mode and rules.
    pub fn new(mode: PermissionMode, rules: Vec<ToolPermissionRule>) -> Self {
        Self { mode, rules }
    }

    /// Check if a tool call is permitted.
    ///
    /// Returns the permission decision based on the configured rules and mode.
    pub fn check(&self, tool_name: &str, args: &serde_json::Value) -> PermissionDecision {
        // Check explicit rules first (highest priority)
        for rule in &self.rules {
            if self.rule_matches(rule, tool_name, args) {
                return match rule.decision {
                    RuleDecision::Allow => PermissionDecision::Allow,
                    RuleDecision::Deny => PermissionDecision::Deny {
                        reason: format!("Tool '{}' denied by permission rule", tool_name),
                    },
                    RuleDecision::Ask => PermissionDecision::Ask,
                };
            }
        }

        // Fall back to mode-based defaults
        match self.mode {
            PermissionMode::Auto => PermissionDecision::Allow,
            PermissionMode::Plan => {
                if READ_ONLY_TOOLS.contains(&tool_name) {
                    PermissionDecision::Allow
                } else {
                    PermissionDecision::Ask
                }
            }
            PermissionMode::Ask => PermissionDecision::Ask,
        }
    }

    /// Check if a rule matches a tool call.
    fn rule_matches(&self, rule: &ToolPermissionRule, tool_name: &str, _args: &serde_json::Value) -> bool {
        // Check tool name match
        if rule.tool != "*" && rule.tool != tool_name {
            return false;
        }

        // If no pattern, the rule matches by tool name alone
        if let Some(pattern) = &rule.pattern {
            // For now, patterns are checked against a simplified argument representation
            // Full pattern matching can be enhanced later
            if pattern != "*" {
                // Extract relevant argument values for pattern matching
                let arg_str = self.extract_arg_string(tool_name, _args);
                if !self.simple_glob_match(pattern, &arg_str) {
                    return false;
                }
            }
        }

        true
    }

    /// Extract a simplified string from tool arguments for pattern matching.
    fn extract_arg_string(&self, tool_name: &str, args: &serde_json::Value) -> String {
        match tool_name {
            "read" | "write" | "edit" | "stat" | "glob" | "head_tail" => {
                args.get("path")
                    .or_else(|| args.get("file_path"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            }
            "grep" => {
                args.get("path")
                    .or_else(|| args.get("pattern"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            }
            "bash" => {
                args.get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            }
            "list" => {
                args.get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            }
            _ => args.to_string(),
        }
    }

    /// Simple glob match: supports * as wildcard.
    fn simple_glob_match(&self, pattern: &str, text: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // Simple prefix/suffix matching
        if let Some(prefix) = pattern.strip_suffix('*') {
            return text.starts_with(prefix);
        }
        if let Some(suffix) = pattern.strip_prefix('*') {
            return text.ends_with(suffix);
        }

        pattern == text
    }

    /// Get the current permission mode.
    pub fn mode(&self) -> PermissionMode {
        self.mode
    }
}

impl Default for PermissionEngine {
    fn default() -> Self {
        Self {
            mode: PermissionMode::Ask,
            rules: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_mode_allows_everything() {
        let engine = PermissionEngine::new(PermissionMode::Auto, vec![]);
        assert!(matches!(engine.check("bash", &serde_json::json!({})), PermissionDecision::Allow));
        assert!(matches!(engine.check("read", &serde_json::json!({})), PermissionDecision::Allow));
    }

    #[test]
    fn test_ask_mode_prompts_all() {
        let engine = PermissionEngine::new(PermissionMode::Ask, vec![]);
        assert!(matches!(engine.check("bash", &serde_json::json!({})), PermissionDecision::Ask));
        assert!(matches!(engine.check("read", &serde_json::json!({})), PermissionDecision::Ask));
    }

    #[test]
    fn test_plan_mode_allows_read_only() {
        let engine = PermissionEngine::new(PermissionMode::Plan, vec![]);
        assert!(matches!(engine.check("read", &serde_json::json!({})), PermissionDecision::Allow));
        assert!(matches!(engine.check("grep", &serde_json::json!({})), PermissionDecision::Allow));
        assert!(matches!(engine.check("bash", &serde_json::json!({})), PermissionDecision::Ask));
        assert!(matches!(engine.check("write", &serde_json::json!({})), PermissionDecision::Ask));
    }

    #[test]
    fn test_explicit_allow_rule() {
        let engine = PermissionEngine::new(
            PermissionMode::Ask,
            vec![ToolPermissionRule {
                tool: "read".to_string(),
                pattern: None,
                decision: RuleDecision::Allow,
            }],
        );
        assert!(matches!(engine.check("read", &serde_json::json!({})), PermissionDecision::Allow));
        assert!(matches!(engine.check("bash", &serde_json::json!({})), PermissionDecision::Ask));
    }

    #[test]
    fn test_explicit_deny_rule() {
        let engine = PermissionEngine::new(
            PermissionMode::Auto,
            vec![ToolPermissionRule {
                tool: "bash".to_string(),
                pattern: Some("rm *".to_string()),
                decision: RuleDecision::Deny,
            }],
        );
        // Should deny rm commands
        if let PermissionDecision::Deny { reason } = engine.check("bash", &serde_json::json!({"command": "rm -rf /"})) {
            assert!(reason.contains("denied"));
        } else {
            panic!("Expected Deny decision");
        }
        // Other commands should be allowed
        assert!(matches!(engine.check("bash", &serde_json::json!({"command": "ls"})), PermissionDecision::Allow));
    }

    #[test]
    fn test_wildcard_rule() {
        let engine = PermissionEngine::new(
            PermissionMode::Ask,
            vec![ToolPermissionRule {
                tool: "*".to_string(),
                pattern: None,
                decision: RuleDecision::Allow,
            }],
        );
        assert!(matches!(engine.check("bash", &serde_json::json!({})), PermissionDecision::Allow));
        assert!(matches!(engine.check("write", &serde_json::json!({})), PermissionDecision::Allow));
    }

    #[test]
    fn test_cargo_allow_rule() {
        let engine = PermissionEngine::new(
            PermissionMode::Ask,
            vec![
                ToolPermissionRule {
                    tool: "bash".to_string(),
                    pattern: Some("cargo *".to_string()),
                    decision: RuleDecision::Allow,
                },
            ],
        );
        // cargo commands should be allowed
        assert!(matches!(
            engine.check("bash", &serde_json::json!({"command": "cargo build"})),
            PermissionDecision::Allow
        ));
        // Other commands should ask
        assert!(matches!(
            engine.check("bash", &serde_json::json!({"command": "rm -rf /"})),
            PermissionDecision::Ask
        ));
    }

    #[test]
    fn test_default_engine() {
        let engine = PermissionEngine::default();
        assert_eq!(engine.mode(), PermissionMode::Ask);
    }
}
