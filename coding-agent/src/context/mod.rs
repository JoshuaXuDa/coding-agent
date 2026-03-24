//! Context injection module for @ symbol file references
//!
//! This module provides functionality to parse @ symbols in user messages,
//! search for matching files, and inject their content into the conversation context.

pub mod domain;
pub mod application;

pub use application::context_builder::ContextBuilder;
