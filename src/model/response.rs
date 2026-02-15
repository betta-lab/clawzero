#[derive(Debug, Clone)]
pub enum StreamEvent {
    MessageStart { id: String, model: String },
    TextDelta { text: String },
    ToolUseStart { id: String, name: String },
    ToolInputDelta { partial_json: String },
    ToolUseEnd,
    MessageEnd { stop_reason: StopReason, usage: Usage },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    StopSequence,
}

#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
