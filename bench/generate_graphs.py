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
plt.rcParams['font.size'] = 11
plt.rcParams['axes.labelsize'] = 12
plt.rcParams['axes.titlesize'] = 14
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

# Benchmark data
tools = ['clawzero', 'Claude Code', 'OpenClaw']
colors = [COLORS['clawzero'], COLORS['claude-code'], COLORS['openclaw']]

# Metrics (from actual benchmark results)
startup_time = [2.16, 838.18, 1424.61]      # milliseconds
e2e_time = [2100.19, 4673.81, 18718.04]     # milliseconds
ttft = [1689.21, 3232.39, 9610.64]          # milliseconds
memory = [10.1, 242.1, 398.0]               # megabytes

output_dir = Path(__file__).parent.parent / 'docs' / 'img' / 'bench'
output_dir.mkdir(parents=True, exist_ok=True)

def add_value_labels(ax, bars, values, format_str='{:.0f}'):
    """Add value labels on top of bars"""
    for bar, value in zip(bars, values):
        height = bar.get_height()
        ax.text(bar.get_x() + bar.get_width()/2., height,
                format_str.format(value),
                ha='center', va='bottom', fontweight='bold', fontsize=10, color='#2c3e50')

def format_subplot(ax, title, ylabel):
    """Apply consistent formatting to subplot"""
    ax.set_title(title, fontsize=13, fontweight='bold', pad=12, color='#2c3e50')
    ax.set_ylabel(ylabel, fontsize=11, fontweight='600', color='#34495e')
    ax.set_xlabel('')
    ax.spines['top'].set_visible(False)
    ax.spines['right'].set_visible(False)
    ax.spines['left'].set_color('#95a5a6')
    ax.spines['bottom'].set_color('#95a5a6')
    ax.tick_params(colors='#34495e', which='both')
    ax.yaxis.grid(True, alpha=0.3, linestyle='--', linewidth=0.8, color='#bdc3c7')
    ax.set_axisbelow(True)

# Create 2x2 small multiples layout
fig = plt.figure(figsize=(14, 10))
fig.patch.set_facecolor('white')

# Startup Time (top-left)
ax1 = plt.subplot(2, 2, 1)
x_pos = np.arange(len(tools))
bars1 = ax1.bar(x_pos, startup_time, color=colors, alpha=0.85, edgecolor='white', linewidth=2.5, width=0.6)
add_value_labels(ax1, bars1, startup_time, '{:.1f}')
ax1.set_xticks(x_pos)
ax1.set_xticklabels(tools, fontsize=11, color='#2c3e50')
ax1.set_ylim(0, max(startup_time) * 1.2)
format_subplot(ax1, 'Startup Time (--help)', 'Time (milliseconds)')

# TTFT (top-right)
ax2 = plt.subplot(2, 2, 2)
bars2 = ax2.bar(x_pos, ttft, color=colors, alpha=0.85, edgecolor='white', linewidth=2.5, width=0.6)
add_value_labels(ax2, bars2, ttft, '{:.0f}')
ax2.set_xticks(x_pos)
ax2.set_xticklabels(tools, fontsize=11, color='#2c3e50')
ax2.set_ylim(0, max(ttft) * 1.2)
format_subplot(ax2, 'Time to First Token (TTFT)', 'Time (milliseconds)')

# E2E Response Time (bottom-left)
ax3 = plt.subplot(2, 2, 3)
bars3 = ax3.bar(x_pos, e2e_time, color=colors, alpha=0.85, edgecolor='white', linewidth=2.5, width=0.6)
add_value_labels(ax3, bars3, e2e_time, '{:.0f}')
ax3.set_xticks(x_pos)
ax3.set_xticklabels(tools, fontsize=11, color='#2c3e50')
ax3.set_ylim(0, max(e2e_time) * 1.2)
format_subplot(ax3, 'End-to-End Time ("What is 1+1?")', 'Time (milliseconds)')

# Memory Usage (bottom-right)
ax4 = plt.subplot(2, 2, 4)
bars4 = ax4.bar(x_pos, memory, color=colors, alpha=0.85, edgecolor='white', linewidth=2.5, width=0.6)
add_value_labels(ax4, bars4, memory, '{:.1f}')
ax4.set_xticks(x_pos)
ax4.set_xticklabels(tools, fontsize=11, color='#2c3e50')
ax4.set_ylim(0, max(memory) * 1.2)
format_subplot(ax4, 'Peak Memory Usage', 'Memory (megabytes)')

# Overall title
fig.suptitle('Performance Comparison: clawzero vs Claude Code vs OpenClaw\n(All using Sonnet 4.5 model)',
             fontsize=16, fontweight='bold', y=0.98, color='#2c3e50')

# Add footnote
fig.text(0.5, 0.01, 'Environment: Docker (Ubuntu 24.04) • Measured with hyperfine & /usr/bin/time -v • 5 iterations',
         ha='center', fontsize=9, color='#7f8c8d', style='italic')

plt.tight_layout(rect=[0, 0.02, 1, 0.96])

# Save high-resolution PNG
output_file = output_dir / 'benchmark.png'
plt.savefig(output_file, dpi=150, bbox_inches='tight', facecolor='white', edgecolor='none')
plt.close()

print(f'✓ Generated professional benchmark graph: {output_file}')
print(f'  File size: {output_file.stat().st_size / 1024:.1f} KB')
print(f'  Resolution: 2100x1500 pixels (150 DPI)')
