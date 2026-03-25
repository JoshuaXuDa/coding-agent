//! Markdown rendering for TUI
//!
//! Provides markdown-to-terminal rendering using pulldown-cmark.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use unicode_width::UnicodeWidthStr;

/// Markdown renderer for TUI
pub struct MarkdownRenderer;

impl MarkdownRenderer {
    /// Render markdown text to ratatui Text
    pub fn render(markdown: &str, width: usize) -> Text {
        let parser = Parser::new(markdown);
        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut in_paragraph = false;
        let mut list_level = 0;
        let mut code_block = false;
        let mut quote_level: usize = 0;
        let mut table_alignments = Vec::new();
        let mut in_table = false;
        let mut table_row = Vec::new();
        let mut table_header = false;

        for event in parser {
            match event {
                // Start tags
                Event::Start(tag) => match tag {
                    Tag::Paragraph(_) => {
                        in_paragraph = true;
                    }
                    Tag::Heading { level, .. } => {
                        // Close current line if needed
                        if !current_line.is_empty() {
                            lines.push(current_line.clone());
                            current_line.clear();
                        }
                        // Add heading prefix
                        let prefix = "#".repeat(level as usize);
                        let style = Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD);
                        current_line.push(Span::styled(format!("{} ", prefix), style));
                    }
                    Tag::BlockQuote(_) => {
                        quote_level += 1;
                        if !current_line.is_empty() {
                            lines.push(current_line.clone());
                            current_line.clear();
                        }
                    }
                    Tag::CodeBlock(kind) => {
                        code_block = true;
                        if !current_line.is_empty() {
                            lines.push(current_line.clone());
                            current_line.clear();
                        }
                        // Add code block indicator
                        let info = match kind {
                            pulldown_cmark::CodeBlockKind::Fenced(info) => {
                                info.to_string()
                            }
                            pulldown_cmark::CodeBlockKind::Indented => "".to_string(),
                        };
                        if !info.is_empty() {
                            lines.push(vec![Span::styled(
                                format!("```{}", info),
                                Style::default().fg(Color::DarkGray),
                            )]);
                        } else {
                            lines.push(vec![Span::styled(
                                "```",
                                Style::default().fg(Color::DarkGray),
                            )]);
                        }
                    }
                    Tag::List(_) => {
                        // List start, handled by Item
                    }
                    Tag::Item(_) => {
                        if !current_line.is_empty() {
                            lines.push(current_line.clone());
                            current_line.clear();
                        }
                        let indent = "  ".repeat(list_level);
                        let style = Style::default().fg(Color::Yellow);
                        current_line.push(Span::styled(format!("{}• ", indent), style));
                    }
                    Tag::Emphasis(_, _) => {
                        // Start emphasis (italic) - use dim for terminals that don't support italic
                    }
                    Tag::Strong(_, _) => {
                        // Start strong (bold) - marker handled in text
                    }
                    Tag::Strikethrough(_) => {
                        // Start strikethrough - use dim
                    }
                    Tag::Link { .. } => {
                        // Link start - underline
                    }
                    Tag::Table(alignments) => {
                        in_table = true;
                        table_alignments = alignments;
                    }
                    Tag::TableHead => {
                        table_header = true;
                    }
                    Tag::TableRow => {
                        table_row.clear();
                    }
                    Tag::TableCell => {
                        // Table cell content handled in Text
                    }
                    Tag::FootnoteDefinition(_) => {}
                    Tag::HtmlBlock(_) => {}
                },

                // End tags
                Event::End(tag) => match tag {
                    TagEnd::Paragraph => {
                        if !current_line.is_empty() {
                            lines.push(current_line.clone());
                            current_line.clear();
                        }
                        lines.push(vec![]); // Blank line after paragraph
                        in_paragraph = false;
                    }
                    TagEnd::Heading(_) => {
                        if !current_line.is_empty() {
                            lines.push(current_line.clone());
                            current_line.clear();
                        }
                        lines.push(vec![]); // Blank line after heading
                    }
                    TagEnd::BlockQuote => {
                        if !current_line.is_empty() {
                            lines.push(current_line.clone());
                            current_line.clear();
                        }
                        quote_level = quote_level.saturating_sub(1);
                        lines.push(vec![]); // Blank line after quote
                    }
                    TagEnd::CodeBlock => {
                        lines.push(vec![Span::styled(
                            "```",
                            Style::default().fg(Color::DarkGray),
                        )]);
                        code_block = false;
                        lines.push(vec![]); // Blank line after code block
                    }
                    TagEnd::List(_) => {}
                    TagEnd::Item => {}
                    TagEnd::Emphasis => {}
                    TagEnd::Strong => {}
                    TagEnd::Strikethrough => {}
                    TagEnd::Link => {}
                    TagEnd::Table => {
                        in_table = false;
                        lines.push(vec![]); // Blank line after table
                    }
                    TagEnd::TableHead => {
                        table_header = false;
                        // Add separator line
                        let mut sep_line = Vec::new();
                        sep_line.push(Span::styled("|", Style::default().fg(Color::DarkGray)));
                        for _ in 0..table_alignments.len() {
                            sep_line.push(Span::styled(
                                "---|",
                                Style::default().fg(Color::DarkGray),
                            ));
                        }
                        lines.push(sep_line);
                    }
                    TagEnd::TableRow => {
                        // Render table row
                        let mut row_line = Vec::new();
                        row_line.push(Span::styled("|", Style::default().fg(Color::DarkGray)));
                        for cell in &table_row {
                            row_line.push(Span::styled(
                                format!(" {} |", cell),
                                if table_header {
                                    Style::default()
                                        .fg(Color::Cyan)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(Color::White)
                                },
                            ));
                        }
                        lines.push(row_line);
                    }
                    TagEnd::TableCell => {}
                    TagEnd::FootnoteDefinition => {}
                    TagEnd::HtmlBlock => {}
                },

                // Text and inline elements
                Event::Text(text) => {
                    let text = text.to_string();

                    if code_block {
                        // Code block text - monospace style
                        lines.extend(text.lines().map(|line| {
                            vec![Span::styled(
                                format!(" {}", line),
                                Style::default().fg(Color::Cyan),
                            )]
                        }));
                    } else if quote_level > 0 {
                        // Quote text
                        let prefix = "│ ".repeat(quote_level);
                        let style = Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM);
                        current_line.push(Span::styled(prefix, style));
                        current_line.push(Span::styled(text.clone(), style));
                    } else {
                        // Regular text
                        current_line.push(Span::styled(text, Style::default().fg(Color::White)));
                    }
                }
                Event::Code(code) => {
                    // Inline code
                    let style = Style::default().fg(Color::Yellow);
                    current_line.push(Span::styled(
                        format!("`{}`", code),
                        style,
                    ));
                }
                Event::SoftBreak | Event::HardBreak => {
                    if !current_line.is_empty() {
                        lines.push(current_line.clone());
                        current_line.clear();
                    }
                }
                Event::Rule => {
                    if !current_line.is_empty() {
                        lines.push(current_line.clone());
                        current_line.clear();
                    }
                    lines.push(vec![Span::styled(
                        "─".repeat(width.min(40)),
                        Style::default().fg(Color::DarkGray),
                    )]);
                }
                Event::TaskListMarker(checked) => {
                    let marker = if checked { "[x]" } else { "[ ]" };
                    let style = Style::default().fg(Color::Green);
                    current_line.push(Span::styled(
                        format!("{} ", marker),
                        style,
                    ));
                }
                Event::Html(html) => {
                    // Skip HTML in TUI
                }
                Event::InlineHtml(html) => {
                    // Skip inline HTML
                }
                Event::InlineMath(math) => {
                    // Math - just display as text
                    current_line.push(Span::styled(
                        format!("${}$", math),
                        Style::default().fg(Color::Magenta),
                    ));
                }
                Event::DisplayMath(math) => {
                    // Display math
                    if !current_line.is_empty() {
                        lines.push(current_line.clone());
                        current_line.clear();
                    }
                    lines.push(vec![Span::styled(
                        format!("$${}$$", math),
                        Style::default().fg(Color::Magenta),
                    )]);
                }
                Event::FootnoteReference(_) => {}
            }
        }

        // Don't forget the last line
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Convert Vec<Vec<Span>> to Vec<Line>
        let line_items: Vec<Line> = lines.into_iter().map(Line::from).collect();
        Text::from(line_items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_simple_text() {
        let text = MarkdownRenderer::render("Hello world", 80);
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_render_bold() {
        let text = MarkdownRenderer::render("**bold text**", 80);
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_render_code_block() {
        let text = MarkdownRenderer::render("```rust\nfn main() {}\n```", 80);
        assert!(!text.lines.is_empty());
    }
}
