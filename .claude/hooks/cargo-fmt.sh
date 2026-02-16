#!/bin/bash
# PostToolUse hook: auto-format Rust files after Edit/Write
INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

if [[ "$FILE_PATH" == *.rs ]]; then
  cargo fmt -q 2>/dev/null
fi

exit 0
