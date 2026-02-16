#!/usr/bin/env bash
# run.sh — Main benchmark runner for clawzero
#
# Usage:
#   ./bench/run.sh [OPTIONS]
#
# Options:
#   --tools <t1,t2,...>       Tools to benchmark (default: clawzero,claude-code,openclaw)
#   --scenarios <s1,s2,...>   Scenarios to run (default: startup,simple,tool_use)
#   --iterations <N>          Number of iterations (default: $BENCH_ITERATIONS or 5)
#   --results-dir <path>      Output directory (default: bench/results)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ADAPTERS_DIR="${SCRIPT_DIR}/adapters"
FIXTURES_DIR="${SCRIPT_DIR}/fixtures"

# Defaults
ALL_TOOLS="clawzero,claude-code,openclaw"
ALL_SCENARIOS="startup,simple,tool_use"
ITERATIONS="${BENCH_ITERATIONS:-5}"
RESULTS_DIR="${SCRIPT_DIR}/results"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --tools)
            ALL_TOOLS="$2"; shift 2 ;;
        --scenarios)
            ALL_SCENARIOS="$2"; shift 2 ;;
        --iterations)
            ITERATIONS="$2"; shift 2 ;;
        --results-dir)
            RESULTS_DIR="$2"; shift 2 ;;
        -h|--help)
            sed -n '2,/^$/p' "$0" | sed 's/^# \?//'
            exit 0 ;;
        *)
            echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

IFS=',' read -ra TOOLS <<< "$ALL_TOOLS"
IFS=',' read -ra SCENARIOS <<< "$ALL_SCENARIOS"

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
RUN_DIR="${RESULTS_DIR}/${TIMESTAMP}"
mkdir -p "$RUN_DIR"

# Prepare fixtures
cp "${FIXTURES_DIR}/bench_input.txt" /tmp/bench_input.txt

echo "============================================"
echo " clawzero Benchmark Suite"
echo "============================================"
echo " Tools:      ${TOOLS[*]}"
echo " Scenarios:  ${SCENARIOS[*]}"
echo " Iterations: ${ITERATIONS}"
echo " Output:     ${RUN_DIR}"
echo "============================================"
echo ""

# JSON result accumulator
JSON_RESULTS="[]"

# Helper: add a result to the JSON array
add_result() {
    local tool="$1" scenario="$2" metric="$3" value="$4" unit="$5"
    JSON_RESULTS=$(echo "$JSON_RESULTS" | python3 -c "
import json, sys
data = json.load(sys.stdin)
data.append({
    'tool': '$tool',
    'scenario': '$scenario',
    'metric': '$metric',
    'value': $value,
    'unit': '$unit',
})
json.dump(data, sys.stdout)
")
}

# Helper: extract median from hyperfine JSON
hyperfine_median() {
    python3 -c "
import json, sys
data = json.load(open(sys.argv[1]))
print(f\"{data['results'][0]['median'] * 1000:.2f}\")
" "$1"
}

# Helper: extract peak RSS from /usr/bin/time output (in KB)
extract_peak_rss() {
    grep "Maximum resident set size" "$1" | awk '{print $NF}'
}

# Helper: compute median from CSV (column 2)
csv_median() {
    awk -F',' 'NR>0 && $2!="error" {a[NR]=$2} END {
        n=asort(a);
        if (n%2==1) print a[int(n/2)+1];
        else printf "%.2f\n", (a[n/2]+a[n/2+1])/2
    }' "$1"
}

run_benchmark() {
    local tool="$1"
    local scenario="$2"
    local adapter="${ADAPTERS_DIR}/${tool}.sh"

    if [[ ! -f "$adapter" ]]; then
        echo "  [SKIP] Adapter not found: ${adapter}"
        return
    fi

    # Source the adapter to get cmd_* functions
    source "$adapter"

    # Build the command for this scenario
    local cmd_func="cmd_${scenario}"
    if ! type "$cmd_func" &>/dev/null; then
        echo "  [SKIP] Function ${cmd_func} not defined in ${tool} adapter"
        return
    fi

    local prefix="${RUN_DIR}/${tool}_${scenario}"

    echo "  Running: ${tool} / ${scenario}"

    # 1. E2E time with hyperfine
    echo "    -> E2E time (hyperfine)..."
    local hyperfine_json="${prefix}_hyperfine.json"
    # Export the function so hyperfine's shell can use it
    export -f "$cmd_func" 2>/dev/null || true
    timeout 180 hyperfine \
        --runs "$ITERATIONS" \
        --export-json "$hyperfine_json" \
        --warmup 1 \
        --shell bash \
        --ignore-failure \
        --command-name "${tool}/${scenario}" \
        "source ${adapter} && ${cmd_func}" \
        2>/dev/null || {
            echo "    [WARN] hyperfine failed or timed out for ${tool}/${scenario}"
        }

    if [[ -f "$hyperfine_json" ]] && [[ -s "$hyperfine_json" ]]; then
        local median_ms
        median_ms=$(hyperfine_median "$hyperfine_json" 2>/dev/null) || true
        if [[ -n "$median_ms" ]]; then
            echo "    E2E median: ${median_ms} ms"
            add_result "$tool" "$scenario" "e2e_ms" "$median_ms" "ms"
        else
            echo "    [WARN] Could not parse hyperfine results"
        fi
    fi

    # 2. Memory (peak RSS) with /usr/bin/time
    echo "    -> Peak RSS (/usr/bin/time)..."
    local time_output="${prefix}_time.txt"
    /usr/bin/time -v bash -c "source ${adapter} && ${cmd_func}" \
        >"${prefix}_stdout.txt" 2>"$time_output" || true

    if [[ -f "$time_output" ]]; then
        local peak_rss_kb
        peak_rss_kb=$(extract_peak_rss "$time_output")
        if [[ -n "$peak_rss_kb" ]]; then
            local peak_rss_mb
            peak_rss_mb=$(awk "BEGIN {printf \"%.1f\", $peak_rss_kb / 1024}")
            echo "    Peak RSS: ${peak_rss_mb} MB"
            add_result "$tool" "$scenario" "peak_rss_mb" "$peak_rss_mb" "MB"
        fi
    fi

    # 3. TTFT (only for scenarios that produce output, skip startup)
    if [[ "$scenario" != "startup" ]]; then
        echo "    -> TTFT..."
        local ttft_csv="${prefix}_ttft.csv"
        bash "${SCRIPT_DIR}/measure_ttft.sh" "$ITERATIONS" \
            bash -c "source ${adapter} && ${cmd_func}" \
            > "$ttft_csv" 2>/dev/null || true

        if [[ -f "$ttft_csv" ]] && [[ -s "$ttft_csv" ]]; then
            local ttft_median
            ttft_median=$(csv_median "$ttft_csv")
            if [[ -n "$ttft_median" ]]; then
                echo "    TTFT median: ${ttft_median} ms"
                add_result "$tool" "$scenario" "ttft_ms" "$ttft_median" "ms"
            fi
        fi
    fi

    # 4. Token throughput (output chars / E2E time) — for non-startup scenarios
    if [[ "$scenario" != "startup" ]] && [[ -f "${prefix}_stdout.txt" ]] && [[ -f "$hyperfine_json" ]] && [[ -s "$hyperfine_json" ]]; then
        local char_count median_s throughput
        char_count=$(wc -c < "${prefix}_stdout.txt")
        median_s=$(python3 -c "
import json
data = json.load(open('${hyperfine_json}'))
print(f\"{data['results'][0]['median']:.4f}\")
" 2>/dev/null) || true
        if [[ -n "$median_s" ]] && [[ "$median_s" != "0" ]]; then
            throughput=$(awk "BEGIN {printf \"%.1f\", $char_count / $median_s}")
            echo "    Throughput: ${throughput} chars/s"
            add_result "$tool" "$scenario" "throughput_chars_per_s" "$throughput" "chars/s"
        fi
    fi

    echo ""
}

# Run all combinations
for scenario in "${SCENARIOS[@]}"; do
    echo ""
    echo "--- Scenario: ${scenario} ---"
    for tool in "${TOOLS[@]}"; do
        run_benchmark "$tool" "$scenario"
    done
done

# Save JSON results
RESULTS_JSON="${RUN_DIR}/results.json"
echo "$JSON_RESULTS" | python3 -m json.tool > "$RESULTS_JSON"
echo "JSON results saved to: ${RESULTS_JSON}"

# Print summary table
echo ""
echo "============================================"
echo " Summary"
echo "============================================"
python3 -c "
import json

data = json.load(open('${RESULTS_JSON}'))
if not data:
    print('No results collected.')
    exit()

# Group by scenario
scenarios = {}
for r in data:
    key = r['scenario']
    if key not in scenarios:
        scenarios[key] = {}
    tool = r['tool']
    if tool not in scenarios[key]:
        scenarios[key][tool] = {}
    scenarios[key][tool][r['metric']] = r['value']

for scenario, tools in scenarios.items():
    print(f'\n  Scenario: {scenario}')
    print(f'  {\"Tool\":<15} {\"E2E (ms)\":<15} {\"Peak RSS (MB)\":<15} {\"TTFT (ms)\":<15} {\"Throughput\":<20}')
    print(f'  {\"-\"*15} {\"-\"*15} {\"-\"*15} {\"-\"*15} {\"-\"*20}')
    for tool, metrics in tools.items():
        e2e = metrics.get('e2e_ms', '-')
        rss = metrics.get('peak_rss_mb', '-')
        ttft = metrics.get('ttft_ms', '-')
        tp = metrics.get('throughput_chars_per_s', '-')
        print(f'  {tool:<15} {e2e:<15} {rss:<15} {ttft:<15} {tp:<20}')
"

echo ""
echo "Full results: ${RUN_DIR}/"
