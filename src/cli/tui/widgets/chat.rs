use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::cli::tui::app::ToolStatus;
use crate::cli::tui::markdown::markdown_to_lines;

/// Render a user message as lines for insert_before.
pub fn render_user_message(text: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::styled(
                " > ",
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(text.to_string()),
        ]),
        Line::from(""),
    ]
}

/// Background color for assistant response lines (subtle dark gray).
const ASSISTANT_BG: Color = Color::Indexed(235);

/// Render an assistant message (markdown) as lines for insert_before.
pub fn render_assistant_message(text: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let md_lines = markdown_to_lines(text);
    for line in md_lines {
        let mut prefixed: Vec<Span<'static>> = vec![Span::raw(" ")];
        prefixed.extend(line.spans);
        lines.push(Line::from(prefixed).style(Style::default().bg(ASSISTANT_BG)));
    }
    lines.push(Line::from(""));
    lines
}

/// Render a completed tool card as lines for insert_before.
pub fn render_tool_card(
    name: &str,
    input_summary: &str,
    status: &ToolStatus,
    output: Option<&str>,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    let status_str = match status {
        ToolStatus::Running => Span::styled("RUNNING", Style::default().fg(Color::Yellow)),
        ToolStatus::Ok => Span::styled("OK", Style::default().fg(Color::Green)),
        ToolStatus::Error => Span::styled("ERROR", Style::default().fg(Color::Red)),
    };

    // Tool card top border
    lines.push(Line::from(vec![
        Span::raw(" "),
        Span::styled(
            format!("\u{250c}\u{2500} {name} "),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            "\u{2500}".repeat(30_usize.saturating_sub(name.len())),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(" "),
        status_str,
        Span::styled(" \u{2500}\u{2510}", Style::default().fg(Color::DarkGray)),
    ]));

    // Input summary
    if !input_summary.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(" \u{2502} ", Style::default().fg(Color::DarkGray)),
            Span::styled(input_summary.to_string(), Style::default().fg(Color::White)),
        ]));
    }

    // Output (truncated)
    if let Some(out) = output {
        let truncated = if out.len() > 200 {
            format!("{}...", &out[..200])
        } else {
            out.to_string()
        };
        for out_line in truncated.lines().take(5) {
            lines.push(Line::from(vec![
                Span::styled(" \u{2502} ", Style::default().fg(Color::DarkGray)),
                Span::raw(out_line.to_string()),
            ]));
        }
    }

    // Tool card bottom border
    lines.push(Line::from(Span::styled(
        " \u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(""));

    lines
}

/// Render a system notice as lines for insert_before.
pub fn render_system_notice(text: &str) -> Vec<Line<'static>> {
    vec![
        Line::from(Span::styled(
            format!(" [{text}]"),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ]
}

/// Render streaming assistant text as lines for the live viewport.
pub fn render_streaming_lines(streaming_buffer: &str) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if !streaming_buffer.is_empty() {
        let md_lines = markdown_to_lines(streaming_buffer);
        for line in md_lines {
            let mut prefixed: Vec<Span<'static>> = vec![Span::raw(" ")];
            prefixed.extend(line.spans);
            lines.push(Line::from(prefixed).style(Style::default().bg(ASSISTANT_BG)));
        }
    }
    lines
}

/// Render a live (in-progress) tool card as lines for the viewport.
pub fn render_live_tool_card(name: &str, spinner_char: char) -> Vec<Line<'static>> {
    vec![
        Line::from(vec![
            Span::raw(" "),
            Span::styled(
                format!("\u{250c}\u{2500} {name} "),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                "\u{2500}".repeat(30_usize.saturating_sub(name.len())),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(" "),
            Span::styled(
                format!("RUNNING {spinner_char}"),
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(" \u{2500}\u{2510}", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(Span::styled(
            " \u{2514}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2500}\u{2518}",
            Style::default().fg(Color::DarkGray),
        )),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_lines() {
        let lines = render_user_message("hello");
        assert!(
            lines
                .iter()
                .any(|l| { l.spans.iter().any(|s| s.content.contains("hello")) })
        );
        assert_eq!(lines.len(), 2); // message + blank
    }

    #[test]
    fn assistant_message_lines() {
        let lines = render_assistant_message("world");
        assert!(
            lines
                .iter()
                .any(|l| { l.spans.iter().any(|s| s.content.contains("world")) })
        );
    }

    #[test]
    fn tool_card_lines() {
        let lines = render_tool_card("bash", "$ ls", &ToolStatus::Ok, Some("file1\nfile2"));
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("bash"));
        assert!(text.contains("$ ls"));
        assert!(text.contains("OK"));
        assert!(text.contains("file1"));
    }

    #[test]
    fn system_notice_lines() {
        let lines = render_system_notice("test notice");
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("test notice"));
    }

    #[test]
    fn streaming_buffer_lines() {
        let lines = render_streaming_lines("streaming...");
        assert!(
            lines
                .iter()
                .any(|l| { l.spans.iter().any(|s| s.content.contains("streaming...")) })
        );
    }

    #[test]
    fn streaming_buffer_empty() {
        let lines = render_streaming_lines("");
        assert!(lines.is_empty());
    }

    #[test]
    fn live_tool_card_lines() {
        let lines = render_live_tool_card("bash", '◐');
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect::<Vec<_>>()
            .join("");
        assert!(text.contains("bash"));
        assert!(text.contains("RUNNING"));
    }
}
