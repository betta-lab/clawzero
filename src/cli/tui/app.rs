use crate::agent::event::AgentEvent;
use crate::cli::repl::format_tool_input;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::text::Line;

/// Application mode reflecting the agent's state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Idle,
    Thinking,
    ToolExecuting { name: String },
}

/// Status of a tool call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Ok,
    Error,
}

/// A live (in-progress) tool call being displayed in the viewport.
#[derive(Debug, Clone)]
pub struct LiveToolCall {
    pub id: String,
    pub name: String,
    pub input_summary: String,
    pub status: ToolStatus,
    pub output: Option<String>,
}

/// Action returned from key event handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    Submit(String),
    Quit,
}

/// Multi-line text input state.
#[derive(Debug, Clone)]
pub struct InputState {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl InputState {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, c: char) {
        self.lines[self.cursor_row].insert(self.cursor_col, c);
        self.cursor_col += c.len_utf8();
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            // Find the previous char boundary
            let line = &self.lines[self.cursor_row];
            let prev = line[..self.cursor_col]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.lines[self.cursor_row].remove(prev);
            self.cursor_col = prev;
        } else if self.cursor_row > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
            self.lines[self.cursor_row].push_str(&current_line);
        }
    }

    /// Insert a newline at cursor position.
    pub fn insert_newline(&mut self) {
        let rest = self.lines[self.cursor_row][self.cursor_col..].to_string();
        self.lines[self.cursor_row].truncate(self.cursor_col);
        self.cursor_row += 1;
        self.lines.insert(self.cursor_row, rest);
        self.cursor_col = 0;
    }

    /// Get the full text content.
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// Check if the input is empty (all whitespace).
    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.trim().is_empty())
    }

    /// Clear input and reset cursor.
    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    /// Move cursor left.
    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            let line = &self.lines[self.cursor_row];
            self.cursor_col = line[..self.cursor_col]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    /// Move cursor right.
    pub fn move_right(&mut self) {
        let line_len = self.lines[self.cursor_row].len();
        if self.cursor_col < line_len {
            let line = &self.lines[self.cursor_row];
            self.cursor_col = line[self.cursor_col..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_col + i)
                .unwrap_or(line_len);
        }
    }

    /// Move cursor to beginning of line (Ctrl-A / Home).
    pub fn move_home(&mut self) {
        self.cursor_col = 0;
    }

    /// Move cursor to end of line (Ctrl-E / End).
    pub fn move_end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].len();
    }

    /// Delete from cursor to end of line (Ctrl-K).
    pub fn kill_to_end(&mut self) {
        self.lines[self.cursor_row].truncate(self.cursor_col);
    }

    /// Delete the word before the cursor (Ctrl-W).
    pub fn delete_word_back(&mut self) {
        if self.cursor_col == 0 {
            return;
        }
        let line = &self.lines[self.cursor_row][..self.cursor_col];
        // Skip trailing whitespace, then skip non-whitespace
        let trimmed_end = line.trim_end().len();
        let word_start = line[..trimmed_end]
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        self.lines[self.cursor_row].replace_range(word_start..self.cursor_col, "");
        self.cursor_col = word_start;
    }
}

/// Main application state for inline TUI.
///
/// Confirmed content is pushed to `pending_inserts` for `insert_before()`.
/// Only live (in-progress) content is displayed in the viewport.
pub struct App {
    pub mode: AppMode,
    pub input: InputState,
    pub usage_in: u32,
    pub usage_out: u32,
    pub model: String,
    pub session_id: Option<String>,
    pub streaming_buffer: String,
    pub spinner_tick: usize,
    /// Lines to be flushed via `terminal.insert_before()`.
    pub pending_inserts: Vec<Line<'static>>,
    /// Currently executing tool card (shown live in viewport).
    pub live_tool: Option<LiveToolCall>,
    /// Number of streaming lines already flushed to pending_inserts (progressive flush).
    pub streaming_lines_flushed: usize,
}

impl App {
    pub fn new(model: String, session_id: Option<String>) -> Self {
        Self {
            mode: AppMode::Idle,
            input: InputState::new(),
            usage_in: 0,
            usage_out: 0,
            model,
            session_id,
            streaming_buffer: String::new(),
            spinner_tick: 0,
            pending_inserts: Vec::new(),
            live_tool: None,
            streaming_lines_flushed: 0,
        }
    }

    /// Push user message lines to pending_inserts (for insert_before).
    pub fn push_user_message(&mut self, text: &str) {
        use super::widgets::chat;
        self.pending_inserts.extend(chat::render_user_message(text));
    }

    /// Handle an agent event, updating state. Returns true if the UI needs redrawing.
    pub fn handle_agent_event(&mut self, event: &AgentEvent) -> bool {
        match event {
            AgentEvent::TextDelta(text) => {
                self.streaming_buffer.push_str(text);
                self.flush_streaming_overflow();
                true
            }
            AgentEvent::ToolCallStart { id, name } => {
                // Flush streaming buffer as assistant text → pending_inserts
                if !self.streaming_buffer.is_empty() {
                    let text = std::mem::take(&mut self.streaming_buffer);
                    self.flush_assistant_text(&text);
                }
                self.mode = AppMode::ToolExecuting { name: name.clone() };
                self.live_tool = Some(LiveToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    input_summary: String::new(),
                    status: ToolStatus::Running,
                    output: None,
                });
                true
            }
            AgentEvent::ToolResult {
                id,
                name,
                input,
                output,
                is_error,
            } => {
                let status = if *is_error {
                    ToolStatus::Error
                } else {
                    ToolStatus::Ok
                };
                let input_summary = format_tool_input(name, input);

                // Complete the live tool and push it to pending_inserts
                if let Some(mut tool) = self.live_tool.take()
                    && tool.id == *id
                {
                    tool.status = status;
                    tool.input_summary = input_summary;
                    tool.output = Some(output.clone());
                    self.flush_tool_card(&tool);
                }
                self.mode = AppMode::Thinking;
                true
            }
            AgentEvent::TurnComplete { usage } => {
                self.usage_in += usage.input_tokens;
                self.usage_out += usage.output_tokens;
                true
            }
            AgentEvent::Done { total_usage } => {
                // Flush any remaining streaming buffer
                if !self.streaming_buffer.is_empty() {
                    let text = std::mem::take(&mut self.streaming_buffer);
                    self.flush_assistant_text(&text);
                }
                self.usage_in = total_usage.input_tokens;
                self.usage_out = total_usage.output_tokens;
                self.mode = AppMode::Idle;
                true
            }
            AgentEvent::ContextCompacted {
                original_tokens,
                compacted_tokens,
                messages_dropped,
            } => {
                use super::widgets::chat;
                let notice = format!(
                    "Context compacted: {messages_dropped} messages dropped, {original_tokens} → {compacted_tokens} tokens"
                );
                self.pending_inserts
                    .extend(chat::render_system_notice(&notice));
                true
            }
            AgentEvent::Error(msg) => {
                // Flush any remaining streaming buffer
                if !self.streaming_buffer.is_empty() {
                    let text = std::mem::take(&mut self.streaming_buffer);
                    self.flush_assistant_text(&text);
                }
                use super::widgets::chat;
                self.pending_inserts
                    .extend(chat::render_system_notice(&format!("Error: {msg}")));
                self.mode = AppMode::Idle;
                true
            }
        }
    }

    /// Progressively flush overflow streaming lines to pending_inserts.
    ///
    /// Keeps at most `MAX_STREAMING_VISIBLE` lines in the viewport;
    /// excess lines are pushed to pending_inserts for insert_before.
    fn flush_streaming_overflow(&mut self) {
        const MAX_STREAMING_VISIBLE: usize = 12;

        if self.streaming_buffer.is_empty() {
            return;
        }
        use super::widgets::chat;
        let lines = chat::render_streaming_lines(&self.streaming_buffer);
        let total = lines.len();
        if total > self.streaming_lines_flushed + MAX_STREAMING_VISIBLE {
            let flush_up_to = total - MAX_STREAMING_VISIBLE;
            self.pending_inserts.extend(
                lines
                    .into_iter()
                    .skip(self.streaming_lines_flushed)
                    .take(flush_up_to - self.streaming_lines_flushed),
            );
            self.streaming_lines_flushed = flush_up_to;
        }
    }

    /// Flush remaining assistant text to pending_inserts (on Done / ToolCallStart).
    ///
    /// Skips lines already flushed during progressive streaming.
    fn flush_assistant_text(&mut self, text: &str) {
        use super::widgets::chat;
        let lines = chat::render_assistant_message(text);
        // Skip lines already flushed progressively during streaming
        self.pending_inserts
            .extend(lines.into_iter().skip(self.streaming_lines_flushed));
        self.streaming_lines_flushed = 0;
    }

    /// Flush a completed tool card to pending_inserts.
    fn flush_tool_card(&mut self, tool: &LiveToolCall) {
        use super::widgets::chat;
        self.pending_inserts.extend(chat::render_tool_card(
            &tool.name,
            &tool.input_summary,
            &tool.status,
            tool.output.as_deref(),
        ));
    }

    /// Handle a key event, returning an action if one should be performed.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppAction> {
        // Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Some(AppAction::Quit);
        }

        // When not idle, ignore most input
        if self.mode != AppMode::Idle {
            return None;
        }

        match key.code {
            KeyCode::Enter => {
                if self.input.is_empty() {
                    return None;
                }
                let text = self.input.text();
                self.input.clear();

                // Check for exit commands
                let trimmed = text.trim();
                if trimmed == "/exit" || trimmed == "/quit" {
                    return Some(AppAction::Quit);
                }

                self.push_user_message(&text);
                self.mode = AppMode::Thinking;
                Some(AppAction::Submit(text))
            }
            KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.insert_newline();
                None
            }
            KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.move_home();
                None
            }
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.move_end();
                None
            }
            KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.kill_to_end();
                None
            }
            KeyCode::Char('w') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.input.delete_word_back();
                None
            }
            KeyCode::Home => {
                self.input.move_home();
                None
            }
            KeyCode::End => {
                self.input.move_end();
                None
            }
            KeyCode::Char(c) => {
                self.input.insert_char(c);
                None
            }
            KeyCode::Backspace => {
                self.input.backspace();
                None
            }
            KeyCode::Left => {
                self.input.move_left();
                None
            }
            KeyCode::Right => {
                self.input.move_right();
                None
            }
            _ => None,
        }
    }

    /// Advance the spinner animation tick.
    pub fn tick(&mut self) {
        self.spinner_tick = (self.spinner_tick + 1) % 4;
    }

    /// Get the current spinner character.
    pub fn spinner_char(&self) -> char {
        ['◐', '◓', '◑', '◒'][self.spinner_tick]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::response::Usage;

    // ── InputState tests ──

    #[test]
    fn input_state_new_is_empty() {
        let input = InputState::new();
        assert!(input.is_empty());
        assert_eq!(input.text(), "");
        assert_eq!(input.cursor_row, 0);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn input_insert_char() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        assert_eq!(input.text(), "ab");
        assert_eq!(input.cursor_col, 2);
        assert!(!input.is_empty());
    }

    #[test]
    fn input_insert_multibyte_char() {
        let mut input = InputState::new();
        input.insert_char('あ');
        input.insert_char('い');
        assert_eq!(input.text(), "あい");
        assert_eq!(input.cursor_col, 6); // 3 bytes each
    }

    #[test]
    fn input_backspace_removes_char() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.backspace();
        assert_eq!(input.text(), "a");
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn input_backspace_at_start_does_nothing() {
        let mut input = InputState::new();
        input.backspace();
        assert_eq!(input.text(), "");
    }

    #[test]
    fn input_backspace_merges_lines() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_newline();
        input.insert_char('b');
        assert_eq!(input.text(), "a\nb");

        // Move to start of second line
        input.cursor_col = 0;
        input.backspace();
        assert_eq!(input.text(), "ab");
        assert_eq!(input.cursor_row, 0);
        assert_eq!(input.cursor_col, 1);
    }

    #[test]
    fn input_insert_newline() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.cursor_col = 1; // between 'a' and 'b'
        input.insert_newline();
        assert_eq!(input.lines, vec!["a", "b"]);
        assert_eq!(input.cursor_row, 1);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn input_clear_resets() {
        let mut input = InputState::new();
        input.insert_char('h');
        input.insert_char('i');
        input.clear();
        assert!(input.is_empty());
        assert_eq!(input.cursor_row, 0);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn input_move_left_right() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        assert_eq!(input.cursor_col, 2);

        input.move_left();
        assert_eq!(input.cursor_col, 1);

        input.move_left();
        assert_eq!(input.cursor_col, 0);

        input.move_left(); // at start, no-op
        assert_eq!(input.cursor_col, 0);

        input.move_right();
        assert_eq!(input.cursor_col, 1);

        input.move_right();
        assert_eq!(input.cursor_col, 2);

        input.move_right(); // at end, no-op
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn input_whitespace_only_is_empty() {
        let mut input = InputState::new();
        input.insert_char(' ');
        input.insert_char(' ');
        assert!(input.is_empty());
    }

    // ── AgentEvent handling tests ──

    #[test]
    fn handle_text_delta_accumulates() {
        let mut app = App::new("test/model".into(), None);
        app.mode = AppMode::Thinking;

        assert!(app.handle_agent_event(&AgentEvent::TextDelta("hello ".into())));
        assert!(app.handle_agent_event(&AgentEvent::TextDelta("world".into())));
        assert_eq!(app.streaming_buffer, "hello world");
        assert!(app.pending_inserts.is_empty()); // not flushed yet
    }

    #[test]
    fn handle_tool_call_start_flushes_buffer() {
        let mut app = App::new("test/model".into(), None);
        app.mode = AppMode::Thinking;
        app.streaming_buffer = "some text".into();

        app.handle_agent_event(&AgentEvent::ToolCallStart {
            id: "t1".into(),
            name: "bash".into(),
        });

        assert_eq!(app.streaming_buffer, "");
        // Flushed text goes to pending_inserts
        assert!(!app.pending_inserts.is_empty());
        // Live tool is set
        assert!(app.live_tool.is_some());
        let tool = app.live_tool.as_ref().unwrap();
        assert_eq!(tool.name, "bash");
        assert_eq!(tool.status, ToolStatus::Running);
        assert_eq!(
            app.mode,
            AppMode::ToolExecuting {
                name: "bash".into()
            }
        );
    }

    #[test]
    fn handle_tool_result_flushes_tool_card() {
        let mut app = App::new("test/model".into(), None);
        app.mode = AppMode::ToolExecuting {
            name: "bash".into(),
        };
        app.live_tool = Some(LiveToolCall {
            id: "t1".into(),
            name: "bash".into(),
            input_summary: String::new(),
            status: ToolStatus::Running,
            output: None,
        });

        app.handle_agent_event(&AgentEvent::ToolResult {
            id: "t1".into(),
            name: "bash".into(),
            input: serde_json::json!({"command": "ls"}),
            output: "file1\nfile2".into(),
            is_error: false,
        });

        // Live tool cleared, tool card pushed to pending_inserts
        assert!(app.live_tool.is_none());
        assert!(!app.pending_inserts.is_empty());
        assert_eq!(app.mode, AppMode::Thinking);
    }

    #[test]
    fn handle_done_flushes_and_idles() {
        let mut app = App::new("test/model".into(), None);
        app.mode = AppMode::Thinking;
        app.streaming_buffer = "final text".into();

        app.handle_agent_event(&AgentEvent::Done {
            total_usage: Usage {
                input_tokens: 100,
                output_tokens: 50,
            },
        });

        assert_eq!(app.mode, AppMode::Idle);
        assert_eq!(app.streaming_buffer, "");
        assert!(!app.pending_inserts.is_empty()); // flushed text
        assert_eq!(app.usage_in, 100);
        assert_eq!(app.usage_out, 50);
    }

    #[test]
    fn handle_error_flushes_and_idles() {
        let mut app = App::new("test/model".into(), None);
        app.mode = AppMode::Thinking;
        app.streaming_buffer = "partial".into();

        app.handle_agent_event(&AgentEvent::Error("timeout".into()));

        assert_eq!(app.mode, AppMode::Idle);
        // Both partial text and error notice flushed to pending_inserts
        assert!(!app.pending_inserts.is_empty());
    }

    #[test]
    fn handle_context_compacted() {
        let mut app = App::new("test/model".into(), None);

        app.handle_agent_event(&AgentEvent::ContextCompacted {
            original_tokens: 10000,
            compacted_tokens: 5000,
            messages_dropped: 3,
        });

        assert!(!app.pending_inserts.is_empty());
    }

    #[test]
    fn handle_turn_complete_accumulates_usage() {
        let mut app = App::new("test/model".into(), None);

        app.handle_agent_event(&AgentEvent::TurnComplete {
            usage: Usage {
                input_tokens: 100,
                output_tokens: 50,
            },
        });
        app.handle_agent_event(&AgentEvent::TurnComplete {
            usage: Usage {
                input_tokens: 200,
                output_tokens: 100,
            },
        });

        assert_eq!(app.usage_in, 300);
        assert_eq!(app.usage_out, 150);
    }

    // ── Key event handling tests ──

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn key_ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn key_ctrl_c_quits() {
        let mut app = App::new("test/model".into(), None);
        assert_eq!(app.handle_key_event(key_ctrl('c')), Some(AppAction::Quit));
    }

    #[test]
    fn key_ctrl_c_quits_even_while_thinking() {
        let mut app = App::new("test/model".into(), None);
        app.mode = AppMode::Thinking;
        assert_eq!(app.handle_key_event(key_ctrl('c')), Some(AppAction::Quit));
    }

    #[test]
    fn key_enter_submits_text() {
        let mut app = App::new("test/model".into(), None);
        app.input.insert_char('h');
        app.input.insert_char('i');

        let action = app.handle_key_event(key(KeyCode::Enter));
        assert_eq!(action, Some(AppAction::Submit("hi".into())));
        assert!(app.input.is_empty());
        assert_eq!(app.mode, AppMode::Thinking);
        // User message pushed to pending_inserts
        assert!(!app.pending_inserts.is_empty());
    }

    #[test]
    fn key_enter_empty_does_nothing() {
        let mut app = App::new("test/model".into(), None);
        assert_eq!(app.handle_key_event(key(KeyCode::Enter)), None);
    }

    #[test]
    fn key_exit_command_quits() {
        let mut app = App::new("test/model".into(), None);
        for c in "/exit".chars() {
            app.input.insert_char(c);
        }
        assert_eq!(
            app.handle_key_event(key(KeyCode::Enter)),
            Some(AppAction::Quit)
        );
    }

    #[test]
    fn key_quit_command_quits() {
        let mut app = App::new("test/model".into(), None);
        for c in "/quit".chars() {
            app.input.insert_char(c);
        }
        assert_eq!(
            app.handle_key_event(key(KeyCode::Enter)),
            Some(AppAction::Quit)
        );
    }

    #[test]
    fn key_typing_while_thinking_ignored() {
        let mut app = App::new("test/model".into(), None);
        app.mode = AppMode::Thinking;
        assert_eq!(app.handle_key_event(key(KeyCode::Char('a'))), None);
        assert!(app.input.is_empty());
    }

    #[test]
    fn key_ctrl_j_inserts_newline() {
        let mut app = App::new("test/model".into(), None);
        app.input.insert_char('a');
        app.handle_key_event(key_ctrl('j'));
        app.input.insert_char('b');
        assert_eq!(app.input.text(), "a\nb");
    }

    #[test]
    fn key_ctrl_a_moves_home() {
        let mut app = App::new("test/model".into(), None);
        app.input.insert_char('a');
        app.input.insert_char('b');
        app.input.insert_char('c');
        assert_eq!(app.input.cursor_col, 3);
        app.handle_key_event(key_ctrl('a'));
        assert_eq!(app.input.cursor_col, 0);
    }

    #[test]
    fn key_ctrl_e_moves_end() {
        let mut app = App::new("test/model".into(), None);
        app.input.insert_char('a');
        app.input.insert_char('b');
        app.input.cursor_col = 0;
        app.handle_key_event(key_ctrl('e'));
        assert_eq!(app.input.cursor_col, 2);
    }

    #[test]
    fn key_ctrl_k_kills_to_end() {
        let mut app = App::new("test/model".into(), None);
        for c in "hello world".chars() {
            app.input.insert_char(c);
        }
        app.input.cursor_col = 5;
        app.handle_key_event(key_ctrl('k'));
        assert_eq!(app.input.text(), "hello");
        assert_eq!(app.input.cursor_col, 5);
    }

    #[test]
    fn key_ctrl_w_deletes_word_back() {
        let mut app = App::new("test/model".into(), None);
        for c in "hello world".chars() {
            app.input.insert_char(c);
        }
        app.handle_key_event(key_ctrl('w'));
        assert_eq!(app.input.text(), "hello ");
        assert_eq!(app.input.cursor_col, 6);
    }

    #[test]
    fn key_home_end_keys() {
        let mut app = App::new("test/model".into(), None);
        app.input.insert_char('x');
        app.input.insert_char('y');
        app.handle_key_event(key(KeyCode::Home));
        assert_eq!(app.input.cursor_col, 0);
        app.handle_key_event(key(KeyCode::End));
        assert_eq!(app.input.cursor_col, 2);
    }

    #[test]
    fn input_move_home_end() {
        let mut input = InputState::new();
        input.insert_char('a');
        input.insert_char('b');
        input.insert_char('c');
        input.move_home();
        assert_eq!(input.cursor_col, 0);
        input.move_end();
        assert_eq!(input.cursor_col, 3);
    }

    #[test]
    fn input_kill_to_end() {
        let mut input = InputState::new();
        for c in "abcdef".chars() {
            input.insert_char(c);
        }
        input.cursor_col = 3;
        input.kill_to_end();
        assert_eq!(input.text(), "abc");
    }

    #[test]
    fn input_delete_word_back() {
        let mut input = InputState::new();
        for c in "one two three".chars() {
            input.insert_char(c);
        }
        input.delete_word_back();
        assert_eq!(input.text(), "one two ");
        input.delete_word_back();
        assert_eq!(input.text(), "one ");
        input.delete_word_back();
        assert_eq!(input.text(), "");
    }

    #[test]
    fn input_delete_word_back_at_start() {
        let mut input = InputState::new();
        input.delete_word_back(); // should not panic
        assert_eq!(input.text(), "");
    }

    #[test]
    fn spinner_cycles() {
        let mut app = App::new("test/model".into(), None);
        let chars: Vec<char> = (0..8)
            .map(|_| {
                let c = app.spinner_char();
                app.tick();
                c
            })
            .collect();
        assert_eq!(chars, vec!['◐', '◓', '◑', '◒', '◐', '◓', '◑', '◒']);
    }
}
