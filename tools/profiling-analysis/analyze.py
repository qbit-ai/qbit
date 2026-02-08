#!/usr/bin/env python3
"""Analyze React DevTools Profiler v5 JSON exports.

Usage:
    python analyze.py <profiling-data.json>
    python analyze.py <profiling-data.json> --top 50
    python analyze.py <profiling-data.json> --json          # machine-readable output
    python analyze.py <profiling-data.json> --csv components # dump component stats
    python analyze.py <profiling-data.json> --csv commits    # dump commit timeline
"""

import argparse
import csv
import json
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class ComponentStats:
    name: str
    fiber_id: int
    render_count: int = 0
    total_self_time: float = 0.0
    max_self_time: float = 0.0
    total_actual_time: float = 0.0
    max_actual_time: float = 0.0
    compiled_with_forget: bool = False


@dataclass
class CommitInfo:
    index: int
    timestamp: float
    duration: float
    priority_level: str
    num_components: int
    top_components: list = field(default_factory=list)  # [(name, self_time)]


def parse_profiling_data(path: str) -> dict:
    """Parse the profiling JSON. Returns the raw data."""
    with open(path, "r") as f:
        return json.load(f)


def iter_pairs(data) -> list[tuple[int, any]]:
    """Normalize profiler v5 map data: handles both [[k,v],...] lists and {k:v} dicts."""
    if isinstance(data, dict):
        return [(int(k), v) for k, v in data.items()]
    if isinstance(data, list):
        return [(int(pair[0]), pair[1]) for pair in data if isinstance(pair, list) and len(pair) >= 2]
    return []


def build_snapshot_map(data: dict) -> dict[int, dict]:
    """Build fiber_id -> snapshot metadata map across all roots."""
    snapshots = {}
    for root in data.get("dataForRoots", []):
        for fid, snap in iter_pairs(root.get("snapshots", [])):
            name = snap.get("displayName") or snap.get("name")
            snap["name"] = name  # normalize to "name" key
            snapshots[fid] = snap
    return snapshots


def analyze(data: dict) -> tuple[dict, dict[int, ComponentStats], list[CommitInfo]]:
    """Analyze profiling data. Returns (overall_stats, component_stats, commits)."""
    snapshots = build_snapshot_map(data)
    components: dict[int, ComponentStats] = {}
    commits: list[CommitInfo] = []

    total_duration = 0.0
    all_durations = []

    for root in data.get("dataForRoots", []):
        for ci, commit in enumerate(root.get("commitData", [])):
            self_durations = iter_pairs(commit.get("fiberSelfDurations", []))
            actual_durations = iter_pairs(commit.get("fiberActualDurations", []))
            timestamp = commit.get("timestamp", 0)
            priority = commit.get("priorityLevel", "unknown")

            commit_duration = 0.0
            commit_components = []

            # Process self durations
            for fid, dur in self_durations:
                dur = float(dur)

                if fid not in components:
                    snap = snapshots.get(fid, {})
                    name = snap.get("name") or f"Unknown#{fid}"
                    components[fid] = ComponentStats(
                        name=name,
                        fiber_id=fid,
                        compiled_with_forget=snap.get("compiledWithForget", False),
                    )

                comp = components[fid]
                if dur > 0:
                    comp.render_count += 1
                    comp.total_self_time += dur
                    comp.max_self_time = max(comp.max_self_time, dur)
                    commit_components.append((comp.name, dur))

                commit_duration += dur

            # Process actual (inclusive) durations
            for fid, dur in actual_durations:
                dur = float(dur)
                if fid in components:
                    comp = components[fid]
                    comp.total_actual_time += dur
                    comp.max_actual_time = max(comp.max_actual_time, dur)

            total_duration += commit_duration
            all_durations.append(commit_duration)

            # Sort components by self-time descending for this commit
            commit_components.sort(key=lambda x: x[1], reverse=True)

            commits.append(
                CommitInfo(
                    index=ci,
                    timestamp=timestamp,
                    duration=commit_duration,
                    priority_level=str(priority),
                    num_components=len(commit_components),
                    top_components=commit_components[:5],
                )
            )

    all_durations.sort()
    n = len(all_durations)

    def percentile(p):
        if n == 0:
            return 0
        idx = int(p / 100 * n)
        return all_durations[min(idx, n - 1)]

    dropped = sum(1 for d in all_durations if d > 16.67)

    # Duration distribution buckets
    buckets = {"0ms": 0, "1ms": 0, "2ms": 0, "3-5ms": 0, "6-10ms": 0,
               "11-16ms": 0, "17-33ms": 0, "34+ms": 0}
    for d in all_durations:
        if d < 0.5:
            buckets["0ms"] += 1
        elif d < 1.5:
            buckets["1ms"] += 1
        elif d < 2.5:
            buckets["2ms"] += 1
        elif d < 5.5:
            buckets["3-5ms"] += 1
        elif d < 10.5:
            buckets["6-10ms"] += 1
        elif d < 16.68:
            buckets["11-16ms"] += 1
        elif d < 33.5:
            buckets["17-33ms"] += 1
        else:
            buckets["34+ms"] += 1

    # Expensive commit clusters (3+ consecutive commits > 10ms)
    clusters = []
    current_cluster = []
    for commit in commits:
        if commit.duration > 10:
            current_cluster.append(commit)
        else:
            if len(current_cluster) >= 3:
                clusters.append(current_cluster)
            current_cluster = []
    if len(current_cluster) >= 3:
        clusters.append(current_cluster)

    compiled_count = sum(1 for c in components.values() if c.compiled_with_forget)

    overall = {
        "total_commits": n,
        "total_duration_ms": round(total_duration, 2),
        "avg_duration_ms": round(total_duration / n, 2) if n else 0,
        "median_duration_ms": round(percentile(50), 2),
        "max_duration_ms": round(max(all_durations) if all_durations else 0, 2),
        "p95_duration_ms": round(percentile(95), 2),
        "p99_duration_ms": round(percentile(99), 2),
        "dropped_frames": dropped,
        "dropped_pct": round(dropped / n * 100, 2) if n else 0,
        "total_components": len(components),
        "compiled_with_forget": compiled_count,
        "duration_buckets": buckets,
        "clusters": [
            {
                "start_commit": cl[0].index,
                "end_commit": cl[-1].index,
                "size": len(cl),
                "total_ms": round(sum(c.duration for c in cl), 2),
                "avg_ms": round(sum(c.duration for c in cl) / len(cl), 2),
            }
            for cl in clusters
        ],
    }

    return overall, components, commits


def print_table(headers: list[str], rows: list[list], alignments: str = ""):
    """Print a formatted table. alignments: 'l'=left, 'r'=right per column."""
    if not rows:
        return
    col_widths = [len(h) for h in headers]
    str_rows = [[str(c) for c in row] for row in rows]
    for row in str_rows:
        for i, cell in enumerate(row):
            col_widths[i] = max(col_widths[i], len(cell))

    def fmt_row(cells):
        parts = []
        for i, cell in enumerate(cells):
            align = alignments[i] if i < len(alignments) else "r"
            width = col_widths[i]
            parts.append(cell.ljust(width) if align == "l" else cell.rjust(width))
        return " | ".join(parts)

    header_line = fmt_row(headers)
    sep = "-+-".join("-" * w for w in col_widths)
    print(header_line)
    print(sep)
    for row in str_rows:
        print(fmt_row(row))


def print_report(overall: dict, components: dict[int, ComponentStats],
                 commits: list[CommitInfo], top_n: int):
    """Print human-readable report."""
    print("=" * 70)
    print("REACT DEVTOOLS PROFILING ANALYSIS")
    print("=" * 70)

    # Overall stats
    print(f"\n{'OVERALL STATS':=^70}")
    print(f"  Total commits:       {overall['total_commits']:,}")
    print(f"  Total render time:   {overall['total_duration_ms']:,.2f} ms")
    print(f"  Avg commit:          {overall['avg_duration_ms']:.2f} ms")
    print(f"  Median commit:       {overall['median_duration_ms']:.2f} ms")
    print(f"  P95 commit:          {overall['p95_duration_ms']:.2f} ms")
    print(f"  P99 commit:          {overall['p99_duration_ms']:.2f} ms")
    print(f"  Max commit:          {overall['max_duration_ms']:.2f} ms")
    print(f"  Dropped frames:      {overall['dropped_frames']} ({overall['dropped_pct']:.1f}%)")
    print(f"  Components:          {overall['total_components']}")
    print(f"  React Compiler:      {overall['compiled_with_forget']} compiled")

    # Duration distribution
    print(f"\n{'COMMIT DURATION DISTRIBUTION':=^70}")
    total = overall["total_commits"]
    max_count = max(overall["duration_buckets"].values()) if total else 1
    for bucket, count in overall["duration_buckets"].items():
        pct = count / total * 100 if total else 0
        bar_len = int(count / max_count * 30) if max_count else 0
        bar = "#" * bar_len
        flag = " !!!" if bucket in ("17-33ms", "34+ms") and count > 0 else ""
        print(f"  {bucket:>7s}: {count:>6,} ({pct:5.1f}%) {bar}{flag}")

    # Top components by self-time
    by_self_time = sorted(components.values(), key=lambda c: c.total_self_time, reverse=True)
    print(f"\n{'TOP COMPONENTS BY SELF-TIME':=^70}")
    rows = []
    for c in by_self_time[:top_n]:
        avg = c.total_self_time / c.render_count if c.render_count else 0
        compiler = "yes" if c.compiled_with_forget else ""
        rows.append([
            c.name[:40], str(c.render_count), f"{c.total_self_time:.1f}",
            f"{avg:.2f}", f"{c.max_self_time:.1f}", compiler,
        ])
    print_table(
        ["Component", "Renders", "Total(ms)", "Avg(ms)", "Max(ms)", "Compiler"],
        rows, "lrrrrr",
    )

    # Top components by render count
    by_renders = sorted(components.values(), key=lambda c: c.render_count, reverse=True)
    print(f"\n{'TOP COMPONENTS BY RENDER COUNT':=^70}")
    rows = []
    for c in by_renders[:top_n]:
        avg = c.total_self_time / c.render_count if c.render_count else 0
        rows.append([
            c.name[:40], str(c.render_count), f"{c.total_self_time:.1f}", f"{avg:.2f}",
        ])
    print_table(["Component", "Renders", "Total(ms)", "Avg(ms)"], rows, "lrrr")

    # Expensive commits
    expensive = sorted(commits, key=lambda c: c.duration, reverse=True)[:20]
    if expensive and expensive[0].duration > 0:
        print(f"\n{'TOP 20 MOST EXPENSIVE COMMITS':=^70}")
        rows = []
        for c in expensive:
            top = c.top_components[0] if c.top_components else ("—", 0)
            rows.append([
                str(c.index), f"{c.duration:.1f}", f"{c.timestamp:.0f}",
                top[0][:30], f"{top[1]:.1f}",
            ])
        print_table(
            ["Commit#", "Duration(ms)", "Timestamp", "Top Component", "Self(ms)"],
            rows, "rrrll",
        )

    # Clusters
    if overall["clusters"]:
        print(f"\n{'EXPENSIVE COMMIT CLUSTERS (3+ consecutive >10ms)':=^70}")
        for i, cl in enumerate(overall["clusters"], 1):
            print(f"  Cluster {i}: commits {cl['start_commit']}-{cl['end_commit']}"
                  f" ({cl['size']} commits, {cl['total_ms']:.1f}ms total, {cl['avg_ms']:.1f}ms avg)")

    print()


def export_csv_components(components: dict[int, ComponentStats], out):
    """Export component stats to CSV."""
    writer = csv.writer(out)
    writer.writerow([
        "component", "fiber_id", "renders", "total_self_ms", "avg_self_ms",
        "max_self_ms", "total_actual_ms", "max_actual_ms", "compiled_with_forget",
    ])
    for c in sorted(components.values(), key=lambda c: c.total_self_time, reverse=True):
        avg = c.total_self_time / c.render_count if c.render_count else 0
        writer.writerow([
            c.name, c.fiber_id, c.render_count,
            round(c.total_self_time, 2), round(avg, 2), round(c.max_self_time, 2),
            round(c.total_actual_time, 2), round(c.max_actual_time, 2),
            c.compiled_with_forget,
        ])


def export_csv_commits(commits: list[CommitInfo], out):
    """Export commit timeline to CSV."""
    writer = csv.writer(out)
    writer.writerow([
        "commit_index", "timestamp", "duration_ms", "priority",
        "num_components", "exceeds_frame_budget", "top_component", "top_self_ms",
    ])
    for c in commits:
        top = c.top_components[0] if c.top_components else ("", 0)
        writer.writerow([
            c.index, round(c.timestamp, 2), round(c.duration, 2), c.priority_level,
            c.num_components, c.duration > 16.67, top[0], round(top[1], 2),
        ])


def main():
    parser = argparse.ArgumentParser(
        description="Analyze React DevTools Profiler v5 JSON exports.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="Examples:\n"
               "  python analyze.py profiling-data.json\n"
               "  python analyze.py profiling-data.json --top 50\n"
               "  python analyze.py profiling-data.json --json\n"
               "  python analyze.py profiling-data.json --csv components > stats.csv\n",
    )
    parser.add_argument("file", help="Path to profiling JSON file")
    parser.add_argument("--top", type=int, default=20, help="Number of top components to show (default: 20)")
    parser.add_argument("--json", action="store_true", help="Output overall stats as JSON")
    parser.add_argument("--csv", choices=["components", "commits"], help="Export CSV to stdout")
    args = parser.parse_args()

    path = Path(args.file)
    if not path.exists():
        print(f"Error: {path} not found", file=sys.stderr)
        sys.exit(1)

    size_mb = path.stat().st_size / 1024 / 1024
    print(f"Loading {path.name} ({size_mb:.1f} MB)...", file=sys.stderr)

    data = parse_profiling_data(str(path))
    overall, components, commits = analyze(data)

    if args.json:
        json.dump(overall, sys.stdout, indent=2)
        print()
    elif args.csv == "components":
        export_csv_components(components, sys.stdout)
    elif args.csv == "commits":
        export_csv_commits(commits, sys.stdout)
    else:
        print_report(overall, components, commits, args.top)


if __name__ == "__main__":
    main()
