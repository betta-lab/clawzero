# Memory System

clawzero provides a persistent memory system using MEMORY.md files. Memory content is automatically injected into the system prompt, giving the agent access to stored context across sessions.

## Memory scopes

| Scope | Location | Use case |
|-------|----------|----------|
| Global | `~/.config/clawzero/MEMORY.md` | User-wide preferences, conventions |
| Project | `.clawzero/MEMORY.md` (project root) | Project-specific context, decisions |

The project root is detected by walking up from the current directory looking for `.git` or `Cargo.toml`.

## How it works

1. On startup, clawzero reads both global and project MEMORY.md files
2. Their contents are concatenated (with headers) and injected into the system prompt
3. The agent can read memory at any time using the `memory_read` tool
4. The agent can update memory using the `memory_write` tool

## Tools

### memory_read

Returns both global and project memory content, prefixed with `# Global Memory` and `# Project Memory` headers.

### memory_write

Writes to either global or project memory. The `scope` parameter selects which file to update. Writing replaces the entire file content.

```
scope: "global" or "project"
content: "The markdown content to write"
```

## Best practices

- Use global memory for personal preferences and conventions
- Use project memory for project-specific context (architecture decisions, key paths, patterns)
- Keep memory concise — it's injected into every system prompt
