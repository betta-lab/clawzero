use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::cli::tui::app::{App, AppMode};
use crate::cli::tui::widgets::{chat, input, status};

/// Render the live viewport area (inline TUI).
///
/// The viewport has a fixed height. Content is rendered top-aligned:
/// - Streaming assistant text (while thinking)
/// - Live tool card (while executing)
/// - Status bar (always)
/// - Input area (when idle)
/// - Remaining space is left blank
pub fn draw_live(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Build live content lines
    let mut live_lines = Vec::new();

    // Streaming text (while thinking) — only show lines not yet flushed
    if !app.streaming_buffer.is_empty() {
        let streaming = chat::render_streaming_lines(&app.streaming_buffer);
        live_lines.extend(streaming.into_iter().skip(app.streaming_lines_flushed));
    }

    // Live tool card (while executing)
    if let Some(ref tool) = app.live_tool {
        live_lines.extend(chat::render_live_tool_card(&tool.name, app.spinner_char()));
    }

    let live_content_height = live_lines.len() as u16;
    let status_height = 1_u16;
    let input_height = if app.mode == AppMode::Idle { 1_u16 } else { 0 };

    // Layout: [live content | status | input | spacer (absorbs remaining)]
    let mut constraints = Vec::new();
    if live_content_height > 0 {
        constraints.push(Constraint::Length(live_content_height));
    }
    constraints.push(Constraint::Length(status_height));
    if input_height > 0 {
        constraints.push(Constraint::Length(input_height));
    }
    // Spacer to absorb remaining viewport height
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // Render live content
    if live_content_height > 0 {
        let paragraph = ratatui::widgets::Paragraph::new(live_lines);
        frame.render_widget(paragraph, chunks[chunk_idx]);
        chunk_idx += 1;
    }

    // Render status bar (borderless, single line)
    status::render_status(
        frame,
        chunks[chunk_idx],
        &app.mode,
        app.spinner_char(),
        app.usage_in,
        app.usage_out,
    );
    chunk_idx += 1;

    // Render input (only when idle)
    if input_height > 0 {
        let is_idle = app.mode == AppMode::Idle;
        input::render_input(frame, chunks[chunk_idx], &app.input, is_idle);
    }

    // Spacer chunk is left blank (auto-cleared by ratatui)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn live_layout_renders_idle_without_panic() {
        let backend = TestBackend::new(80, 16);
        let mut terminal = Terminal::new(backend).unwrap();
        let app = App::new("test/model".into(), Some("session123".into()));

        terminal
            .draw(|frame| {
                draw_live(frame, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("IDLE"));
    }

    #[test]
    fn live_layout_renders_thinking_with_streaming() {
        let backend = TestBackend::new(80, 16);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut app = App::new("test/model".into(), None);
        app.mode = AppMode::Thinking;
        app.streaming_buffer = "Hello world".into();

        terminal
            .draw(|frame| {
                draw_live(frame, &app);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer_to_string(&buffer);
        assert!(content.contains("Hello world"));
        assert!(content.contains("THINKING"));
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
