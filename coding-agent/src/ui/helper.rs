//! rustyline helper for file reference completion

use crate::ui::completer::FileReferenceCompleter;
use crate::platform::domain::filesystem::FileSystem;
use rustyline::completion::Completer;
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Helper, Context};
use std::sync::Arc;

/// rustyline helper that enables @ file reference autocompletion
pub struct FileReferenceHelper {
    /// File reference completer
    completer: FileReferenceCompleter,
    /// Matching bracket highlighter
    bracket_highlighter: MatchingBracketHighlighter,
}

impl FileReferenceHelper {
    /// Create a new file reference helper
    pub fn new(fs: Arc<dyn FileSystem>) -> Self {
        Self {
            completer: FileReferenceCompleter::new(fs),
            bracket_highlighter: MatchingBracketHighlighter::new(),
        }
    }
}

impl Completer for FileReferenceHelper {
    type Candidate = <FileReferenceCompleter as Completer>::Candidate;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Self::Candidate>), ReadlineError> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Hinter for FileReferenceHelper {}

impl Highlighter for FileReferenceHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> rustyline::HighlightedText<'l> {
        self.bracket_highlighter.highlight(line, pos)
    }

    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.bracket_highlighter.highlight_char(line, pos)
    }
}

impl Validator for FileReferenceHelper {}

impl Helper for FileReferenceHelper {}
