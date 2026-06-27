#!/usr/bin/env python3
"""ac-0 label-trust audit for the clean-label retrain (spec 2026-06-28).

Author's non-negotiable: "pull the real values and check they make sense" before
training on ANY label source — vocab-membership has its own failure modes (region
collapses on GeoNames admin1). This emits a readable markdown so a human can eyeball
each label source per semantic family and decide which to trust.

For each target family it reports, side by side:
  - REAL v3 distilled rows (the noisy distilled-Sherlock training labels) with the
    vocab-membership verdict (keep leaf vs drop) + fraction in vocab.
  - The CLEAN generator output (GeoNames geo / Wikidata person) sampled rows.
  - For region: admin1-only (known-bad) vs improved (admin1+admin2+suffix+states+ISO)
    pass-rates on the SAME sample, to quantify the collapse + whether the fix recovers.

Writes output/clean-label-retrain/ac0_label_trust_audit.md
"""
from __future__ import annotations
import csv, gzip, json, random, re, sys
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
LR = REPO / "eval/gold/lens_reference"
GN = Path("/Users/hugh/datasets/geonames/2026-05-24")
WD = Path("/Users/hugh/datasets/wikidata/2026-05-25")
V3 = REPO / "output/distillation-v3/sherlock_distilled.csv.gz"
GEO_GEN = REPO / "output/distillation-v21-geonames/geonames_geography.csv.gz"
PERSON_GEN = REPO / "output/distillation-v22/wikidata_persons.csv.gz"
OUT = REPO / "output/clean-label-retrain/ac0_label_trust_audit.md"

SUF = re.compile(r"\s+(county|parish|borough|district|municipality|co\.?|shire|prefecture|"
                 r"province|region|oblast|raion|department|governorate)$", re.I)


def norm(s): return str(s).strip().lower()


def col(path, i, enc="utf-8"):
    out = []
    for line in open(path, encoding=enc):
        p = line.split("\t")
        if len(p) > i and p[i].strip():
            out.append(p[i].strip())
    return out


# ---- build vocabularies ----
cities = set(norm(x) for x in col(LR / "cities15000.txt", 1) + col(LR / "cities15000.txt", 2))
countries = set(norm(r["name"]) for r in csv.DictReader(open(LR / "iso3166.csv")))
ccodes = set()
for r in csv.DictReader(open(LR / "iso3166.csv")):
    ccodes |= {norm(r["alpha-2"]), norm(r["alpha-3"])}
continents = {"africa", "antarctica", "asia", "europe", "north america", "south america",
              "oceania", "australia", "af", "an", "as", "eu", "na", "sa", "oc"}

# region: admin1-only (the known-bad) vs improved (ac01_region_fix)
reg_admin1 = set(norm(x) for x in col(LR / "admin1CodesASCII.txt", 1) + col(LR / "admin1CodesASCII.txt", 2))
reg_imp = set()
for x in (col(LR / "admin1CodesASCII.txt", 1) + col(LR / "admin1CodesASCII.txt", 2)
          + col(GN / "admin2Codes.txt", 1)):
    reg_imp.add(norm(x)); reg_imp.add(SUF.sub("", norm(x)))
for row in csv.reader(open(REPO / "eval/datasets/csv/us_states.csv")):
    for c in row:
        if c.strip():
            reg_imp.add(norm(c))
reg_imp |= {"brooklyn", "queens", "bronx", "manhattan", "staten island", "kings"}

# person: given + family name tokens from Wikidata
def load_name_tokens(path):
    s = set()
    if not path.exists():
        return s
    for r in csv.DictReader(open(path, encoding="utf-8"), delimiter="\t"):
        lab = (r.get("label") or "").strip()
        if lab and any(c.isalpha() for c in lab):
            s.add(norm(lab))
    return s
givens = load_name_tokens(WD / "given_names.tsv")
families = load_name_tokens(WD / "family_names.tsv")
person_tokens = givens | families


def city_match(v): return norm(v) in cities
def country_match(v): return norm(v) in countries
def ccode_match(v): return norm(v) in ccodes
def continent_match(v): return norm(v) in continents
def reg_admin1_match(v):
    n = norm(v); return n in reg_admin1 or SUF.sub("", n) in reg_admin1
def reg_imp_match(v):
    n = norm(v)
    return (n in reg_imp or SUF.sub("", n) in reg_imp
            or (n[:3] in ("us-", "gb-", "ca-") and n[3:] in reg_imp))


def name_token_frac(v):
    """fraction of alpha tokens of a person value that are known given/family names."""
    toks = [norm(t) for t in re.split(r"[\s,]+", str(v)) if t and any(c.isalpha() for c in t)]
    toks = [t.strip(".") for t in toks]
    if not toks:
        return 0.0
    return sum(t in person_tokens for t in toks) / len(toks)


def name_shape(v):
    """heuristic 'looks like a person name': 1-3 alpha tokens, mostly capitalised, no digits."""
    s = str(v).strip()
    if any(c.isdigit() for c in s):
        return False
    toks = [t for t in re.split(r"[\s,]+", s) if t]
    if not (1 <= len(toks) <= 4):
        return False
    return all(any(c.isalpha() for c in t) for t in toks)


FAMILIES = {
    "geography.location.city": ("city", city_match, 0.5),
    "geography.location.country": ("country", country_match, 0.5),
    "geography.location.country_code": ("country_code", ccode_match, 0.5),
    "geography.location.region": ("region", reg_imp_match, 0.4),
    "geography.location.continent": ("continent", continent_match, 0.5),
    "identity.person.full_name": ("person", name_token_frac, 0.5),
}


def frac_in(vals, fn):
    if not vals:
        return 0.0
    return sum((fn(v) if fn.__name__ != "name_token_frac" else (fn(v) >= 0.5)) for v in vals) / len(vals)


def load_v3_by_family(targets, rng, per=40):
    """Sample up to `per` v3 rows per target family. Returns {family: [(header, values)]}."""
    buckets = {t: [] for t in targets}
    with gzip.open(V3, "rt", newline="") as f:
        for row in csv.DictReader(f):
            lab = (row.get("final_label") or "").strip()
            if lab not in buckets:
                continue
            try:
                vals = [str(v).strip() for v in json.loads(row.get("sample_values") or "[]")
                        if str(v).strip() and str(v) != "None"]
            except Exception:
                continue
            if len(vals) < 5:
                continue
            buckets[lab].append((row.get("column_name", "").strip(), vals))
    return buckets


def load_gen_by_family(path, rng, per=12):
    buckets = {}
    if not path.exists():
        return buckets
    with gzip.open(path, "rt", newline="") as f:
        for row in csv.DictReader(f):
            lab = (row.get("final_label") or "").strip()
            try:
                vals = [str(v).strip() for v in json.loads(row.get("sample_values") or "[]") if str(v).strip()]
            except Exception:
                continue
            buckets.setdefault(lab, []).append((row.get("column_name", "").strip(), vals))
    return {k: rng.sample(v, min(per, len(v))) for k, v in buckets.items()}


def fmt_vals(vals, n=12):
    show = vals[:n]
    return " · ".join(show) + (f"  …(+{len(vals)-n})" if len(vals) > n else "")


def main():
    rng = random.Random(42)
    targets = list(FAMILIES)
    v3 = load_v3_by_family(targets, rng)
    geo_gen = load_gen_by_family(GEO_GEN, rng)
    person_gen = load_gen_by_family(PERSON_GEN, rng)

    L = []
    L.append("# ac-0 — Label-trust audit (clean-label retrain)\n")
    L.append(f"vocab sizes: city={len(cities)} country={len(countries)} country_code={len(ccodes)} "
             f"region_admin1={len(reg_admin1)} region_improved={len(reg_imp)} "
             f"person_tokens={len(person_tokens)} (given={len(givens)} family={len(families)})\n")
    L.append("Membership keep rule: fraction of a column's values in the family vocab ≥ threshold → KEEP leaf, else DROP (noise).\n")

    for fam in targets:
        vocab_name, fn, thr = FAMILIES[fam]
        rows = v3.get(fam, [])
        L.append(f"\n## {fam}  (v3 rows: {len(rows)}, threshold {thr})\n")
        if not rows:
            L.append("_no v3 rows_\n")
        else:
            # pass-rate over ALL rows
            def fr(vals):
                if vocab_name == "person":
                    return sum(fn(v) >= 0.5 for v in vals) / max(len(vals), 1)
                return sum(fn(v) for v in vals) / max(len(vals), 1)
            keeps = sum(fr(vals) >= thr for _, vals in rows)
            L.append(f"**membership pass-rate: {keeps}/{len(rows)} = {keeps/len(rows):.1%} would be KEPT**\n")
            if fam == "geography.location.region":
                ka = sum(sum(reg_admin1_match(v) for v in vals)/max(len(vals),1) >= thr for _, vals in rows)
                L.append(f"**region collapse check — admin1-only pass-rate: {ka}/{len(rows)} = {ka/len(rows):.1%}** "
                         f"(improved: {keeps/len(rows):.1%}; the gap is the collapse the fix recovers)\n")
            L.append("\nSample (verdict | frac | header | values):\n")
            sample = rng.sample(rows, min(22, len(rows)))
            for hdr, vals in sample:
                f = fr(vals)
                verdict = "KEEP" if f >= thr else "drop"
                L.append(f"- `{verdict}` f={f:.2f} | **{hdr or '∅'}** | {fmt_vals(vals)}")
        # generator sample
        gen = (person_gen if vocab_name == "person" else geo_gen).get(fam, [])
        if gen:
            L.append(f"\n_CLEAN generator sample for {fam} ({len(gen)} shown):_")
            for hdr, vals in gen[:10]:
                L.append(f"  - **{hdr or '∅'}** | {fmt_vals(vals)}")
        L.append("")

    # generator label coverage summary
    L.append("\n## Generator label coverage\n")
    def gen_counts(path):
        c = Counter()
        if path.exists():
            with gzip.open(path, "rt") as f:
                for row in csv.DictReader(f):
                    c[(row.get("final_label") or "").strip()] += 1
        return c
    gc = gen_counts(GEO_GEN)
    pc = gen_counts(PERSON_GEN)
    L.append("GeoNames generator labels:")
    for lab, n in gc.most_common():
        L.append(f"  - {n:>6}  {lab}")
    L.append("\nWikidata person generator labels:")
    for lab, n in pc.most_common():
        L.append(f"  - {n:>6}  {lab}")

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(L) + "\n")
    print(f"wrote {OUT}")
    print(f"v3 family sizes: " + ", ".join(f"{k.split('.')[-1]}={len(v3.get(k,[]))}" for k in targets))


if __name__ == "__main__":
    main()
