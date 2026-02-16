# Built-in Tools

clawzero provides 6 built-in tools that the agent can use autonomously during the Think → ToolCall → Observe cycle.

## bash

Execute a bash command and return stdout/stderr.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `command` | string | Yes | The bash command to execute |
| `timeout_ms` | integer | No | Timeout in milliseconds (default: 120000) |

**Example use**: Running build commands, git operations, installing packages.

## file_read

Read the contents of a file. Returns the file contents with line numbers.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | Yes | The file path to read |
| `offset` | integer | No | Line number to start reading from (1-based, default: 1) |
| `limit` | integer | No | Maximum number of lines to read (default: 2000) |

**Example use**: Reading source code, configuration files, logs.

## file_write

Write content to a file. Creates the file if it doesn't exist, or overwrites it.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | Yes | The file path to write to |
| `content` | string | Yes | The content to write |

**Example use**: Creating new files, writing generated code.

## file_edit

Edit a file by replacing a specific text string with new text. The old_text must be unique in the file.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `path` | string | Yes | The file path to edit |
| `old_text` | string | Yes | The exact text to find and replace (must be unique in the file) |
| `new_text` | string | Yes | The text to replace it with |

**Example use**: Modifying existing code, fixing bugs, refactoring.

## memory_read

Read persistent memory (MEMORY.md). Returns both global and project-local memory content.

No parameters.

**Example use**: Recalling project conventions, previous decisions, stored context.

## memory_write

Write to persistent memory (MEMORY.md). Use this to store information that should persist across sessions.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `scope` | string | Yes | Where to write: `global` (user-wide) or `project` (project-local) |
| `content` | string | Yes | The markdown content to write to MEMORY.md (replaces entire file) |

**Example use**: Saving project conventions, recording decisions, noting important context.
