#!/usr/bin/env python3
"""v27 recall retrain ac-01 — mine + curate hard POSITIVES for the two gold
recall pools: representation.discrete.categorical and
representation.identifier.alphanumeric_id.

Spec 2026-06-11-categorical-identifier-recall-retrain. Sources are v19's OWN
corpus mistakes (output/ydf-validation-gate/v19_gated.parquet — latdec ac-01
established sense_prediction there IS the shipped v19's). Buckets mirror the
gold-measured mechanisms:

  alphanumeric_id (gold recall 0.111; 32/48 FNs = exotic tight-code assertion
  then veto→unknown):
    A1_hash      sense=technology.cryptographic.hash, ydf corroborates alnum
                 OR id-shaped header on plain_text/entity_name/NULL ydf
    A2_h3        sense=geography.index.h3, same corroboration
    A3_geohash   sense=geography.coordinate.geohash AND ydf_gated=alnum
    A4_secur     sense in (cusip, sedol, hcpcs, unlocode) AND ydf_gated=alnum
    A5_legacy    sense=representation.alphanumeric.alphanumeric_id (the
                 deterministic layer's non-taxonomy alias) AND ydf_gated=alnum

  categorical (gold recall 0.386; FNs scatter to word/ordinal/entity/codes):
    B1_tld          sense=top_level_domain      (cluster safety 0.972 HIGH)
    B2_word         sense=text.word             (safety 0.690 MODERATE)
    B3_gender_code  sense=gender_code           (safety 0.600; v23-clean)
    B4_blood_type   sense=blood_type            (safety 0.722 MODERATE)
    B5_ordinal      sense=discrete.ordinal      (safety 0.517 MODERATE)
    all requiring ydf_prediction_gated=categorical.

EXCLUSION LEDGER (buckets deliberately NOT mined — recorded in the manifest):
  - geography.location.city / iata_code / identity.person.full_name: v23 blast
    paths; the safety metric itself scores them <0.5 or unscored.
  - datetime.component.periodicity, technology.internet.url: v23 empirically
    unreachable via training_data_addition.
  - representation.boolean.terms: crisp family (gold P=1.0) — do not blur.
  - identity.person.gender / phone_number / finance.currency.amount: identity/
    finance-adjacent, MODERATE-low safety, no gold-FN corroboration.

Value-based curation (the precision principle, applied to mining):
  categorical = LOW cardinality with repetition (n_distinct<=12,
                distinct_ratio<=0.6, n_values>=4, n_distinct>=2)
  alnum id    = HIGH cardinality (distinct_ratio>=0.7) and >=60% of values mix
                letters+digits; n_values>=3
  both        = drop headerless artefacts (__index_level_*, Unnamed:*, hex
                headers), <=3 columns per file sha, vocab-signature dedupe for
                categorical (<=40 per identical vocabulary), and EXCLUDE every
                file sha present in eval/gold/gold_corpus_v1.tsv (leakage).

Deterministic: ordering by hash(sha||column), no RNG.

Run: eval/gittables/.venv/bin/python scripts/mine_v27_recall_positives.py
Out: output/v27-recall-retrain/hard_positives.parquet
     output/v27-recall-retrain/mining_manifest.json
     output/v27-recall-retrain/spot_check.md  (>=20 rows per bucket)
"""
from __future__ import annotations

import csv
import json
import re
import sys
from collections import defaultdict
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
GATED = REPO / "output/ydf-validation-gate/v19_gated.parquet"
GOLD = REPO / "eval/gold/gold_corpus_v1.tsv"
OUT_DIR = REPO / "output/v27-recall-retrain"

ALNUM = "representation.identifier.alphanumeric_id"
CAT = "representation.discrete.categorical"

ID_HEADER_RE = r"(^|_)(id|key|code|uid|guid|ref)s?$|_id_|^id_"
BAD_HEADER_RE = re.compile(r"^__index_level_|^Unnamed|^0x[0-9a-fA-F]+$|^$")

# bucket -> (label, sql condition, per-bucket cap)
ID_CORR = (
    f"(ydf_prediction_gated = '{ALNUM}' OR ((ydf_prediction_gated IS NULL OR "
    f"ydf_prediction_gated IN ('representation.text.plain_text',"
    f"'representation.text.entity_name')) AND "
    f"regexp_matches(lower(column_name), '{ID_HEADER_RE}')))"
)
BUCKETS = {
    "A1_hash": (ALNUM, f"sense_prediction='technology.cryptographic.hash' AND {ID_CORR}", 900),
    "A2_h3": (ALNUM, f"sense_prediction='geography.index.h3' AND {ID_CORR}", 500),
    "A3_geohash": (ALNUM, f"sense_prediction='geography.coordinate.geohash' AND ydf_prediction_gated='{ALNUM}'", 300),
    "A4_secur": (ALNUM, "sense_prediction IN ('finance.securities.cusip','finance.securities.sedol',"
                        f"'identity.medical.hcpcs','geography.transportation.unlocode') AND ydf_prediction_gated='{ALNUM}'", 300),
    "A5_legacy": (ALNUM, f"sense_prediction='representation.alphanumeric.alphanumeric_id' AND ydf_prediction_gated='{ALNUM}'", 400),
    "B1_tld": (CAT, f"sense_prediction='technology.internet.top_level_domain' AND ydf_prediction_gated='{CAT}'", 900),
    "B3_gender_code": (CAT, f"sense_prediction='identity.person.gender_code' AND ydf_prediction_gated='{CAT}'", 450),
    "B4_blood_type": (CAT, f"sense_prediction='identity.person.blood_type' AND ydf_prediction_gated='{CAT}'", 450),
    "B5_ordinal": (CAT, f"sense_prediction='representation.discrete.ordinal' AND ydf_prediction_gated='{CAT}'", 300),
}

EXCLUSION_LEDGER = {
    "geography.location.city": "v23 blast path (48k absorbed); safety unscored/<0.5",
    "geography.transportation.iata_code": "v23-adjacent geography code; excluded per spec",
    "identity.person.full_name": "identity-adjacent; 92.8k pool is mostly genuine names",
    "datetime.component.periodicity": "v23 empirically unreachable",
    "technology.internet.url": "v23 empirically unreachable",
    "representation.boolean.terms": "crisp family, gold P=1.0 — do not blur",
    "identity.person.gender": "identity-adjacent, safety 0.524 low-MODERATE",
    "identity.person.phone_number": "identity-adjacent, no gold-FN corroboration",
    "finance.currency.amount": "finance-adjacent; gold currency family already broken (P=0.100)",
    "sense=hash AND ydf=hash": "genuine hashes (523 cols) — correct as-is",
    "representation.text.word (B2, dropped round 2)": (
        "proxy round-1 NO-GO: categorical 5.76x over-emit with word collapsing "
        "to 0.07x — the fuzziest boundary bucket, dropped in the pre-committed "
        "re-scope alongside the categorical cap halving (3400->1700)"),
}


def parse_values(raw: str) -> list[str]:
    vals = [v for v in (raw or "").split("│") if v]
    if len(vals) > 1:
        vals = vals[:-1]  # truncation may cut the final token
    return vals


def categorical_ok(vals: list[str]) -> bool:
    if len(vals) < 4:
        return False
    d = set(vals)
    return 2 <= len(d) <= 12 and len(d) / len(vals) <= 0.6


def alnum_ok(vals: list[str]) -> bool:
    if len(vals) < 3:
        return False
    d = set(vals)
    if len(d) / len(vals) < 0.7:
        return False
    mixed = sum(1 for v in vals if re.search(r"[A-Za-z]", v) and re.search(r"[0-9]", v))
    return mixed / len(vals) >= 0.6


def main() -> int:
    import duckdb

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    gold_shas = set()
    with open(GOLD, newline="") as f:
        for row in csv.DictReader(f, delimiter="\t"):
            gold_shas.add(row["file_content_sha256"])
    print(f"gold firewall: {len(gold_shas)} file shas excluded", file=sys.stderr)

    con = duckdb.connect()
    kept: list[dict] = []
    manifest_buckets: dict[str, dict] = {}
    spot_lines: list[str] = ["# v27 recall positives — spot-check (>=20/bucket)\n"]

    for name, (label, cond, cap) in BUCKETS.items():
        rows = con.execute(f"""
            SELECT file_content_sha256, column_name, sample_values_truncated
              FROM read_parquet('{GATED.as_posix()}')
             WHERE {cond} AND is_trivial = false
               AND length(sample_values_truncated) > 3
             ORDER BY hash(file_content_sha256 || column_name)
        """).fetchall()
        per_file: dict[str, int] = defaultdict(int)
        per_vocab: dict[tuple, int] = defaultdict(int)
        bucket_kept = []
        filtered = {"gold_sha": 0, "bad_header": 0, "value_shape": 0,
                    "per_file_cap": 0, "vocab_dedupe": 0}
        for sha, cname, raw in rows:
            if len(bucket_kept) >= cap:
                break
            if sha in gold_shas:
                filtered["gold_sha"] += 1
                continue
            if BAD_HEADER_RE.search(cname or ""):
                filtered["bad_header"] += 1
                continue
            vals = parse_values(raw)
            ok = categorical_ok(vals) if label == CAT else alnum_ok(vals)
            if not ok:
                filtered["value_shape"] += 1
                continue
            if per_file[sha] >= 3:
                filtered["per_file_cap"] += 1
                continue
            if label == CAT:
                sig = tuple(sorted(set(vals)))
                if per_vocab[sig] >= 40:
                    filtered["vocab_dedupe"] += 1
                    continue
                per_vocab[sig] += 1
            per_file[sha] += 1
            bucket_kept.append({
                "correct_label": label,
                "sample_values": raw,
                "column_name": cname or "",
                "cluster_id": name,
                "file_content_sha256": sha,
            })
        kept.extend(bucket_kept)
        manifest_buckets[name] = {
            "label": label, "pool_scanned": len(rows), "kept": len(bucket_kept),
            "cap": cap, "filtered": filtered,
        }
        print(f"  {name:>15s}: kept {len(bucket_kept):>4d} / scanned {len(rows):>6d} (cap {cap})",
              file=sys.stderr)
        spot_lines.append(f"\n## {name} ({label}) — kept {len(bucket_kept)}\n")
        spot_lines.append("| column_name | sample_values (first 100 chars) |")
        spot_lines.append("|---|---|")
        for r in bucket_kept[:25]:
            sv = r["sample_values"][:100].replace("|", "/")
            spot_lines.append(f"| `{r['column_name'][:40]}` | `{sv}` |")

    per_label = defaultdict(int)
    for r in kept:
        per_label[r["correct_label"]] += 1

    con.execute("CREATE TABLE hp (correct_label VARCHAR, sample_values VARCHAR, "
                "column_name VARCHAR, cluster_id VARCHAR, file_content_sha256 VARCHAR)")
    con.executemany("INSERT INTO hp VALUES (?,?,?,?,?)",
                    [(r["correct_label"], r["sample_values"], r["column_name"],
                      r["cluster_id"], r["file_content_sha256"]) for r in kept])
    out_parquet = OUT_DIR / "hard_positives.parquet"
    con.execute(f"COPY hp TO '{out_parquet.as_posix()}' (FORMAT PARQUET)")

    manifest = {
        "spec": "2026-06-11-categorical-identifier-recall-retrain",
        "source": str(GATED.relative_to(REPO)),
        "gold_firewall_file_shas": len(gold_shas),
        "buckets": manifest_buckets,
        "per_label_totals": dict(per_label),
        "exclusion_ledger": EXCLUSION_LEDGER,
        "filters": {
            "categorical": "n_values>=4, 2<=n_distinct<=12, distinct_ratio<=0.6, vocab_sig<=40",
            "alnum": "n_values>=3, distinct_ratio>=0.7, letter+digit mix>=60%",
            "both": "is_trivial=false, header not __index_level_/Unnamed/hex, <=3 cols/file sha, gold shas excluded",
        },
    }
    (OUT_DIR / "mining_manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    (OUT_DIR / "spot_check.md").write_text("\n".join(spot_lines) + "\n")
    print(f"\nwrote {out_parquet} ({len(kept)} rows): {dict(per_label)}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
