#!/usr/bin/env python3
"""Compare two React DevTools profiling sessions (before/after).

Usage:
    python compare.py <before.json> <after.json>
    python compare.py <before.json> <after.json> --top 30
    python compare.py <before.json> <after.json> --json
"""

import argparse
import json
import sys
from pathlib import Path

from analyze import analyze, parse_profiling_data, print_table, ComponentStats


def compare_overall(before: dict, after: dict) -> list[list]:
    """Compare overall stats, return rows for display."""
    metrics = [
        ("Total commits", "total_commits", "", False),
        ("Total render time", "total_duration_ms", "ms", False),
        ("Avg commit", "avg_duration_ms", "ms", True),
        ("Median commit", "median_duration_ms", "ms", True),
        ("P95 commit", "p95_duration_ms", "ms", True),
        ("P99 commit", "p99_duration_ms", "ms", True),
        ("Max commit", "max_duration_ms", "ms", True),
        ("Dropped frames", "dropped_frames", "", True),
        ("Dropped %", "dropped_pct", "%", True),
        ("Components", "total_components", "", False),
        ("React Compiler", "compiled_with_forget", "", False),
    ]

    rows = []
    for label, key, unit, lower_better in metrics:
        bval = before.get(key, 0)
        aval = after.get(key, 0)

        if isinstance(bval, float):
            bstr = f"{bval:.2f}{unit}"
            astr = f"{aval:.2f}{unit}"
        else:
            bstr = f"{bval:,}{unit}"
            astr = f"{aval:,}{unit}"

        if bval != 0:
            change_pct = (aval - bval) / bval * 100
            sign = "+" if change_pct > 0 else ""
            change_str = f"{sign}{change_pct:.1f}%"

            if lower_better:
                if change_pct < -5:
                    indicator = "improved"
                elif change_pct > 5:
                    indicator = "REGRESSED"
                else:
                    indicator = "~same"
            else:
                indicator = ""
        else:
            change_str = "—"
            indicator = ""

        rows.append([label, bstr, astr, change_str, indicator])

    return rows


def match_components(
    before_comps: dict[int, ComponentStats],
    after_comps: dict[int, ComponentStats],
) -> list[tuple[str, ComponentStats | None, ComponentStats | None]]:
    """Match components by name between before/after sessions.
    Returns [(name, before_stats, after_stats)]."""

    # Aggregate by name (multiple fiber IDs may share a name)
    def aggregate_by_name(comps):
        by_name: dict[str, ComponentStats] = {}
        for c in comps.values():
            if c.name.startswith("Unknown#"):
                continue  # Skip unresolved fiber IDs
            if c.name not in by_name:
                by_name[c.name] = ComponentStats(name=c.name, fiber_id=c.fiber_id,
                                                  compiled_with_forget=c.compiled_with_forget)
            agg = by_name[c.name]
            agg.render_count += c.render_count
            agg.total_self_time += c.total_self_time
            agg.max_self_time = max(agg.max_self_time, c.max_self_time)
            agg.total_actual_time += c.total_actual_time
            agg.max_actual_time = max(agg.max_actual_time, c.max_actual_time)
        return by_name

    before_names = aggregate_by_name(before_comps)
    after_names = aggregate_by_name(after_comps)

    all_names = set(before_names.keys()) | set(after_names.keys())
    result = []
    for name in all_names:
        result.append((name, before_names.get(name), after_names.get(name)))

    return result


def main():
    parser = argparse.ArgumentParser(
        description="Compare two React DevTools profiling sessions.",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="Examples:\n"
               "  python compare.py before.json after.json\n"
               "  python compare.py before.json after.json --top 30\n",
    )
    parser.add_argument("before", help="Path to BEFORE profiling JSON")
    parser.add_argument("after", help="Path to AFTER profiling JSON")
    parser.add_argument("--top", type=int, default=20, help="Number of top components (default: 20)")
    parser.add_argument("--json", action="store_true", help="Output as JSON")
    args = parser.parse_args()

    for label, path_str in [("Before", args.before), ("After", args.after)]:
        if not Path(path_str).exists():
            print(f"Error: {label} file not found: {path_str}", file=sys.stderr)
            sys.exit(1)

    print("Loading BEFORE...", file=sys.stderr)
    before_data = parse_profiling_data(args.before)
    before_overall, before_comps, _ = analyze(before_data)

    print("Loading AFTER...", file=sys.stderr)
    after_data = parse_profiling_data(args.after)
    after_overall, after_comps, _ = analyze(after_data)

    if args.json:
        result = {
            "before": before_overall,
            "after": after_overall,
        }
        json.dump(result, sys.stdout, indent=2)
        print()
        return

    # Print comparison
    print("=" * 78)
    print("PROFILING COMPARISON: BEFORE vs AFTER")
    print("=" * 78)
    print(f"  Before: {Path(args.before).name}")
    print(f"  After:  {Path(args.after).name}")

    # Overall stats comparison
    print(f"\n{'OVERALL STATS':=^78}")
    overall_rows = compare_overall(before_overall, after_overall)
    print_table(
        ["Metric", "Before", "After", "Change", "Status"],
        overall_rows, "lrrrl",
    )

    # Duration distribution comparison
    print(f"\n{'COMMIT DURATION DISTRIBUTION':=^78}")
    bb = before_overall["duration_buckets"]
    ab = after_overall["duration_buckets"]
    bt = before_overall["total_commits"]
    at_ = after_overall["total_commits"]
    dist_rows = []
    for bucket in bb:
        bc = bb[bucket]
        ac = ab.get(bucket, 0)
        bpct = bc / bt * 100 if bt else 0
        apct = ac / at_ * 100 if at_ else 0
        delta = apct - bpct
        sign = "+" if delta > 0 else ""
        dist_rows.append([
            bucket, f"{bc:,} ({bpct:.1f}%)", f"{ac:,} ({apct:.1f}%)", f"{sign}{delta:.1f}pp",
        ])
    print_table(["Bucket", "Before", "After", "Change"], dist_rows, "lrrl")

    # Component comparison
    matched = match_components(before_comps, after_comps)

    # Sort by before total_self_time (biggest before components first)
    def sort_key(item):
        _, b, a = item
        return max(b.total_self_time if b else 0, a.total_self_time if a else 0)

    matched.sort(key=sort_key, reverse=True)

    # Components that improved
    print(f"\n{'COMPONENT CHANGES (by self-time)':=^78}")
    rows = []
    for name, b, a in matched[:args.top]:
        b_renders = b.render_count if b else 0
        a_renders = a.render_count if a else 0
        b_time = b.total_self_time if b else 0
        a_time = a.total_self_time if a else 0

        if b_renders > 0:
            render_change = (a_renders - b_renders) / b_renders * 100
            render_str = f"{render_change:+.0f}%"
        elif a_renders > 0:
            render_str = "NEW"
        else:
            render_str = "—"

        if b_time > 0:
            time_change = (a_time - b_time) / b_time * 100
            time_str = f"{time_change:+.0f}%"
        elif a_time > 0:
            time_str = "NEW"
        else:
            time_str = "—"

        if b_renders > 0 and a_renders == 0:
            status = "ELIMINATED"
        elif b_time > 0 and a_time < b_time * 0.5:
            status = "improved"
        elif b_time > 0 and a_time > b_time * 1.5:
            status = "REGRESSED"
        elif a_renders > 0 and b_renders == 0:
            status = "new"
        else:
            status = ""

        rows.append([
            name[:35],
            f"{b_renders}", f"{a_renders}", render_str,
            f"{b_time:.1f}", f"{a_time:.1f}", time_str,
            status,
        ])

    print_table(
        ["Component", "B.Renders", "A.Renders", "R.Change",
         "B.Time", "A.Time", "T.Change", "Status"],
        rows, "lrrrrrrl",
    )

    # Summary of eliminated components
    eliminated = [(name, b) for name, b, a in matched
                  if b and b.render_count > 0 and (not a or a.render_count == 0)]
    if eliminated:
        print(f"\n{'ELIMINATED COMPONENTS (rendered before, zero after)':=^78}")
        for name, b in sorted(eliminated, key=lambda x: x[1].total_self_time, reverse=True)[:15]:
            print(f"  {name}: was {b.render_count} renders, {b.total_self_time:.1f}ms total")

    # New hot components (not in before, or grew significantly)
    new_hot = [(name, a) for name, b, a in matched
               if a and a.total_self_time > 10 and (not b or b.render_count == 0)]
    if new_hot:
        print(f"\n{'NEW HOT COMPONENTS (not in before, >10ms)':=^78}")
        for name, a in sorted(new_hot, key=lambda x: x[1].total_self_time, reverse=True)[:15]:
            print(f"  {name}: {a.render_count} renders, {a.total_self_time:.1f}ms total")

    print()


if __name__ == "__main__":
    main()
