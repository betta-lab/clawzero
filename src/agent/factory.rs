use std::sync::Arc;

use crate::agent::context::ConversationContext;
use crate::agent::r#loop::Agent;
use crate::agent::token::ContextLimits;
use crate::memory::store::MemoryStore;
use crate::model::message::Message;
use crate::provider::traits::Provider;
use crate::session::store::SessionWriter;
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

/// Builds system prompt with memory content injected.
pub fn build_system_prompt(memory_store: &MemoryStore) -> String {
    let memory_content = memory_store.read_all();
    if memory_content.is_empty() {
        SYSTEM_PROMPT.to_string()
    } else {
        format!("{SYSTEM_PROMPT}\n\n## Persistent Memory\n\n{memory_content}")
    }
}

/// Factory for creating Agent instances with shared configuration.
pub struct AgentFactory {
    provider: Arc<dyn Provider>,
    model: String,
    system_prompt: String,
    max_tokens: u32,
    max_turns: usize,
    context_limits: ContextLimits,
    plugin_tools: Vec<PluginToolConfig>,
    memory_store: Arc<MemoryStore>,
}

impl AgentFactory {
    pub fn new(
        provider: Arc<dyn Provider>,
        model: String,
        max_tokens: u32,
        max_turns: usize,
        context_limit: u32,
        plugin_tools: Vec<PluginToolConfig>,
    ) -> Self {
        let memory_store = Arc::new(MemoryStore::new());
        let system_prompt = build_system_prompt(&memory_store);
        Self {
            provider,
            model,
            system_prompt,
            max_tokens,
            max_turns,
            context_limits: ContextLimits::new(context_limit),
            plugin_tools,
            memory_store,
        }
    }

    /// Create a new Agent (no session persistence).
    pub fn create(&self) -> Agent {
        let context = ConversationContext::new(self.system_prompt.clone(), self.max_tokens);
        let mut tool_registry = builtin_tools(Arc::clone(&self.memory_store));
        tool_registry.register_all(load_plugin_tools(&self.plugin_tools));
        let mut agent = Agent::new(
            Arc::clone(&self.provider),
            self.model.clone(),
            tool_registry,
            context,
            self.max_turns,
        );
        agent.set_context_limits(ContextLimits::new(self.context_limits.max_context_tokens));
        agent
    }

    /// Create a new Agent with session persistence.
    pub fn create_with_session(&self, writer: SessionWriter) -> Agent {
        let mut agent = self.create();
        agent.set_session_writer(writer);
        agent
    }

    /// Create an Agent resumed from a previous session.
    pub fn create_resumed(&self, writer: SessionWriter, messages: Vec<Message>) -> Agent {
        let mut context = ConversationContext::new(self.system_prompt.clone(), self.max_tokens);
        context.restore_messages(messages);
        let mut tool_registry = builtin_tools(Arc::clone(&self.memory_store));
        tool_registry.register_all(load_plugin_tools(&self.plugin_tools));
        let mut agent = Agent::new(
            Arc::clone(&self.provider),
            self.model.clone(),
            tool_registry,
            context,
            self.max_turns,
        );
        agent.set_context_limits(ContextLimits::new(self.context_limits.max_context_tokens));
        agent.set_session_writer(writer);
        agent
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ClawError;
    use crate::model::request::CompletionRequest;
    use crate::model::response::{StreamEvent, Usage};
    use crate::provider::traits::EventStream;
    use crate::session::store::SessionStore;
    use std::future::Future;
    use std::pin::Pin;

    struct MockProvider;

    impl Provider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        fn complete<'a>(
            &'a self,
            _request: &'a CompletionRequest,
        ) -> Pin<Box<dyn Future<Output = Result<EventStream, ClawError>> + Send + 'a>> {
            Box::pin(async {
                let stream = futures_util::stream::iter(vec![Ok(StreamEvent::MessageEnd {
                    stop_reason: crate::model::response::StopReason::EndTurn,
                    usage: Usage {
                        input_tokens: 0,
                        output_tokens: 0,
                    },
                })]);
                Ok(Box::pin(stream) as EventStream)
            })
        }
    }

    #[test]
    fn factory_creates_agent() {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider);
        let factory = AgentFactory::new(provider, "mock/model".into(), 4096, 10, 200_000, vec![]);
        let agent = factory.create();
        assert!(agent.session_id().is_none());
    }

    #[test]
    fn factory_creates_with_session() {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider);
        let factory = AgentFactory::new(provider, "mock/model".into(), 4096, 10, 200_000, vec![]);

        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::with_dir(dir.path().to_path_buf()).unwrap();
        let writer = store.create_session("mock/model").unwrap();

        let agent = factory.create_with_session(writer);
        assert!(agent.session_id().is_some());
    }

    #[test]
    fn factory_creates_resumed() {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider);
        let factory = AgentFactory::new(provider, "mock/model".into(), 4096, 10, 200_000, vec![]);

        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::with_dir(dir.path().to_path_buf()).unwrap();
        let writer = store.create_session("mock/model").unwrap();

        let messages = vec![Message {
            role: crate::model::message::Role::User,
            content: vec![crate::model::message::ContentBlock::Text {
                text: "hello".into(),
            }],
        }];

        let agent = factory.create_resumed(writer, messages);
        assert!(agent.session_id().is_some());
    }

    #[test]
    fn factory_model_accessor() {
        let provider: Arc<dyn Provider> = Arc::new(MockProvider);
        let factory = AgentFactory::new(provider, "test/model".into(), 4096, 10, 200_000, vec![]);
        assert_eq!(factory.model(), "test/model");
    }
}
