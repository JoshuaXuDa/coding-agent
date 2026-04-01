//! Tool provider trait - Domain layer
//!
//! Each tool module implements this trait to describe itself,
//! replacing the manual HashMap insertion in collect_tools! macro.

use std::sync::Arc;
use tirea::prelude::Tool;
use crate::platform::domain::{FileSystem, CommandExecutor};

/// Dependency type required by a tool
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DependencyType {
    /// Requires a FileSystem implementation
    FileSystem,
    /// Requires a CommandExecutor implementation
    CommandExecutor,
    /// No external dependencies
    None,
}

/// Trait for declarative tool registration.
///
/// Each tool module provides a unit struct implementing this trait.
/// The ToolRegistry collects all providers and builds the tool map.
pub trait ToolProvider: Send + Sync {
    /// Unique tool identifier (e.g., "read", "bash")
    fn tool_id(&self) -> &str;

    /// What dependency the tool needs
    fn dependency_type(&self) -> DependencyType;

    /// Build the tool instance with the given dependencies.
    /// `fs` and `executor` are provided based on dependency_type.
    fn build(
        &self,
        fs: Option<Arc<dyn FileSystem>>,
        executor: Option<Arc<dyn CommandExecutor>>,
    ) -> Arc<dyn Tool>;
}
