use crate::model::message::{ContentBlock, Message};

/// Approximate token count for a string using chars/4 heuristic.
pub fn estimate_tokens(text: &str) -> u32 {
    (text.len() as u32).div_ceil(4)
}

/// Estimate tokens for a single Message (sum of all content blocks).
pub fn estimate_message_tokens(msg: &Message) -> u32 {
    let mut tokens = 0u32;
    for block in &msg.content {
        tokens += match block {
            ContentBlock::Text { text } => estimate_tokens(text),
            ContentBlock::ToolUse { id, name, input } => {
                estimate_tokens(id) + estimate_tokens(name) + estimate_tokens(&input.to_string())
            }
            ContentBlock::ToolResult { content, .. } => estimate_tokens(content),
        };
    }
    // Overhead per message (role, structure)
    tokens + 4
}

/// Estimate total tokens for a conversation context.
pub fn estimate_context_tokens(system_prompt: &str, messages: &[Message]) -> u32 {
    let system_tokens = estimate_tokens(system_prompt);
    let message_tokens: u32 = messages.iter().map(estimate_message_tokens).sum();
    system_tokens + message_tokens
}

/// Limits for context window management.
pub struct ContextLimits {
    /// Maximum context tokens for the model (e.g. 200_000 for Claude).
    pub max_context_tokens: u32,
    /// Compact when token usage exceeds this fraction of max (e.g. 0.8).
    pub compaction_threshold: f32,
}

impl ContextLimits {
    pub fn new(max_context_tokens: u32) -> Self {
        Self {
            max_context_tokens,
            compaction_threshold: 0.8,
        }
    }

    /// The token count that triggers compaction.
    pub fn threshold_tokens(&self) -> u32 {
        (self.max_context_tokens as f32 * self.compaction_threshold) as u32
    }
}

impl Default for ContextLimits {
    fn default() -> Self {
        Self::new(200_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::message::Role;

    #[test]
    fn test_estimate_tokens_ascii() {
        // "hello world" = 11 bytes -> (11+3)/4 = 3
        assert_eq!(estimate_tokens("hello world"), 3);
    }

    #[test]
    fn test_estimate_tokens_empty() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_estimate_tokens_long() {
        let text = "a".repeat(400);
        assert_eq!(estimate_tokens(&text), 100);
    }

    #[test]
    fn test_estimate_message_tokens() {
        let msg = Message {
            role: Role::User,
            content: vec![
                ContentBlock::Text {
                    text: "hello".to_string(),
                },
                ContentBlock::Text {
                    text: "world".to_string(),
                },
            ],
        };
        let tokens = estimate_message_tokens(&msg);
        // "hello" = (5+3)/4=2, "world" = (5+3)/4=2, overhead=4 -> 8
        assert_eq!(tokens, 8);
    }

    #[test]
    fn test_estimate_context_tokens() {
        let system = "You are an AI assistant.";
        let messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Hello".to_string(),
            }],
        }];
        let total = estimate_context_tokens(system, &messages);
        // system: (24+3)/4=6, message: (5+3)/4=2 + 4 overhead = 6 -> total 12
        assert_eq!(total, 12);
    }

    #[test]
    fn test_context_limits_threshold() {
        let limits = ContextLimits::new(200_000);
        assert_eq!(limits.threshold_tokens(), 160_000);
    }
}
