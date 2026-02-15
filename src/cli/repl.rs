use std::io::{self, BufRead, Write};
use std::sync::Arc;

use crate::agent::context::ConversationContext;
use crate::agent::event::AgentEvent;
use crate::agent::r#loop::Agent;
use crate::provider::traits::Provider;
use crate::tool::builtin::builtin_tools;

const SYSTEM_PROMPT: &str = r#"You are a helpful AI coding assistant. You have access to tools for executing bash commands, reading files, writing files, and editing files. Use these tools to help the user with their tasks.

When using tools:
- Use bash to run commands and explore the system
- Use file_read to examine file contents
- Use file_write to create or overwrite files
- Use file_edit to make targeted changes to existing files

Be concise and direct in your responses."#;

/// Run a one-shot prompt and print the result.
pub async fn run_oneshot(
    provider: Arc<dyn Provider>,
    model: String,
    prompt: String,
    max_tokens: u32,
    max_turns: usize,
) {
    let context = ConversationContext::new(SYSTEM_PROMPT.to_string(), max_tokens);
    let tool_registry = builtin_tools();
    let mut agent = Agent::new(provider, model, tool_registry, context, max_turns);

    agent
        .run(prompt, |event| {
            print_event(event);
        })
        .await;

    println!();
}

/// Run an interactive REPL session.
pub async fn run_repl(
    provider: Arc<dyn Provider>,
    model: String,
    max_tokens: u32,
    max_turns: usize,
) {
    let context = ConversationContext::new(SYSTEM_PROMPT.to_string(), max_tokens);
    let tool_registry = builtin_tools();
    let mut agent = Agent::new(provider, model.clone(), tool_registry, context, max_turns);

    println!("clawzero chat (model: {model})");
    println!("Type /exit to quit.\n");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("> ");
        stdout.flush().unwrap();

        let mut input = String::new();
        match stdin.lock().read_line(&mut input) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("Input error: {e}");
                break;
            }
        }

        let input = input.trim().to_string();
        if input.is_empty() {
            continue;
        }
        if input == "/exit" || input == "/quit" {
            break;
        }

        agent
            .run(input, |event| {
                print_event(event);
            })
            .await;

        println!("\n");
    }
}

fn print_event(event: &AgentEvent) {
    match event {
        AgentEvent::TextDelta(text) => {
            print!("{text}");
            io::stdout().flush().unwrap();
        }
        AgentEvent::ToolCallStart { name, .. } => {
            eprintln!("\n[tool: {name}]");
        }
        AgentEvent::ToolResult {
            name,
            output,
            is_error,
            ..
        } => {
            let status = if *is_error { "error" } else { "ok" };
            // Truncate long output for display
            let display = if output.len() > 500 {
                format!("{}... ({} bytes)", &output[..500], output.len())
            } else {
                output.clone()
            };
            eprintln!("[{name} -> {status}] {display}");
        }
        AgentEvent::TurnComplete { usage } => {
            tracing::debug!(
                input_tokens = usage.input_tokens,
                output_tokens = usage.output_tokens,
                "Turn complete"
            );
        }
        AgentEvent::Done { total_usage } => {
            tracing::debug!(
                input_tokens = total_usage.input_tokens,
                output_tokens = total_usage.output_tokens,
                "Agent done"
            );
        }
        AgentEvent::Error(msg) => {
            eprintln!("\n[error] {msg}");
        }
    }
}
