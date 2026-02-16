use crate::agent::token::estimate_message_tokens;
use crate::model::message::{ContentBlock, Message, Role};

/// Compact messages by dropping the oldest messages to fit within the target token count.
///
/// Rules:
/// - Always preserves at least the last `min_keep_pairs` user-assistant exchanges.
/// - ToolUse and ToolResult messages are kept as pairs (never orphaned).
/// - Prepends a truncation notice when messages are dropped.
pub fn compact_messages(
    messages: &[Message],
    target_tokens: u32,
    min_keep_pairs: usize,
) -> Vec<Message> {
    if messages.is_empty() {
        return Vec::new();
    }

    let total: u32 = messages.iter().map(|m| estimate_message_tokens(m)).sum();
    if total <= target_tokens {
        return messages.to_vec();
    }

    // Work backwards, accumulating messages until we exceed the target
    let mut kept: Vec<&Message> = Vec::new();
    let mut kept_tokens = 0u32;
    let mut pairs_kept = 0usize;

    // Walk from the end
    let mut i = messages.len();
    while i > 0 {
        i -= 1;
        let msg = &messages[i];
        let msg_tokens = estimate_message_tokens(msg);

        // Check if this message is a tool result (User role with ToolResult content)
        let is_tool_result = msg.role == Role::User
            && msg
                .content
                .iter()
                .any(|b| matches!(b, ContentBlock::ToolResult { .. }));

        if is_tool_result {
            // Must also keep the preceding assistant message (contains ToolUse)
            // First, keep this tool result
            kept.push(msg);
            kept_tokens += msg_tokens;

            // Look for the preceding assistant message with ToolUse
            if i > 0 {
                i -= 1;
                let prev = &messages[i];
                kept.push(prev);
                kept_tokens += estimate_message_tokens(prev);
            }
            continue;
        }

        // For regular user/assistant messages, count pairs
        if msg.role == Role::User {
            pairs_kept += 1;
        }

        // Check if we'd exceed target (but always keep min_keep_pairs)
        if kept_tokens + msg_tokens > target_tokens && pairs_kept > min_keep_pairs {
            break;
        }

        kept.push(msg);
        kept_tokens += msg_tokens;
    }

    // Reverse since we collected from end
    kept.reverse();

    let mut result: Vec<Message> = Vec::new();

    // Add truncation notice if we dropped messages
    if kept.len() < messages.len() {
        let dropped = messages.len() - kept.len();
        result.push(Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: format!(
                    "[Earlier conversation was truncated. {dropped} messages were removed to fit context window.]"
                ),
            }],
        });
    }

    result.extend(kept.into_iter().cloned());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::message::{ContentBlock, Message, Role};

    fn text_msg(role: Role, text: &str) -> Message {
        Message {
            role,
            content: vec![ContentBlock::Text {
                text: text.to_string(),
            }],
        }
    }

    fn tool_use_msg(id: &str, name: &str) -> Message {
        Message {
            role: Role::Assistant,
            content: vec![ContentBlock::ToolUse {
                id: id.to_string(),
                name: name.to_string(),
                input: serde_json::json!({}),
            }],
        }
    }

    fn tool_result_msg(id: &str, content: &str) -> Message {
        Message {
            role: Role::User,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: id.to_string(),
                content: content.to_string(),
                is_error: false,
            }],
        }
    }

    #[test]
    fn test_compact_already_small() {
        let messages = vec![
            text_msg(Role::User, "hello"),
            text_msg(Role::Assistant, "hi"),
        ];
        let result = compact_messages(&messages, 10000, 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_drop_oldest_basic() {
        // Create many messages so total exceeds target
        let mut messages = Vec::new();
        for i in 0..20 {
            messages.push(text_msg(Role::User, &"x".repeat(100)));
            messages.push(text_msg(Role::Assistant, &format!("response {i}")));
        }

        // Target small enough to force dropping
        let result = compact_messages(&messages, 200, 2);
        // Should have truncation notice + kept messages
        assert!(result.len() < messages.len());
        // First message should be truncation notice
        match &result[0].content[0] {
            ContentBlock::Text { text } => {
                assert!(text.contains("truncated"));
            }
            _ => panic!("Expected truncation notice"),
        }
    }

    #[test]
    fn test_drop_oldest_preserves_minimum() {
        let mut messages = Vec::new();
        for _ in 0..10 {
            messages.push(text_msg(Role::User, &"x".repeat(100)));
            messages.push(text_msg(Role::Assistant, &"y".repeat(100)));
        }

        // Very small target, but min_keep_pairs=2
        let result = compact_messages(&messages, 50, 2);
        // Should keep at least 2 pairs (4 messages) + truncation notice
        // Count user messages (excluding truncation notice which is also User role)
        let user_msgs: Vec<_> = result
            .iter()
            .filter(|m| {
                m.role == Role::User
                    && m.content.iter().any(|b| match b {
                        ContentBlock::Text { text } => !text.contains("truncated"),
                        _ => true,
                    })
            })
            .collect();
        assert!(user_msgs.len() >= 2);
    }

    #[test]
    fn test_compact_tool_pairs_preserved() {
        let messages = vec![
            text_msg(Role::User, &"x".repeat(100)),
            text_msg(Role::Assistant, &"y".repeat(100)),
            text_msg(Role::User, &"x".repeat(100)),
            tool_use_msg("t1", "bash"),
            tool_result_msg("t1", &"output".repeat(50)),
            text_msg(Role::User, "final question"),
            text_msg(Role::Assistant, "final answer"),
        ];

        let result = compact_messages(&messages, 200, 1);
        // Verify that if ToolResult is present, ToolUse is also present
        let has_tool_result = result.iter().any(|m| {
            m.content
                .iter()
                .any(|b| matches!(b, ContentBlock::ToolResult { .. }))
        });
        let has_tool_use = result.iter().any(|m| {
            m.content
                .iter()
                .any(|b| matches!(b, ContentBlock::ToolUse { .. }))
        });
        if has_tool_result {
            assert!(has_tool_use, "ToolResult without ToolUse");
        }
    }

    #[test]
    fn test_compact_empty() {
        let result = compact_messages(&[], 1000, 2);
        assert!(result.is_empty());
    }
}
