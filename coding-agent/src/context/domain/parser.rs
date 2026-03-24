//! @ symbol parser

use crate::context::domain::reference::{FileReference, ReferenceType};
use regex::Regex;
use std::path::PathBuf;

/// Parser for @ symbol file references
pub struct AtSymbolParser;

impl AtSymbolParser {
    /// Extract all @ references from a user message
    pub fn extract_references(message: &str) -> Vec<FileReference> {
        let mut references = Vec::new();

        // Regex to match @ followed by non-whitespace characters
        let re = Regex::new(r"@([^\s@]+)").unwrap();

        for cap in re.captures_iter(message) {
            let raw = cap[0].to_string();
            let target = &cap[1];

            let ref_type = Self::detect_reference_type(target);

            references.push(FileReference {
                ref_type,
                raw_reference: raw,
                path: Self::try_parse_path(target),
                pattern: Some(target.to_string()),
            });
        }

        references
    }

    /// Detect the type of reference based on the target string
    fn detect_reference_type(target: &str) -> ReferenceType {
        if target.contains(':') {
            ReferenceType::Symbol
        } else if target.contains('*') || target.contains('?') {
            ReferenceType::GlobPattern
        } else if target.contains('/') || target.contains('\\') {
            ReferenceType::DirectPath
        } else {
            ReferenceType::FuzzyMatch
        }
    }

    /// Try to parse the target as a file path
    fn try_parse_path(target: &str) -> Option<PathBuf> {
        if target.contains('/') || target.contains('\\') {
            Some(PathBuf::from(target))
        } else {
            None
        }
    }

    /// Check if a message contains any @ references
    pub fn has_references(message: &str) -> bool {
        message.contains('@')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_direct_path() {
        let message = "查看 @src/main.rs 的实现";
        let refs = AtSymbolParser::extract_references(message);

        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::DirectPath);
        assert_eq!(refs[0].raw_reference, "@src/main.rs");
        assert_eq!(refs[0].path, Some(PathBuf::from("src/main.rs")));
    }

    #[test]
    fn test_parse_fuzzy_match() {
        let message = "帮我查看 @main.rs";
        let refs = AtSymbolParser::extract_references(message);

        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::FuzzyMatch);
        assert_eq!(refs[0].raw_reference, "@main.rs");
        assert_eq!(refs[0].path, None);
    }

    #[test]
    fn test_parse_glob_pattern() {
        let message = "查找所有 @**/*.rs 文件";
        let refs = AtSymbolParser::extract_references(message);

        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::GlobPattern);
        assert_eq!(refs[0].raw_reference, "@**/*.rs");
    }

    #[test]
    fn test_parse_multiple_references() {
        let message = "比较 @file1.rs 和 @file2.rs 的差异";
        let refs = AtSymbolParser::extract_references(message);

        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0].raw_reference, "@file1.rs");
        assert_eq!(refs[1].raw_reference, "@file2.rs");
    }

    #[test]
    fn test_has_references() {
        assert!(AtSymbolParser::has_references("查看 @main.rs"));
        assert!(!AtSymbolParser::has_references("普通消息没有引用"));
    }

    #[test]
    fn test_parse_symbol_reference() {
        let message = "查找 @fn:main 函数";
        let refs = AtSymbolParser::extract_references(message);

        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].ref_type, ReferenceType::Symbol);
        assert_eq!(refs[0].raw_reference, "@fn:main");
    }
}
