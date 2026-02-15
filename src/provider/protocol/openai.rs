use std::collections::HashMap;

use futures_util::StreamExt;
use serde_json::json;

use crate::error::ClawError;
use crate::model::message::{ContentBlock, Role};
use crate::model::request::CompletionRequest;
use crate::model::response::{StopReason, StreamEvent, Usage};
use crate::provider::http::{build_http_client, parse_sse_stream};
use crate::provider::traits::{EventStream, Provider};

pub struct OpenAiProtocol {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    extra_headers: HashMap<String, String>,
    provider_name: String,
}

impl OpenAiProtocol {
    pub fn new(
        provider_name: String,
        base_url: String,
        api_key: String,
        extra_headers: HashMap<String, String>,
    ) -> Self {
        Self {
            client: build_http_client(),
            base_url,
            api_key,
            extra_headers,
            provider_name,
        }
    }

    fn build_body(&self, request: &CompletionRequest) -> serde_json::Value {
        let mut messages: Vec<serde_json::Value> = Vec::new();

        // System message
        if let Some(system) = &request.system {
            messages.push(json!({
                "role": "system",
                "content": system,
            }));
        }

        // Conversation messages
        for msg in &request.messages {
            match msg.role {
                Role::User => {
                    // Check if this is a tool results message
                    let has_tool_results = msg
                        .content
                        .iter()
                        .any(|b| matches!(b, ContentBlock::ToolResult { .. }));

                    if has_tool_results {
                        // Each tool result becomes a separate "tool" role message
                        for block in &msg.content {
                            if let ContentBlock::ToolResult {
                                tool_use_id,
                                content,
                                ..
                            } = block
                            {
                                messages.push(json!({
                                    "role": "tool",
                                    "tool_call_id": tool_use_id,
                                    "content": content,
                                }));
                            }
                        }
                    } else {
                        // Regular user message
                        let text: String = msg
                            .content
                            .iter()
                            .filter_map(|b| match b {
                                ContentBlock::Text { text } => Some(text.as_str()),
                                _ => None,
                            })
                            .collect::<Vec<_>>()
                            .join("");
                        messages.push(json!({
                            "role": "user",
                            "content": text,
                        }));
                    }
                }
                Role::Assistant => {
                    let mut content_text = String::new();
                    let mut tool_calls: Vec<serde_json::Value> = Vec::new();

                    for block in &msg.content {
                        match block {
                            ContentBlock::Text { text } => {
                                content_text.push_str(text);
                            }
                            ContentBlock::ToolUse { id, name, input } => {
                                tool_calls.push(json!({
                                    "id": id,
                                    "type": "function",
                                    "function": {
                                        "name": name,
                                        "arguments": input.to_string(),
                                    },
                                }));
                            }
                            _ => {}
                        }
                    }

                    let mut assistant_msg = json!({ "role": "assistant" });
                    if !content_text.is_empty() {
                        assistant_msg["content"] = json!(content_text);
                    }
                    if !tool_calls.is_empty() {
                        assistant_msg["tool_calls"] = json!(tool_calls);
                    }
                    messages.push(assistant_msg);
                }
            }
        }

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "stream": true,
            "stream_options": { "include_usage": true },
        });

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        if !request.stop_sequences.is_empty() {
            body["stop"] = json!(request.stop_sequences);
        }

        if !request.tools.is_empty() {
            let tools: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.input_schema,
                        },
                    })
                })
                .collect();
            body["tools"] = json!(tools);
        }

        body
    }
}

impl Provider for OpenAiProtocol {
    fn name(&self) -> &str {
        &self.provider_name
    }

    fn complete<'a>(
        &'a self,
        request: &'a CompletionRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<EventStream, ClawError>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!("{}/v1/chat/completions", self.base_url);
            let body = self.build_body(request);

            let mut req = self
                .client
                .post(&url)
                .header("content-type", "application/json");

            if !self.api_key.is_empty() {
                req = req.header("authorization", format!("Bearer {}", self.api_key));
            }

            for (key, value) in &self.extra_headers {
                req = req.header(key.as_str(), value.as_str());
            }

            let response = req.json(&body).send().await?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();

                if status == 429 {
                    return Err(ClawError::RateLimited {
                        retry_after_ms: 1000,
                    });
                }
                return Err(ClawError::ProviderError {
                    status,
                    message: body,
                });
            }

            let stream = parse_sse_stream(response);

            let mapped = stream.filter_map(|result| async move {
                match result {
                    Err(e) => Some(Err(e)),
                    Ok(sse) => {
                        if sse.data.trim() == "[DONE]" {
                            return None;
                        }
                        parse_openai_chunk(&sse.data)
                    }
                }
            });

            Ok(Box::pin(mapped) as EventStream)
        })
    }
}

fn parse_openai_chunk(data: &str) -> Option<Result<StreamEvent, ClawError>> {
    let parsed: serde_json::Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(e) => return Some(Err(ClawError::Json(e))),
    };

    // Check for usage-only chunk (final chunk with stream_options.include_usage)
    if let Some(usage) = parsed.get("usage") {
        if !usage.is_null() {
            // Determine stop reason from choices if present
            let stop_reason = parsed["choices"]
                .get(0)
                .and_then(|c| c["finish_reason"].as_str())
                .map(|r| match r {
                    "stop" => StopReason::EndTurn,
                    "tool_calls" => StopReason::ToolUse,
                    "length" => StopReason::MaxTokens,
                    _ => StopReason::EndTurn,
                })
                .unwrap_or(StopReason::EndTurn);

            return Some(Ok(StreamEvent::MessageEnd {
                stop_reason,
                usage: Usage {
                    input_tokens: usage["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                    output_tokens: usage["completion_tokens"].as_u64().unwrap_or(0) as u32,
                },
            }));
        }
    }

    let choice = parsed["choices"].get(0)?;
    let delta = &choice["delta"];
    let finish_reason = choice["finish_reason"].as_str();

    // Handle finish reason without usage (emit MessageEnd only if no usage chunk expected)
    if let Some(reason) = finish_reason {
        let stop_reason = match reason {
            "stop" => StopReason::EndTurn,
            "tool_calls" => StopReason::ToolUse,
            "length" => StopReason::MaxTokens,
            _ => StopReason::EndTurn,
        };
        // If this is a tool_calls finish, emit ToolUseEnd
        if stop_reason == StopReason::ToolUse {
            return Some(Ok(StreamEvent::ToolUseEnd));
        }
        // For other finishes, we'll get the usage chunk separately
        return None;
    }

    // Text content delta
    if let Some(content) = delta["content"].as_str() {
        if !content.is_empty() {
            return Some(Ok(StreamEvent::TextDelta {
                text: content.to_string(),
            }));
        }
    }

    // Tool calls
    if let Some(tool_calls) = delta.get("tool_calls") {
        if let Some(tc) = tool_calls.get(0) {
            // Tool call with id = new tool call start
            if let Some(id) = tc["id"].as_str() {
                let name = tc["function"]["name"].as_str().unwrap_or("").to_string();
                return Some(Ok(StreamEvent::ToolUseStart {
                    id: id.to_string(),
                    name,
                }));
            }
            // Tool call with arguments delta
            if let Some(args) = tc["function"]["arguments"].as_str() {
                if !args.is_empty() {
                    return Some(Ok(StreamEvent::ToolInputDelta {
                        partial_json: args.to_string(),
                    }));
                }
            }
        }
    }

    // First chunk often has model info
    if let Some(model) = parsed["model"].as_str() {
        if delta.is_object() && delta.as_object().is_some_and(|o| o.is_empty()) {
            return Some(Ok(StreamEvent::MessageStart {
                id: parsed["id"].as_str().unwrap_or("").to_string(),
                model: model.to_string(),
            }));
        }
    }

    None
}
