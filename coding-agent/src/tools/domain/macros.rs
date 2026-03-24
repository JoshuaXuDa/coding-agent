//! Tool registration macros - Domain layer
//!
//! Provides macros for automatic tool registration with metadata.

/// Register a FileSystem-dependent tool with metadata
///
/// # Example
/// ```rust,ignore
/// register_tool_fs!(
///     ReadTool,
///     "read",
///     metadata = ToolMetadata {
///         priority: 1,
///         disclosure: DisclosurePolicy::Always,
///         ..Default::default()
///     }
/// );
/// ```
#[macro_export]
macro_rules! register_tool_fs {
    ($tool_type:ty, $id:expr, metadata = $metadata:expr) => {
        paste::paste! {
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            static [<$tool_type:upper _REGISTRATION>]: $crate::tools::domain::registry::ToolRegistration =
                $crate::tools::domain::registry::ToolRegistration::new(
                    $id,
                    |fs_opt, _exec_opt| {
                        let fs = fs_opt.expect("FileSystem required");
                        Arc::new(<$tool_type>::new(fs)) as Arc<dyn tirea::prelude::Tool>
                    },
                    $crate::tools::domain::registry::DependencyType::FileSystem,
                    $metadata,
                );
        }
    };

    // Simplified version without metadata (uses default)
    ($tool_type:ty, $id:expr) => {
        $crate::register_tool_fs!($tool_type, $id, metadata = $crate::tools::domain::tool_metadata::ToolMetadata::default());
    };
}

/// Register a CommandExecutor-dependent tool with metadata
///
/// # Example
/// ```rust,ignore
/// register_tool_executor!(
///     BashTool,
///     "bash",
///     metadata = ToolMetadata {
///         priority: 2,
///         disclosure: DisclosurePolicy::Always,
///         ..Default::default()
///     }
/// );
/// ```
#[macro_export]
macro_rules! register_tool_executor {
    ($tool_type:ty, $id:expr, metadata = $metadata:expr) => {
        paste::paste! {
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            static [<$tool_type:upper _REGISTRATION>]: $crate::tools::domain::registry::ToolRegistration =
                $crate::tools::domain::registry::ToolRegistration::new(
                    $id,
                    |_fs_opt, exec_opt| {
                        let executor = exec_opt.expect("CommandExecutor required");
                        Arc::new(<$tool_type>::new(executor)) as Arc<dyn tirea::prelude::Tool>
                    },
                    $crate::tools::domain::registry::DependencyType::CommandExecutor,
                    $metadata,
                );
        }
    };

    ($tool_type:ty, $id:expr) => {
        $crate::register_tool_executor!($tool_type, $id, metadata = $crate::tools::domain::tool_metadata::ToolMetadata::default());
    };
}

/// Register a tool with custom factory
///
/// # Example
/// ```rust,ignore
/// register_tool_custom!(
///     WriteTool,
///     "write",
///     |fs_opt, _exec_opt| {
///         let fs = fs_opt.expect("FileSystem required");
///         Arc::new(WriteTool::new(fs)) as Arc<dyn Tool>
///     },
///     metadata = ToolMetadata::default()
/// );
/// ```
#[macro_export]
macro_rules! register_tool_custom {
    ($tool_type:ty, $id:expr, $factory:expr, metadata = $metadata:expr) => {
        paste::paste! {
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            static [<$tool_type:upper _REGISTRATION>]: $crate::tools::domain::registry::ToolRegistration =
                $crate::tools::domain::registry::ToolRegistration::new(
                    $id,
                    $factory,
                    $crate::tools::domain::registry::DependencyType::Custom,
                    $metadata,
                );
        }
    };
}

/// Helper macro to collect tool registrations into a HashMap
///
/// This macro expands to a series of insert statements for each tool.
/// It is used by the build_tool_map function.
#[macro_export]
macro_rules! collect_tools {
    ($tools:ident, $fs:expr, $executor:expr) => {
        {
            // ListTool
            register_tool_fs!(crate::tools::application::list_tool::ListTool, "list");
            $tools.insert("list".to_string(), (ListTool::new($fs.clone())) as Arc<dyn tirea::prelude::Tool>);

            // ReadTool
            register_tool_fs!(crate::tools::application::read_tool::ReadTool, "read");
            $tools.insert("read".to_string(), (crate::tools::application::read_tool::ReadTool::new($fs.clone())) as Arc<dyn tirea::prelude::Tool>);

            // WriteTool
            register_tool_fs!(crate::tools::application::write_tool::WriteTool, "write");
            $tools.insert("write".to_string(), (crate::tools::application::write_tool::WriteTool::new($fs.clone())) as Arc<dyn tirea::prelude::Tool>);

            // StatTool
            register_tool_fs!(crate::tools::application::stat_tool::StatTool, "stat");
            $tools.insert("stat".to_string(), (crate::tools::application::stat_tool::StatTool::new($fs.clone())) as Arc<dyn tirea::prelude::Tool>);

            // GlobTool
            register_tool_fs!(crate::tools::application::glob_tool::GlobTool, "glob");
            $tools.insert("glob".to_string(), (crate::tools::application::glob_tool::GlobTool::new($fs.clone())) as Arc<dyn tirea::prelude::Tool>);

            // GrepTool
            register_tool_fs!(crate::tools::application::grep_tool::GrepTool, "grep");
            $tools.insert("grep".to_string(), (crate::tools::application::grep_tool::GrepTool::new($fs.clone())) as Arc<dyn tirea::prelude::Tool>);

            // BashTool
            register_tool_executor!(crate::tools::application::bash_tool::BashTool, "bash");
            $tools.insert("bash".to_string(), (crate::tools::application::bash_tool::BashTool::new($executor)) as Arc<dyn tirea::prelude::Tool>);

            // EditTool
            register_tool_fs!(crate::tools::application::edit_tool::EditTool, "edit");
            $tools.insert("edit".to_string(), (crate::tools::application::edit_tool::EditTool::new($fs.clone())) as Arc<dyn tirea::prelude::Tool>);

            // HeadTailTool
            register_tool_fs!(crate::tools::application::head_tail_tool::HeadTailTool, "head_tail");
            $tools.insert("head_tail".to_string(), (crate::tools::application::head_tail_tool::HeadTailTool::new($fs)) as Arc<dyn tirea::prelude::Tool>);
        }
    };
}
