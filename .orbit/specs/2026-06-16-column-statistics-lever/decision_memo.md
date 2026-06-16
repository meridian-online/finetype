# Column-statistics lever — decision memo (ac-08, spec close)

**Date:** 2026-06-16

## What the lever was
The session's architecture search ruled out every model-side bet (additive retrains
0-for-5, fusion, sibling-context, hierarchical head). The surviving lever — the analyst's
reframe — is **full-column statistics fed as cheap value-based rules**, recovering the
residual recall gaps the per-VALUE model is structurally blind to. The probe
(`output/column-stats-probe/`) proved cardinality separates the residual gaps decisively
(categorical vs alphanumeric_id AUC 0.985; integer vs increment 0.972).

## What shipped (the proven win)
**`increment_substance_veto` (v0.6.31, ac-04).** `value_sharpen` judged sequentiality on
the 100-value STEPPED sample (can't see contiguity), over-emitting `increment` (gold
precision 0.056). The guard re-checks the FULL column (contiguous + near-unique). At ship:
gold 728→738, increment FP 17→7 (P 0.056→0.125), integer recall 0.796→0.847, corpus-honest
GO, zero regressions. This is the lever's first and validating instance.

## What is falsified (won't do)
**ac-03 broad cardinality → categorical.** Already killed at the corpus-honest gate (the
R32 round-1 finding: 3,752 entity_name + 2,115 plain_text oracle-refuted moves). The safe
slice ships as `R32 text_vocab_override` (word-only). Generalising cardinality→categorical
to entity_name/city/plain_text is the falsified broad variant — do not retry.

## Deferred — genuine, but separate bets (filed as follow-ups, not blocking)
- **DuckDB free-stats path (ac-07)** — the analyst's "run less inference" idea: read
  full-column stats from the DuckDB extension near-free + a short-circuit (ac-05) that skips
  the neural pass on a decisive stat. Real value, but a DuckDB-extension engineering bet;
  scope on demand when extension perf matters.
- **Binary-domain rule (ac-04 binary half)** — 6 gold FN, inverse of binary_vocab_veto,
  low value / collateral risk. Park.
- **General stat interface (ac-02)** — only needed if more stat rules follow; the increment
  rule used full-column `values` directly, no interface required.

## Verdict
Lever validated, first rule shipped and gated, falsified branch documented. Remaining ACs
are either falsified (ac-03) or separate engineering (ac-02/05/07) — close the spec. The
DuckDB free-stats path is the one worth a dedicated spec later (it pairs with the gold
work: a faster, full-column-stat-aware inference path).
