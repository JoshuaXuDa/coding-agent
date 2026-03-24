//! Auto-generate tool documentation from metadata

use crate::tools::domain::registry::ToolRegistration;
use super::tool_metadata::DisclosurePolicy;

/// Generate tool usage documentation for system prompt
pub fn generate_tool_docs(registrations: &[ToolRegistration]) -> String {
    let mut docs = String::from("## Tool Usage Guidelines\n\n");

    // Group by disclosure policy
    let mut always_tools: Vec<&ToolRegistration> = Vec::new();
    let mut conditional_tools: Vec<&ToolRegistration> = Vec::new();

    for reg in registrations {
        match &reg.metadata.disclosure {
            DisclosurePolicy::Always => always_tools.push(reg),
            _ => conditional_tools.push(reg),
        }
    }

    // Always-available tools
    if !always_tools.is_empty() {
        docs.push_str("### Always Available\n\n");
        for reg in &always_tools {
            docs.push_str(&format!("#### {} Tool\n", reg.id.to_uppercase()));

            if let Some(hints) = &reg.metadata.prompt_hints {
                docs.push_str(&format!("- **Use for**: {}\n", hints));
            }

            if !reg.metadata.tags.is_empty() {
                docs.push_str(&format!("- **Tags**: {}\n", reg.metadata.tags.join(", ")));
            }

            if !reg.metadata.related_tools.is_empty() {
                docs.push_str(&format!("- **Related tools**: {}\n", reg.metadata.related_tools.join(", ")));
            }

            if !reg.metadata.examples.is_empty() {
                docs.push_str("- **Examples**:\n");
                for example in &reg.metadata.examples {
                    docs.push_str(&format!("  - {}: {}\n", example.description, example.outcome));
                }
            }

            docs.push('\n');
        }
    }

    // Conditionally available tools
    if !conditional_tools.is_empty() {
        docs.push_str("### Contextually Available\n\n");
        docs.push_str("These tools may be available depending on context:\n\n");
        for reg in &conditional_tools {
            docs.push_str(&format!("#### {} Tool\n", reg.id.to_uppercase()));
            docs.push_str(&format!("- **Priority**: {}\n", reg.metadata.priority));
            docs.push_str(&format!("- **Disclosure**: {:?}\n", reg.metadata.disclosure));

            if let Some(hints) = &reg.metadata.prompt_hints {
                docs.push_str(&format!("- **Use for**: {}\n", hints));
            }

            docs.push('\n');
        }
    }

    docs
}

/// Generate progressive tool set based on context
pub fn filter_tools_by_context<'a>(
    registrations: &'a [ToolRegistration],
    used_tools: &[String],
    max_priority: u8,
) -> Vec<&'a ToolRegistration> {
    registrations
        .iter()
        .filter(|reg| {
            match &reg.metadata.disclosure {
                DisclosurePolicy::Always => true,
                DisclosurePolicy::Sequential { after } => {
                    used_tools.iter().any(|t| after.contains(t))
                }
                _ => reg.metadata.priority <= max_priority,
            }
        })
        .collect()
}

/// Generate a summary of all tools for quick reference
pub fn generate_tool_summary(registrations: &[ToolRegistration]) -> String {
    let mut summary = String::from("## Available Tools\n\n");

    // Sort by priority
    let mut sorted: Vec<_> = registrations.iter().collect();
    sorted.sort_by_key(|r| r.metadata.priority);

    for reg in sorted {
        summary.push_str(&format!("**{}** (priority: {}): ", reg.id, reg.metadata.priority));

        if let Some(hints) = &reg.metadata.prompt_hints {
            summary.push_str(hints);
        } else {
            summary.push_str("Various operations");
        }

        summary.push('\n');
    }

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::domain::tool_metadata::{ToolMetadata, ToolExample, DisclosurePolicy};
    use std::collections::HashMap;

    fn create_test_registration(id: &'static str, priority: u8, disclosure: DisclosurePolicy) -> ToolRegistration {
        let mut params = HashMap::new();
        params.insert("test".to_string(), serde_json::json!("value"));

        ToolRegistration {
            id,
            factory: |_fs, _exec| {
                struct TestTool;
                impl tirea::prelude::Tool for TestTool {
                    fn descriptor(&self) -> tirea::prelude::ToolDescriptor {
                        tirea::prelude::ToolDescriptor {
                            id: "test".to_string(),
                            name: "test".to_string(),
                            description: "Test".to_string(),
                            category: None,
                            parameters: serde_json::json!({}),
                            metadata: Default::default(),
                        }
                    }

                    fn execute<'life0, 'life1, 'life2, 'async_trait>(
                        &'life0 self,
                        _args: serde_json::Value,
                        _context: &'life1 tirea_contract::ToolCallContext<'life2>,
                    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<tirea::prelude::ToolResult, tirea::prelude::ToolError>> + Send + 'async_trait>> {
                        Box::pin(async { Ok(tirea::prelude::ToolResult::success("test", "test".to_string())) })
                    }
                }
                Arc::new(TestTool)
            },
            dependency_type: crate::tools::domain::registry::DependencyType::Custom,
            metadata: ToolMetadata {
                priority,
                disclosure,
                examples: vec![ToolExample {
                    description: "Test example".to_string(),
                    parameters: params,
                    outcome: "Test outcome".to_string(),
                }],
                related_tools: vec!["other".to_string()],
                tags: vec!["test".to_string()],
                prompt_hints: Some("Test hints".to_string()),
            },
        }
    }

    #[test]
    fn test_generate_tool_docs() {
        let reg1 = create_test_registration("tool1", 1, DisclosurePolicy::Always);
        let reg2 = create_test_registration("tool2", 2, DisclosurePolicy::Always);
        let reg3 = create_test_registration("tool3", 5, DisclosurePolicy::Sequential {
            after: vec!["tool1".to_string()],
        });

        let docs = generate_tool_docs(&[reg1, reg2, reg3]);

        assert!(docs.contains("## Tool Usage Guidelines"));
        assert!(docs.contains("### Always Available"));
        assert!(docs.contains("#### TOOL1 Tool"));
        assert!(docs.contains("Test hints"));
        assert!(docs.contains("Test example"));
        assert!(docs.contains("### Contextually Available"));
    }

    #[test]
    fn test_filter_tools_by_context() {
        let reg1 = create_test_registration("always", 1, DisclosurePolicy::Always);
        let reg2 = create_test_registration("sequential", 3, DisclosurePolicy::Sequential {
            after: vec!["always".to_string()],
        });
        let reg3 = create_test_registration("priority", 10, DisclosurePolicy::Explicit);

        // No tools used yet
        let filtered = filter_tools_by_context(&[reg1, reg2, reg3], &[], 5);
        assert_eq!(filtered.len(), 2); // always + priority <= 5

        // After using "always" tool
        let filtered = filter_tools_by_context(&[reg1, reg2, reg3], &["always".to_string()], 5);
        assert_eq!(filtered.len(), 3); // all three
    }

    #[test]
    fn test_generate_tool_summary() {
        let reg1 = create_test_registration("low_priority", 1, DisclosurePolicy::Always);
        let reg2 = create_test_registration("high_priority", 10, DisclosurePolicy::Always);

        let summary = generate_tool_summary(&[reg2, reg1]);

        assert!(summary.contains("## Available Tools"));
        // Should be sorted by priority
        assert!(summary.find("low_priority").unwrap() < summary.find("high_priority").unwrap());
    }
}
