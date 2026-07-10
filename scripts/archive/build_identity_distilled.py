#!/usr/bin/env python3
"""Identity fortification — build the distilled blend (spec
2026-06-18-identity-header-hint-fortification ac-02).

Additive over the v3 distilled base — the EXACT base the shipped v19 trained on
(overnight_v19_paired.sh: DISTILLED_FILE=output/distillation-v3/sherlock_distilled.csv.gz)
— so the only training delta vs v19 is the identity additions, and the drift
proxy's drift-from-v19 isolates the identity effect rather than a base mismatch.

Concatenates, in the 3-column format prepare_multibranch_data.py accepts
(final_label, sample_values, column_name):
  1. v3 distilled (102,461 rows) — final_label + sample_values, column_name="" (v19
     contributed NO header from the distilled; the header branch learned from
     prepare's synthetic generation — see load_distilled_columns row.get).
  2. identity additions (identity_hard_negatives.parquet, ac-02) — final_label +
     sample_values + column_name="Author"/"login"/... so the header branch now sees
     author/login HEADER paired with handle VALUES -> username (and multi-token ->
     full_name). This is the fortification signal.

Output: output/distillation-identity/sherlock_distilled_identity.csv.gz
Run under the gittables venv (duckdb module for parquet read).
"""
from __future__ import annotations

import argparse
import csv
import gzip
import json
import subprocess
import sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
V3_DISTILLED = REPO / "output/distillation-v3/sherlock_distilled.csv.gz"
IDENTITY_PARQUET = REPO / "output/identity-fortification/identity_hard_negatives.parquet"
DEFAULT_OUT = REPO / "output/distillation-identity/sherlock_distilled_identity.csv.gz"
SAMPLE_DELIM = "│"


def read_identity_rows(parquet: Path) -> list[tuple[str, str, str]]:
    """(final_label, sample_values_json_list, column_name) from the parquet."""
    # Emit sample_values as a JSON-ish list string matching the v3 distilled
    # sample_values column convention (a bracketed list the prepare parser splits).
    sql = (
        f"SELECT correct_label, list_aggregate(sample_values, 'string_agg', '{SAMPLE_DELIM}') AS sv, "
        f"  column_name FROM read_parquet('{parquet.as_posix()}')"
    )
    r = subprocess.run(["duckdb", "-noheader", "-csv", "-c", sql],
                       capture_output=True, text=True)
    if r.returncode != 0:
        print(f"duckdb error:\n{r.stderr}", file=sys.stderr)
        sys.exit(1)
    rows = []
    for row in csv.reader(r.stdout.splitlines()):
        if len(row) < 3:
            continue
        label, sv, col = row[0], row[1], row[2]
        # v3 sample_values are a JSON array string; match that so the same parser
        # path handles both. Build a JSON array from the delim-split values.
        vals = [v for v in sv.split(SAMPLE_DELIM) if v]
        rows.append((label, json.dumps(vals), col))
    return rows


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--v3", type=Path, default=V3_DISTILLED)
    ap.add_argument("--identity", type=Path, default=IDENTITY_PARQUET)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args()
    args.out.parent.mkdir(parents=True, exist_ok=True)

    label_counts: Counter = Counter()
    n_v3 = n_id = 0
    with gzip.open(args.out, "wt", newline="", encoding="utf-8") as fout:
        w = csv.writer(fout)
        w.writerow(["final_label", "sample_values", "column_name"])
        # 1. v3 base — pass through final_label + sample_values, empty column_name.
        with gzip.open(args.v3, "rt", newline="", encoding="utf-8") as fin:
            for row in csv.DictReader(fin):
                label = (row.get("final_label") or "").strip()
                sv = row.get("sample_values") or ""
                if not label or not sv:
                    continue
                w.writerow([label, sv, ""])
                label_counts[label] += 1
                n_v3 += 1
        # 2. identity additions — with column_name (the header signal).
        for label, sv, col in read_identity_rows(args.identity):
            w.writerow([label, sv, col])
            label_counts[label] += 1
            n_id += 1

    print(f"v3 base rows: {n_v3:,}")
    print(f"identity additions: {n_id:,}")
    print(f"  username:  {label_counts['identity.person.username']:,}")
    print(f"  full_name: {label_counts['identity.person.full_name']:,}")
    print(f"wrote {args.out} ({n_v3 + n_id:,} rows)")
    # Sanity: identity additions present and non-trivial.
    if n_id < 2000:
        print(f"WARN: only {n_id} identity rows — expected ~2700", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
