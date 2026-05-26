#!/usr/bin/env python3
"""ac-02 — Build the v23 precision-retrain distilled blend.

Additive over the v22 boundary blend (Option A in the spec): the
v22 distilled is kept whole and the v23 hard negatives are appended.
v22's monotone-mover gains on geography subtypes (per the
v22-direction-review trajectory) are preserved by construction;
the only delta is the precision-targeting addition.

Concatenates, in this order:
  1. v22 distilled (sherlock_distilled_v22.csv.gz)        (~170,000+ rows)
  2. v23 hard negatives (hard_negatives.parquet, ac-01)   (~114,000 rows)

For the hard-negative rows, the `correct_label` column (= YDF
prediction for the cluster) becomes `final_label`. The
`cluster_id` and `sense_prediction_v22` provenance fields are
preserved in `v23_blend_manifest.json` for telemetry, but the
training CSV itself only carries `(final_label, sample_values,
column_name)` to remain drop-in compatible with
`prepare_multibranch_data.py`.

Audit gate (per spec 2026-05-27-v23-precision-retrain ac-02):
  - Total rows ≥ 250,000 (= v22 ~170k + v23 hard negs ~80k+ floor)
  - All six target clusters contribute ≥ 1,000 rows
  - v22 geography preserved: ≥ 2,000 geography rows present
    (mirrors v22's gate 7a; if this fails, the v22 baseline was
    corrupted during blending)

Failing any gate returns exit code 3; the overnight launcher
treats that as a hard stop.

Output:
  output/distillation-v23/sherlock_distilled_v23.csv.gz
  output/distillation-v23/v23_blend_manifest.json
"""
from __future__ import annotations

import argparse
import csv
import gzip
import json
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent

V22_DISTILLED = REPO / "output/distillation-v22/sherlock_distilled_v22.csv.gz"
V23_HARD_NEGATIVES = REPO / "output/v23-precision-retrain/hard_negatives.parquet"
DEFAULT_OUT = REPO / "output/distillation-v23/sherlock_distilled_v23.csv.gz"
DEFAULT_MANIFEST = REPO / "output/distillation-v23/v23_blend_manifest.json"

AUDIT_TOTAL_ROWS = 250_000
AUDIT_PER_CLUSTER_ROWS = 1_000
AUDIT_GEOGRAPHY_ROWS = 2_000

TARGET_CLUSTER_IDS = [
    "721b890ea74d",
    "1b858e0d073b",
    "20803deffbad",
    "81b63a52e3ef",
    "cdde5d05b73a",
    "3f2aa8465552",
]


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--v22-distilled", type=Path, default=V22_DISTILLED)
    p.add_argument("--v23-hard-negatives", type=Path, default=V23_HARD_NEGATIVES)
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    p.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    p.add_argument("--skip-audit", action="store_true",
                   help="Skip the audit gate (diagnostic; do not ship a model trained with this).")
    args = p.parse_args()

    for path in (args.v22_distilled, args.v23_hard_negatives):
        if not path.exists():
            print(f"error: source missing — {path}", file=sys.stderr)
            return 2

    args.out.parent.mkdir(parents=True, exist_ok=True)

    per_source_rows: dict[str, int] = {}
    per_label_rows: dict[str, int] = defaultdict(int)
    per_cluster_rows: dict[str, int] = defaultdict(int)
    total = 0

    print(f"writing {args.out}...", file=sys.stderr)
    with gzip.open(args.out, "wt", newline="", encoding="utf-8") as fout:
        writer = csv.writer(fout)
        writer.writerow(["final_label", "sample_values", "column_name"])

        # ── (1) v22 distilled (pass-through) ──────────────────────────
        n_v22 = 0
        with gzip.open(args.v22_distilled, "rt", newline="",
                       encoding="utf-8") as fin:
            reader = csv.DictReader(fin)
            for row in reader:
                label = (row.get("final_label") or "").strip()
                sample = row.get("sample_values") or ""
                cname = row.get("column_name") or ""
                if not label:
                    continue
                writer.writerow([label, sample, cname])
                per_label_rows[label] += 1
                n_v22 += 1
                total += 1
        per_source_rows["v22_distilled"] = n_v22
        print(f"  {'v22_distilled':>26s}: {n_v22:>8,} rows",
              file=sys.stderr)

        # ── (2) v23 hard negatives (parquet → 3-col CSV) ──────────────
        try:
            import duckdb  # type: ignore
        except ImportError as exc:
            print(f"error: duckdb missing ({exc}).", file=sys.stderr)
            return 2
        con = duckdb.connect()
        arrow = con.execute(f"""
            SELECT correct_label, sample_values, column_name, cluster_id
              FROM read_parquet('{args.v23_hard_negatives.as_posix()}')
        """).to_arrow_table()

        n_v23 = 0
        labels = arrow.column("correct_label").to_pylist()
        samples = arrow.column("sample_values").to_pylist()
        cnames = arrow.column("column_name").to_pylist()
        clusters = arrow.column("cluster_id").to_pylist()

        for label, sample_list, cname, cluster_id in zip(
            labels, samples, cnames, clusters
        ):
            if not label:
                continue
            # parquet sample_values is list<string>; the training
            # pipeline reads it as a JSON-encoded list, matching the
            # v19/v22 format.
            sample_json = json.dumps(sample_list or [])
            writer.writerow([label, sample_json, cname or ""])
            per_label_rows[label] += 1
            per_cluster_rows[cluster_id] += 1
            n_v23 += 1
            total += 1
        per_source_rows["v23_hard_negatives"] = n_v23
        print(f"  {'v23_hard_negatives':>26s}: {n_v23:>8,} rows",
              file=sys.stderr)

    print(f"  total before cap:           {total:>8,} rows", file=sys.stderr)

    # ── Audit gate ────────────────────────────────────────────────────
    audit_failures: list[str] = []

    if total < AUDIT_TOTAL_ROWS:
        audit_failures.append(
            f"total rows {total:,} < gate {AUDIT_TOTAL_ROWS:,}")

    for cid in TARGET_CLUSTER_IDS:
        n = per_cluster_rows.get(cid, 0)
        if n < AUDIT_PER_CLUSTER_ROWS:
            audit_failures.append(
                f"cluster {cid}: {n:,} rows < gate {AUDIT_PER_CLUSTER_ROWS:,}")

    geo_total = sum(v for k, v in per_label_rows.items()
                    if k.startswith("geography."))
    if geo_total < AUDIT_GEOGRAPHY_ROWS:
        audit_failures.append(
            f"geography rows {geo_total:,} < gate {AUDIT_GEOGRAPHY_ROWS:,} "
            "— v22 baseline corrupted during blend")

    manifest = {
        "out": str(args.out),
        "total_rows": total,
        "per_source_rows": per_source_rows,
        "per_label_rows": dict(sorted(per_label_rows.items())),
        "per_cluster_rows": dict(sorted(per_cluster_rows.items())),
        "audit": {
            "total_rows_gate": AUDIT_TOTAL_ROWS,
            "per_cluster_rows_gate": AUDIT_PER_CLUSTER_ROWS,
            "geography_rows_gate": AUDIT_GEOGRAPHY_ROWS,
            "geography_total": geo_total,
            "failures": audit_failures,
            "passed": not audit_failures,
        },
    }
    args.manifest.parent.mkdir(parents=True, exist_ok=True)
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"manifest: {args.manifest}", file=sys.stderr)

    if audit_failures:
        print("AUDIT GATE FAILED:", file=sys.stderr)
        for f in audit_failures:
            print(f"  - {f}", file=sys.stderr)
        if args.skip_audit:
            print("(--skip-audit set; continuing)", file=sys.stderr)
            return 0
        return 3
    print("AUDIT GATE PASSED.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
