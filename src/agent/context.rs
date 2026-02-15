use crate::model::message::{ContentBlock, Message, Role};
use crate::model::request::CompletionRequest;
use crate::model::tool_schema::ToolDefinition;

/// Manages conversation history and builds completion requests.
pub struct ConversationContext {
    system_prompt: String,
    messages: Vec<Message>,
    max_tokens: u32,
}

impl ConversationContext {
    pub fn new(system_prompt: String, max_tokens: u32) -> Self {
        Self {
            system_prompt,
            messages: Vec::new(),
            max_tokens,
        }
    }

    pub fn push_user_message(&mut self, text: String) {
        self.messages.push(Message {
            role: Role::User,
            content: vec![ContentBlock::Text { text }],
        });
    }

    pub fn push_assistant_message(&mut self, blocks: Vec<ContentBlock>) {
        self.messages.push(Message {
            role: Role::Assistant,
            content: blocks,
        });
    }

    pub fn push_tool_results(&mut self, results: Vec<ContentBlock>) {
        self.messages.push(Message {
            role: Role::User,
            content: results,
        });
    }

    pub fn build_request(&self, model: &str, tools: &[ToolDefinition]) -> CompletionRequest {
        CompletionRequest {
            model: model.to_string(),
            system: Some(self.system_prompt.clone()),
            messages: self.messages.clone(),
            tools: tools.to_vec(),
            max_tokens: self.max_tokens,
            temperature: None,
            stop_sequences: Vec::new(),
        }
    }
}
