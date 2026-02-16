use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::cli::tui::app::InputState;

/// Render the input area with a bold `>` prompt and cursor.
pub fn render_input(frame: &mut Frame, area: Rect, input: &InputState, is_idle: bool) {
    let prompt = Span::styled(
        " > ",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );

    let lines: Vec<Line> = if input.is_empty() && is_idle {
        vec![Line::from(vec![
            prompt,
            Span::styled(
                "Type a message... (Ctrl+J for newline, Ctrl+C to quit)",
                Style::default().fg(Color::Gray),
            ),
        ])]
    } else {
        input
            .lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                if i == 0 {
                    Line::from(vec![prompt.clone(), Span::raw(l.clone())])
                } else {
                    // Continuation lines: align with first line after prompt
                    Line::from(vec![Span::raw("   "), Span::raw(l.clone())])
                }
            })
            .collect()
    };

    let paragraph = Paragraph::new(lines).style(Style::default().bg(Color::DarkGray));
    frame.render_widget(paragraph, area);

    // Show cursor when idle
    if is_idle {
        // +3 for the " > " prompt
        let cursor_x = area.x + input.cursor_col as u16 + 3;
        let cursor_y = area.y + input.cursor_row as u16;
        if cursor_x < area.x + area.width && cursor_y < area.y + area.height {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn input_renders_placeholder_when_empty() {
        let backend = TestBackend::new(60, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let input = InputState::new();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_input(frame, area, &input, true);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains(">"));
        assert!(content.contains("Type a message"));
    }

    #[test]
    fn input_renders_text_with_prompt() {
        let backend = TestBackend::new(60, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut input = InputState::new();
        input.insert_char('h');
        input.insert_char('i');
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_input(frame, area, &input, true);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains(">"));
        assert!(content.contains("hi"));
    }

    #[test]
    fn input_no_placeholder_when_not_idle() {
        let backend = TestBackend::new(60, 1);
        let mut terminal = Terminal::new(backend).unwrap();
        let input = InputState::new();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_input(frame, area, &input, false);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(!content.contains("Type a message"));
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
