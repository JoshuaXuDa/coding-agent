//! Domain layer for @ symbol file reference parsing

pub mod reference;
pub mod parser;
pub mod injector;

pub use reference::{FileReference, InjectedContent, ReferenceType, FileMetadata};
pub use parser::AtSymbolParser;
pub use injector::ContextInjector;
