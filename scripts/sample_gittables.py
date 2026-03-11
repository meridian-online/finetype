#!/usr/bin/env python3
"""Sample GitTables parquet files and convert to CSV for LLM labelling.

Usage:
    python3 scripts/sample_gittables.py /home/hugh/datasets/gittables/topics data/csvs --max-files 500

Selects parquet files across multiple topics for diversity,
converts each to CSV (first 200 rows), and saves to the output directory.
"""

import os
import random
import sys

import pyarrow.parquet as pq


def main():
    if len(sys.argv) < 3:
        print("Usage: python3 sample_gittables.py <topics_dir> <output_dir> [--max-files N] [--seed S]")
        sys.exit(1)

    topics_dir = sys.argv[1]
    output_dir = sys.argv[2]

    max_files = 500
    seed = 42

    args = sys.argv[3:]
    i = 0
    while i < len(args):
        if args[i] == "--max-files":
            max_files = int(args[i + 1])
            i += 2
        elif args[i] == "--seed":
            seed = int(args[i + 1])
            i += 2
        else:
            print(f"Unknown option: {args[i]}")
            sys.exit(1)

    random.seed(seed)
    os.makedirs(output_dir, exist_ok=True)

    # Collect all topics
    topics = sorted([
        d for d in os.listdir(topics_dir)
        if os.path.isdir(os.path.join(topics_dir, d))
    ])
    print(f"Found {len(topics)} topics")

    # Collect parquet files per topic
    topic_files = {}
    for topic in topics:
        topic_path = os.path.join(topics_dir, topic)
        parquets = [
            os.path.join(topic_path, f)
            for f in os.listdir(topic_path)
            if f.endswith(".parquet")
        ]
        if parquets:
            topic_files[topic] = parquets

    print(f"Topics with parquet files: {len(topic_files)}")
    total_parquets = sum(len(v) for v in topic_files.values())
    print(f"Total parquet files: {total_parquets}")

    # Strategy: sample proportionally from each topic, with a minimum of 1 per topic
    # to ensure diversity
    selected = []

    if max_files >= len(topic_files):
        # At least 1 per topic, then distribute remainder
        per_topic_base = 1
        remainder = max_files - len(topic_files)

        for topic, files in topic_files.items():
            # Base selection
            sample_n = min(per_topic_base, len(files))
            picked = random.sample(files, sample_n)
            selected.extend([(topic, f) for f in picked])

        # Distribute remainder proportionally
        if remainder > 0:
            remaining_files = []
            for topic, files in topic_files.items():
                already_picked = [f for t, f in selected if t == topic]
                leftover = [f for f in files if f not in already_picked]
                remaining_files.extend([(topic, f) for f in leftover])

            random.shuffle(remaining_files)
            selected.extend(remaining_files[:remainder])
    else:
        # Fewer files than topics — just random sample across all
        all_files = [(t, f) for t, files in topic_files.items() for f in files]
        random.shuffle(all_files)
        selected = all_files[:max_files]

    print(f"Selected {len(selected)} files for conversion")

    # Convert to CSV
    converted = 0
    skipped = 0
    errors = 0

    for topic, parquet_path in selected:
        try:
            table = pq.read_table(parquet_path)

            # Skip tiny tables (< 3 rows) or tables with no columns
            if table.num_rows < 3 or table.num_columns == 0:
                skipped += 1
                continue

            # Take first 200 rows
            if table.num_rows > 200:
                table = table.slice(0, 200)

            # Generate output filename
            basename = os.path.splitext(os.path.basename(parquet_path))[0]
            safe_name = f"gt_{topic}_{basename}".replace(" ", "_")[:100]
            csv_path = os.path.join(output_dir, f"{safe_name}.csv")

            # Convert to pandas and save as CSV
            df = table.to_pandas()
            df.to_csv(csv_path, index=False)
            converted += 1

            if converted % 50 == 0:
                print(f"  Converted {converted}/{len(selected)}...")

        except Exception as e:
            errors += 1
            if errors <= 5:
                print(f"  Error: {os.path.basename(parquet_path)}: {e}")

    print(f"\nDone: {converted} converted, {skipped} skipped (too small), {errors} errors")
    print(f"Output: {output_dir}")

    # Count total columns
    total_cols = 0
    for f in os.listdir(output_dir):
        if f.endswith(".csv"):
            try:
                with open(os.path.join(output_dir, f)) as fh:
                    header = fh.readline().strip()
                    total_cols += len(header.split(","))
            except Exception:
                pass
    print(f"Estimated total columns: ~{total_cols}")


if __name__ == "__main__":
    main()
