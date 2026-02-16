use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Parse a single line of markdown into styled spans.
fn parse_inline(line: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut chars = line.char_indices().peekable();
    let mut current = String::new();

    while let Some((i, c)) = chars.next() {
        match c {
            '`' => {
                // Inline code
                if !current.is_empty() {
                    spans.push(Span::raw(std::mem::take(&mut current)));
                }
                let mut code = String::new();
                let mut found_end = false;
                for (_j, c2) in chars.by_ref() {
                    if c2 == '`' {
                        found_end = true;
                        break;
                    }
                    code.push(c2);
                }
                if found_end {
                    spans.push(Span::styled(code, Style::default().fg(Color::Yellow)));
                } else {
                    // No closing backtick, treat as literal
                    current.push('`');
                    current.push_str(&code);
                }
            }
            '*' => {
                // Check for bold (**)
                if chars.peek().is_some_and(|(_, c2)| *c2 == '*') {
                    chars.next(); // consume second *
                    if !current.is_empty() {
                        spans.push(Span::raw(std::mem::take(&mut current)));
                    }
                    let mut bold = String::new();
                    let mut found_end = false;
                    while let Some((_, c2)) = chars.next() {
                        if c2 == '*' && chars.peek().is_some_and(|(_, c3)| *c3 == '*') {
                            chars.next(); // consume closing **
                            found_end = true;
                            break;
                        }
                        bold.push(c2);
                    }
                    if found_end {
                        spans.push(Span::styled(
                            bold,
                            Style::default().add_modifier(Modifier::BOLD),
                        ));
                    } else {
                        current.push_str("**");
                        current.push_str(&bold);
                    }
                } else {
                    current.push('*');
                }
            }
            _ => {
                let _ = i;
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        spans.push(Span::raw(current));
    }

    if spans.is_empty() {
        spans.push(Span::raw(""));
    }

    spans
}

/// Convert a markdown string into ratatui Lines.
///
/// Supports: headings (#), code blocks (```), inline code (`), bold (**)
pub fn markdown_to_lines(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lines: Vec<String> = Vec::new();

    for line in text.lines() {
        if line.starts_with("```") {
            if in_code_block {
                // End code block
                for code_line in &code_block_lines {
                    lines.push(Line::from(Span::styled(
                        format!("  {code_line}"),
                        Style::default().fg(Color::Green),
                    )));
                }
                code_block_lines.clear();
                in_code_block = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            code_block_lines.push(line.to_string());
            continue;
        }

        if let Some(heading) = line.strip_prefix("### ") {
            lines.push(Line::from(Span::styled(
                heading.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if let Some(heading) = line.strip_prefix("## ") {
            lines.push(Line::from(Span::styled(
                heading.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if let Some(heading) = line.strip_prefix("# ") {
            lines.push(Line::from(Span::styled(
                heading.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )));
        } else {
            lines.push(Line::from(parse_inline(line)));
        }
    }

    // Handle unclosed code block
    if in_code_block {
        for code_line in &code_block_lines {
            lines.push(Line::from(Span::styled(
                format!("  {code_line}"),
                Style::default().fg(Color::Green),
            )));
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text() {
        let lines = markdown_to_lines("hello world");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "hello world");
    }

    #[test]
    fn heading() {
        let lines = markdown_to_lines("# Title");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "Title");
        assert!(
            lines[0].spans[0]
                .style
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn code_block() {
        let input = "```\nlet x = 1;\nlet y = 2;\n```";
        let lines = markdown_to_lines(input);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].spans[0].content, "  let x = 1;");
        assert_eq!(lines[0].spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn inline_code() {
        let lines = markdown_to_lines("use `foo` here");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 3);
        assert_eq!(lines[0].spans[0].content, "use ");
        assert_eq!(lines[0].spans[1].content, "foo");
        assert_eq!(lines[0].spans[1].style.fg, Some(Color::Yellow));
        assert_eq!(lines[0].spans[2].content, " here");
    }

    #[test]
    fn bold_text() {
        let lines = markdown_to_lines("this is **bold** text");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans.len(), 3);
        assert_eq!(lines[0].spans[1].content, "bold");
        assert!(
            lines[0].spans[1]
                .style
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn unclosed_code_block() {
        let input = "```\norphan code";
        let lines = markdown_to_lines(input);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].spans[0].content, "  orphan code");
    }

    #[test]
    fn empty_input() {
        let lines = markdown_to_lines("");
        assert_eq!(lines.len(), 0);
    }

    #[test]
    fn multiline_mixed() {
        let input = "# Heading\nSome text with `code`\n```\nblock\n```\n**bold**";
        let lines = markdown_to_lines(input);
        assert_eq!(lines.len(), 4); // heading, inline, code block, bold
    }
}
