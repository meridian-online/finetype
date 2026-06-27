#!/usr/bin/env python3
"""Build the clean-label distilled blend for the clean-label retrain (spec 2026-06-28).

The experiment: hold the shipped architecture + Sharpen FIXED and swap ONLY the
training labels for the SEMANTIC families (geography + person/full_name) from the
noisy distilled-Sherlock labels to authoritative vocab-membership clean positives
(GeoNames geo, Wikidata person). One variable: the source of the semantic-family rows.

Per the ac-0 label-trust audit, vocab-membership as a FILTER on real columns injects
its own noise (false-drops of "City, ST" formats, short country names, non-Anglo names;
state-code contamination of country_code). So we do NOT relabel real columns. Instead we
REPLACE each target family's noisy v3 rows with clean generator positives (clean by
construction — generated FROM the vocab, no false-drop risk). Caps are held identical to
the shipped s43 build (geography 3000 via DOMAIN_CAP_OVERRIDES, identity 600), so the only
config-level change is the distilled source for these families.

Output schema is sherlock_distilled (final_label, sample_values JSON, column_name) —
a drop-in for prepare_multibranch_data.py's --distilled flag.

  clean_label_blend.csv.gz =
      [ all v3 rows whose final_label NOT in REPLACE_LEAVES ]
    + [ GeoNames generator rows whose final_label in GEO_KEEP ]   (clean geo positives)
    + [ Wikidata generator rows (all identity.person.full_name) ]  (clean person positives)
"""
from __future__ import annotations
import argparse, csv, gzip, json, sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
V3 = REPO / "output/distillation-v3/sherlock_distilled.csv.gz"
GEO_GEN = REPO / "output/distillation-v21-geonames/geonames_geography.csv.gz"
PERSON_GEN = REPO / "output/distillation-v22/wikidata_persons.csv.gz"
OUT = REPO / "output/clean-label-retrain/clean_label_blend.csv.gz"
MANIFEST = REPO / "output/clean-label-retrain/clean_label_blend_manifest.json"

# v3 canonical leaves to DROP (replaced by clean generator positives).
REPLACE_LEAVES = {
    "geography.location.city",
    "geography.location.country",
    "geography.location.country_code",
    "geography.location.region",
    "geography.location.continent",
    "identity.person.full_name",
}

# Generator (raw) geo labels to KEEP — the SEMANTIC, open-vocab families only.
# label_remap.json folds us_state/county -> region and country_code_iso3 -> country_code
# downstream in prepare_multibranch_data. We EXCLUDE the validator-owned structural geo
# (postal_code, latitude, longitude) — Sharpen owns those; including them widens the
# variable beyond the semantic mass the thesis targets.
GEO_KEEP = {
    "geography.location.city",
    "geography.location.country",
    "geography.location.country_code",
    "geography.location.country_code_iso3",
    "geography.location.region",
    "geography.location.continent",
    "geography.location.us_state",     # -> region via label_remap
    "geography.location.county",       # -> region via label_remap (if present)
}
GEO_DROP = {
    "geography.address.postal_code",
    "geography.coordinate.latitude",
    "geography.coordinate.longitude",
}


def stream(path):
    with gzip.open(path, "rt", newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            yield row


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, default=OUT)
    ap.add_argument("--manifest", type=Path, default=MANIFEST)
    args = ap.parse_args()

    for name, p in [("v3", V3), ("geo_gen", GEO_GEN), ("person_gen", PERSON_GEN)]:
        if not p.exists():
            print(f"error: missing {name} = {p}", file=sys.stderr)
            return 2

    args.out.parent.mkdir(parents=True, exist_ok=True)
    per_source = Counter()
    per_label = Counter()
    geo_seen = Counter()
    total = 0

    with gzip.open(args.out, "wt", newline="", encoding="utf-8") as fout:
        w = csv.writer(fout)
        w.writerow(["final_label", "sample_values", "column_name"])

        # (1) v3 base, minus the replaced families
        for row in stream(V3):
            lab = (row.get("final_label") or "").strip()
            if not lab or lab in REPLACE_LEAVES:
                continue
            w.writerow([lab, row.get("sample_values") or "", row.get("column_name") or ""])
            per_source["v3_base"] += 1
            per_label[lab] += 1
            total += 1

        # (2) clean GeoNames positives (semantic geo only)
        for row in stream(GEO_GEN):
            lab = (row.get("final_label") or "").strip()
            geo_seen[lab] += 1
            if lab not in GEO_KEEP or lab in GEO_DROP:
                continue
            w.writerow([lab, row.get("sample_values") or "", row.get("column_name") or ""])
            per_source["geo_gen"] += 1
            per_label[lab] += 1
            total += 1

        # (3) clean Wikidata person positives
        for row in stream(PERSON_GEN):
            lab = (row.get("final_label") or "").strip()
            if not lab:
                continue
            w.writerow([lab, row.get("sample_values") or "", row.get("column_name") or ""])
            per_source["person_gen"] += 1
            per_label[lab] += 1
            total += 1

    manifest = {
        "out": str(args.out),
        "total_rows": total,
        "replace_leaves": sorted(REPLACE_LEAVES),
        "geo_keep": sorted(GEO_KEEP),
        "geo_drop": sorted(GEO_DROP),
        "per_source_rows": dict(per_source),
        "geo_gen_labels_seen": dict(sorted(geo_seen.items())),
        "per_label_rows_target": {k: per_label[k] for k in sorted(
            REPLACE_LEAVES | {"geography.location.state_code"})},
    }
    args.manifest.write_text(json.dumps(manifest, indent=2) + "\n")

    print(f"wrote {total:,} rows -> {args.out}", file=sys.stderr)
    print(f"  per source: {dict(per_source)}", file=sys.stderr)
    print("  target-family rows after replace:", file=sys.stderr)
    for k in sorted(REPLACE_LEAVES):
        print(f"    {per_label.get(k,0):>7}  {k}", file=sys.stderr)
    print(f"  manifest: {args.manifest}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
