use crate::model::response::Usage;

/// Events emitted by the agent loop to the UI layer.
#[derive(Debug)]
pub enum AgentEvent {
    /// Streaming text from the model
    TextDelta(String),
    /// Model wants to call a tool
    ToolCallStart { id: String, name: String },
    /// Tool execution result
    ToolResult {
        id: String,
        name: String,
        output: String,
        is_error: bool,
    },
    /// A complete turn finished
    TurnComplete { usage: Usage },
    /// Agent finished (end_turn or max_tokens)
    Done { total_usage: Usage },
    /// Error occurred
    Error(String),
}
