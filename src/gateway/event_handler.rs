use std::time::{Duration, Instant};

use crate::agent::event::AgentEvent;
use crate::cli::repl::format_tool_input;

/// Converts AgentEvents to displayable text for bot platforms (Slack/Discord).
/// Includes rate limiting to avoid excessive message updates.
pub struct BotEventHandler {
    full_text: String,
    tool_status: Option<String>,
    min_interval: Duration,
    min_chars: usize,
    last_update: Instant,
    dirty: bool,
}

impl BotEventHandler {
    pub fn new(min_interval: Duration) -> Self {
        Self {
            full_text: String::new(),
            tool_status: None,
            min_interval,
            min_chars: 50,
            last_update: Instant::now() - min_interval, // allow immediate first update
            dirty: false,
        }
    }

    /// Process an event. Returns `Some(text)` when the bot message should be updated.
    pub fn handle_event(&mut self, event: &AgentEvent) -> Option<String> {
        match event {
            AgentEvent::TextDelta(text) => {
                self.full_text.push_str(text);
                self.dirty = true;
                if self.last_update.elapsed() >= self.min_interval
                    && self.full_text.len() >= self.min_chars
                {
                    self.last_update = Instant::now();
                    self.dirty = false;
                    Some(self.current_display())
                } else {
                    None
                }
            }
            AgentEvent::ToolCallStart { name, .. } => {
                self.tool_status = Some(format!("\n\u{2699}\u{fe0f} {name}..."));
                self.dirty = true;
                if self.last_update.elapsed() >= self.min_interval {
                    self.last_update = Instant::now();
                    self.dirty = false;
                    Some(self.current_display())
                } else {
                    None
                }
            }
            AgentEvent::ToolResult {
                name,
                input,
                is_error,
                ..
            } => {
                let status_char = if *is_error { '\u{274c}' } else { '\u{2705}' };
                let input_display = format_tool_input(name, input);
                self.tool_status = None;
                self.full_text
                    .push_str(&format!("\n{status_char} {name}: {input_display}"));
                self.dirty = true;
                self.last_update = Instant::now();
                self.dirty = false;
                Some(self.current_display())
            }
            AgentEvent::Done { total_usage } => {
                self.full_text.push_str(&format!(
                    "\n\n_tokens: {} in / {} out_",
                    total_usage.input_tokens, total_usage.output_tokens
                ));
                self.dirty = false;
                Some(self.current_display())
            }
            AgentEvent::Error(msg) => {
                self.full_text
                    .push_str(&format!("\n\n```\nError: {msg}\n```"));
                self.dirty = false;
                Some(self.current_display())
            }
            _ => None,
        }
    }

    /// Return the final complete text (call after agent run completes).
    pub fn finalize(&mut self) -> String {
        self.dirty = false;
        self.current_display()
    }

    fn current_display(&self) -> String {
        let mut text = self.full_text.clone();
        if let Some(ref status) = self.tool_status {
            text.push_str(status);
        }
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::response::Usage;

    #[test]
    fn handler_accumulates_text() {
        let mut handler = BotEventHandler::new(Duration::from_millis(0));
        handler.min_chars = 0;

        let result = handler.handle_event(&AgentEvent::TextDelta("Hello ".into()));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Hello ");

        let result = handler.handle_event(&AgentEvent::TextDelta("world".into()));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "Hello world");
    }

    #[test]
    fn handler_throttles_rapid_updates() {
        let mut handler = BotEventHandler::new(Duration::from_secs(10));
        handler.min_chars = 0;

        // First update succeeds (initial last_update is in the past)
        let result = handler.handle_event(&AgentEvent::TextDelta("Hello".into()));
        assert!(result.is_some());

        // Second update within interval → throttled
        let result = handler.handle_event(&AgentEvent::TextDelta(" world".into()));
        assert!(result.is_none());
    }

    #[test]
    fn handler_shows_tool_status() {
        let mut handler = BotEventHandler::new(Duration::from_millis(0));
        handler.min_chars = 0;

        handler.handle_event(&AgentEvent::TextDelta("thinking...".into()));
        let result = handler.handle_event(&AgentEvent::ToolCallStart {
            id: "t1".into(),
            name: "bash".into(),
        });
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("\u{2699}\u{fe0f} bash..."));
    }

    #[test]
    fn handler_replaces_tool_status_on_result() {
        let mut handler = BotEventHandler::new(Duration::from_millis(0));
        handler.min_chars = 0;

        handler.handle_event(&AgentEvent::ToolCallStart {
            id: "t1".into(),
            name: "bash".into(),
        });

        let result = handler.handle_event(&AgentEvent::ToolResult {
            id: "t1".into(),
            name: "bash".into(),
            input: serde_json::json!({"command": "ls"}),
            output: "file.txt".into(),
            is_error: false,
        });
        assert!(result.is_some());
        let text = result.unwrap();
        // Tool status line should be gone
        assert!(!text.contains("\u{2699}\u{fe0f}"));
        // Result line should be present
        assert!(text.contains("\u{2705} bash: $ ls"));
    }

    #[test]
    fn handler_finalize_returns_complete_text() {
        let mut handler = BotEventHandler::new(Duration::from_secs(10));
        handler.min_chars = 0;

        // First goes through
        handler.handle_event(&AgentEvent::TextDelta("Hello".into()));
        // This is throttled
        handler.handle_event(&AgentEvent::TextDelta(" world".into()));

        let final_text = handler.finalize();
        assert_eq!(final_text, "Hello world");
    }

    #[test]
    fn handler_done_shows_usage() {
        let mut handler = BotEventHandler::new(Duration::from_millis(0));
        handler.min_chars = 0;

        handler.handle_event(&AgentEvent::TextDelta("Answer".into()));
        let result = handler.handle_event(&AgentEvent::Done {
            total_usage: Usage {
                input_tokens: 100,
                output_tokens: 50,
            },
        });
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("tokens: 100 in / 50 out"));
    }
}
