#!/usr/bin/env python3
"""GitTables 1M evaluation via FineType CLI batch mode (NNFT-130).

Reads pre-extracted column_values.parquet, groups by column, pipes through
`finetype infer --mode column --batch` with header hints (col_name is the
real column header from GitTables), and writes cli_predictions.csv for
downstream SQL scoring.

Usage:
    python3 eval/gittables/eval_cli.py
    GITTABLES_DIR=~/datasets/gittables python3 eval/gittables/eval_cli.py

Environment variables:
    GITTABLES_DIR  — GitTables corpus root (default: ~/datasets/gittables)
    EVAL_OUTPUT    — Output directory with column_values.parquet (default: $GITTABLES_DIR/eval_output)
    FINETYPE_BIN   — Path to finetype binary (default: cargo run --)
"""
import csv
import json
import os
import subprocess
import sys
import time
from collections import defaultdict
from pathlib import Path

_GITTABLES_DIR = os.environ.get("GITTABLES_DIR", os.path.expanduser("~/datasets/gittables"))
EVAL_OUTPUT = Path(os.environ.get("EVAL_OUTPUT", os.path.join(_GITTABLES_DIR, "eval_output")))
FINETYPE_BIN = os.environ.get("FINETYPE_BIN", "").strip()


def load_column_values(parquet_path: Path) -> dict[tuple[str, str, str], list[str]]:
    """Load column_values.parquet and group by (topic, table_name, col_name)."""
    try:
        import duckdb
    except ImportError:
        print("Need duckdb: pip install duckdb", file=sys.stderr)
        sys.exit(1)

    con = duckdb.connect()
    rows = con.execute(
        f"SELECT topic, table_name, col_name, col_value FROM read_parquet('{parquet_path}')"
    ).fetchall()
    con.close()

    columns: dict[tuple[str, str, str], list[str]] = defaultdict(list)
    for topic, table_name, col_name, col_value in rows:
        columns[(topic, table_name, col_name)].append(col_value)

    return columns


def run_batch_classification(
    columns: dict[tuple[str, str, str], list[str]],
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

    # Generate JSONL input
    jsonl_lines = []
    for topic, table_name, col_name in keys:
        values = columns[(topic, table_name, col_name)]
        obj = {"header": col_name, "values": values}
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
        topic, table_name, col_name = key
        try:
            pred = json.loads(line)
        except json.JSONDecodeError:
            print(f"WARNING: Invalid JSON on line {i}: {line[:100]}", file=sys.stderr)
            pred = {"label": "PARSE_ERROR", "confidence": 0.0}

        results.append(
            {
                "topic": topic,
                "table_name": table_name,
                "col_name": col_name,
                "predicted_label": pred.get("label", "UNKNOWN"),
                "confidence": pred.get("confidence", 0.0),
                "samples_used": pred.get("samples_used", 0),
                "disambiguation_rule": pred.get("disambiguation_rule", ""),
            }
        )

    return results


def main():
    parquet_path = EVAL_OUTPUT / "column_values.parquet"
    if not parquet_path.exists():
        print(f"column_values.parquet not found at {parquet_path}", file=sys.stderr)
        print("Run: make eval-values", file=sys.stderr)
        sys.exit(1)

    print(f"Loading column values from {parquet_path}...", file=sys.stderr)
    columns = load_column_values(parquet_path)
    print(
        f"  {len(columns)} columns, {sum(len(v) for v in columns.values())} values",
        file=sys.stderr,
    )

    results = run_batch_classification(columns)

    # Write predictions CSV
    output_path = EVAL_OUTPUT / "cli_predictions.csv"
    with open(output_path, "w", newline="") as f:
        writer = csv.DictWriter(
            f,
            fieldnames=[
                "topic",
                "table_name",
                "col_name",
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
