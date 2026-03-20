#!/usr/bin/env python3
"""Extract GitTables columns into JSONL for FineType distillation v3.

Usage:
    python3 scripts/extract_gittables.py [OPTIONS]

Options:
    --source DIR          GitTables parquet directory (default: ~/datasets/gittables/parquet/)
    --dest DIR            Output directory (default: output/distillation-v3/)
    --files-per-topic N   Files to sample per topic (default: 50)
    --seed N              Random seed (default: 42)
    --max-values N        Max sample values per column (default: 20)
    --topics T1,T2,...    Comma-separated topic filter (default: all)

Input:  GitTables parquet files organized by topic subdirectories
Output: <dest>/gittables_sample.jsonl

Each JSONL record:
    {"source": "gittables", "source_file": "topic/file.parquet",
     "column_name": "col", "values": [...], "topic": "topic",
     "ground_truth_label": "", "ground_truth_source": ""}
"""

import json
import os
import random
import sys
import time

try:
    import pyarrow.parquet as pq
except ImportError:
    print("ERROR: pyarrow is required. Install with: pip install pyarrow", file=sys.stderr)
    sys.exit(1)


DEFAULT_SOURCE = os.path.expanduser("~/datasets/gittables/parquet/")
DEFAULT_DEST = "output/distillation-v3/"
DEFAULT_FILES_PER_TOPIC = 50
DEFAULT_SEED = 42
DEFAULT_MAX_VALUES = 20
MAX_ROWS_READ = 500
MIN_NON_NULL_VALUES = 3


def discover_topics(source_dir, topic_filter=None):
    """List topic subdirectories, sorted alphabetically."""
    topics = sorted(
        d for d in os.listdir(source_dir)
        if os.path.isdir(os.path.join(source_dir, d))
    )
    if topic_filter:
        allowed = set(topic_filter)
        topics = [t for t in topics if t in allowed]
    return topics


def sample_files(topic_dir, files_per_topic, rng):
    """List and sample parquet files from a topic directory."""
    files = sorted(
        f for f in os.listdir(topic_dir)
        if f.endswith(".parquet")
    )
    if not files:
        return []
    k = min(files_per_topic, len(files))
    return rng.sample(files, k)


def extract_columns_from_parquet(filepath, max_values):
    """Extract column names and sample values from a parquet file.

    Reads only the first MAX_ROWS_READ rows for performance.
    Returns list of (column_name, values) tuples.
    """
    columns = []
    try:
        pf = pq.ParquetFile(filepath)
        # Read only first batch of rows for performance
        batch_iter = pf.iter_batches(batch_size=MAX_ROWS_READ)
        try:
            first_batch = next(batch_iter)
        except StopIteration:
            return columns

        table = first_batch.to_pydict()

        for col_name, col_values in table.items():
            # Collect non-null values, convert to string
            sampled = []
            for v in col_values:
                if v is None:
                    continue
                try:
                    s = str(v)
                    # Skip empty strings and overly long values
                    if s and len(s) <= 1000:
                        sampled.append(s)
                except Exception:
                    continue
                if len(sampled) >= max_values:
                    break

            if len(sampled) >= MIN_NON_NULL_VALUES:
                columns.append((col_name, sampled))

    except Exception as e:
        print(f"  WARNING: error reading {filepath}: {e}", file=sys.stderr)

    return columns


def main():
    args = sys.argv[1:]
    source_dir = DEFAULT_SOURCE
    dest_dir = DEFAULT_DEST
    files_per_topic = DEFAULT_FILES_PER_TOPIC
    seed = DEFAULT_SEED
    max_values = DEFAULT_MAX_VALUES
    topic_filter = None

    i = 0
    while i < len(args):
        if args[i] == "--source":
            source_dir = os.path.expanduser(args[i + 1])
            i += 2
        elif args[i] == "--dest":
            dest_dir = args[i + 1]
            i += 2
        elif args[i] == "--files-per-topic":
            files_per_topic = int(args[i + 1])
            i += 2
        elif args[i] == "--seed":
            seed = int(args[i + 1])
            i += 2
        elif args[i] == "--max-values":
            max_values = int(args[i + 1])
            i += 2
        elif args[i] == "--topics":
            topic_filter = args[i + 1].split(",")
            i += 2
        elif args[i] in ("-h", "--help"):
            print(__doc__)
            sys.exit(0)
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    if not os.path.isdir(source_dir):
        print(f"ERROR: source directory not found: {source_dir}", file=sys.stderr)
        sys.exit(1)

    os.makedirs(dest_dir, exist_ok=True)
    output_path = os.path.join(dest_dir, "gittables_sample.jsonl")

    # Discover topics
    topics = discover_topics(source_dir, topic_filter)
    if not topics:
        print("ERROR: no topics found", file=sys.stderr)
        sys.exit(1)

    print(f"GitTables extraction: {len(topics)} topics, {files_per_topic} files/topic, seed={seed}")
    print(f"Source: {source_dir}")
    print(f"Output: {output_path}")
    print()

    rng = random.Random(seed)
    start_time = time.time()

    total_files = 0
    total_columns = 0
    total_errors = 0
    topic_stats = []

    with open(output_path, "w") as out:
        for ti, topic in enumerate(topics):
            topic_dir = os.path.join(source_dir, topic)
            sampled_files = sample_files(topic_dir, files_per_topic, rng)

            topic_columns = 0
            topic_errors = 0

            for filename in sampled_files:
                filepath = os.path.join(topic_dir, filename)
                columns = extract_columns_from_parquet(filepath, max_values)

                if columns is None:
                    topic_errors += 1
                    continue

                for col_name, values in columns:
                    record = {
                        "source": "gittables",
                        "source_file": f"{topic}/{filename}",
                        "column_name": col_name,
                        "values": values,
                        "topic": topic,
                        "ground_truth_label": "",
                        "ground_truth_source": "",
                    }
                    out.write(json.dumps(record) + "\n")
                    topic_columns += 1

            total_files += len(sampled_files)
            total_columns += topic_columns
            total_errors += topic_errors

            topic_stats.append((topic, len(sampled_files), topic_columns, topic_errors))
            print(f"  [{ti + 1}/{len(topics)}] {topic}: {len(sampled_files)} files, {topic_columns} columns" +
                  (f", {topic_errors} errors" if topic_errors else ""))

    elapsed = time.time() - start_time

    # Final summary
    print()
    print("=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print(f"Topics:      {len(topics)}")
    print(f"Files:       {total_files:,}")
    print(f"Columns:     {total_columns:,}")
    print(f"Errors:      {total_errors:,}")
    print(f"Time:        {elapsed:.1f}s")
    print(f"Output:      {output_path}")
    print()
    print("Per-topic breakdown:")
    print(f"  {'Topic':<35} {'Files':>6} {'Columns':>8} {'Errors':>7}")
    print(f"  {'-' * 35} {'-' * 6} {'-' * 8} {'-' * 7}")
    for topic, nfiles, ncols, nerrs in topic_stats:
        err_str = str(nerrs) if nerrs else ""
        print(f"  {topic:<35} {nfiles:>6} {ncols:>8} {err_str:>7}")


if __name__ == "__main__":
    main()
