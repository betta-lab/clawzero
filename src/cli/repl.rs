use std::io::{self, BufRead, Write};
use std::sync::Arc;

use crate::agent::context::ConversationContext;
use crate::agent::event::AgentEvent;
use crate::agent::r#loop::Agent;
use crate::agent::token::ContextLimits;
use crate::memory::store::MemoryStore;
use crate::provider::traits::Provider;
use crate::session::store::SessionStore;
use crate::tool::builtin::builtin_tools;
use crate::tool::plugin::loader::load_plugin_tools;
use crate::tool::plugin::types::PluginToolConfig;

const SYSTEM_PROMPT: &str = r#"You are a helpful AI coding assistant. You have access to tools for executing bash commands, reading files, writing files, and editing files. Use these tools to help the user with their tasks.

When using tools:
- Use bash to run commands and explore the system
- Use file_read to examine file contents
- Use file_write to create or overwrite files
- Use file_edit to make targeted changes to existing files
- Use memory_read to recall information from persistent memory
- Use memory_write to store important information across sessions

Be concise and direct in your responses."#;

/// Build system prompt with memory content injected.
fn build_system_prompt(memory_store: &MemoryStore) -> String {
    let memory_content = memory_store.read_all();
    if memory_content.is_empty() {
        SYSTEM_PROMPT.to_string()
    } else {
        format!(
            "{SYSTEM_PROMPT}\n\n## Persistent Memory\n\n{memory_content}"
        )
    }
}

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
    let memory_store = Arc::new(MemoryStore::new());
    let system_prompt = build_system_prompt(&memory_store);
    let context = ConversationContext::new(system_prompt, max_tokens);
    let mut tool_registry = builtin_tools(memory_store);
    tool_registry.register_all(load_plugin_tools(plugin_tools));
    let mut agent = Agent::new(provider, model.clone(), tool_registry, context, max_turns);
    agent.set_context_limits(ContextLimits::new(context_limit));

    // Auto-save session
    if let Ok(store) = SessionStore::new() {
        if let Ok(writer) = store.create_session(&model) {
            agent.set_session_writer(writer);
        }
    }

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
    let memory_store = Arc::new(MemoryStore::new());
    let system_prompt = build_system_prompt(&memory_store);
    let context = ConversationContext::new(system_prompt, max_tokens);
    let mut tool_registry = builtin_tools(memory_store);
    tool_registry.register_all(load_plugin_tools(plugin_tools));
    let mut agent = Agent::new(provider, model.clone(), tool_registry, context, max_turns);
    agent.set_context_limits(ContextLimits::new(context_limit));

    // Auto-save session
    if let Ok(store) = SessionStore::new() {
        if let Ok(writer) = store.create_session(&model) {
            agent.set_session_writer(writer);
        }
    }

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

    let memory_store = Arc::new(MemoryStore::new());
    let system_prompt = build_system_prompt(&memory_store);
    let mut context = ConversationContext::new(system_prompt, max_tokens);
    context.restore_messages(messages);

    let mut tool_registry = builtin_tools(memory_store);
    tool_registry.register_all(load_plugin_tools(plugin_tools));
    let mut agent = Agent::new(provider, model.clone(), tool_registry, context, max_turns);
    agent.set_context_limits(ContextLimits::new(context_limit));
    agent.set_session_writer(writer);

    println!("clawzero chat (model: {model})");
    println!("Resumed session: {session_id}");
    println!("Type /exit to quit.\n");

    repl_loop(&mut agent).await;
}

async fn repl_loop(agent: &mut Agent) {
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
