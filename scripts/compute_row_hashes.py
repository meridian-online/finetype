#!/usr/bin/env python3
"""Compute eval row hashes for the train↔eval/validate leakage firewall.

Reads `eval/datasets/manifest.csv` (column-level, role=eval) AND
`eval/datasets/validate_manifest.csv` (dataset-level, role=validate)
and writes `eval/row_hashes.tsv` — one row per (dataset, column, value)
with the SHA256 hash over (normalised_header || 0x00 || normalised_value).

The output is byte-deterministic: running the script twice produces
identical output. The shared normaliser in `scripts/eval_leakage/`
is imported by both this script and the training-pipeline filter
so the two cannot drift.

Manifest schema detection (DictReader.fieldnames):
  - If `column_name` is present (eval manifest) — each row names one column
    in one CSV file (path in `file_path`); hash all non-empty values.
  - If `gt_sidecar_path` is present (validate manifest) — each row names a
    whole CSV file; enumerate every column from the CSV header and hash
    each column's non-empty values.

Both manifests' rows are forbidden in training data (MADR 0056 §Layer 1
extended by spec 2026-04-28-validate-precision-corpus to cover
role=validate sources).

Usage:
    python scripts/compute_row_hashes.py                    # both manifests
    python scripts/compute_row_hashes.py --dry-run          # report only, no write
    python scripts/compute_row_hashes.py --manifest <path>  # override eval manifest
    python scripts/compute_row_hashes.py --validate-manifest <path>  # override
    python scripts/compute_row_hashes.py --no-validate      # skip validate manifest

Output schema (TSV):
    dataset \\t column_name \\t normalised_header \\t row_hash

Notes:
  - `normalised_value` is intentionally NOT written to the output file,
    only the hash. This keeps the artefact compact; debugging a
    suspected leak goes through the shared normaliser + live value.
  - Sorted output ensures byte-deterministic regeneration.

See:
    .orbit/choices/0056-train-eval-leakage-prevention.md
    .orbit/specs/2026-04-21-eval-expansion/spec.yaml (ac-06)
    .orbit/specs/2026-04-28-validate-precision-corpus/spec.yaml (ac-03)
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


def csv_columns(file_path: Path) -> list[str]:
    """Return the column headers of a CSV file (empty list on missing/error)."""
    if not file_path.exists():
        return []
    try:
        with open(file_path, newline="", encoding="utf-8-sig", errors="replace") as fh:
            reader = csv.reader(fh)
            return next(reader, []) or []
    except OSError:
        return []


def process_manifest(
    manifest_path: Path,
    out_tuples: set[tuple[str, str, str, str]],
    max_rows: int | None,
    role: str,
) -> tuple[int, int, int]:
    """Process one manifest file. Schema detected from fieldnames.

    Returns (n_rows_scanned, n_columns_scanned, n_missing).
    """
    if not manifest_path.exists():
        print(f"  skipped (missing): {manifest_path}", file=sys.stderr)
        return (0, 0, 0)

    delim = "\t" if manifest_path.suffix.lower() == ".tsv" else ","
    with open(manifest_path, newline="", encoding="utf-8") as fh:
        reader = csv.DictReader(fh, delimiter=delim)
        fieldnames = reader.fieldnames or []
        manifest_rows = list(reader)

    column_level = "column_name" in fieldnames

    n_rows_scanned = 0
    n_columns_scanned = 0
    n_missing = 0

    for mrow in manifest_rows:
        dataset = mrow.get("dataset", "") or mrow.get("source", "")
        file_path_str = mrow.get("file_path", "")

        resolved = Path(file_path_str)
        if not resolved.is_absolute():
            resolved = REPO_ROOT / resolved

        if column_level:
            # eval-manifest shape: one row per (dataset, column)
            column_name = mrow.get("column_name", "")
            columns = [column_name] if column_name else []
        else:
            # validate-manifest shape: one row per CSV; enumerate all columns
            columns = csv_columns(resolved)

        if not columns:
            n_missing += 1
            continue

        for column_name in columns:
            values = load_column_values(resolved, column_name, max_rows)
            if not values:
                if column_level:
                    n_missing += 1
                continue
            n_columns_scanned += 1
            nh = normalise_header(column_name)
            for v in values:
                n_rows_scanned += 1
                h = row_hash(column_name, v)
                out_tuples.add((dataset, column_name, nh, h))

    print(
        f"  manifest [{role}] {manifest_path.relative_to(REPO_ROOT)}: "
        f"scanned {n_rows_scanned} values across {n_columns_scanned} columns; "
        f"{n_missing} missing"
    )
    return (n_rows_scanned, n_columns_scanned, n_missing)


def main() -> int:
    parser = argparse.ArgumentParser(description="Compute eval/validate row hashes.")
    parser.add_argument(
        "--manifest",
        type=Path,
        default=REPO_ROOT / "eval" / "datasets" / "manifest.csv",
        help="Eval manifest (column-level schema). Default: eval/datasets/manifest.csv",
    )
    parser.add_argument(
        "--validate-manifest",
        type=Path,
        default=REPO_ROOT / "eval" / "datasets" / "validate_manifest.csv",
        help="Validate manifest (dataset-level schema). "
        "Default: eval/datasets/validate_manifest.csv",
    )
    parser.add_argument(
        "--gold-manifest",
        type=Path,
        default=REPO_ROOT / "eval" / "gold" / "gold_corpus_candidates_external.tsv",
        help="Gold external fixture (column-level TSV; spec 2026-06-10-"
        "human-verified-gold-corpus ac-05). Vendored gold columns are "
        "forbidden in training data. Default: eval/gold/"
        "gold_corpus_candidates_external.tsv",
    )
    parser.add_argument(
        "--no-gold",
        action="store_true",
        help="Skip the gold external fixture.",
    )
    parser.add_argument(
        "--no-validate",
        action="store_true",
        help="Skip the validate manifest (eval-only mode).",
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
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Compute and report counts but do not write the output file.",
    )
    args = parser.parse_args()

    out_tuples: set[tuple[str, str, str, str]] = set()

    eval_stats = process_manifest(args.manifest, out_tuples, args.max_rows, "eval")
    if not args.no_gold:
        process_manifest(args.gold_manifest, out_tuples, args.max_rows, "gold")
    if not args.no_validate:
        val_stats = process_manifest(
            args.validate_manifest, out_tuples, args.max_rows, "validate"
        )
    else:
        val_stats = (0, 0, 0)

    n_rows_scanned = eval_stats[0] + val_stats[0]
    n_columns_scanned = eval_stats[1] + val_stats[1]
    n_missing = eval_stats[2] + val_stats[2]

    # Sort for byte-deterministic output
    sorted_tuples = sorted(out_tuples)

    if args.dry_run:
        print(
            f"row_hashes [DRY-RUN]: {len(sorted_tuples)} distinct "
            f"(dataset, column, hash) rows would be written to {args.output}"
        )
    else:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with open(args.output, "w", encoding="utf-8") as fh:
            fh.write(f"# normaliser_version: {NORMALISER_VERSION}\n")
            fh.write("# See .orbit/choices/0056-train-eval-leakage-prevention.md\n")
            fh.write("dataset\tcolumn_name\tnormalised_header\trow_hash\n")
            for tup in sorted_tuples:
                fh.write("\t".join(tup) + "\n")
        print(
            f"row_hashes: {len(sorted_tuples)} distinct (dataset, column, hash) rows "
            f"written to {args.output}"
        )

    print(
        f"  total: scanned {n_rows_scanned} values across {n_columns_scanned} "
        f"columns; {n_missing} manifest rows missing"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
