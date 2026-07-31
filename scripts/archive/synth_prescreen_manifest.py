#!/usr/bin/env python3
"""Synthesise an eval-shaped column-level manifest from validate-corpus.

The validate-corpus manifest at eval/datasets/validate_manifest.csv is
dataset-level (1 row per dataset, 9 columns). The prescreen tool at
scripts/prescreen_eval.py expects column-level rows (1 row per column,
with `dataset, file_path, column_name, gt_label, …` columns) — the same
shape eval/datasets/manifest.csv carries.

This adapter walks the validate manifest, opens each dataset's GT sidecar
(referenced by `gt_sidecar_path`), and emits one row per (dataset, column)
pair. The output is suitable for piping straight into prescreen_eval.py via
its `--manifest` flag.

Usage:
    python scripts/synth_prescreen_manifest.py \\
        --validate-manifest eval/datasets/validate_manifest.csv \\
        --output /tmp/iter2_prescreen.csv

    python scripts/prescreen_eval.py \\
        --manifest /tmp/iter2_prescreen.csv \\
        --output /tmp/iter2_prescreen_metrics.tsv

This is the load-bearing AC-06 deliverable for the iter-2 validate-corpus
curation spec (spec 2026-04-28-validate-corpus-curation).
"""

from __future__ import annotations

import argparse
import csv
import sys
from pathlib import Path

try:
    import yaml
except ImportError:
    sys.stderr.write("error: PyYAML required. `pip install pyyaml`.\n")
    sys.exit(2)


REPO_ROOT = Path(__file__).resolve().parent.parent

# Column shape matches eval/datasets/manifest.csv (7 cols) so prescreen_eval.py
# can read either source without branching.
OUT_FIELDS = [
    "dataset",
    "file_path",
    "column_name",
    "gt_label",
    "source_url",
    "licence",
    "fetched_date",
]


def load_gt_columns(sidecar_path: Path) -> list[tuple[str, str]]:
    """Load (column_name, gt_label) pairs from a GT sidecar YAML.

    The sidecar shape is:

        columns:
          <column_name>: <full_type_label>
          ...
        notes: |
          ...
    """
    if not sidecar_path.exists():
        raise FileNotFoundError(f"sidecar not found: {sidecar_path}")
    with open(sidecar_path) as fh:
        data = yaml.safe_load(fh) or {}
    cols = data.get("columns") or {}
    if not isinstance(cols, dict):
        raise ValueError(f"sidecar {sidecar_path} has no usable `columns:` map")
    return [(str(k), str(v)) for k, v in cols.items()]


def main() -> int:
    doc = __doc__ or ""
    parser = argparse.ArgumentParser(description=doc.splitlines()[0] if doc else "")
    parser.add_argument(
        "--validate-manifest",
        type=Path,
        default=REPO_ROOT / "eval" / "datasets" / "validate_manifest.csv",
    )
    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="Path for the synthesised eval-shaped manifest (CSV, 7 cols).",
    )
    args = parser.parse_args()

    if not args.validate_manifest.exists():
        sys.stderr.write(f"error: validate manifest missing: {args.validate_manifest}\n")
        return 2

    args.output.parent.mkdir(parents=True, exist_ok=True)

    n_datasets = 0
    n_rows = 0
    with open(args.validate_manifest, newline="", encoding="utf-8") as fin, \
         open(args.output, "w", newline="", encoding="utf-8") as fout:
        reader = csv.DictReader(fin)
        writer = csv.DictWriter(fout, fieldnames=OUT_FIELDS)
        writer.writeheader()

        for ds_row in reader:
            dataset = ds_row.get("dataset", "").strip()
            if not dataset:
                continue  # skip blank trailing line
            sidecar_rel = ds_row.get("gt_sidecar_path", "").strip()
            if not sidecar_rel:
                sys.stderr.write(
                    f"warning: dataset {dataset!r} has no gt_sidecar_path; skipping\n"
                )
                continue
            sidecar = REPO_ROOT / sidecar_rel
            try:
                cols = load_gt_columns(sidecar)
            except Exception as e:
                sys.stderr.write(f"error: {dataset}: {e}\n")
                return 1
            file_path = ds_row.get("file_path", "")
            source_url = ds_row.get("source_url", "")
            licence = ds_row.get("licence", "")
            fetched_date = ds_row.get("fetched_date", "")
            for column_name, gt_label in cols:
                writer.writerow(
                    {
                        "dataset": dataset,
                        "file_path": file_path,
                        "column_name": column_name,
                        "gt_label": gt_label,
                        "source_url": source_url,
                        "licence": licence,
                        "fetched_date": fetched_date,
                    }
                )
                n_rows += 1
            n_datasets += 1

    print(
        f"synthesised {n_rows} (dataset, column) rows from {n_datasets} datasets "
        f"-> {args.output}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
