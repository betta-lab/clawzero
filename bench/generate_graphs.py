#!/usr/bin/env python3
"""
Generate beautiful benchmark graphs for README
"""
import matplotlib.pyplot as plt
import seaborn as sns
import numpy as np
from pathlib import Path

# Set style
sns.set_theme(style="whitegrid", palette="muted")
plt.rcParams['font.family'] = 'sans-serif'
plt.rcParams['font.size'] = 11
plt.rcParams['axes.labelsize'] = 12
plt.rcParams['axes.titlesize'] = 14
plt.rcParams['axes.titleweight'] = 'bold'

# Data from benchmark results
tools = ['clawzero', 'Claude Code', 'OpenClaw']
colors = ['#10b981', '#6366f1', '#f59e0b']  # Green, Indigo, Amber

# Metrics
startup_time = [2.16, 838.18, 1424.61]
e2e_simple = [2100.19, 4673.81, 18718.04]
ttft_simple = [1689.21, 3232.39, 9610.64]
memory_simple = [10.1, 242.1, 398.0]

output_dir = Path(__file__).parent.parent / 'docs' / 'img' / 'bench'
output_dir.mkdir(parents=True, exist_ok=True)

def create_bar_chart(data, title, ylabel, filename, log_scale=False):
    """Create a beautiful bar chart"""
    fig, ax = plt.subplots(figsize=(10, 6))

    x_pos = np.arange(len(tools))
    bars = ax.bar(x_pos, data, color=colors, alpha=0.85, edgecolor='white', linewidth=2)

    # Add value labels on bars
    for i, (bar, value) in enumerate(zip(bars, data)):
        height = bar.get_height()
        if log_scale:
            ax.text(bar.get_x() + bar.get_width()/2., height * 1.1,
                   f'{value:.1f}',
                   ha='center', va='bottom', fontweight='bold', fontsize=10)
        else:
            ax.text(bar.get_x() + bar.get_width()/2., height + max(data) * 0.02,
                   f'{value:.1f}',
                   ha='center', va='bottom', fontweight='bold', fontsize=10)

    ax.set_xlabel('Tool', fontweight='bold', fontsize=13)
    ax.set_ylabel(ylabel, fontweight='bold', fontsize=13)
    ax.set_title(title, fontsize=16, fontweight='bold', pad=20)
    ax.set_xticks(x_pos)
    ax.set_xticklabels(tools, fontsize=12)

    if log_scale:
        ax.set_yscale('log')
        ax.yaxis.grid(True, alpha=0.3, linestyle='--')
    else:
        ax.yaxis.grid(True, alpha=0.3)

    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    ax.set_axisbelow(True)

    plt.tight_layout()
    plt.savefig(output_dir / filename, dpi=150, bbox_inches='tight', facecolor='white')
    plt.close()
    print(f'✓ Generated {filename}')

# Generate all charts
create_bar_chart(
    startup_time,
    '🚀 Startup Time (--help, no API calls)',
    'Time (milliseconds)',
    'startup_time.png',
    log_scale=True
)

create_bar_chart(
    e2e_simple,
    '⚡ End-to-End Response Time ("What is 1+1?")',
    'Time (milliseconds)',
    'e2e_time.png',
    log_scale=True
)

create_bar_chart(
    ttft_simple,
    '⏱️ Time to First Token',
    'Time (milliseconds)',
    'ttft.png',
    log_scale=True
)

create_bar_chart(
    memory_simple,
    '💾 Peak Memory Usage',
    'Memory (megabytes)',
    'memory.png',
    log_scale=True
)

print(f'\n✅ All graphs generated in {output_dir}/')
