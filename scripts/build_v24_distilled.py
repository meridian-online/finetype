#!/usr/bin/env python3
"""v24 ac-02 — build the v24 numeric-precision distilled blend + audit gate.

Additive over the v22 boundary blend, exactly as v23 was (same base; the only
delta vs v23 is the hard-negative set — v24's is numeric-target ONLY, with zero
categorical, the v23 failure mode it must not repeat).

Concatenates, in order:
  1. v22 distilled (sherlock_distilled_v22.csv.gz)        — pass-through
  2. v24 hard negatives (hard_negatives.parquet, ac-01)   — 78,612 rows,
     correct_label ∈ {integer_number, decimal_number} only

Hard-negative `correct_label` becomes `final_label`; only
`(final_label, sample_values, column_name)` are written, drop-in compatible with
`prepare_multibranch_data.py` (the FTMB step, where --distilled-cap is applied —
NOT here). cluster_id provenance is kept in the manifest.

Audit gate (v24 ac-02). Fails (exit 3) on any of:
  - total rows < AUDIT_TOTAL_ROWS
  - any of the four numeric clusters contributes < AUDIT_PER_CLUSTER_ROWS
  - geography rows < AUDIT_GEOGRAPHY_ROWS  (the v23-death guard: a blend that
    drops v22's geography is corrupted)
  - ANY hard-negative row carries final_label = discrete.categorical
    (the v23 failure mode — must be zero by construction)

Output:
  output/distillation-v24/sherlock_distilled_v24.csv.gz
  output/distillation-v24/v24_blend_manifest.json

Run under the gittables venv (needs the python duckdb module):
  eval/gittables/.venv/bin/python scripts/build_v24_distilled.py
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
V24_HARD_NEGATIVES = REPO / "output/v24-numeric-precision/hard_negatives.parquet"
DEFAULT_OUT = REPO / "output/distillation-v24/sherlock_distilled_v24.csv.gz"
DEFAULT_MANIFEST = REPO / "output/distillation-v24/v24_blend_manifest.json"

CATEGORICAL = "representation.discrete.categorical"

# v22 base (~170k) + v24 hard negs (~78.6k) ≈ 248k; floor a touch under.
AUDIT_TOTAL_ROWS = 240_000
AUDIT_PER_CLUSTER_ROWS = 1_000
AUDIT_GEOGRAPHY_ROWS = 2_000

TARGET_CLUSTER_IDS = ["utc_to_int", "bool_to_int", "url_to_int", "int_to_dec"]


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--v22-distilled", type=Path, default=V22_DISTILLED)
    p.add_argument("--v24-hard-negatives", type=Path, default=V24_HARD_NEGATIVES)
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    p.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    p.add_argument("--skip-audit", action="store_true",
                   help="Diagnostic only; never ship a model trained with this.")
    args = p.parse_args()

    for path in (args.v22_distilled, args.v24_hard_negatives):
        if not path.exists():
            print(f"error: source missing — {path}", file=sys.stderr)
            return 2
    args.out.parent.mkdir(parents=True, exist_ok=True)

    per_source_rows: dict[str, int] = {}
    per_label_rows: dict[str, int] = defaultdict(int)
    per_cluster_rows: dict[str, int] = defaultdict(int)
    hardneg_categorical = 0
    total = 0

    print(f"writing {args.out}...", file=sys.stderr)
    with gzip.open(args.out, "wt", newline="", encoding="utf-8") as fout:
        writer = csv.writer(fout)
        writer.writerow(["final_label", "sample_values", "column_name"])

        # ── (1) v22 distilled (pass-through) ──────────────────────────
        n_v22 = 0
        with gzip.open(args.v22_distilled, "rt", newline="",
                       encoding="utf-8") as fin:
            for row in csv.DictReader(fin):
                label = (row.get("final_label") or "").strip()
                if not label:
                    continue
                writer.writerow([label, row.get("sample_values") or "",
                                 row.get("column_name") or ""])
                per_label_rows[label] += 1
                n_v22 += 1
                total += 1
        per_source_rows["v22_distilled"] = n_v22
        print(f"  {'v22_distilled':>26s}: {n_v22:>8,} rows", file=sys.stderr)

        # ── (2) v24 hard negatives (parquet → 3-col CSV) ──────────────
        try:
            import duckdb  # type: ignore
        except ImportError as exc:
            print(f"error: duckdb missing ({exc}). Run under the venv.",
                  file=sys.stderr)
            return 2
        con = duckdb.connect()
        arrow = con.execute(f"""
            SELECT correct_label, sample_values, column_name, cluster_id
              FROM read_parquet('{args.v24_hard_negatives.as_posix()}')
        """).to_arrow_table()

        n_v24 = 0
        for label, sample_list, cname, cluster_id in zip(
            arrow.column("correct_label").to_pylist(),
            arrow.column("sample_values").to_pylist(),
            arrow.column("column_name").to_pylist(),
            arrow.column("cluster_id").to_pylist(),
        ):
            if not label:
                continue
            if label == CATEGORICAL:
                hardneg_categorical += 1
            writer.writerow([label, json.dumps(sample_list or []), cname or ""])
            per_label_rows[label] += 1
            per_cluster_rows[cluster_id] += 1
            n_v24 += 1
            total += 1
        per_source_rows["v24_hard_negatives"] = n_v24
        print(f"  {'v24_hard_negatives':>26s}: {n_v24:>8,} rows",
              file=sys.stderr)

    print(f"  total: {total:>8,} rows", file=sys.stderr)

    # ── Audit gate ────────────────────────────────────────────────────
    audit_failures: list[str] = []
    if total < AUDIT_TOTAL_ROWS:
        audit_failures.append(f"total rows {total:,} < gate {AUDIT_TOTAL_ROWS:,}")
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
    if hardneg_categorical > 0:
        audit_failures.append(
            f"{hardneg_categorical:,} hard-negative rows are discrete.categorical "
            "— the v23 failure mode; must be zero")

    manifest = {
        "out": str(args.out),
        "total_rows": total,
        "per_source_rows": per_source_rows,
        "per_cluster_rows": dict(sorted(per_cluster_rows.items())),
        "hardneg_categorical_rows": hardneg_categorical,
        "geography_total": geo_total,
        "per_label_rows": dict(sorted(per_label_rows.items())),
        "audit": {
            "total_rows_gate": AUDIT_TOTAL_ROWS,
            "per_cluster_rows_gate": AUDIT_PER_CLUSTER_ROWS,
            "geography_rows_gate": AUDIT_GEOGRAPHY_ROWS,
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
        return 0 if args.skip_audit else 3
    print("AUDIT GATE PASSED.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
