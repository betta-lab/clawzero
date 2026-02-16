#!/usr/bin/env bash
# Adapter for clawzero

TOOL_NAME="clawzero"

cmd_startup() {
    clawzero --help
}

cmd_simple() {
    clawzero --no-tui "What is 1+1?"
}

cmd_tool_use() {
    clawzero --no-tui "Read /tmp/bench_input.txt and count the lines"
}
