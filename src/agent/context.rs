use crate::agent::compaction::compact_messages;
use crate::agent::token::{estimate_context_tokens, ContextLimits};
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

    /// Get a reference to the current message history.
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Restore messages from a previous session.
    pub fn restore_messages(&mut self, messages: Vec<Message>) {
        self.messages = messages;
    }

    /// Check if compaction is needed given the context limits.
    pub fn needs_compaction(&self, limits: &ContextLimits) -> bool {
        let tokens = estimate_context_tokens(&self.system_prompt, &self.messages);
        tokens > limits.threshold_tokens()
    }

    /// Compact the conversation to fit within context limits.
    /// Returns the number of messages dropped.
    pub fn compact(&mut self, limits: &ContextLimits) -> usize {
        let target = limits.threshold_tokens();
        let original_count = self.messages.len();
        self.messages = compact_messages(&self.messages, target, 2);
        let new_count = self.messages.len();
        if new_count < original_count {
            original_count - new_count
        } else {
            0
        }
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
