#!/usr/bin/env python3
"""ac-03: survivor census — the cheap go/no-go before the triad is built.

From the ac-02 scored-values parquet, count high-confidence (conf >= floor)
survivors per type, with latitude and utc called out explicitly. If a disease
type falls below the usable per-type floor, the triad should NOT be built on a
starved type — HALT and relax the floor first.

Counts are reported at three granularities per type:
  - distinct values  (the de-duplicated clean training pool)
  - value-rows       (occurrences across the corpus)
  - distinct columns (how many real columns contribute a surviving value)

Run from the eval venv:
  PYTHONPATH=scripts/value_ydf eval/gittables/.venv/bin/python \
    scripts/value_ydf/ac03_survivor_census.py
"""
from __future__ import annotations

import argparse
import json

import pandas as pd

import common as C

DISEASES = ["geography.coordinate.latitude", "datetime.offset.utc"]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--floor", type=float, default=0.85, help="confidence floor")
    ap.add_argument("--per-type-floor", type=int, default=50, help="usable per-type floor (distinct values)")
    args = ap.parse_args()

    scored = C.OUT_DIR / "scored_values.parquet"
    df = pd.read_parquet(scored, columns=["file_path", "column_name", "value", "ydf_label", "ydf_confidence"])
    n_total = len(df)
    surv = df[df["ydf_confidence"] >= args.floor]

    g = surv.groupby("ydf_label")
    census = pd.DataFrame(
        {
            "distinct_values": g["value"].nunique(),
            "value_rows": g.size(),
            "distinct_columns": g.apply(lambda x: x[["file_path", "column_name"]].drop_duplicates().shape[0], include_groups=False),
        }
    ).sort_values("distinct_values", ascending=False)

    starved = {}
    for d in DISEASES:
        n = int(census.loc[d, "distinct_values"]) if d in census.index else 0
        starved[d] = n

    halt = any(n < args.per_type_floor for n in starved.values())

    report = {
        "floor": args.floor,
        "per_type_floor": args.per_type_floor,
        "scored_rows": n_total,
        "survivor_rows": int(len(surv)),
        "survivor_fraction": round(len(surv) / n_total, 4) if n_total else 0.0,
        "types_with_survivors": int(census.shape[0]),
        "disease_survivors": starved,
        "halt": halt,
    }
    (C.OUT_DIR / "ac03_census.json").write_text(json.dumps(report, indent=2))
    census.to_csv(C.OUT_DIR / "ac03_census.csv")

    lines = [
        "# ac-03 — survivor census (value-level prior @ conf >= "
        f"{args.floor})",
        "",
        f"- scored value-rows: **{n_total:,}**",
        f"- survivors (conf >= {args.floor}): **{len(surv):,}** "
        f"({report['survivor_fraction']*100:.1f}%)",
        f"- types with >=1 survivor: **{census.shape[0]}**",
        f"- usable per-type floor: **{args.per_type_floor}** distinct values",
        "",
        "## Disease types (the head-to-head exists to test these)",
        "",
        "| type | distinct values | value-rows | distinct columns | status |",
        "|---|---|---|---|---|",
    ]
    for d in DISEASES:
        if d in census.index:
            row = census.loc[d]
            dv, vr, dc = int(row["distinct_values"]), int(row["value_rows"]), int(row["distinct_columns"])
        else:
            dv = vr = dc = 0
        status = "OK" if dv >= args.per_type_floor else "STARVED"
        lines.append(f"| {d} | {dv} | {vr} | {dc} | {status} |")
    lines += [
        "",
        "## Top 20 surviving types by distinct values",
        "",
        "| type | distinct values | value-rows | distinct columns |",
        "|---|---|---|---|",
    ]
    for lbl, row in census.head(20).iterrows():
        lines.append(f"| {lbl} | {int(row['distinct_values'])} | {int(row['value_rows'])} | {int(row['distinct_columns'])} |")
    lines += [
        "",
        f"**Verdict:** {'HALT — relax the floor; a disease type is starved.' if halt else 'PROCEED — disease types clear the floor; build the triad.'}",
    ]
    (C.OUT_DIR / "ac03_census.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines[-12:]))
    print(f"\nhalt={halt}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
