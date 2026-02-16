#!/usr/bin/env bash
# measure_ttft.sh — Measure Time to First Token (TTFT)
#
# Usage: ./measure_ttft.sh <iterations> <command...>
#
# Measures the time from process launch to the first byte of stdout.
# Outputs CSV rows: iteration,ttft_ms

set -euo pipefail

iterations="${1:?Usage: measure_ttft.sh <iterations> <command...>}"
shift
cmd=("$@")

for i in $(seq 1 "$iterations"); do
    # Record start time in nanoseconds
    start_ns=$(date +%s%N)

    # Run the command; use `read -n 1` to detect the first byte on stdout.
    # We pipe stdout through a subshell that records the timestamp at first byte.
    ttft_ns=$(
        "${cmd[@]}" 2>/dev/null | {
            # Read a single byte — this blocks until stdout produces output
            if IFS= read -r -n 1 -d '' first_byte; then
                echo "$(date +%s%N)"
            else
                echo "0"
            fi
            # Drain remaining output to avoid SIGPIPE
            cat >/dev/null
        }
    )

    if [ "$ttft_ns" = "0" ]; then
        echo "${i},error"
    else
        elapsed_ns=$((ttft_ns - start_ns))
        elapsed_ms=$(awk "BEGIN {printf \"%.2f\", $elapsed_ns / 1000000}")
        echo "${i},${elapsed_ms}"
    fi
done
