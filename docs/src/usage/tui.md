# TUI

clawzero features a Claude Code-style inline terminal UI. The TUI grows in-place without taking over the full screen — confirmed output scrolls into terminal history while the live viewport shows only active content.

## Keybindings

| Key | Action |
|-----|--------|
| Enter | Send message |
| Ctrl+J | Insert newline |
| Ctrl+A / Home | Move cursor to beginning of line |
| Ctrl+E / End | Move cursor to end of line |
| Ctrl+K | Delete from cursor to end of line |
| Ctrl+W | Delete word before cursor |
| Ctrl+C | Quit |
| `/exit`, `/quit` | Quit |

## Auto-detection

The TUI is enabled by default when stdin is a TTY. Piped input automatically uses plain text mode:

```bash
# TUI mode (default for interactive use)
clawzero chat

# Plain text mode (piped input)
echo "hello" | clawzero
```

## Disabling the TUI

Use `--no-tui` to fall back to plain text mode:

```bash
clawzero --no-tui chat
```

## Features

- **Streaming text display** — Responses stream in real-time with Markdown rendering
- **Tool call cards** — Visual cards showing tool name, parameters, and status
- **Spinner animation** — Animated indicator during thinking and tool execution
- **Multi-line input** — Use Ctrl+J to insert newlines in your prompt
- **Scrollback** — Past output scrolls into terminal history and can be viewed with your terminal's scrollback
