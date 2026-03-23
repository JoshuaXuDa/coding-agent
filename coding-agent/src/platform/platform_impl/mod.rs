//! Platform-specific implementations
//!
//! This module provides concrete implementations of domain services
//! for different operating systems.

pub mod factory;

#[cfg(unix)]
pub mod unix;

#[cfg(windows)]
pub mod windows;

pub use factory::*;
