#!/usr/bin/env python3
"""Run FineType column inference on a batch of JSONL records.

Usage:
    python3 scripts/distill_batch.py --input <jsonl> --batch-id <NNN> \
        [--offset N] [--limit N] [--dest output/distillation-v3/]

Reads JSONL records (from extraction scripts), runs FineType column-mode
inference on each, and writes results to a batch CSV. This is the FineType
pass of the distillation pipeline — the blind Claude pass runs separately
in distill_agent.py.

Each output row contains:
  source, source_file, column_name, sample_values, finetype_label,
  finetype_confidence, ground_truth_label, ground_truth_source

Input JSONL format (from extract_*.py):
  {"source": "...", "source_file": "...", "column_name": "...",
   "values": [...], "ground_truth_label": "...", "ground_truth_source": "..."}
"""

import csv
import json
import os
import subprocess
import sys
import time


FINETYPE_CWD = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def run_finetype_batch(records, cwd):
    """Run finetype infer --mode column --batch on a list of records.

    Returns list of (label, confidence) tuples, one per record.
    """
    # Build JSONL input for finetype batch mode
    jsonl_input = ""
    for rec in records:
        header = rec.get("column_name") or ""
        values = rec.get("values", [])
        jsonl_input += json.dumps({"header": header, "values": values}) + "\n"

    try:
        result = subprocess.run(
            ["finetype", "infer", "--mode", "column", "--batch", "-o", "json"],
            input=jsonl_input,
            capture_output=True,
            text=True,
            timeout=300,
            cwd=cwd,
        )
    except subprocess.TimeoutExpired:
        print("  WARNING: finetype batch timed out", file=sys.stderr)
        return [(None, None)] * len(records)
    except FileNotFoundError:
        print("  ERROR: finetype binary not found on PATH", file=sys.stderr)
        sys.exit(1)

    if result.returncode != 0:
        print(f"  WARNING: finetype returned {result.returncode}: {result.stderr[:200]}",
              file=sys.stderr)
        return [(None, None)] * len(records)

    # Parse output — one JSON line per input
    results = []
    for line in result.stdout.strip().split("\n"):
        if not line.strip():
            continue
        try:
            obj = json.loads(line)
            results.append((obj.get("label"), obj.get("confidence")))
        except json.JSONDecodeError:
            results.append((None, None))

    # Pad if finetype returned fewer results than expected
    while len(results) < len(records):
        results.append((None, None))

    return results


def main():
    args = sys.argv[1:]
    input_path = None
    batch_id = None
    offset = 0
    limit = 0  # 0 = all
    dest_dir = "output/distillation-v3/"
    chunk_size = 50  # Records per finetype batch call

    i = 0
    while i < len(args):
        if args[i] == "--input":
            input_path = args[i + 1]
            i += 2
        elif args[i] == "--batch-id":
            batch_id = args[i + 1]
            i += 2
        elif args[i] == "--offset":
            offset = int(args[i + 1])
            i += 2
        elif args[i] == "--limit":
            limit = int(args[i + 1])
            i += 2
        elif args[i] == "--dest":
            dest_dir = args[i + 1]
            i += 2
        elif args[i] == "--chunk-size":
            chunk_size = int(args[i + 1])
            i += 2
        elif args[i] in ("-h", "--help"):
            print(__doc__)
            sys.exit(0)
        else:
            print(f"Unknown argument: {args[i]}", file=sys.stderr)
            sys.exit(1)

    if not input_path or not batch_id:
        print("ERROR: --input and --batch-id are required", file=sys.stderr)
        sys.exit(1)

    os.makedirs(dest_dir, exist_ok=True)
    output_csv = os.path.join(dest_dir, f"finetype_batch_{batch_id}.csv")
    done_marker = os.path.join(dest_dir, f"finetype_batch_{batch_id}.done")

    # Skip if already completed
    if os.path.exists(done_marker):
        print(f"SKIP: {done_marker} exists — batch already complete")
        return

    # Read input JSONL
    print(f"Reading {input_path} (offset={offset}, limit={limit or 'all'})...")
    records = []
    with open(input_path) as f:
        for line_num, line in enumerate(f):
            if line_num < offset:
                continue
            if limit and len(records) >= limit:
                break
            try:
                records.append(json.loads(line))
            except json.JSONDecodeError:
                continue

    print(f"  {len(records)} records loaded")

    if not records:
        print("No records to process")
        return

    # Process in chunks
    fieldnames = [
        "source", "source_file", "column_name", "sample_values",
        "finetype_label", "finetype_confidence",
        "ground_truth_label", "ground_truth_source",
    ]

    start_time = time.time()
    total_processed = 0
    total_errors = 0

    with open(output_csv, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()

        for chunk_start in range(0, len(records), chunk_size):
            chunk = records[chunk_start:chunk_start + chunk_size]
            results = run_finetype_batch(chunk, FINETYPE_CWD)

            for rec, (label, confidence) in zip(chunk, results):
                row = {
                    "source": rec.get("source", ""),
                    "source_file": rec.get("source_file", ""),
                    "column_name": rec.get("column_name", ""),
                    "sample_values": json.dumps(rec.get("values", [])),
                    "finetype_label": label or "",
                    "finetype_confidence": f"{confidence:.4f}" if confidence else "",
                    "ground_truth_label": rec.get("ground_truth_label", "") or "",
                    "ground_truth_source": rec.get("ground_truth_source", "") or "",
                }
                writer.writerow(row)
                total_processed += 1
                if not label:
                    total_errors += 1

            elapsed = time.time() - start_time
            rate = total_processed / elapsed if elapsed > 0 else 0
            print(f"  Processed {total_processed}/{len(records)} "
                  f"({total_errors} errors, {rate:.0f} cols/sec)")

    # Validate and write done marker
    # Quick row count check
    with open(output_csv) as f:
        row_count = sum(1 for _ in f) - 1  # minus header

    if row_count == total_processed:
        with open(done_marker, "w") as f:
            f.write(f"rows={row_count}\nerrors={total_errors}\n"
                    f"elapsed={time.time() - start_time:.1f}s\n")
        print(f"\nDone: {output_csv} ({row_count} rows, {total_errors} errors)")
        print(f"Marker: {done_marker}")
    else:
        print(f"\nWARNING: Row count mismatch: expected {total_processed}, got {row_count}",
              file=sys.stderr)
        print(f"Output written but no .done marker — may be incomplete")


if __name__ == "__main__":
    main()
