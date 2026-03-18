#!/usr/bin/env python3
"""Check distillation v2 progress and identify incomplete batches.

Usage:
    python3 scripts/distillation_status.py [output/distillation-v2/]

Shows which batches are complete, partial, or missing. Designed for
resume across Claude Code sessions on memory-constrained machines.
"""

import os
import sys


def main():
    output_dir = sys.argv[1] if len(sys.argv) > 1 else "output/distillation-v2"

    # Find all batch input files
    jsonl_files = sorted(
        f for f in os.listdir(output_dir)
        if f.startswith("batch_") and f.endswith(".jsonl")
    )

    if not jsonl_files:
        print(f"No batch_*.jsonl files in {output_dir}")
        sys.exit(1)

    complete = []
    partial = []
    missing = []
    total_columns = 0
    done_columns = 0

    for jsonl in jsonl_files:
        batch = jsonl.replace(".jsonl", "")
        jsonl_path = os.path.join(output_dir, jsonl)
        csv_path = os.path.join(output_dir, f"{batch}.csv")

        with open(jsonl_path) as f:
            expected = sum(1 for _ in f)
        total_columns += expected

        if os.path.exists(csv_path):
            with open(csv_path) as f:
                lines = sum(1 for _ in f)
            actual = lines - 1  # subtract header
            if actual >= expected:
                complete.append((batch, expected))
                done_columns += expected
            else:
                partial.append((batch, actual, expected))
                done_columns += actual
        else:
            missing.append((batch, expected))

    # Summary
    print(f"Distillation v2 Progress")
    print(f"{'=' * 50}")
    print(f"  Total batches:  {len(jsonl_files)}")
    print(f"  Complete:       {len(complete)}")
    print(f"  Partial:        {len(partial)}")
    print(f"  Not started:    {len(missing)}")
    print(f"  Columns:        {done_columns}/{total_columns} ({done_columns*100//total_columns if total_columns else 0}%)")
    print()

    if partial:
        print("Partial batches (will be re-processed):")
        for batch, actual, expected in partial:
            print(f"  {batch}: {actual}/{expected} columns")
        print()

    if missing:
        # Sort by batch number and group consecutive ranges
        missing.sort(key=lambda x: int(x[0].split("_")[1]))
        ranges: list[tuple[int, int]] = []
        start_num: int | None = None
        prev_num: int = 0
        for batch, _cols in missing:
            num = int(batch.split("_")[1])
            if start_num is None:
                start_num = num
                prev_num = num
            elif num == prev_num + 1:
                prev_num = num
            else:
                ranges.append((start_num, prev_num))
                start_num = num
                prev_num = num
        if start_num is not None:
            ranges.append((start_num, prev_num))

        print("Not started:")
        for s, e in ranges:
            if s == e:
                print(f"  batch_{s:02d}")
            else:
                total_in_range = sum(
                    c for b, c in missing
                    if s <= int(b.split("_")[1]) <= e
                )
                print(f"  batch_{s:02d}..batch_{e:02d} ({e-s+1} batches, {total_in_range} columns)")
        print()

    # Output next wave recommendation
    remaining = list(missing) + [(b, e) for b, _, e in partial]
    remaining.sort(key=lambda x: int(x[0].split("_")[1]))
    if remaining:
        # Recommend wave size based on available memory
        try:
            with open("/proc/meminfo") as f:
                for line in f:
                    if line.startswith("MemAvailable:"):
                        mem_gb = int(line.split()[1]) / (1024 * 1024)
                        break
                else:
                    mem_gb = 8
        except Exception:
            mem_gb = 8

        # ~500MB per agent is conservative
        max_parallel = max(2, min(int(mem_gb / 1.5), 5))
        wave_size = min(max_parallel, len(remaining))

        print(f"Recommendation (available RAM: {mem_gb:.1f} GB):")
        print(f"  Max parallel agents: {max_parallel}")
        print(f"  Next wave: {wave_size} batches")
        next_batches = [b for b, _ in remaining[:wave_size]]
        print(f"  Batches: {', '.join(next_batches)}")
        next_cols = sum(c for _, c in remaining[:wave_size])
        print(f"  Columns: {next_cols}")
        print(f"  Est. tokens: ~{next_cols * 800:,}")
    else:
        print("✓ All batches complete!")
        print(f"  Merge: python3 scripts/merge_distillation.py {output_dir}")


if __name__ == "__main__":
    main()
