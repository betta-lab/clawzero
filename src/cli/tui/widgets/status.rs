use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::cli::tui::app::AppMode;

/// Render the status bar showing mode and token usage (borderless, single line).
pub fn render_status(
    frame: &mut Frame,
    area: Rect,
    mode: &AppMode,
    spinner_char: char,
    usage_in: u32,
    usage_out: u32,
) {
    let mode_span = match mode {
        AppMode::Idle => Span::styled(
            " IDLE",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ),
        AppMode::Thinking => Span::styled(
            format!(" THINKING {spinner_char}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        AppMode::ToolExecuting { name } => Span::styled(
            format!(" TOOL: {name} {spinner_char}"),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
    };

    let usage_span = Span::styled(
        format!(
            "Tokens: {} in / {} out ",
            format_number(usage_in),
            format_number(usage_out)
        ),
        Style::default().fg(Color::DarkGray),
    );

    let line = Line::from(vec![
        mode_span,
        Span::raw("  "),
        Span::styled("\u{2502}", Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        usage_span,
    ]);

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

/// Format a number with comma separators.
fn format_number(n: u32) -> String {
    if n < 1000 {
        return n.to_string();
    }
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn format_number_small() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn format_number_with_commas() {
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn status_renders_idle() {
        let backend = TestBackend::new(60, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_status(frame, area, &AppMode::Idle, '◐', 1234, 567);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("IDLE"));
        assert!(content.contains("1,234"));
    }

    #[test]
    fn status_renders_thinking() {
        let backend = TestBackend::new(60, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_status(frame, area, &AppMode::Thinking, '◓', 0, 0);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("THINKING"));
    }

    #[test]
    fn status_renders_tool_executing() {
        let backend = TestBackend::new(60, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_status(
                    frame,
                    area,
                    &AppMode::ToolExecuting {
                        name: "bash".into(),
                    },
                    '◑',
                    100,
                    50,
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("TOOL: bash"));
    }

    fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
        let mut s = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                s.push_str(cell.symbol());
            }
            s.push('\n');
        }
        s
    }
}
