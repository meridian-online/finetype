# Prototype: an oracle-free pre-promotion scoreboard for contested rare types

**Date:** 2026-06-08
**Script:** `scripts/build_rare_type_gold.py`
**Purpose:** prove that trusted, oracle-free metrics on the contested rare types **move across rounds** where aggregate corpus precision (~0.49 vs gated-YDF) is flat — closing the blind spot in `output/eval-ceiling-diagnosis/finding.md`. Covers the three header-driven boundaries the recent rounds fought: **latitude**, **url**, **utc**.

## How the gold set is built

These types are *header-identifiable but value-ambiguous* — a latitude column and an RMS-error column have the same values; only the header separates them. So the **header is the trusted label**, validated against the values. No gated-YDF, no oracle.

- **latitude** — pos: unambiguous lat header (`lat`/`latitude`/`y_lat`/…, anchored so `translate`/`platitude` are excluded) + decimals in [-90,90]; neg: numeric column with a non-coordinate header (`population`/`score`/`rms`/`error`/…). FP = neg called latitude.
- **url** — pos: url header (`url`/`link`/`href`/`website`/…) + `http`/`www` values; neg: text column with a clearly-non-url header (`name`/`title`/`description`/…). The signal here is **recall** (FP is already ~0).
- **utc** — utc-offset columns are ~absent in the corpus (49k `utc`/`timezone` headers, **0** with offset-shaped values), so there are no meaningful positives. The real battle is **utc-on-integers**, so utc is an **FP-rate-only** guard: neg = integer column with a quantity header (`count`/`id`/`number`/…). FP = neg called utc.

## The scoreboard — moves where corpus precision doesn't

`sense_prediction` scored on the gold set, per round (v19/v22/v23 = full corpus; latdec/v0624/fusion_v27 = 33k stratified sample). **Corpus precision is ~0.49 ±0.001 for every row.**

| round | lat FP-rate ↓ | url recall ↑ | utc FP-rate ↓ |
|---|---:|---:|---:|
| v19 (shipped) | 0.0013 | 0.949 | 0.0001 |
| v22 | 0.0070 | 0.953 | 0.0003 |
| **v23** | **0.0163** | **0.896** | 0.0000 |
| latdec | 0.0017 | 0.968 | 0.0000 |
| v0624 | 0.0012 | 0.925 | 0.0000 |
| **fusion_v27** | 0.0026 | **0.989** | 0.0000 |

**The story the headline metric never told:**
- **Latitude** — v22/v23 wrecked it (FP-rate 0.0013 → **0.0163, 12×**; v23 calls 16 of every 1,000 non-coordinate numeric columns "latitude"). Recall never moved (~0.99), so the damage is pure over-emission — invisible to any recall bar. The fusion patches pulled it back to the v19 floor.
- **Url** — the battle is recall, and it moved too: v23 *lost* 5pp of url recall (0.949 → 0.896), fusion_v27 recovered to **0.989**. FP-rate is negligible throughout — url over-emission was never the real problem; url *under*-recognition was.
- **Utc** — flat and near-zero across the models on hand; v22 is marginally worst (0.0003). The metric is a standing guard: a v24-style utc explosion (the v24 memo reported 5.1× growth) would light up here. *(v24's own predictions aren't in this set; add them to confirm.)*

Every one of these swings was **invisible** to corpus precision.

## What this buys the next round

A round touching any of these boundaries can be scored on **the thing it changes**, in seconds, *before* a 9-hour corpus pass whose headline number cannot move — and v22/v23 would have been caught here pre-promotion. The compact scoreboard is the gate.

## Extending it

`TYPES` is a config dict — `(target_label, pos-SQL | None, neg-SQL)` over value features (`n_num`, `n_int`, `n_latlike`, `n_url`, `n_str`) computed once per scan. Add a boundary by adding a row. `MODELS` is `(label, parquet)`; add any pass with `column_name, sense_prediction, sample_values_truncated` (e.g. drop in v24's parquet to confirm the utc/latitude explosions).

**Caveat:** header-anchored labels are high-precision, not hand-verified — this is a prototype scoreboard, not a canonical gold standard. The step to canonical is a small human spot-check of each type's positives and FP columns to confirm the header labels hold; the FP-rate is the robust cross-round comparator (precision/recall absolutes depend on the positive:negative ratio, which differs between the full-corpus and 33k-sample passes).
