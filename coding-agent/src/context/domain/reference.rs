//! File reference domain models

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// @ symbol reference type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceType {
    /// Direct file path: @src/main.rs
    DirectPath,
    /// Fuzzy file name match: @main.rs
    FuzzyMatch,
    /// Glob pattern: @**/*.rs
    GlobPattern,
    /// Symbol reference (future): @fn:process_message
    Symbol,
}

/// Parsed file reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReference {
    /// Reference type
    pub ref_type: ReferenceType,
    /// Original reference string (including @)
    pub raw_reference: String,
    /// Parsed path (for direct references)
    pub path: Option<PathBuf>,
    /// Search pattern (for fuzzy/glob matches)
    pub pattern: Option<String>,
}

/// Injected file content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectedContent {
    /// File path
    pub path: PathBuf,
    /// File content
    pub content: String,
    /// Language type (for syntax highlighting)
    pub language: String,
    /// Metadata
    pub metadata: FileMetadata,
}

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Total number of lines
    pub total_lines: usize,
    /// Character encoding
    pub encoding: String,
    /// Size in bytes
    pub size_bytes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reference_type_serialization() {
        let ref_type = ReferenceType::DirectPath;
        let serialized = serde_json::to_string(&ref_type).unwrap();
        assert_eq!(serialized, "\"DirectPath\"");

        let deserialized: ReferenceType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized, ReferenceType::DirectPath);
    }

    #[test]
    fn test_file_reference_creation() {
        let reference = FileReference {
            ref_type: ReferenceType::DirectPath,
            raw_reference: "@src/main.rs".to_string(),
            path: Some(PathBuf::from("src/main.rs")),
            pattern: None,
        };

        assert_eq!(reference.raw_reference, "@src/main.rs");
        assert_eq!(reference.path, Some(PathBuf::from("src/main.rs")));
    }
}
