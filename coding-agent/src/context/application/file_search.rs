//! File search coordinator

use crate::context::domain::reference::{FileReference, ReferenceType};
use crate::platform::domain::filesystem::FileSystem;
use anyhow::Result;
use glob::glob;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Coordinates file search operations
pub struct FileSearchCoordinator {
    fs: Arc<dyn FileSystem>,
}

impl FileSearchCoordinator {
    /// Create a new file search coordinator
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Resolve a file reference and return candidate files
    pub async fn resolve_reference(
        &self,
        reference: &FileReference,
    ) -> Result<Vec<PathBuf>> {
        match &reference.ref_type {
            ReferenceType::DirectPath => self.resolve_direct_path(reference),
            ReferenceType::FuzzyMatch => self.resolve_fuzzy_match(reference),
            ReferenceType::GlobPattern => self.resolve_glob_pattern(reference),
            ReferenceType::Symbol => {
                // Future: Use LSP to resolve symbols
                Ok(vec![])
            }
        }
    }

    /// Resolve direct path reference
    fn resolve_direct_path(&self, reference: &FileReference) -> Result<Vec<PathBuf>> {
        if let Some(path) = &reference.path {
            // Check if path exists
            if self.fs.exists(path) {
                return Ok(vec![path.clone()]);
            }
        }
        Ok(vec![])
    }

    /// Resolve fuzzy file match
    fn resolve_fuzzy_match(&self, reference: &FileReference) -> Result<Vec<PathBuf>> {
        let pattern = reference.pattern.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Pattern missing for fuzzy match"))?;

        // Search in current directory recursively
        let mut candidates = Vec::new();

        // Use glob to find all files matching the pattern anywhere
        let glob_pattern = format!("**/*{}*", pattern);
        if let Ok(entries) = glob(&glob_pattern) {
            for entry in entries.flatten() {
                if self.fs.is_file(&entry) {
                    candidates.push(entry);
                }
            }
        }

        // Sort candidates by relevance (prefer closer matches)
        candidates.sort_by_key(|p| {
            let name = p.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            // Exact match gets highest priority
            if name == *pattern {
                0
            // Starts with pattern gets second priority
            } else if name.starts_with(pattern) {
                1
            // Contains pattern gets third priority
            } else if name.contains(pattern) {
                2
            } else {
                3
            }
        });

        // Limit results
        candidates.truncate(10);

        Ok(candidates)
    }

    /// Resolve glob pattern
    fn resolve_glob_pattern(&self, reference: &FileReference) -> Result<Vec<PathBuf>> {
        let pattern = reference.pattern.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Pattern missing for glob match"))?;

        let mut matches = Vec::new();

        if let Ok(entries) = glob(pattern) {
            for entry in entries.flatten() {
                if self.fs.is_file(&entry) {
                    matches.push(entry);
                }
            }
        }

        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests would require a mock filesystem for proper testing
    // For now, we'll have basic structure tests

    #[test]
    fn test_file_search_coordinator_creation() {
        // This would need a mock filesystem in real tests
        // For now, just verify the type is correct
        let _: FileSearchCoordinator;
    }
}
