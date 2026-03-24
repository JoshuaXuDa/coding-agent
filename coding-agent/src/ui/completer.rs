//! File reference autocompleter for @ symbols

use crate::platform::domain::filesystem::FileSystem;
use glob::glob;
use rustyline::completion::{Completer, Pair};
use rustyline::Context;
use std::sync::Arc;

/// Auto-completer for @ file references
pub struct FileReferenceCompleter {
    /// Filesystem for searching files
    fs: Arc<dyn FileSystem>,
}

impl FileReferenceCompleter {
    /// Create a new file reference completer
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self { fs }
    }

    /// Search for files matching the pattern
    fn search_files(&self, pattern: &str) -> Vec<String> {
        let mut matches = Vec::new();

        // If pattern is empty, list files in current directory
        let search_pattern = if pattern.is_empty() {
            "*".to_string()
        } else if pattern.contains('/') {
            // Pattern contains path, search recursively
            format!("**/*{}*", pattern)
        } else {
            // Just filename, search recursively
            format!("**/*{}*", pattern)
        };

        if let Ok(entries) = glob(&search_pattern) {
            for entry in entries.flatten() {
                if self.fs.is_file(&entry) {
                    if let Some(path_str) = entry.to_str() {
                        matches.push(path_str.to_string());
                    }
                }
            }
        }

        // Sort and limit results
        matches.sort();
        matches.truncate(20);

        matches
    }
}

impl Completer for FileReferenceCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> std::result::Result<(usize, Vec<Pair>), rustyline::error::ReadlineError> {
        // Check if cursor is after an @ symbol
        let before_cursor = &line[..pos];

        if let Some(at_pos) = before_cursor.rfind('@') {
            // Get the pattern after @ (up to cursor)
            let pattern = &before_cursor[at_pos + 1..];

            // Search for matching files
            let matches = self.search_files(pattern);

            // Create completion candidates
            let candidates: Vec<Pair> = matches
                .into_iter()
                .map(|path| Pair {
                    display: path.clone(),
                    replacement: path,
                })
                .collect();

            Ok((at_pos, candidates))
        } else {
            // No @ symbol found, return empty completion
            Ok((0, vec![]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completer_creation() {
        // Verify the type exists
        let _: FileReferenceCompleter;
    }
}
