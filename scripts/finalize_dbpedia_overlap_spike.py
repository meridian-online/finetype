#!/usr/bin/env python3
"""Finalize the DBpedia / Schema.org overlap spike.

Spec: `.orbit/specs/2026-05-20-gittables-multi-lens-diagnostic/spec.yaml`
ac-02 (DBpedia overlap spike) — phase 2.

Consumes:
  - `eval/gittables/dbpedia_overlap_spike_raw.tsv` (Phase 1 output of
    `scripts/spike_dbpedia_overlap.py`).
  - `eval/gittables/dbpedia_finetype_mapping.tsv` (the hand-curated
    DBpedia → FineType mapping; ac-05 seed).

Computes:
  - `overlap_fraction = n_mappable / n_columns_total`, where a column
    is "mappable" iff it has a non-empty `dbpedia_class` AND that
    class's `mapping_status ∈ {direct, partial}`.
  - Wilson 95% confidence interval on the proportion.
  - `design_path`: "proceed" if the CI upper bound ≥ 0.20, else
    "downgrade_dbpedia".

Emits:
  - `eval/gittables/dbpedia_overlap_spike.json` (the ac-02
    deliverable).

Usage:
  python3 scripts/finalize_dbpedia_overlap_spike.py
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import sys
from pathlib import Path

DEFAULT_EVAL_DIR = Path("eval/gittables")
SAMPLE_SEED = 20260520  # matches ac-02 spec; used for tie-breaking only
MAPPABLE_STATUSES = {"direct", "partial"}
DESIGN_THRESHOLD = 0.20


def _load_mapping(path: Path) -> dict[str, str]:
    """Returns {dbpedia_class -> mapping_status}."""
    out: dict[str, str] = {}
    with path.open() as fh:
        reader = csv.DictReader(fh, delimiter="\t")
        for row in reader:
            cls = row["dbpedia_or_schemaorg_class"].strip()
            status = row["mapping_status"].strip()
            if cls and status:
                out[cls] = status
    return out


def _load_raw(path: Path) -> list[dict[str, str]]:
    with path.open() as fh:
        return list(csv.DictReader(fh, delimiter="\t"))


def _wilson_95_ci(successes: int, n: int) -> tuple[float, float, float]:
    """Returns (point_estimate, ci_lower, ci_upper) for Wilson 95% CI."""
    if n == 0:
        return (0.0, 0.0, 0.0)
    z = 1.959963984540054  # 95% two-sided
    p = successes / n
    denom = 1.0 + z * z / n
    center = (p + z * z / (2.0 * n)) / denom
    margin = (z * math.sqrt(p * (1.0 - p) / n + z * z / (4.0 * n * n))) / denom
    return (p, max(0.0, center - margin), min(1.0, center + margin))


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Finalize the DBpedia / Schema.org overlap spike."
    )
    parser.add_argument("--eval-dir", type=Path, default=DEFAULT_EVAL_DIR)
    args = parser.parse_args()

    raw_path = args.eval_dir / "dbpedia_overlap_spike_raw.tsv"
    mapping_path = args.eval_dir / "dbpedia_finetype_mapping.tsv"
    out_path = args.eval_dir / "dbpedia_overlap_spike.json"

    for p in (raw_path, mapping_path):
        if not p.exists():
            print(f"error: required input missing: {p}", file=sys.stderr)
            return 2

    mapping = _load_mapping(mapping_path)
    raw_rows = _load_raw(raw_path)

    sampled_tables: set[str] = set()
    for r in raw_rows:
        sampled_tables.add(f"{r['topic']}/{r['table']}")
    n_tables = len(sampled_tables)
    n_columns_total = len(raw_rows)

    n_columns_with_dbpedia = 0
    n_columns_mappable = 0
    n_columns_with_dbpedia_unmappable = 0
    unknown_classes: dict[str, int] = {}
    for r in raw_rows:
        dbp = (r.get("dbpedia_class") or "").strip()
        if not dbp:
            continue
        n_columns_with_dbpedia += 1
        status = mapping.get(dbp)
        if status is None:
            unknown_classes[dbp] = unknown_classes.get(dbp, 0) + 1
            continue
        if status in MAPPABLE_STATUSES:
            n_columns_mappable += 1
        else:
            n_columns_with_dbpedia_unmappable += 1

    overlap_fraction, ci_lower, ci_upper = _wilson_95_ci(
        n_columns_mappable, n_columns_total
    )
    design_path = (
        "proceed" if ci_upper >= DESIGN_THRESHOLD else "downgrade_dbpedia"
    )

    summary = {
        "sample_seed": SAMPLE_SEED,
        "n_tables": n_tables,
        "n_columns_total": n_columns_total,
        "n_annotated_columns": n_columns_with_dbpedia,
        "n_mappable_columns": n_columns_mappable,
        "n_dbpedia_annotated_unmappable": n_columns_with_dbpedia_unmappable,
        "n_unknown_dbpedia_classes": len(unknown_classes),
        "unknown_dbpedia_classes_top10": [
            {"class": cls, "count": n}
            for cls, n in sorted(
                unknown_classes.items(), key=lambda x: (-x[1], x[0])
            )[:10]
        ],
        "overlap_fraction": round(overlap_fraction, 6),
        "overlap_ci_lower": round(ci_lower, 6),
        "overlap_ci_upper": round(ci_upper, 6),
        "design_threshold": DESIGN_THRESHOLD,
        "design_path": design_path,
        "mapping_table_path": str(mapping_path),
        "raw_data_path": str(raw_path),
    }

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(summary, indent=2) + "\n")
    print(f"wrote {out_path}", file=sys.stderr)
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
