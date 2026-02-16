# Benchmarking

clawzero includes a reproducible Docker-based benchmark environment for performance comparison against Claude Code and OpenClaw.

## Quick Start

```bash
# Run all tools × all scenarios
docker compose -f bench/docker-compose.yml run bench

# Measure clawzero startup time only
docker compose -f bench/docker-compose.yml run bench --tools clawzero --scenarios startup

# Specify iteration count
docker compose -f bench/docker-compose.yml run bench --iterations 10
```

## Prerequisites

- Docker / Docker Compose
- `ANTHROPIC_API_KEY` environment variable (required for API scenarios)
- `OPENAI_API_KEY` environment variable (optional, for OpenClaw)

## Metrics

| Metric | Measurement Method |
|---|---|
| Startup time (cold start) | Wall-clock time of `--help` execution via `hyperfine` |
| TTFT (Time to First Token) | Time until first stdout byte via custom wrapper |
| E2E completion time | Wall-clock time of prompt execution via `hyperfine` |
| Memory usage (peak RSS) | Maximum resident set size via `/usr/bin/time -v` |
| Token throughput | Output characters / E2E time |

## Scenarios

| Scenario | Description | API Call |
|---|---|---|
| `startup` | `--help` execution time | No |
| `simple` | Response to `"What is 1+1?"` | Yes |
| `tool_use` | File read + line count | Yes |

## File Structure

```
bench/
├── Dockerfile              # Multi-stage build
├── docker-compose.yml      # Environment variables and volume mounts
├── run.sh                  # Main benchmark runner
├── adapters/
│   ├── clawzero.sh         # clawzero invocation adapter
│   ├── claude-code.sh      # Claude Code invocation adapter
│   └── openclaw.sh         # OpenClaw invocation adapter
├── measure_ttft.sh         # TTFT measurement helper
├── fixtures/
│   └── bench_input.txt     # Test file for tool_use scenario
└── results/                # Output directory (.gitignore)
```

## run.sh Options

```
--tools <t1,t2,...>       Tools to benchmark (default: clawzero,claude-code,openclaw)
--scenarios <s1,s2,...>   Scenarios to run (default: startup,simple,tool_use)
--iterations <N>          Iteration count (default: $BENCH_ITERATIONS or 5)
--results-dir <path>      Output directory (default: bench/results)
```

## Environment Variables

| Variable | Description | Default |
|---|---|---|
| `ANTHROPIC_API_KEY` | Anthropic API key | (required) |
| `OPENAI_API_KEY` | OpenAI API key | (optional) |
| `BENCH_ITERATIONS` | Iteration count | `5` |
| `BENCH_MODEL` | Model used by clawzero | `anthropic/claude-sonnet-4-5-20250929` |

## Results

Results are saved to `bench/results/<timestamp>/`:

- `results.json` — All metrics in JSON format
- `<tool>_<scenario>_hyperfine.json` — hyperfine raw data
- `<tool>_<scenario>_time.txt` — `/usr/bin/time` output
- `<tool>_<scenario>_ttft.csv` — TTFT CSV data

A summary table is printed to the console when execution completes.

## Adding New Adapters

To add a new tool, create `bench/adapters/<name>.sh` and define the following functions:

```bash
TOOL_NAME="my-tool"

cmd_startup() {
    my-tool --help
}

cmd_simple() {
    my-tool "What is 1+1?"
}

cmd_tool_use() {
    my-tool "Read /tmp/bench_input.txt and count the lines"
}
```

Specify `--tools my-tool` to automatically load the adapter.
