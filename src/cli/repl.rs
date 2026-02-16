use std::io::{self, BufRead, Write};
use std::sync::Arc;

use crate::agent::event::AgentEvent;
use crate::agent::factory::AgentFactory;
use crate::provider::traits::Provider;
use crate::session::store::SessionStore;
use crate::tool::plugin::types::PluginToolConfig;

/// Run a one-shot prompt and print the result.
pub async fn run_oneshot(
    provider: Arc<dyn Provider>,
    model: String,
    prompt: String,
    max_tokens: u32,
    max_turns: usize,
    context_limit: u32,
    plugin_tools: &[PluginToolConfig],
) {
    let factory = AgentFactory::new(
        provider,
        model.clone(),
        max_tokens,
        max_turns,
        context_limit,
        plugin_tools.to_vec(),
    );

    let mut agent = if let Ok(store) = SessionStore::new() {
        if let Ok(writer) = store.create_session(&model) {
            factory.create_with_session(writer)
        } else {
            factory.create()
        }
    } else {
        factory.create()
    };

    agent
        .run(prompt, |event| {
            print_event(event);
        })
        .await;

    if let Some(sid) = agent.session_id() {
        eprintln!("\n[session: {sid}]");
    }
    println!();
}

/// Run an interactive REPL session.
pub async fn run_repl(
    provider: Arc<dyn Provider>,
    model: String,
    max_tokens: u32,
    max_turns: usize,
    context_limit: u32,
    plugin_tools: &[PluginToolConfig],
) {
    let factory = AgentFactory::new(
        provider,
        model.clone(),
        max_tokens,
        max_turns,
        context_limit,
        plugin_tools.to_vec(),
    );

    let mut agent = if let Ok(store) = SessionStore::new() {
        if let Ok(writer) = store.create_session(&model) {
            factory.create_with_session(writer)
        } else {
            factory.create()
        }
    } else {
        factory.create()
    };

    println!("clawzero chat (model: {model})");
    if let Some(sid) = agent.session_id() {
        println!("Session: {sid}");
    }
    println!("Type /exit to quit.\n");

    repl_loop(&mut agent).await;
}

/// Resume an existing session in REPL mode.
pub async fn run_repl_resume(
    provider: Arc<dyn Provider>,
    model: String,
    max_tokens: u32,
    max_turns: usize,
    context_limit: u32,
    plugin_tools: &[PluginToolConfig],
    store: &SessionStore,
    session_id: &str,
) {
    let (writer, messages) = match store.resume_session(session_id) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("[error] Failed to resume session: {e}");
            return;
        }
    };

    let factory = AgentFactory::new(
        provider,
        model.clone(),
        max_tokens,
        max_turns,
        context_limit,
        plugin_tools.to_vec(),
    );

    let mut agent = factory.create_resumed(writer, messages);

    println!("clawzero chat (model: {model})");
    println!("Resumed session: {session_id}");
    println!("Type /exit to quit.\n");

    repl_loop(&mut agent).await;
}

async fn repl_loop(agent: &mut crate::agent::r#loop::Agent) {
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

/// Format tool input for display based on tool name.
/// Returns a human-readable summary of what the tool is about to do.
pub fn format_tool_input(name: &str, input: &serde_json::Value) -> String {
    match name {
        "bash" => {
            let cmd = input["command"].as_str().unwrap_or("(unknown)");
            format!("$ {cmd}")
        }
        "file_read" => {
            let path = input["path"].as_str().unwrap_or("(unknown)");
            format!("path: {path}")
        }
        "file_write" => {
            let path = input["path"].as_str().unwrap_or("(unknown)");
            let size = input["content"].as_str().map(|c| c.len()).unwrap_or(0);
            format!("path: {path} ({size} bytes)")
        }
        "file_edit" => {
            let path = input["path"].as_str().unwrap_or("(unknown)");
            format!("path: {path}")
        }
        _ => {
            let s = input.to_string();
            if s.len() > 200 {
                format!("{}...", &s[..200])
            } else {
                s
            }
        }
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
            input,
            output,
            is_error,
            ..
        } => {
            let status = if *is_error { "error" } else { "ok" };
            let input_display = format_tool_input(name, input);
            // Truncate long output for display
            let display = if output.len() > 500 {
                format!("{}... ({} bytes)", &output[..500], output.len())
            } else {
                output.clone()
            };
            eprintln!("[{name}: {input_display} -> {status}] {display}");
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
        AgentEvent::ContextCompacted {
            original_tokens,
            compacted_tokens,
            messages_dropped,
        } => {
            eprintln!(
                "[context compacted: {messages_dropped} messages dropped, {original_tokens} -> {compacted_tokens} tokens]"
            );
        }
        AgentEvent::Error(msg) => {
            eprintln!("\n[error] {msg}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn format_bash_command() {
        let input = json!({"command": "echo hello"});
        assert_eq!(format_tool_input("bash", &input), "$ echo hello");
    }

    #[test]
    fn format_bash_missing_command() {
        let input = json!({});
        assert_eq!(format_tool_input("bash", &input), "$ (unknown)");
    }

    #[test]
    fn format_file_read_path() {
        let input = json!({"path": "src/main.rs"});
        assert_eq!(format_tool_input("file_read", &input), "path: src/main.rs");
    }

    #[test]
    fn format_file_write_path_and_size() {
        let input = json!({"path": "out.txt", "content": "hello"});
        assert_eq!(
            format_tool_input("file_write", &input),
            "path: out.txt (5 bytes)"
        );
    }

    #[test]
    fn format_file_edit_path() {
        let input = json!({"path": "src/lib.rs", "old_text": "a", "new_text": "b"});
        assert_eq!(format_tool_input("file_edit", &input), "path: src/lib.rs");
    }

    #[test]
    fn format_unknown_tool_shows_json() {
        let input = json!({"key": "value"});
        assert_eq!(
            format_tool_input("custom_tool", &input),
            r#"{"key":"value"}"#
        );
    }

    #[test]
    fn format_unknown_tool_truncates_long_json() {
        let long_value = "x".repeat(300);
        let input = json!({"key": long_value});
        let result = format_tool_input("custom_tool", &input);
        assert!(result.len() <= 203); // 200 + "..."
        assert!(result.ends_with("..."));
    }
}
