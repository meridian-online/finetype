# v27 recall blend — ac-01/ac-02 memo

**Spec:** `2026-06-11-categorical-identifier-recall-retrain` ·
**Candidate:** sherlock-v27-recall (multi-branch, v19 lineage, ReLU+BN) ·
**Date:** 2026-06-11

## What the blend bets

The two largest gold recall pools are starved-positive problems, not
rule problems: v19 trains on ~600 distilled rows each (the global
`--distilled-cap 600`) for labels whose corpus presence is far larger and
whose gold recalls are 0.386 (categorical) and 0.111 (alphanumeric_id). The
blend adds 4,754 curated positives mined from v19's OWN corpus mistakes and
lifts exactly two per-label caps so the additions survive the cap:

| label | gold recall | base distilled rows | mined added | FTMB `--type-cap` |
|---|---:|---:|---:|---:|
| representation.discrete.categorical | 0.386 | 16,994 (capped at 600) | +2,660 | 3,400 |
| representation.identifier.alphanumeric_id | 0.111 | 3,065 (capped at 600) | +2,094 | 2,400 |

Everything else is byte-identical to the v19 recipe
(`overnight_v19_paired.sh`: v3 distilled, samples-per-type 1200, synthetic
1200, ratio 0.7, aug 0.35, filter+decontaminate, cap 600, hn 75/50/25, v4,
seed 42). The latdec-era `TYPE_CAP_OVERRIDES` decimal=2600 entry was removed
from the dict and replaced by a CLI flag (`--type-cap LABEL=N`) so per-build
overrides cannot leak across retrains — this build does NOT carry the decimal
lift.

## Where the positives come from (mechanism-matched buckets)

Mining: `scripts/mine_v27_recall_positives.py` →
`output/v27-recall-retrain/hard_positives.parquet` (manifest + ≥20-row
spot-check tables alongside). Source: `output/ydf-validation-gate/
v19_gated.parquet` — latdec ac-01 established its `sense_prediction` IS the
shipped v19's.

**alphanumeric_id (2,094)** — gold mechanism: 32/48 FNs are Sense asserting
exotic tight codes (h3 ×17, cryptographic hash ×8) that the validation veto
rightly rejects → unknown; the rest absorbed by tight-code validators.

- A1_hash 900 / A2_h3 500 — exotic-code assertions with ydf-alnum
  corroboration or id-shaped headers (`wfo-0000891258`, `msg24159928`)
- A3_geohash 300, A4_secur 300 (cusip/sedol/hcpcs/unlocode absorption,
  Wikidata QIDs, node codes), A5_legacy 94 (the deterministic layer's
  non-taxonomy alias `representation.alphanumeric.alphanumeric_id`, ydf-alnum
  corroborated — GenBank accessions, ticket numbers)

**categorical (2,660)** — gold mechanism: vocab columns (type/status/class)
scatter into word/ordinal/entity_name/specific codes.

- B1_tld 900 (cluster safety 0.972 HIGH — `comment|story`,
  `freshman|sophomore`), B2_word 700 (0.690 — `act_tag`, `difficulty`,
  `Discourse Facet`), B3_gender_code 310 (0.600, v23-clean precedent —
  basketball `START_POSITION`), B4_blood_type 450 (0.722 — `RACE`,
  `speaker`), B5_ordinal 300 (0.517 — taxonomic `rank`, forum `Rank`)

**Value-based curation** (precision principle applied to mining):
categoricals must look categorical (n_distinct ≤ 12, distinct_ratio ≤ 0.6,
repetition), ids must look like ids (distinct_ratio ≥ 0.7, ≥60% of values mix
letters+digits). Plus hygiene: no `__index_level_*`/`Unnamed:`/hex headers,
≤3 columns per file sha, ≤40 columns per identical vocabulary (kills the
Hacker-News `comment|story` monoculture), and **every file sha in the gold
corpus excluded (756 shas)** — the train↔gold firewall, `make leakage-guard`
green on the built blend.

## Exclusion ledger (what was deliberately NOT mined)

| bucket | why |
|---|---|
| city, iata_code, full_name | v23 blast paths (city absorbed 48k); safety unscored/<0.5 |
| periodicity, url | v23 empirically unreachable via training_data_addition |
| boolean.terms | crisp family (gold P=1.0) — do not blur |
| gender, phone_number, currency.amount | identity/finance-adjacent, low-MODERATE safety, no gold-FN corroboration |
| sense=hash ∧ ydf=hash (523 cols) | genuine hashes — correct as-is |

## Risk register

This is the first recall-direction retrain since v23 (categorical +529%,
48k city columns absorbed). The lift is deliberately moderate — categorical
training presence 600 → ≤3,400 (~5.7×) of a 107k-row corpus, alnum 600 →
≤2,400 — and the drift proxy (ac-03, blocking) runs before any overnight
spend, with the calibrated full-label band that caught v23 and v24
retroactively. Watch labels at proxy time: categorical, alphanumeric_id
(intended movers — the band tolerates intended movement below 3× relative),
city, word, entity_name, tld, ordinal, boolean.terms (must NOT move).

Pre-committed halts: proxy NO-GO → re-scope once (drop riskiest bucket or
lower caps) → second NO-GO closes Failed-informative. Corpus-honest NO-GO
post-train is final for this candidate.

## Artefacts

- `output/v27-recall-retrain/hard_positives.parquet` (4,754 rows)
- `output/v27-recall-retrain/mining_manifest.json` (per-bucket counts, filters, ledger)
- `output/v27-recall-retrain/spot_check.md` (≥20 rows per bucket, reviewed)
- `output/v27-recall-retrain/sherlock_distilled_v27.csv.gz` (107,195 rows; audit gate PASSED)
- `output/v27-recall-retrain/v27_blend_manifest.json`
