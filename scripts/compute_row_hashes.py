#!/usr/bin/env python3
"""Compute eval row hashes for the train↔eval leakage firewall (ac-06).

Reads eval/datasets/manifest.csv, loads each referenced file, and writes
eval/row_hashes.tsv — one row per (dataset, column, value) with the
SHA256 hash over (normalised_header || 0x00 || normalised_value).

The output is byte-deterministic: running the script twice produces
identical output. The shared normaliser in `scripts/eval_leakage/`
is imported by both this script and the training-pipeline filter
(ac-07) so the two cannot drift.

Usage:
    python scripts/compute_row_hashes.py                        # defaults
    python scripts/compute_row_hashes.py --manifest <path> --output <path>

Output schema (TSV):
    dataset \\t column_name \\t normalised_header \\t row_hash

Notes:
  - `normalised_value` is intentionally NOT written to the output file,
    only the hash. This keeps the artefact compact; debugging a
    suspected leak goes through the shared normaliser + live value.
  - Sorted output ensures byte-deterministic regeneration.

See:
    orbit/decisions/0056-train-eval-leakage-prevention.md
    orbit/specs/2026-04-21-eval-expansion/spec.yaml (ac-06)
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from pathlib import Path

# Add scripts/ to sys.path so the shared module imports cleanly whether
# the script is invoked as `python scripts/compute_row_hashes.py` or
# `scripts/compute_row_hashes.py` from the repo root.
_HERE = Path(__file__).resolve().parent
if str(_HERE) not in sys.path:
    sys.path.insert(0, str(_HERE))

from eval_leakage import (  # noqa: E402  — deliberate post-path insert  # pyright: ignore[reportMissingImports]
    NORMALISER_VERSION,
    normalise_header,
    row_hash,
)


REPO_ROOT = _HERE.parent


def load_column_values(
    file_path: Path, column_name: str, max_rows: int | None = None
) -> list[str]:
    """Load all values of a column from CSV/NDJSON/JSON.

    Missing file or column returns an empty list (caller records the miss).
    """
    if not file_path.exists():
        return []

    suffix = file_path.suffix.lower()
    vals: list[str] = []
    try:
        if suffix in (".csv", ".tsv"):
            delim = "," if suffix == ".csv" else "\t"
            with open(file_path, newline="", encoding="utf-8-sig", errors="replace") as fh:
                reader = csv.DictReader(fh, delimiter=delim)
                if column_name not in (reader.fieldnames or []):
                    return []
                for i, row in enumerate(reader):
                    if max_rows is not None and i >= max_rows:
                        break
                    v = row.get(column_name)
                    if v is None or str(v).strip() == "":
                        continue
                    vals.append(str(v))
        elif suffix in (".ndjson", ".jsonl"):
            with open(file_path, encoding="utf-8") as fh:
                for i, line in enumerate(fh):
                    if max_rows is not None and i >= max_rows:
                        break
                    line = line.strip()
                    if not line:
                        continue
                    try:
                        obj = json.loads(line)
                    except json.JSONDecodeError:
                        continue
                    if column_name in obj and obj[column_name] is not None:
                        s = str(obj[column_name])
                        if s.strip() != "":
                            vals.append(s)
        elif suffix == ".json":
            with open(file_path, encoding="utf-8") as fh:
                data = json.load(fh)
            if isinstance(data, list):
                for item in data:
                    if not isinstance(item, dict):
                        continue
                    if column_name in item and item[column_name] is not None:
                        s = str(item[column_name])
                        if s.strip() != "":
                            vals.append(s)
                        if max_rows is not None and len(vals) >= max_rows:
                            break
    except OSError:
        return []
    return vals


def main() -> int:
    parser = argparse.ArgumentParser(description="Compute eval row hashes.")
    parser.add_argument(
        "--manifest",
        type=Path,
        default=REPO_ROOT / "eval" / "datasets" / "manifest.csv",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=REPO_ROOT / "eval" / "row_hashes.tsv",
    )
    parser.add_argument(
        "--max-rows",
        type=int,
        default=None,
        help="Optional per-column row cap (default: full column)",
    )
    args = parser.parse_args()

    # Read manifest (tolerant of 4-col legacy + 7-col current schema)
    with open(args.manifest, newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh)
        manifest_rows = list(reader)

    # Collect (dataset, column, normalised_header, hash) tuples; dedupe then sort
    out_tuples: set[tuple[str, str, str, str]] = set()
    n_rows_scanned = 0
    n_columns_scanned = 0
    n_missing = 0

    for mrow in manifest_rows:
        dataset = mrow.get("dataset", "")
        file_path_str = mrow.get("file_path", "")
        column_name = mrow.get("column_name", "")

        resolved = Path(file_path_str)
        if not resolved.is_absolute():
            resolved = REPO_ROOT / resolved

        values = load_column_values(resolved, column_name, args.max_rows)
        if not values:
            n_missing += 1
            continue

        n_columns_scanned += 1
        nh = normalise_header(column_name)
        for v in values:
            n_rows_scanned += 1
            h = row_hash(column_name, v)
            out_tuples.add((dataset, column_name, nh, h))

    # Sort for byte-deterministic output
    sorted_tuples = sorted(out_tuples)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with open(args.output, "w", encoding="utf-8") as fh:
        fh.write(f"# normaliser_version: {NORMALISER_VERSION}\n")
        fh.write("# See orbit/decisions/0056-train-eval-leakage-prevention.md\n")
        fh.write("dataset\tcolumn_name\tnormalised_header\trow_hash\n")
        for tup in sorted_tuples:
            fh.write("\t".join(tup) + "\n")

    print(
        f"row_hashes: {len(sorted_tuples)} distinct (dataset, column, hash) rows "
        f"written to {args.output}"
    )
    print(
        f"  scanned: {n_rows_scanned} values across {n_columns_scanned} columns; "
        f"{n_missing} manifest rows missing"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
