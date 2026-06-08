# B3 late-fusion — ac-07 ship gate: NO-GO (blocking, H05). Two cycles, same wall.

**Date:** 2026-06-08
**Candidates:** `fusion-v26` (head α→0.136, no floor) and `fusion-v27` (head α floored to 1.0 + categorical cardinality gate).
**Verdict:** **NO-GO, both.** The general Sense-stage replacement collapses the label space at corpus scale. Nothing promoted; `models/default` stays v19, version stays 0.6.23.

## The headline

We tried to fix v26's collapse with the two levers you authorised — an α floor (stop the head suppressing multi-branch) and a cardinality gate (stop the head dumping free-text into categorical). Both **helped and neither was enough.** Tell FineType to classify the 33,250-table corpus sample and v27 still pours mass into the two generic buckets it should be draining:

- **`unknown`** swells 415k → 699k columns (**+68%**, *worse* than v26's +52%) — the single biggest sink, 41,579 of them refuted by the oracle.
- **`representation.discrete.categorical`** is still **6.05× over-emitted** (v26 was 10.7×, so the gate cut it ~40% — real, but it stays far over the 3× band).
- **47 types still collapse to near-zero**: top-level-domain ×0.001, geohash ×0.005, utc-offset ×0.009, blood-type ×0.013, NPI ×0.022, UPC ×0.070, UUID ×0.141. v19 labels these confidently on thousands of columns each; v27 barely emits them.
- **New damage:** `latitude` flips from gold-anchor win to corpus-scale **over-emit ×3.13** — the value-view's latitude enthusiasm, now at full additive weight, over-fires on 13.5k columns v19 typed otherwise.

63 labels flagged (47 collapse, 16 oracle_fp, 2 over_emit).

## What the levers proved — and what they didn't

The α floor did its job: it stopped the *wholesale* View2 suppression that zeroed 90 types in v26, and val recall confirmed it (collapsed val classes 32 → 19, family-A latitude 0.988 → 1.000). The cardinality gate did its job: categorical dropped 10.7× → 6.05×.

But the gate **didn't clear** because the failure was never only α-suppression. With α pinned to 1.0 the fused score is `head + multi-branch` — pure additive, the head can only *add* to v19's distribution. It still collapses 47 types. That means **the head's own categorical/unknown logits are large enough to override full-weight multi-branch.** The value-level view, faced with the vast majority of real columns that carry no strong per-value signal, confidently votes categorical/unknown — and that confidence swamps multi-branch's correct-but-moderate vote for the rare structured type.

This is structural to "value-level view as a *general* classifier," not a tuning miss. Most real-world columns don't have a decisive per-value fingerprint, so the value view defaults to the generic attractors, and summing it into multi-branch drags the whole corpus toward categorical + unknown. Two cycles, same wall.

## The instrument map held

The gold anchor said GO both times (the four confusion families genuinely improve; family A — tight-code vs alphanumeric, Sharpen-unreachable — is a real 17% → 93% win). It was right about what it can see and blind to corpus breadth, exactly as CLAUDE.md warns: *gold-anchor + ship-gate is not sufficient to promote; corpus breadth is.* The corpus-honest gate exists for this failure and it fired, twice.

## The fork — your call (pre-committed halt)

1. **Penalise the attractor and retrain once more.** Add a categorical/unknown emission penalty (class-weighted loss or a logit prior) to the head, re-gate. Cheapest in wall-clock (cached features → seconds to train + one 1.4h pass). But it's the *third* attempt at the general replacement against the same structural pull, and a penalty risks trading collapse for under-emission. Diminishing confidence.
2. **Narrow B3 to a family-A booster.** Override v19 *only* on the tight-code-vs-alphanumeric boundary where the gold anchor proves fusion wins decisively (17% → 93%); leave the other 239 types on v19 untouched. Sidesteps the collapse entirely, banks the one real Sharpen-unreachable win, ships. Abandons the "general replacement" ambition.
3. **Abandon.** Keep v19; bank the value-level findings (lat/lon recovery, decimal survival, family-A win) as evidence for a future design.

**My read: option 2.** Two general-replacement cycles have now failed the corpus gate by the same mechanism — the value view defaults to generic attractors at corpus breadth. That's strong evidence the value view is an *efficacy booster on a specific boundary*, not a breadth-holder. The honest move is to capture the proven family-A win in a bounded override and stop trying to make the value view classify all 240 types. Option 1 is cheap enough that if you'd rather exhaust the general-replacement thesis first, it's one more head-train + one gate pass to know — but I wouldn't bet the release on it.

## State

- Both levers committed (`7dcbcc9`), parity-reusable. α-floor sweep {0.5,0.7,1.0} → 1.0 chosen.
- Nothing promoted. `models/default` = v19, Cargo 0.6.23, no HF publish, CI pin unchanged.
- Evidence: `output/corpus-honest-gate/gate_fusion-v27.json`, `fusion_v27_pass/corpus_pass/columns.parquet`, this memo.
