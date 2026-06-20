# ac-01 result — gte-tiny embed feature binary (FTMB v5)

**Deliverable:** `output/multibranch-training/v5-gte-blend.ftmb` (924 MB, gitignored)
Built 2026-06-20 by `scripts/build_ftmb_v5_gte.py` (89.9 min, 0 extraction errors).

## What it is
v19's **exact** recipe (`overnight_v19_paired.sh` args, run verbatim via monkeypatch
of `prepare_multibranch_data.py` — no reimplementation, so the data blend cannot
drift) with **only the embed slot swapped**: frozen gte-tiny, mean-pooled over each
column's sampled values → 384-dim. Char/stats/header(Model2Vec 128)/validation
branches untouched.

## Verification (full structural parse)
| field | value | check |
|---|---|---|
| version | 5 | gte marker, distinct from v4/Model2Vec |
| records | 131,831 | all parsed, **0 trailing bytes** (aligned) |
| table groups | 16,788 (avg 7.9 cols) | sibling-context intact |
| embed dim | **384** | gte-tiny |
| valid dim | **244** | live taxonomy (v19 was 240) |
| char/stats/header | 960 / 27 / 128 | unchanged from v19 |
| distinct labels | 243 / 244 | 1 absent (v19-normal; gate ≥238) |
| min label count | 57 | ≥50 trainability gate PASS |
| embed distinctness | 289/300 distinct, stdev 0.13 | gte varies per column, not collapsed |

Smoke-validated before the full build: 120-record v5 round-tripped through
`read_ftmb.py` AND a live 1-epoch Rust train on Metal ("Loaded … 384 embed … 40
table groups").

## Carry-forward to ac-02 / ac-03
- **Model config for ac-02:** `embed_dim=384`, `valid_dim=244`; the Rust trainer
  auto-derives n_classes from the taxonomy (244). Base on `models/sherlock-v13-config.json`
  (the v19 ReLU+BN config) with those two dims changed.
- **Confound for ac-03:** gte rebuild is 244-class, v19 is 240-class — the
  improve-or-hold vs v19 (gold 0.798 / repr 0.691) is not a pure embed-only delta.
  See memory `gte-embed-build-taxonomy-244`.
