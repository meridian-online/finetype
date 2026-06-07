#!/usr/bin/env python3
"""B2 mining factory — ac-02: JSON-Schema validation-as-veto precision funnel.

Spec 2026-06-07-reference-data-mining-factory, ac-02.

Every manufactured value is authoritative *by source*, but authoritative-by-source
is not the same as well-formed-for-its-label. A GeoNames place row can carry a
malformed postal code; a CLDR name can be punctuation; a coordinate can format
oddly. This funnel adjudicates each (value, type) by the type's own JSON-Schema —
the same per-value contract the value-level pipeline uses (the 277-dim
extract_value_features export, of which 240 dims are `schema_pass:<type>` 0/1
validity flags) — and keeps only values that validate as the label they carry.

Two stages:

  1. SELF-VETO. Drop (value, type) where schema_pass:type == 0. This is the
     load-bearing precision filter — the value does not validate as its own label.

  2. COLLISION RESOLUTION. When the SAME value string is manufactured under >= 2
     DIFFERENT Tier-1 labels (e.g. "CA" as both state_code and country_code,
     "Georgia" as both country and region), that is contradictory supervision.
     Resolve by a documented source-authority precedence: the most specific
     canonical registry wins; the others are dropped.

     EXEMPTION: the {latitude, longitude} pair is NOT collision-resolved. Their
     value spaces genuinely overlap ([-90,90] vs [-180,180]); a bare coordinate is
     ambiguous by value alone and is disambiguated at COLUMN level (range + header)
     where the multi-branch model operates. This overlap IS the confusion family
     the spec exists to teach, not a defect to sand off.

  NOTE on schema-level collision: judging "validates as >= 2 types" against the
  full 240-schema vector is uninformative here because several Tier-1 schemas are
  deliberately permissive (city/region accept almost any token — the Precision
  Principle's "a validation that confirms 90% of input is not a validation"). So
  collision is scoped to same-value-multiple-MANUFACTURED-label, which is the real
  training-label conflict.

Output:
  output/mining-factory/filtered_values.ndjson  — survivors {value,type,source,locale}
  output/mining-factory/ac02_precision_funnel.md — per-type survival + drop reasons
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
sys.path.insert(0, str(REPO / "scripts" / "value_ydf"))

from value_ydf.common import export_features  # noqa: E402

OUT_DIR = REPO / "output" / "mining-factory"

# Source-authority precedence — most specific canonical registry first. A value
# that is a valid country_code is a country_code before it is anything softer.
PRECEDENCE = [
    "geography.location.country_code",
    "finance.currency.currency_code",
    "technology.code.locale_code",
    "technology.internet.top_level_domain",
    "technology.internet.http_method",
    "identity.person.gender_code",
    "identity.person.blood_type",
    "finance.currency.currency_symbol",
    "geography.location.continent",
    "geography.location.state_code",
    "geography.coordinate.latitude",
    "geography.coordinate.longitude",
    "geography.location.country",
    "geography.location.region",
    "geography.location.city",
    "geography.address.postal_code",
    "geography.address.street_suffix",
    "geography.address.street_name",
    "datetime.component.month_name",
    "datetime.date.abbreviated_month",
    "datetime.component.day_of_week",
]
PRECEDENCE_RANK = {t: i for i, t in enumerate(PRECEDENCE)}

# Pairs whose value-space overlap is the deliberate confusion signal — never
# collision-resolved against each other.
COLLISION_EXEMPT = [frozenset({"geography.coordinate.latitude",
                               "geography.coordinate.longitude"})]


def load_rows(path: Path) -> list[dict]:
    rows = []
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def featurise(values: list[str]) -> dict[str, dict[str, int]]:
    """Return value -> {type: 0/1} for the schema_pass vector of each value."""
    import pandas as pd

    ndjson = OUT_DIR / "_funnel_values.ndjson"
    out_csv = OUT_DIR / "_funnel_values.csv"
    with ndjson.open("w", encoding="utf-8") as f:
        for v in values:
            f.write(json.dumps({"text": v}, ensure_ascii=False) + "\n")
    export_features(ndjson, out_csv)
    df = pd.read_csv(out_csv, dtype=str, keep_default_na=False)
    schema_cols = [c for c in df.columns if c.startswith("schema_pass:")]
    out: dict[str, dict[str, int]] = {}
    for _, r in df.iterrows():
        passes = {c.replace("schema_pass:", ""): (1 if r[c] not in ("0", "0.0", "0.000000", "") else 0)
                  for c in schema_cols}
        out[r["text"]] = passes
    ndjson.unlink(missing_ok=True)
    out_csv.unlink(missing_ok=True)
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    ap.add_argument("--in", dest="inp", type=Path,
                    default=OUT_DIR / "manufactured_values.ndjson")
    ap.add_argument("--out", type=Path, default=OUT_DIR / "filtered_values.ndjson")
    args = ap.parse_args()

    rows = load_rows(args.inp)
    distinct_values = sorted({r["value"] for r in rows})
    print(f"featurising {len(distinct_values):,} distinct values "
          f"(from {len(rows):,} rows)...", file=sys.stderr)
    passmap = featurise(distinct_values)

    per_type_in: dict[str, int] = defaultdict(int)
    per_type_schema_veto: dict[str, int] = defaultdict(int)
    per_type_collision_drop: dict[str, int] = defaultdict(int)
    per_type_survive: dict[str, int] = defaultdict(int)

    # Stage 1 — self-veto.
    stage1: list[dict] = []
    for r in rows:
        per_type_in[r["type"]] += 1
        ok = passmap.get(r["value"], {}).get(r["type"], 0)
        if ok:
            stage1.append(r)
        else:
            per_type_schema_veto[r["type"]] += 1

    # Stage 2 — collision resolution on same value carrying >= 2 distinct labels.
    labels_by_value: dict[str, set[str]] = defaultdict(set)
    for r in stage1:
        labels_by_value[r["value"]].add(r["type"])

    def exempt(a: str, b: str) -> bool:
        return any({a, b} <= pair for pair in COLLISION_EXEMPT)

    # For each value, choose the winning label set: highest-precedence label, plus
    # any label exempt-paired with it.
    winners_by_value: dict[str, set[str]] = {}
    for value, labels in labels_by_value.items():
        if len(labels) <= 1:
            winners_by_value[value] = set(labels)
            continue
        best = min(labels, key=lambda t: PRECEDENCE_RANK.get(t, 999))
        keep = {best}
        for other in labels:
            if other != best and exempt(best, other):
                keep.add(other)
        winners_by_value[value] = keep

    survivors: list[dict] = []
    for r in stage1:
        if r["type"] in winners_by_value.get(r["value"], set()):
            survivors.append(r)
            per_type_survive[r["type"]] += 1
        else:
            per_type_collision_drop[r["type"]] += 1

    with args.out.open("w", encoding="utf-8") as f:
        for r in survivors:
            f.write(json.dumps(r, ensure_ascii=False) + "\n")

    # Report.
    all_types = sorted(per_type_in)
    total_in = sum(per_type_in.values())
    total_veto = sum(per_type_schema_veto.values())
    total_coll = sum(per_type_collision_drop.values())
    total_surv = sum(per_type_survive.values())
    lines = [
        "# ac-02 — precision funnel: JSON-Schema validation-as-veto",
        "",
        "Spec `2026-06-07-reference-data-mining-factory`, ac-02. Every manufactured",
        "value adjudicated by its label's own JSON-Schema (the 240-dim",
        "`schema_pass` vector from `extract_value_features`). Two stages: self-veto",
        "(drop where the value does not validate as its own label) then collision",
        "resolution (same value under >= 2 labels -> source-authority precedence;",
        "the latitude/longitude overlap is exempt — it is the deliberate signal).",
        "",
        f"**{total_surv:,} of {total_in:,} rows survive "
        f"({100*total_surv/max(1,total_in):.1f}%). "
        f"Schema-veto dropped {total_veto:,}; collision dropped {total_coll:,}.**",
        "",
        "| type | in | schema-veto | collision-drop | survive | survive % |",
        "|---|---:|---:|---:|---:|---:|",
    ]
    for t in all_types:
        i = per_type_in[t]
        sv = per_type_schema_veto[t]
        cd = per_type_collision_drop[t]
        su = per_type_survive[t]
        lines.append(f"| `{t}` | {i:,} | {sv:,} | {cd:,} | {su:,} | {100*su/max(1,i):.1f}% |")
    (OUT_DIR / "ac02_precision_funnel.md").write_text("\n".join(lines) + "\n",
                                                      encoding="utf-8")
    print(f"survivors: {total_surv:,}/{total_in:,} -> {args.out}", file=sys.stderr)
    print(f"  schema-veto {total_veto:,}, collision-drop {total_coll:,}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
