use std::sync::Arc;

use futures_util::StreamExt;

use crate::agent::context::ConversationContext;
use crate::agent::event::AgentEvent;
use crate::model::message::ContentBlock;
use crate::model::response::{StopReason, StreamEvent, Usage};
use crate::provider::traits::Provider;
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
        }
    }

    /// Run the agent loop for a user prompt.
    /// Calls the callback for each event so the UI can render in real-time.
    pub async fn run(
        &mut self,
        user_input: String,
        mut on_event: impl FnMut(&AgentEvent),
    ) {
        self.context.push_user_message(user_input);

        let mut total_usage = Usage::default();

        for _turn in 0..self.max_turns {
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

            self.context.push_assistant_message(assistant_blocks);

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
                        Some(tool) => tool.execute(input).await,
                        None => crate::tool::traits::ToolOutput {
                            content: format!("Unknown tool: {}", tc.name),
                            is_error: true,
                        },
                    };

                    on_event(&AgentEvent::ToolResult {
                        id: tc.id.clone(),
                        name: tc.name.clone(),
                        output: output.content.clone(),
                        is_error: output.is_error,
                    });

                    tool_results.push(ContentBlock::ToolResult {
                        tool_use_id: tc.id.clone(),
                        content: output.content,
                        is_error: output.is_error,
                    });
                }

                self.context.push_tool_results(tool_results);
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
