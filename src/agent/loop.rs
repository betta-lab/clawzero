use std::sync::Arc;

use futures_util::StreamExt;

use crate::agent::context::ConversationContext;
use crate::agent::event::AgentEvent;
use crate::agent::token::{estimate_context_tokens, ContextLimits};
use crate::model::message::{ContentBlock, Message, Role};
use crate::model::response::{StopReason, StreamEvent, Usage};
use crate::provider::traits::Provider;
use crate::session::store::SessionWriter;
use crate::session::types::SessionEntry;
use crate::tool::traits::ToolRegistry;

/// Pending tool call accumulated from streaming events.
struct PendingToolCall {
    id: String,
    name: String,
    input_json: String,
}

pub struct Agent {
    provider: Arc<dyn Provider>,
    model: String,
    tool_registry: ToolRegistry,
    context: ConversationContext,
    max_turns: usize,
    session_writer: Option<SessionWriter>,
    context_limits: ContextLimits,
}

impl Agent {
    pub fn new(
        provider: Arc<dyn Provider>,
        model: String,
        tool_registry: ToolRegistry,
        context: ConversationContext,
        max_turns: usize,
    ) -> Self {
        Self {
            provider,
            model,
            tool_registry,
            context,
            max_turns,
            session_writer: None,
            context_limits: ContextLimits::default(),
        }
    }

    /// Set context limits for automatic compaction.
    pub fn set_context_limits(&mut self, limits: ContextLimits) {
        self.context_limits = limits;
    }

    /// Set a session writer for auto-saving conversation history.
    pub fn set_session_writer(&mut self, writer: SessionWriter) {
        self.session_writer = Some(writer);
    }

    /// Get the session ID if a session is active.
    pub fn session_id(&self) -> Option<&str> {
        self.session_writer.as_ref().map(|w| w.session_id())
    }

    /// Save a message to the session file if a writer is active.
    fn save_message(&mut self, message: &Message) {
        if let Some(ref mut writer) = self.session_writer {
            let entry = SessionEntry::Message {
                message: message.clone(),
            };
            if let Err(e) = writer.append(&entry) {
                tracing::warn!("Failed to save session entry: {e}");
            }
        }
    }

    /// Save usage stats to the session file.
    fn save_usage(&mut self, usage: &Usage) {
        if let Some(ref mut writer) = self.session_writer {
            let entry = SessionEntry::Usage {
                input_tokens: usage.input_tokens,
                output_tokens: usage.output_tokens,
            };
            if let Err(e) = writer.append(&entry) {
                tracing::warn!("Failed to save usage entry: {e}");
            }
        }
    }

    /// Run the agent loop for a user prompt.
    /// Calls the callback for each event so the UI can render in real-time.
    pub async fn run(
        &mut self,
        user_input: String,
        mut on_event: impl FnMut(&AgentEvent),
    ) {
        self.context.push_user_message(user_input.clone());

        // Save user message to session
        self.save_message(&Message {
            role: Role::User,
            content: vec![ContentBlock::Text { text: user_input }],
        });

        let mut total_usage = Usage::default();

        for _turn in 0..self.max_turns {
            // Auto-compact context if approaching limits
            if self.context.needs_compaction(&self.context_limits) {
                let original_tokens = estimate_context_tokens(
                    "",
                    self.context.messages(),
                );
                let messages_dropped = self.context.compact(&self.context_limits);
                if messages_dropped > 0 {
                    let compacted_tokens = estimate_context_tokens(
                        "",
                        self.context.messages(),
                    );
                    on_event(&AgentEvent::ContextCompacted {
                        original_tokens,
                        compacted_tokens,
                        messages_dropped,
                    });
                }
            }

            let request = self
                .context
                .build_request(&self.model, &self.tool_registry.definitions());

            let event_stream = match self.provider.complete(&request).await {
                Ok(s) => s,
                Err(e) => {
                    on_event(&AgentEvent::Error(e.to_string()));
                    return;
                }
            };

            let mut assistant_blocks: Vec<ContentBlock> = Vec::new();
            let mut pending_tool_calls: Vec<PendingToolCall> = Vec::new();
            let mut current_text = String::new();
            let mut current_tool: Option<PendingToolCall> = None;
            let mut _stop_reason = StopReason::EndTurn;

            futures_util::pin_mut!(event_stream);

            while let Some(event) = event_stream.next().await {
                match event {
                    Err(e) => {
                        on_event(&AgentEvent::Error(e.to_string()));
                        return;
                    }
                    Ok(StreamEvent::MessageStart { .. }) => {}
                    Ok(StreamEvent::TextDelta { text }) => {
                        on_event(&AgentEvent::TextDelta(text.clone()));
                        current_text.push_str(&text);
                    }
                    Ok(StreamEvent::ToolUseStart { id, name }) => {
                        // Flush any accumulated text
                        if !current_text.is_empty() {
                            assistant_blocks.push(ContentBlock::Text {
                                text: std::mem::take(&mut current_text),
                            });
                        }
                        on_event(&AgentEvent::ToolCallStart {
                            id: id.clone(),
                            name: name.clone(),
                        });
                        current_tool = Some(PendingToolCall {
                            id,
                            name,
                            input_json: String::new(),
                        });
                    }
                    Ok(StreamEvent::ToolInputDelta { partial_json }) => {
                        if let Some(ref mut tool) = current_tool {
                            tool.input_json.push_str(&partial_json);
                        }
                    }
                    Ok(StreamEvent::ToolUseEnd) => {
                        if let Some(tool) = current_tool.take() {
                            // Parse the accumulated JSON
                            let input: serde_json::Value =
                                serde_json::from_str(&tool.input_json).unwrap_or_default();

                            assistant_blocks.push(ContentBlock::ToolUse {
                                id: tool.id.clone(),
                                name: tool.name.clone(),
                                input: input.clone(),
                            });

                            pending_tool_calls.push(PendingToolCall {
                                id: tool.id,
                                name: tool.name,
                                input_json: tool.input_json,
                            });
                        }
                    }
                    Ok(StreamEvent::MessageEnd {
                        stop_reason: sr,
                        usage,
                    }) => {
                        _stop_reason = sr;
                        total_usage.input_tokens += usage.input_tokens;
                        total_usage.output_tokens += usage.output_tokens;
                        self.save_usage(&usage);
                        on_event(&AgentEvent::TurnComplete { usage });
                    }
                }
            }

            // Flush remaining text
            if !current_text.is_empty() {
                assistant_blocks.push(ContentBlock::Text {
                    text: current_text,
                });
            }

            // Flush any remaining tool (shouldn't happen but defensive)
            if let Some(tool) = current_tool.take() {
                let input: serde_json::Value =
                    serde_json::from_str(&tool.input_json).unwrap_or_default();
                assistant_blocks.push(ContentBlock::ToolUse {
                    id: tool.id.clone(),
                    name: tool.name.clone(),
                    input,
                });
                pending_tool_calls.push(tool);
            }

            self.context
                .push_assistant_message(assistant_blocks.clone());

            // Save assistant message to session
            self.save_message(&Message {
                role: Role::Assistant,
                content: assistant_blocks,
            });

            // Execute tools if needed
            // Note: check pending_tool_calls directly rather than relying solely on
            // StopReason, because some providers (OpenAI-compatible) may send
            // finish_reason and usage in separate chunks, causing StopReason to be
            // EndTurn even when tool calls were requested.
            if !pending_tool_calls.is_empty() {
                let mut tool_results = Vec::new();

                for tc in &pending_tool_calls {
                    let input: serde_json::Value =
                        serde_json::from_str(&tc.input_json).unwrap_or_default();

                    let output = match self.tool_registry.get(&tc.name) {
                        Some(tool) => tool.execute(input.clone()).await,
                        None => crate::tool::traits::ToolOutput {
                            content: format!("Unknown tool: {}", tc.name),
                            is_error: true,
                        },
                    };

                    on_event(&AgentEvent::ToolResult {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        input,
                        output: output.content.clone(),
                        is_error: output.is_error,
                    });

                    tool_results.push(ContentBlock::ToolResult {
                        tool_use_id: tc.id.clone(),
                        content: output.content,
                        is_error: output.is_error,
                    });
                }

                self.context.push_tool_results(tool_results.clone());

                // Save tool results to session
                self.save_message(&Message {
                    role: Role::User,
                    content: tool_results,
                });
                // Continue loop -> next model call
            } else {
                // Done
                on_event(&AgentEvent::Done {
                    total_usage: total_usage.clone(),
                });
                return;
            }
        }

        on_event(&AgentEvent::Error(format!(
            "Agent exceeded maximum turns ({})",
            self.max_turns
        )));
    }
}
