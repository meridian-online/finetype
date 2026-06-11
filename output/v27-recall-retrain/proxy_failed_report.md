# v27 recall retrain — Failed-informative (proxy gate, two rounds)

**Spec:** `2026-06-11-categorical-identifier-recall-retrain` ·
**Verdict:** HALT after two pre-committed proxy NO-GOs. No overnight run was
spent. Total instrument cost: 2 × ~30 min proxy + 3 FTMB builds.
**Drift tables:** `output/destination-drift-precheck/proxy_drift_v27-proxy.json`
(round 1), `proxy_drift_v27-proxy2.json` (round 2).

## What was bet

The two largest gold recall pools — categorical (R 0.386) and
alphanumeric_id (R 0.111) — were bet as starved-positive problems: add
curated, value-shape-filtered positives mined from v19's own corpus mistakes,
lift the per-label training mass, judge through the 0095 protocol.

## What happened

| | round 1 | round 2 |
|---|---|---|
| categorical training records (FTMB) | 16,063 | **1,921 (8× less)** |
| categorical emission on the fixed corpus list | 1.71% → 9.86% (5.76×) | 1.71% → **10.64% (6.22×)** |
| alphanumeric_id emission | 0.77% → 2.31% (2.99×, inside band) | 0.77% → 2.45% (3.18×, **tripped**) |
| proxy val_acc | 0.857 | 0.872 |

Round 2 cut categorical mass 8×, dropped the fuzziest bucket (B2_word), and
the explosion got *worse*. Both rounds show the same absorption signature:
comma_separated collapses to ~0.0×, word to 0.07×, entity_name halves,
gender_code/uuid/postal/iata all shrink toward zero. city held both rounds
(0.61–0.76×) — the v23 blast path stayed shut; the exclusion ledger and
value-based curation did their jobs. Training metrics were blind to all of
this (round 2 had the *better* val_acc) — the gate exists precisely because
of that blindness.

## What this teaches (the load-bearing findings)

1. **The explosion is presence-driven, not mass-driven.** Once the
   multi-branch flat head sees ANY real categorical vocabulary columns, the
   decision region for "small repeating vocabulary of short strings" sweeps
   a huge fraction of real corpus columns. Halving, even eighth-ing, the mass
   does not change the attractor's reach — it only sharpens it (6.22× > 5.76×).
2. **The `COLUMN_LEVEL_TYPES` guard is load-bearing, not a legacy wart.**
   v19 trains on zero real categorical columns *for a reason that the
   architecture imposes*: `categorical` is a RESIDUAL category — "nothing
   tighter fits" — and a flat 240-way softmax cannot express that precedence.
   Trained as a shape class, it out-competes every tight class whose values
   are also short repeating strings. v23 (+529%) and v27 (2 rounds, 5.76×
   and 6.22×) are now three consistent measurements of the same law.
3. **Categorical recall belongs in Sharpen (or a hierarchical/fusion head),
   not in Sense training data.** The recall mechanism the gold corpus
   measured is: Sense asserts a tight code → validation veto rightly rejects
   → column falls to unknown/word/entity instead of categorical. That is a
   PRECEDENCE decision ("tight assertion failed; is the column shaped like a
   residual vocabulary?") — exactly what the rule-based Sharpen stage is for,
   and exactly what choice 0094's header-corroboration pattern + a
   value-based rule (0048-compliant: n_distinct ≤ 12, repetition ≥ 40%) can
   express. The same template covers alphanumeric_id (veto-rejected tight
   code + high-cardinality letter+digit values → alnum). Decision 0038
   ("prefer retraining over rules") now has direct empirical counter-evidence
   *for residual categories specifically*.
4. **The per-type distilled caps were cosmetic for v4-format training data
   all along** (`ordered_distilled` is never rebuilt after
   `cap_distilled_columns`; `group_distilled_by_proximity` consumes the
   uncapped list). Every v4-era blend — including latdec's decimal=2600
   override — ran on uncapped per-type masses. Task
   `t-00009dd118b80a20a8eb3fe0` tracks the fix; it needs its own validation
   run before any future retrain relies on caps. v27 round 2 worked around it
   by enforcing masses in the blend CSV (`BASE_KEEP`/`MINED_KEEP` in
   `build_v27_recall_distilled.py`) — that mechanism is now proven and
   reusable.
5. **The alphanumeric_id leg is probably viable alone.** It tripped at 3.18×
   only marginally (band 3.0×), in a run where categorical was absorbing the
   text family and redirecting flow. alnum is NOT a column-level type, its
   mined data is clean (wfo-/msg-/GenBank-style ids), and its mechanism
   (veto fallout) is the best-understood in the gold corpus. An alnum-only
   retrain — zero categorical buckets, `COLUMN_LEVEL_TYPES` guard left
   intact — is a legitimate follow-up spec with a strong prior of clearing
   the band.

## Next bets, in order

1. **Sharpen veto-fallback rule family (choice 0094 pattern, value-based per
   0048):** when Sense's assertion is veto-rejected — low-cardinality
   repeating values → categorical; high-cardinality letter+digit values
   (id-corroborating header) → alphanumeric_id. Directly targets 32 of 48
   alnum gold FNs and the categorical unknown/scatter pool. Gold-gated like
   the postal veto (0.6.27 precedent).
2. **alnum-only retrain** (A-buckets only, guard intact) — finding 5.
3. **Categorical as a hierarchical/fusion decision** — the late-fusion spec
   (B3) or a two-stage head is the architectural home for residual
   precedence; fold this evidence into its design before its ac-07 gate run.

## Protocol scorecard

The 0095 promotion order performed exactly as designed: a bet that would
have burned an overnight run and a 9-hour corpus pass in the v22/v23 era was
killed for ~1 hour of proxy compute, with the failure mechanism measured
precisely enough to redirect the work. The drift gate is now 3-for-3 on
catching categorical-direction explosions (v23 retro, v27 ×2 forward).
