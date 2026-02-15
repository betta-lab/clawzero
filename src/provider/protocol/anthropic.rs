use std::collections::HashMap;

use futures_util::StreamExt;
use serde_json::json;

use crate::error::ClawError;
use crate::model::message::{ContentBlock, Role};
use crate::model::request::CompletionRequest;
use crate::model::response::{StopReason, StreamEvent, Usage};
use crate::provider::http::{build_http_client, parse_sse_stream};
use crate::provider::traits::{EventStream, Provider};

pub struct AnthropicProtocol {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    extra_headers: HashMap<String, String>,
}

impl AnthropicProtocol {
    pub fn new(
        base_url: String,
        api_key: String,
        extra_headers: HashMap<String, String>,
    ) -> Self {
        Self {
            client: build_http_client(),
            base_url,
            api_key,
            extra_headers,
        }
    }

    fn build_body(&self, request: &CompletionRequest) -> serde_json::Value {
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|msg| {
                let role = match msg.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                };
                let content: Vec<serde_json::Value> = msg
                    .content
                    .iter()
                    .map(|block| match block {
                        ContentBlock::Text { text } => json!({
                            "type": "text",
                            "text": text,
                        }),
                        ContentBlock::ToolUse { id, name, input } => json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input,
                        }),
                        ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } => json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": content,
                            "is_error": is_error,
                        }),
                    })
                    .collect();
                json!({
                    "role": role,
                    "content": content,
                })
            })
            .collect();

        let mut body = json!({
            "model": request.model,
            "messages": messages,
            "max_tokens": request.max_tokens,
            "stream": true,
        });

        if let Some(system) = &request.system {
            body["system"] = json!(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = json!(temp);
        }

        if !request.stop_sequences.is_empty() {
            body["stop_sequences"] = json!(request.stop_sequences);
        }

        if !request.tools.is_empty() {
            let tools: Vec<serde_json::Value> = request
                .tools
                .iter()
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.input_schema,
                    })
                })
                .collect();
            body["tools"] = json!(tools);
        }

        body
    }
}

impl Provider for AnthropicProtocol {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn complete<'a>(
        &'a self,
        request: &'a CompletionRequest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<EventStream, ClawError>> + Send + 'a>> {
        Box::pin(async move {
            let url = format!("{}/v1/messages", self.base_url);
            let body = self.build_body(request);

            let mut req = self
                .client
                .post(&url)
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&body);

            for (key, value) in &self.extra_headers {
                req = req.header(key.as_str(), value.as_str());
            }

            let response = req.send().await?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body = response.text().await.unwrap_or_default();

                if status == 429 {
                    return Err(ClawError::RateLimited {
                        retry_after_ms: 1000,
                    });
                }
                if status == 529 {
                    return Err(ClawError::Overloaded(body));
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
                    Ok(sse) => parse_anthropic_event(&sse.event_type, &sse.data),
                }
            });

            Ok(Box::pin(mapped) as EventStream)
        })
    }
}

fn parse_anthropic_event(
    event_type: &str,
    data: &str,
) -> Option<Result<StreamEvent, ClawError>> {
    let parsed: serde_json::Value = match serde_json::from_str(data) {
        Ok(v) => v,
        Err(e) => return Some(Err(ClawError::Json(e))),
    };

    match event_type {
        "message_start" => {
            let msg = &parsed["message"];
            Some(Ok(StreamEvent::MessageStart {
                id: msg["id"].as_str().unwrap_or("").to_string(),
                model: msg["model"].as_str().unwrap_or("").to_string(),
            }))
        }
        "content_block_start" => {
            let block = &parsed["content_block"];
            let block_type = block["type"].as_str().unwrap_or("");
            match block_type {
                "tool_use" => Some(Ok(StreamEvent::ToolUseStart {
                    id: block["id"].as_str().unwrap_or("").to_string(),
                    name: block["name"].as_str().unwrap_or("").to_string(),
                })),
                _ => None, // text blocks start implicitly via deltas
            }
        }
        "content_block_delta" => {
            let delta = &parsed["delta"];
            let delta_type = delta["type"].as_str().unwrap_or("");
            match delta_type {
                "text_delta" => Some(Ok(StreamEvent::TextDelta {
                    text: delta["text"].as_str().unwrap_or("").to_string(),
                })),
                "input_json_delta" => Some(Ok(StreamEvent::ToolInputDelta {
                    partial_json: delta["partial_json"].as_str().unwrap_or("").to_string(),
                })),
                _ => None,
            }
        }
        "content_block_stop" => Some(Ok(StreamEvent::ToolUseEnd)),
        "message_delta" => {
            let delta = &parsed["delta"];
            let stop_reason = match delta["stop_reason"].as_str() {
                Some("end_turn") => StopReason::EndTurn,
                Some("tool_use") => StopReason::ToolUse,
                Some("max_tokens") => StopReason::MaxTokens,
                Some("stop_sequence") => StopReason::StopSequence,
                _ => StopReason::EndTurn,
            };
            let usage_obj = &parsed["usage"];
            let usage = Usage {
                input_tokens: usage_obj["input_tokens"].as_u64().unwrap_or(0) as u32,
                output_tokens: usage_obj["output_tokens"].as_u64().unwrap_or(0) as u32,
            };
            Some(Ok(StreamEvent::MessageEnd { stop_reason, usage }))
        }
        "message_stop" | "ping" => None,
        "error" => {
            let msg = parsed["error"]["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();
            Some(Err(ClawError::ProviderError {
                status: 0,
                message: msg,
            }))
        }
        _ => None,
    }
}
