//! Platform bounded context
//!
//! This module provides cross-platform abstractions for file system operations,
//! command execution, and path handling following DDD principles.
//!
//! ## Architecture
//!
//! - **Domain Layer**: Core traits (FileSystem, CommandExecutor, PlatformPath)
//! - **Implementation Layer**: Platform-specific implementations (Unix, Windows)
//! - **Factory Module**: Creates appropriate implementations based on platform
//!
//! ## Usage
//!
//! ```rust
//! use coding_agent::platform::create_filesystem;
//!
//! let fs = create_filesystem();
//! let contents = fs.read_file(Path::new("README.md"))?;
//! ```

pub mod domain;
pub mod platform_impl;

pub use domain::*;
pub use platform_impl::*;
