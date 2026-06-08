# The precision "ceiling" is a measurement artifact, not a model limit

**Date:** 2026-06-08
**Scope:** why corpus precision (~0.49 cell-2 vs gated-YDF) has not moved across v22/v23/v24/latdec despite real precision work.
**Verdict:** the headline metric is **structurally blind to the rare-type fixes the rounds ship, and counts only their collateral damage.** The model is not demonstrably stuck; the instrument is.

## Headline

Corpus "precision" is *agreement with the gated-YDF oracle* over the columns the oracle opines on. Two structural facts make it incapable of refereeing the precision battles the rounds actually fight:

1. **The denominator is 52% plain numbers and ~0% rare targets.** On `v19_gated.parquet` (5,813,522 scored columns): integer+decimal = **3,052,284 (52.5%)**; the rounds' battlegrounds — `geography.coordinate.latitude` + `longitude` + `technology.internet.url` + `datetime.offset.utc` — total **18 columns (0.0003%)**. Raw YDF *proposes* latitude 8 times and utc **zero** times across 6.6M columns, so those columns cannot enter the denominator at all.
2. **The metric sees a bet's downside but not its upside.** The common types a rare-type bet collaterally damages — city (60,529 cols), country (8,457) — *are* well represented. So a round gets debited for side-effects (v22's −10.4% was real country/city collateral) and credited **nothing** for the rare-type fix it was built to make. It is a one-way ratchet: every round can only stay flat or regress. That is the "ceiling."

## Supporting finding — gated-YDF is a noisy judge, not ground truth

"Precision" is agreement with a separate ML model (YDF). Its own gate data (`output/ydf-validation-gate/coverage_report.md`) shows raw YDF is confidently **wrong 70–100%** of the time on many families: year 69% refused, decimal-comma 94%, isbn 90%, credit-card 96%, phone-e164 / iso6346 100%. The gate masks this by NULLing those predictions, which concentrates the surviving (scored) population onto the easy high-volume types (integer 1.3% refused, decimal 0.4%).

Against **independent** DBpedia annotations (`dbpedia_annotations.parquet`, knowledge-base-derived, joined on `(file_content_sha256, column_name)`), the gated-YDF oracle is ~**8× further from truth** than the Sense model it is used to judge (Sense 25.8% exact / 41.2% category vs oracle 3.1% / 8.0% over 1.5M doubly-labelled columns). **Caveat:** the DBpedia mapping is header-derived and coarse (only 233 of 1,710 classes have a finetype equivalent), so trust the *relative* gap, not the absolute 3%. The 0.6.24 finding independently confirmed the direction: **86%** of Sense's "disagreements" with the oracle were Sense abstaining *more honestly*, not being wrong.

## Ruled out — no leakage, no circularity

YDF trains **only** on synthetic generator output + distilled Sherlock rows; it never reads a corpus column (`scripts/train_ydf.py`, no `read_parquet`/`corpus_paths`). The corpus pass scores the held-out measure partition only (`_sha_bucket = int(sha,16) % 2 == 1`); empirically **100%** of scored rows are in bucket 1, zero in the calibrate/eval bucket. Two belt-and-braces firewalls (value-hash + gold-anchor `(sha256, column_name)` identity). The ceiling is **not** memorisation. Firewall discipline is sound.

## Corrections

1. **Demote aggregate corpus-precision (cell-2 vs gated-YDF) as the headline progress metric.** It is 52% plain numbers and cannot register rare-type fixes. The **rare-label-quota'd corpus-honest gate** already exists and is the right headline (latitude lands 2,213 cols there vs 3 here).
2. **Stop treating gated-YDF as correctness** for the contested rare types. Stand up a **trusted ground truth** on those columns (header-anchored + value-validated, or hand-labelled) — the oracle abstains or errs exactly where the battle is. *(Prototype: `output/eval-ceiling-diagnosis/rare_type_gold.md`.)*
3. **Several targets are value-unidentifiable.** A latitude column and an RMS-error column have identical values; only the header separates them (a corpus column literally named `POPULATION`, normalised small decimals, is predicted `latitude`). Value-based hard-negative retrains cannot win these — route to Sharpen/header (the v24 memo's own conclusion; url/utc already flagged header-driven).
4. **Re-baseline the multi-lens diagnostic on v19.** It was run against v22 (−10.4% worse than shipping v19), so the gaps steering round selection may be v22 artifacts (CLAUDE.md already warns this).

## What we don't know yet

- The model's **true** precision on the rare types — no trusted ground truth existed for them before this finding; the prototype is the first cut.
- Whether v22–v24 were actually net-positive on their targets all along — the metric could not tell us, and the rare-type gold set is needed to find out retroactively.

## Substrate

- This finding; memory `eval-precision-metric-blind-to-rare-targets`.
- Prototype gold set + multi-model scoring: `scripts/build_rare_type_gold.py`, report `output/eval-ceiling-diagnosis/rare_type_gold.md`.
- Evidence: `output/ydf-validation-gate/{v19..v23}_gated.parquet`, `coverage_report.md`, `dbpedia_annotations.parquet`.
