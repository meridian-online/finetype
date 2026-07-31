#!/usr/bin/env python3
"""Audit DBpedia → FineType mapping coverage against the corpus.

Spec: `2026-05-20-gittables-multi-lens-diagnostic`
ac-05 verification.

Reads `eval/gittables/corpus_pass/dbpedia_annotations.parquet` and
`eval/gittables/dbpedia_finetype_mapping.tsv`. Asserts every DBpedia
class appearing in ≥10 columns of the measure-half corpus has a row
in the mapping table. Emits `eval/gittables/dbpedia_coverage.json`
(re-derived from the annotation parquet — supersedes the temporary
coverage written by `extract_dbpedia_annotations.py`).

Exit code 0 if all ≥10× classes are mapped, non-zero otherwise.

Usage:
  source eval/gittables/.venv/bin/activate
  python3 scripts/audit_dbpedia_coverage.py
"""

from __future__ import annotations

import argparse
import csv
import json
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DEFAULT_ANNOTATIONS = (
    REPO / "eval" / "gittables" / "corpus_pass" / "dbpedia_annotations.parquet"
)
DEFAULT_MAPPING = REPO / "eval" / "gittables" / "dbpedia_finetype_mapping.tsv"
DEFAULT_OUT = REPO / "eval" / "gittables" / "dbpedia_coverage.json"
MIN_CLASS_FREQ = 10


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Audit DBpedia → FineType mapping coverage (ac-05)."
    )
    parser.add_argument("--annotations", type=Path, default=DEFAULT_ANNOTATIONS)
    parser.add_argument("--mapping", type=Path, default=DEFAULT_MAPPING)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUT)
    args = parser.parse_args()

    try:
        import pyarrow.parquet as pq  # type: ignore
    except ImportError as exc:
        print(f"error: pyarrow not available ({exc}). Activate venv: "
              "source eval/gittables/.venv/bin/activate",
              file=sys.stderr)
        return 2

    if not args.annotations.exists():
        print(f"error: annotations not found: {args.annotations}",
              file=sys.stderr)
        return 2
    if not args.mapping.exists():
        print(f"error: mapping not found: {args.mapping}", file=sys.stderr)
        return 2

    table = pq.read_table(args.annotations, columns=["dbpedia_semantic_class"])
    classes = table["dbpedia_semantic_class"].to_pylist()
    counts: Counter[str] = Counter(c for c in classes if c)
    ge_threshold = sorted(
        [(cls, n) for cls, n in counts.items() if n >= MIN_CLASS_FREQ],
        key=lambda x: (-x[1], x[0]),
    )

    mapping_keys: set[str] = set()
    mapping_status: Counter[str] = Counter()
    with args.mapping.open() as fh:
        for row in csv.DictReader(fh, delimiter="\t"):
            cls = row["dbpedia_or_schemaorg_class"].strip()
            if cls:
                mapping_keys.add(cls)
                mapping_status[row["mapping_status"].strip()] += 1

    missing = [
        {"class": cls, "count": n}
        for cls, n in ge_threshold
        if cls not in mapping_keys
    ]

    summary = {
        "n_column_rows": len(classes),
        "n_distinct_dbpedia_classes": len(counts),
        "min_class_freq_threshold": MIN_CLASS_FREQ,
        "n_classes_at_or_above_threshold": len(ge_threshold),
        "n_mapping_rows": len(mapping_keys),
        "mapping_status_distribution": dict(mapping_status),
        "n_missing_mapping_rows": len(missing),
        "missing": missing[:50],
        "classes_at_or_above_threshold": [
            {"class": c, "count": n} for c, n in ge_threshold
        ],
    }
    args.output.write_text(json.dumps(summary, indent=2) + "\n")
    print(f"wrote {args.output}", file=sys.stderr)

    coverage_msg = (
        f"≥{MIN_CLASS_FREQ}× classes: {len(ge_threshold)}; "
        f"mapped: {len(ge_threshold) - len(missing)}; "
        f"missing: {len(missing)}"
    )
    if missing:
        print(f"FAIL: {coverage_msg}", file=sys.stderr)
        return 1
    print(f"PASS: {coverage_msg}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
