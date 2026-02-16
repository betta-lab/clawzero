#!/usr/bin/env bash
# Adapter for Claude Code

TOOL_NAME="claude-code"

cmd_startup() {
    claude --help
}

cmd_simple() {
    claude -p "What is 1+1?"
}

cmd_tool_use() {
    claude -p "Read /tmp/bench_input.txt and count the lines"
}
