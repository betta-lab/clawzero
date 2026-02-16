#!/usr/bin/env python3
"""
Generate professional benchmark graphs for README
Based on 2025 visualization best practices
"""
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
from pathlib import Path

# Set modern professional style
sns.set_theme(style="whitegrid", context="notebook", palette="muted")
plt.rcParams['font.family'] = 'sans-serif'
plt.rcParams['font.sans-serif'] = ['DejaVu Sans', 'Arial', 'Helvetica']
plt.rcParams['font.size'] = 10
plt.rcParams['axes.labelsize'] = 11
plt.rcParams['axes.titlesize'] = 13
plt.rcParams['axes.titleweight'] = 'bold'
plt.rcParams['figure.dpi'] = 150
plt.rcParams['savefig.dpi'] = 150
plt.rcParams['savefig.bbox'] = 'tight'

# Professional color palette (accessibility-friendly)
COLORS = {
    'clawzero': '#0066CC',      # Professional blue
    'claude-code': '#2ECC71',   # Sage green
    'openclaw': '#E67E22'       # Warm orange
}

tools = ['clawzero', 'Claude Code', 'OpenClaw']
colors = [COLORS['clawzero'], COLORS['claude-code'], COLORS['openclaw']]

# Complete benchmark data from all scenarios (converted to seconds for readability)
data = {
    'startup': {
        'time': [0.002, 0.838, 1.425],  # seconds
        'memory': [5.5, 216.8, 256.3]    # MB
    },
    'simple': {
        'e2e': [2.100, 4.674, 18.718],   # seconds
        'ttft': [1.689, 3.232, 9.611],   # seconds
        'memory': [10.1, 242.1, 398.0]   # MB
    },
    'tool_use': {
        'e2e': [4.423, 16.435, 10.482],  # seconds
        'ttft': [1.162, 12.529, 8.774],  # seconds
        'memory': [10.2, 244.9, 398.6]   # MB
    }
}

output_dir = Path(__file__).parent.parent / 'docs' / 'img' / 'bench'
output_dir.mkdir(parents=True, exist_ok=True)

def format_subplot(ax, title, ylabel):
    """Apply consistent formatting to subplot"""
    ax.set_title(title, fontsize=12, fontweight='bold', pad=10, color='#2c3e50')
    ax.set_ylabel(ylabel, fontsize=10, fontweight='600', color='#34495e')
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    ax.spines['left'].set_color('#95a5a6')
    ax.spines['bottom'].set_color('#95a5a6')
    ax.tick_params(colors='#34495e', which='both', labelsize=9)
    ax.yaxis.grid(True, alpha=0.25, linestyle='--', linewidth=0.8, color='#bdc3c7')
    ax.set_axisbelow(True)

def add_grouped_bars(ax, data_by_scenario, bar_width=0.25):
    """Add grouped bars for multiple scenarios and tools

    data_by_scenario: list of lists, e.g. [[tool1_val1, tool2_val1, tool3_val1], [tool1_val2, ...]]
    """
    n_scenarios = len(data_by_scenario)
    n_tools = len(data_by_scenario[0])
    x = np.arange(n_scenarios)

    bars_list = []
    for tool_idx in range(n_tools):
        # Extract values for this tool across all scenarios
        values = [scenario[tool_idx] for scenario in data_by_scenario]
        offset = (tool_idx - 1) * bar_width
        bars = ax.bar(x + offset, values, bar_width,
                     color=colors[tool_idx], alpha=0.85, edgecolor='white', linewidth=1.5,
                     label=tools[tool_idx])
        bars_list.append(bars)

        # Add value labels
        for bar, value in zip(bars, values):
            height = bar.get_height()
            if height > 0:
                ax.text(bar.get_x() + bar.get_width()/2., height,
                       f'{value:.1f}' if value < 10 else f'{value:.0f}',
                       ha='center', va='bottom', fontsize=7,
                       fontweight='bold', color='#2c3e50')

    return bars_list

# Create comprehensive visualization (2x2 grid)
fig = plt.figure(figsize=(14, 9))
fig.patch.set_facecolor('white')

# 1. Startup Time (top-left)
ax1 = plt.subplot(2, 2, 1)
x = np.arange(1)
for i, (value, color, tool) in enumerate(zip(data['startup']['time'], colors, tools)):
    bars = ax1.bar(i, value, color=color, alpha=0.85, edgecolor='white', linewidth=2, width=0.6)
    # Add value + speedup annotation
    speedup = data['startup']['time'][i] / data['startup']['time'][0]
    label = f'{value:.3f}s' if i == 0 else f'{value:.2f}s\n({speedup:.0f}x)'
    ax1.text(i, value, label, ha='center', va='bottom',
            fontsize=9, fontweight='bold', color='#2c3e50')
ax1.set_xticks(range(3))
ax1.set_xticklabels(tools, fontsize=10, color='#2c3e50')
ax1.set_ylim(0, max(data['startup']['time']) * 1.25)
format_subplot(ax1, 'Startup Time (--help)', 'Time (s)')

# 2. E2E Time Comparison (top-right)
ax2 = plt.subplot(2, 2, 2)
bars = add_grouped_bars(ax2, [data['simple']['e2e'], data['tool_use']['e2e']])
# Add speedup annotations
for scenario_idx, scenario_name in enumerate(['simple', 'tool_use']):
    scenario_data = data[scenario_name]['e2e']
    for tool_idx in range(3):
        speedup = scenario_data[tool_idx] / scenario_data[0]
        if tool_idx > 0:  # Skip clawzero (baseline)
            bar = bars[tool_idx][scenario_idx]
            height = bar.get_height()
            ax2.text(bar.get_x() + bar.get_width()/2., height * 1.05,
                    f'{speedup:.1f}x', ha='center', va='bottom',
                    fontsize=7, style='italic', color='#e74c3c')
ax2.set_xticks([0, 1])
ax2.set_xticklabels(['simple', 'tool_use'], fontsize=10, color='#2c3e50')
ax2.set_ylim(0, max(data['simple']['e2e'] + data['tool_use']['e2e']) * 1.25)
format_subplot(ax2, 'End-to-End Time', 'Time (s)')
ax2.legend(loc='upper left', fontsize=9, frameon=True, fancybox=True, shadow=False)

# 3. TTFT Comparison (bottom-left)
ax3 = plt.subplot(2, 2, 3)
bars = add_grouped_bars(ax3, [data['simple']['ttft'], data['tool_use']['ttft']])
# Add speedup annotations
for scenario_idx, scenario_name in enumerate(['simple', 'tool_use']):
    scenario_data = data[scenario_name]['ttft']
    for tool_idx in range(3):
        speedup = scenario_data[tool_idx] / scenario_data[0]
        if tool_idx > 0:  # Skip clawzero (baseline)
            bar = bars[tool_idx][scenario_idx]
            height = bar.get_height()
            ax3.text(bar.get_x() + bar.get_width()/2., height * 1.05,
                    f'{speedup:.1f}x', ha='center', va='bottom',
                    fontsize=7, style='italic', color='#e74c3c')
ax3.set_xticks([0, 1])
ax3.set_xticklabels(['simple', 'tool_use'], fontsize=10, color='#2c3e50')
ax3.set_ylim(0, max(data['simple']['ttft'] + data['tool_use']['ttft']) * 1.25)
format_subplot(ax3, 'Time to First Token (TTFT)', 'Time (s)')

# 4. Memory by Scenario (bottom-right)
ax4 = plt.subplot(2, 2, 4)
bars = add_grouped_bars(ax4, [data['startup']['memory'], data['simple']['memory'], data['tool_use']['memory']])
# Add memory ratio annotations
for scenario_idx, scenario_name in enumerate(['startup', 'simple', 'tool_use']):
    scenario_data = data[scenario_name]['memory']
    for tool_idx in range(3):
        ratio = scenario_data[tool_idx] / scenario_data[0]
        if tool_idx > 0:  # Skip clawzero (baseline)
            bar = bars[tool_idx][scenario_idx]
            height = bar.get_height()
            ax4.text(bar.get_x() + bar.get_width()/2., height * 1.05,
                    f'{ratio:.0f}x', ha='center', va='bottom',
                    fontsize=7, style='italic', color='#e74c3c')
ax4.set_xticks([0, 1, 2])
ax4.set_xticklabels(['startup', 'simple', 'tool_use'], fontsize=10, color='#2c3e50')
ax4.set_ylim(0, max(data['startup']['memory'] + data['simple']['memory'] + data['tool_use']['memory']) * 1.25)
format_subplot(ax4, 'Peak Memory Usage', 'Memory (MB)')

# Overall title
fig.suptitle('Performance Benchmark: clawzero vs Claude Code vs OpenClaw\n(All using Sonnet 4.5 model)',
             fontsize=15, fontweight='bold', y=0.98, color='#2c3e50')

# Add footnote
fig.text(0.5, 0.01,
         'Environment: Docker (Ubuntu 24.04) • Tools: hyperfine, /usr/bin/time -v • Iterations: 5 • Lower is better',
         ha='center', fontsize=8, color='#7f8c8d', style='italic')

plt.tight_layout(rect=[0, 0.02, 1, 0.96])

# Save high-resolution PNG
output_file = output_dir / 'benchmark.png'
plt.savefig(output_file, dpi=150, bbox_inches='tight', facecolor='white', edgecolor='none')
plt.close()

print(f'✓ Generated comprehensive benchmark graph: {output_file}')
print(f'  File size: {output_file.stat().st_size / 1024:.1f} KB')
print(f'  Scenarios covered: startup, simple, tool_use')
print(f'  Metrics: startup time, E2E, TTFT, memory, relative performance')
