#!/usr/bin/env python3
"""SOTAB CTA evaluation via FineType CLI batch mode (NNFT-130).

Reads pre-extracted column_values.parquet, groups by column, pipes through
`finetype infer --mode column --batch` (no header hints — SOTAB uses integer
column indices, not meaningful names), and writes cli_predictions.csv for
downstream SQL scoring.

Usage:
    python3 eval/sotab/eval_cli.py
    python3 eval/sotab/eval_cli.py --split test

Environment variables:
    SOTAB_DIR      — SOTAB CTA data directory (default: ~/datasets/sotab/cta)
    FINETYPE_BIN   — Path to finetype binary (default: cargo run --)
"""
import argparse
import csv
import json
import os
import subprocess
import sys
import time
from collections import defaultdict
from pathlib import Path

DEFAULT_SOTAB_DIR = Path(
    os.environ.get("SOTAB_DIR", os.path.expanduser("~/datasets/sotab/cta"))
)
FINETYPE_BIN = os.environ.get("FINETYPE_BIN", "").strip()


def load_column_values(
    parquet_path: Path,
) -> tuple[dict[tuple[str, int], list[str]], dict[tuple[str, int], str]]:
    """Load column_values.parquet and group by (table_name, col_index).

    Returns:
        columns: {(table_name, col_index): [values]}
        gt_labels: {(table_name, col_index): gt_label}
    """
    try:
        import duckdb
    except ImportError:
        print("Need duckdb: pip install duckdb", file=sys.stderr)
        sys.exit(1)

    con = duckdb.connect()
    rows = con.execute(
        f"SELECT table_name, col_index, gt_label, col_value FROM read_parquet('{parquet_path}')"
    ).fetchall()
    con.close()

    columns: dict[tuple[str, int], list[str]] = defaultdict(list)
    gt_labels: dict[tuple[str, int], str] = {}
    for table_name, col_index, gt_label, col_value in rows:
        key = (table_name, int(col_index))
        columns[key].append(col_value)
        gt_labels[key] = gt_label

    return columns, gt_labels


def run_batch_classification(
    columns: dict[tuple[str, int], list[str]],
) -> list[dict]:
    """Pipe all columns through finetype batch mode and collect predictions."""
    # Build command
    if FINETYPE_BIN:
        cmd = FINETYPE_BIN.split() + ["infer", "--mode", "column", "--batch"]
    else:
        cmd = ["cargo", "run", "--", "infer", "--mode", "column", "--batch"]

    print(f"Running: {' '.join(cmd)}", file=sys.stderr)
    print(f"Classifying {len(columns)} columns...", file=sys.stderr)

    # Build ordered key list to match output
    keys = list(columns.keys())

    # Generate JSONL input — no header for SOTAB (integer column indices)
    jsonl_lines = []
    for table_name, col_index in keys:
        values = columns[(table_name, col_index)]
        obj = {"values": values}
        jsonl_lines.append(json.dumps(obj, ensure_ascii=False))

    jsonl_input = "\n".join(jsonl_lines) + "\n"

    t_start = time.time()
    proc = subprocess.run(
        cmd,
        input=jsonl_input,
        capture_output=True,
        text=True,
        timeout=3600,  # 1 hour max
    )

    elapsed = time.time() - t_start
    print(f"Classification completed in {elapsed:.1f}s", file=sys.stderr)

    if proc.returncode != 0:
        print(f"finetype stderr:\n{proc.stderr}", file=sys.stderr)
        print(f"finetype exited with code {proc.returncode}", file=sys.stderr)
        sys.exit(1)

    # Print stderr from finetype (progress info)
    if proc.stderr:
        for line in proc.stderr.strip().split("\n"):
            print(f"  [finetype] {line}", file=sys.stderr)

    # Parse output JSONL
    output_lines = [l for l in proc.stdout.strip().split("\n") if l.strip()]

    if len(output_lines) != len(keys):
        print(
            f"WARNING: Expected {len(keys)} output lines, got {len(output_lines)}",
            file=sys.stderr,
        )

    results = []
    for i, (key, line) in enumerate(zip(keys, output_lines)):
        table_name, col_index = key
        try:
            pred = json.loads(line)
        except json.JSONDecodeError:
            print(f"WARNING: Invalid JSON on line {i}: {line[:100]}", file=sys.stderr)
            pred = {"label": "PARSE_ERROR", "confidence": 0.0}

        results.append(
            {
                "table_name": table_name,
                "col_index": col_index,
                "predicted_label": pred.get("label", "UNKNOWN"),
                "confidence": pred.get("confidence", 0.0),
                "samples_used": pred.get("samples_used", 0),
                "disambiguation_rule": pred.get("disambiguation_rule", ""),
            }
        )

    return results


def main():
    parser = argparse.ArgumentParser(description="SOTAB CTA evaluation via CLI batch mode.")
    parser.add_argument(
        "--sotab-dir",
        type=str,
        default=str(DEFAULT_SOTAB_DIR),
        help=f"SOTAB CTA data directory (default: {DEFAULT_SOTAB_DIR})",
    )
    parser.add_argument(
        "--split",
        type=str,
        default="validation",
        choices=["validation", "test"],
        help="Dataset split (default: validation)",
    )
    args = parser.parse_args()

    sotab_dir = Path(args.sotab_dir)
    split = args.split
    parquet_path = sotab_dir / split / "column_values.parquet"

    if not parquet_path.exists():
        print(f"column_values.parquet not found at {parquet_path}", file=sys.stderr)
        print("Run: make eval-sotab-values", file=sys.stderr)
        sys.exit(1)

    print(f"Loading column values from {parquet_path}...", file=sys.stderr)
    columns, gt_labels = load_column_values(parquet_path)
    print(
        f"  {len(columns)} columns, {sum(len(v) for v in columns.values())} values",
        file=sys.stderr,
    )

    results = run_batch_classification(columns)

    # Attach ground truth labels
    for r in results:
        key = (r["table_name"], r["col_index"])
        r["gt_label"] = gt_labels.get(key, "")

    # Write predictions CSV
    output_path = sotab_dir / split / "cli_predictions.csv"
    with open(output_path, "w", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=[
                "table_name",
                "col_index",
                "gt_label",
                "predicted_label",
                "confidence",
                "samples_used",
                "disambiguation_rule",
            ],
        )
        writer.writeheader()
        writer.writerows(results)

    print(f"\nOutput: {output_path}", file=sys.stderr)
    print(f"  Predictions: {len(results)}", file=sys.stderr)
    print(
        f"  Unique labels: {len(set(r['predicted_label'] for r in results))}",
        file=sys.stderr,
    )

    # Quick stats
    disambiguated = sum(1 for r in results if r["disambiguation_rule"])
    print(
        f"  Disambiguated: {disambiguated} ({disambiguated * 100.0 / max(len(results), 1):.1f}%)",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()
