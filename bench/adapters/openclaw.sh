#!/usr/bin/env bash
# Adapter for OpenClaw

TOOL_NAME="openclaw"

cmd_startup() {
    openclaw --help
}

cmd_simple() {
    openclaw agent --agent main --message "What is 1+1?"
}

cmd_tool_use() {
    openclaw agent --agent main --message "Read /tmp/bench_input.txt and count the lines"
}
