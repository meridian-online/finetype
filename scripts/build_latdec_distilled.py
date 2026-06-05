#!/usr/bin/env python3
"""latitude->decimal ac-02 — build the hard-negative distilled blend + audit gate.

Additive over the v19 base distilled (the SHIPPED default's training corpus),
mirroring the v24 builder but for the INVERSE bet: v24 pushed plain numerics INTO
latitude; this pushes feature floats v19 wrongly grabs AS latitude back OUT to
representation.numeric.decimal_number.

Concatenates, in order:
  1. v19 base distilled (distillation-v3/sherlock_distilled.csv.gz) — pass-through
  2. latitude->decimal hard negatives (hard_negatives.parquet, ac-01/ac-02) —
     ~2,540 rows, correct_label = decimal_number ONLY (asserted)

Hard-negative `correct_label` becomes `final_label`; only
`(final_label, sample_values, column_name)` are written — drop-in compatible with
`prepare_multibranch_data.py` (the FTMB step, where the per-type --distilled-cap
override for decimal_number is applied — NOT here). cluster_id stays in manifest.

The base v3 distilled carries no `column_name`; pass-through leaves it blank,
exactly as the v19 FTMB build already sees it. Only the hard negatives carry a
header (used by the multi-branch header branch).

Audit gate. Fails (exit 3) on any of:
  - total rows < AUDIT_TOTAL_ROWS
  - base v19 rows < AUDIT_BASE_ROWS  (the blend must not drop the v19 corpus)
  - the lat_to_dec cluster contributes < AUDIT_CLUSTER_ROWS
  - ANY hard-negative final_label != decimal_number  (the spec's hard invariant:
    a non-decimal hard negative would poison an untargeted boundary by hand)

Output:
  output/distillation-latdec/sherlock_distilled_latdec.csv.gz
  output/distillation-latdec/latdec_blend_manifest.json

Run under the gittables venv (needs the python duckdb module):
  eval/gittables/.venv/bin/python scripts/build_latdec_distilled.py
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

V19_DISTILLED = REPO / "output/distillation-v3/sherlock_distilled.csv.gz"
LATDEC_HARD_NEGATIVES = REPO / "output/latitude-decimal-precision/hard_negatives.parquet"
DEFAULT_OUT = REPO / "output/distillation-latdec/sherlock_distilled_latdec.csv.gz"
DEFAULT_MANIFEST = REPO / "output/distillation-latdec/latdec_blend_manifest.json"

DECIMAL = "representation.numeric.decimal_number"
TARGET_CLUSTER_ID = "lat_to_dec"

# Base v19 ≈ 102,461 + hard negs ≈ 2,540 ≈ 105k; floor a touch under each.
AUDIT_TOTAL_ROWS = 104_000
AUDIT_BASE_ROWS = 100_000
AUDIT_CLUSTER_ROWS = 2_000


def main() -> int:
    p = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    p.add_argument("--v19-distilled", type=Path, default=V19_DISTILLED)
    p.add_argument("--hard-negatives", type=Path, default=LATDEC_HARD_NEGATIVES)
    p.add_argument("--out", type=Path, default=DEFAULT_OUT)
    p.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    p.add_argument("--skip-audit", action="store_true",
                   help="Diagnostic only; never ship a model trained with this.")
    args = p.parse_args()

    for path in (args.v19_distilled, args.hard_negatives):
        if not path.exists():
            print(f"error: source missing — {path}", file=sys.stderr)
            return 2
    args.out.parent.mkdir(parents=True, exist_ok=True)

    per_source_rows: dict[str, int] = {}
    per_label_rows: dict[str, int] = defaultdict(int)
    per_cluster_rows: dict[str, int] = defaultdict(int)
    hardneg_non_decimal = 0
    total = 0

    print(f"writing {args.out}...", file=sys.stderr)
    with gzip.open(args.out, "wt", newline="", encoding="utf-8") as fout:
        writer = csv.writer(fout)
        writer.writerow(["final_label", "sample_values", "column_name"])

        # ── (1) v19 base distilled (pass-through) ─────────────────────
        n_base = 0
        with gzip.open(args.v19_distilled, "rt", newline="",
                       encoding="utf-8") as fin:
            for row in csv.DictReader(fin):
                label = (row.get("final_label") or "").strip()
                if not label:
                    continue
                writer.writerow([label, row.get("sample_values") or "",
                                 row.get("column_name") or ""])
                per_label_rows[label] += 1
                n_base += 1
                total += 1
        per_source_rows["v19_distilled"] = n_base
        print(f"  {'v19_distilled':>26s}: {n_base:>8,} rows", file=sys.stderr)

        # ── (2) latitude->decimal hard negatives (parquet → 3-col CSV) ─
        try:
            import duckdb  # type: ignore
        except ImportError as exc:
            print(f"error: duckdb missing ({exc}). Run under the venv.",
                  file=sys.stderr)
            return 2
        con = duckdb.connect()
        arrow = con.execute(f"""
            SELECT correct_label, sample_values, column_name, cluster_id
              FROM read_parquet('{args.hard_negatives.as_posix()}')
        """).to_arrow_table()

        n_hn = 0
        for label, sample_list, cname, cluster_id in zip(
            arrow.column("correct_label").to_pylist(),
            arrow.column("sample_values").to_pylist(),
            arrow.column("column_name").to_pylist(),
            arrow.column("cluster_id").to_pylist(),
        ):
            if not label:
                continue
            if label != DECIMAL:
                hardneg_non_decimal += 1
            writer.writerow([label, json.dumps(sample_list or []), cname or ""])
            per_label_rows[label] += 1
            per_cluster_rows[cluster_id] += 1
            n_hn += 1
            total += 1
        per_source_rows["latdec_hard_negatives"] = n_hn
        print(f"  {'latdec_hard_negatives':>26s}: {n_hn:>8,} rows",
              file=sys.stderr)

    print(f"  total: {total:>8,} rows", file=sys.stderr)

    # ── Audit gate ────────────────────────────────────────────────────
    audit_failures: list[str] = []
    if total < AUDIT_TOTAL_ROWS:
        audit_failures.append(f"total rows {total:,} < gate {AUDIT_TOTAL_ROWS:,}")
    n_base = per_source_rows.get("v19_distilled", 0)
    if n_base < AUDIT_BASE_ROWS:
        audit_failures.append(
            f"base v19 rows {n_base:,} < gate {AUDIT_BASE_ROWS:,} "
            "— v19 corpus dropped during blend")
    n_cluster = per_cluster_rows.get(TARGET_CLUSTER_ID, 0)
    if n_cluster < AUDIT_CLUSTER_ROWS:
        audit_failures.append(
            f"cluster {TARGET_CLUSTER_ID}: {n_cluster:,} rows < gate "
            f"{AUDIT_CLUSTER_ROWS:,}")
    if hardneg_non_decimal > 0:
        audit_failures.append(
            f"{hardneg_non_decimal:,} hard-negative rows are not decimal_number "
            "— violates the spec's decimal-only invariant; must be zero")

    manifest = {
        "out": str(args.out),
        "total_rows": total,
        "per_source_rows": per_source_rows,
        "per_cluster_rows": dict(sorted(per_cluster_rows.items())),
        "hardneg_non_decimal_rows": hardneg_non_decimal,
        "per_label_rows": dict(sorted(per_label_rows.items())),
        "audit": {
            "total_rows_gate": AUDIT_TOTAL_ROWS,
            "base_rows_gate": AUDIT_BASE_ROWS,
            "cluster_rows_gate": AUDIT_CLUSTER_ROWS,
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
