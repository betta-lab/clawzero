pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod memory_read;
pub mod memory_write;
pub mod shell;

use std::sync::Arc;

use crate::memory::store::MemoryStore;
use crate::tool::traits::{Tool, ToolRegistry};

/// Create a ToolRegistry with all built-in tools registered.
pub fn builtin_tools(memory_store: Arc<MemoryStore>) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    let tools: Vec<Arc<dyn Tool>> = vec![
        shell::ShellTool::create(),
        file_read::FileReadTool::create(),
        file_write::FileWriteTool::create(),
        file_edit::FileEditTool::create(),
        memory_read::MemoryReadTool::create(Arc::clone(&memory_store)),
        memory_write::MemoryWriteTool::create(memory_store),
    ];
    for tool in tools {
        registry.register(tool);
    }
    registry
}
