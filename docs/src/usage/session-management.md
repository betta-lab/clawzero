# Session Management

clawzero automatically saves conversations as sessions using JSONL format. Each session is stored as append-only JSONL with crash-safe flush.

## Listing sessions

```bash
clawzero sessions list
```

## Resuming a session

Resume a previous session to continue the conversation:

```bash
# Using the subcommand
clawzero sessions resume <session-id>

# Using the --resume flag (works with any command)
clawzero --resume <session-id> "Continue from where we left off"
clawzero --resume <session-id> chat
```

## Storage

Sessions are stored in `~/.local/share/clawzero/sessions/` as `.jsonl` files. Each file contains the full conversation history including messages, tool calls, and tool results.
