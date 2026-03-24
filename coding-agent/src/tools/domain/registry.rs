//! Tool registry - Domain layer
//!
//! Central registry for tool factory functions and metadata.

use std::sync::Arc;
use tirea::prelude::Tool;
use crate::platform::domain::{FileSystem, CommandExecutor};
use super::tool_metadata::ToolMetadata;

/// Tool factory function type
pub type ToolFactory = fn(
    fs: Option<Arc<dyn FileSystem>>,
    executor: Option<Arc<dyn CommandExecutor>>,
) -> Arc<dyn Tool>;

/// Tool registration entry with metadata
pub struct ToolRegistration {
    pub id: &'static str,
    pub factory: ToolFactory,
    pub dependency_type: DependencyType,
    pub metadata: ToolMetadata,
}

/// Dependency type for the tool
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DependencyType {
    FileSystem,
    CommandExecutor,
    Custom,
}

impl ToolRegistration {
    pub const fn new(
        id: &'static str,
        factory: ToolFactory,
        dependency_type: DependencyType,
        metadata: ToolMetadata,
    ) -> Self {
        Self { id, factory, dependency_type, metadata }
    }
}

/// Collect all tool registrations
///
/// This function is called by the build_tool_map function to get all registered tools.
/// In the simplified implementation, tools are registered via the collect_tools! macro.
pub fn collect_registrations() -> &'static [ToolRegistration] {
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_type_equality() {
        assert_eq!(DependencyType::FileSystem, DependencyType::FileSystem);
        assert_eq!(DependencyType::CommandExecutor, DependencyType::CommandExecutor);
        assert_eq!(DependencyType::Custom, DependencyType::Custom);
        assert_ne!(DependencyType::FileSystem, DependencyType::CommandExecutor);
    }

    #[test]
    fn test_collect_registrations_empty() {
        let registrations = collect_registrations();
        assert_eq!(registrations.len(), 0);
    }

    #[test]
    fn test_tool_registration_const() {
        // Verify that ToolRegistration::new is a const function
        const _REG: ToolRegistration = ToolRegistration::new(
            "test",
            |_fs, _exec| Arc::new(TestTool) as Arc<dyn Tool>,
            DependencyType::Custom,
            ToolMetadata::default(),
        );

        // If we got here, the const fn works
        struct TestTool;
        impl Tool for TestTool {
            fn descriptor(&self) -> tirea::prelude::ToolDescriptor {
                tirea::prelude::ToolDescriptor {
                    id: "test".to_string(),
                    name: "test".to_string(),
                    description: "Test tool".to_string(),
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
                Box::pin(async {
                    Ok(tirea::prelude::ToolResult::success("test", "test".to_string()))
                })
            }
        }
    }
}
