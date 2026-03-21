#!/usr/bin/env python3
"""Distillation v3 orchestrator — manages batch state and resume.

Usage:
    python3 scripts/distill_run.py status                    # Show progress
    python3 scripts/distill_run.py plan [--batch-size N]     # Generate batch plan
    python3 scripts/distill_run.py next [--count N]          # Print next N pending batches

This script manages batch state only. The actual distillation runs via
Claude Code agents spawned by the lead agent (Nightingale). Each agent
reads its batch assignment, performs blind-first adjudication, and writes
output CSV + .done marker.

Batch state is tracked via .done marker files in output/distillation-v3/.
Resume is safe: re-running skips completed batches.

Batch naming: {source}_batch_{NNN}.csv / {source}_batch_{NNN}.done
"""

import json
import os
import sys
from collections import defaultdict

DEST = "output/distillation-v3"
DEFAULT_BATCH_SIZE = 100

SOURCES = {
    "sherlock": os.path.join(DEST, "sherlock_test.jsonl"),
    "sotab": os.path.join(DEST, "sotab_round2.jsonl"),
    "gittables": os.path.join(DEST, "gittables_sample.jsonl"),
    "eval": os.path.join(DEST, "eval_columns.jsonl"),
}

# Execution order per spec
SOURCE_ORDER = ["sherlock", "sotab", "gittables", "eval"]


def count_lines(path):
    """Count lines in a file efficiently."""
    if not os.path.exists(path):
        return 0
    count = 0
    with open(path, "rb") as f:
        for _ in f:
            count += 1
    return count


def get_batch_plan(batch_size=DEFAULT_BATCH_SIZE):
    """Generate list of all batches with offset/limit."""
    batches = []
    for source in SOURCE_ORDER:
        jsonl_path = SOURCES[source]
        total = count_lines(jsonl_path)
        if total == 0:
            continue
        num_batches = (total + batch_size - 1) // batch_size
        for i in range(num_batches):
            offset = i * batch_size
            limit = min(batch_size, total - offset)
            batch_id = f"{source}_batch_{i:04d}"
            batches.append({
                "batch_id": batch_id,
                "source": source,
                "jsonl_path": jsonl_path,
                "offset": offset,
                "limit": limit,
                "total": total,
            })
    return batches


def get_completed_batches():
    """Find all .done marker files."""
    completed = set()
    if not os.path.exists(DEST):
        return completed
    for fname in os.listdir(DEST):
        if fname.endswith(".done"):
            batch_id = fname[:-5]  # strip .done
            completed.add(batch_id)
    return completed


def cmd_status(batch_size=DEFAULT_BATCH_SIZE):
    """Show progress summary."""
    batches = get_batch_plan(batch_size)
    completed = get_completed_batches()

    by_source = defaultdict(lambda: {"total": 0, "done": 0, "columns": 0, "columns_done": 0})
    for b in batches:
        s = by_source[b["source"]]
        s["total"] += 1
        s["columns"] += b["limit"]
        if b["batch_id"] in completed:
            s["done"] += 1
            s["columns_done"] += b["limit"]

    print(f"Distillation v3 Progress (batch_size={batch_size})")
    print(f"{'=' * 70}")
    print(f"{'Source':<12} {'Batches':>10} {'Done':>8} {'Remaining':>10} {'Columns':>10} {'%':>6}")
    print(f"{'-' * 70}")
    total_batches = 0
    total_done = 0
    total_cols = 0
    total_cols_done = 0
    for source in SOURCE_ORDER:
        s = by_source[source]
        remaining = s["total"] - s["done"]
        pct = 100 * s["done"] / s["total"] if s["total"] > 0 else 0
        print(f"{source:<12} {s['total']:>10,} {s['done']:>8,} {remaining:>10,} {s['columns']:>10,} {pct:>5.1f}%")
        total_batches += s["total"]
        total_done += s["done"]
        total_cols += s["columns"]
        total_cols_done += s["columns_done"]
    print(f"{'-' * 70}")
    remaining = total_batches - total_done
    pct = 100 * total_done / total_batches if total_batches > 0 else 0
    print(f"{'TOTAL':<12} {total_batches:>10,} {total_done:>8,} {remaining:>10,} {total_cols:>10,} {pct:>5.1f}%")


def cmd_plan(batch_size=DEFAULT_BATCH_SIZE):
    """Show full batch plan."""
    batches = get_batch_plan(batch_size)
    completed = get_completed_batches()

    for b in batches:
        status = "DONE" if b["batch_id"] in completed else "PENDING"
        print(f"{status:7s} {b['batch_id']:30s} offset={b['offset']:>7,} limit={b['limit']:>4} source={b['source']}")

    pending = [b for b in batches if b["batch_id"] not in completed]
    print(f"\n{len(batches)} total batches, {len(batches) - len(pending)} done, {len(pending)} pending")


def cmd_next(count=5, batch_size=DEFAULT_BATCH_SIZE, source=None):
    """Print next N pending batches as JSON (for agent consumption)."""
    batches = get_batch_plan(batch_size)
    completed = get_completed_batches()
    pending = [b for b in batches if b["batch_id"] not in completed]
    if source:
        pending = [b for b in pending if b["source"] == source]

    for b in pending[:count]:
        print(json.dumps(b))

    if not pending:
        print("ALL BATCHES COMPLETE", file=sys.stderr)
    else:
        print(f"\n{len(pending)} batches remaining ({len(pending[:count])} shown)",
              file=sys.stderr)


def main():
    args = sys.argv[1:]
    if not args:
        cmd_status()
        return

    command = args[0]
    batch_size = DEFAULT_BATCH_SIZE

    # Parse --batch-size from remaining args
    i = 1
    count = 5
    source = None
    while i < len(args):
        if args[i] == "--batch-size":
            batch_size = int(args[i + 1])
            i += 2
        elif args[i] == "--count":
            count = int(args[i + 1])
            i += 2
        elif args[i] == "--source":
            source = args[i + 1]
            i += 2
        else:
            i += 1

    if command == "status":
        cmd_status(batch_size)
    elif command == "plan":
        cmd_plan(batch_size)
    elif command == "next":
        cmd_next(count, batch_size, source=source)
    elif command in ("-h", "--help", "help"):
        print(__doc__)
    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        print("Commands: status, plan, next", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
