# 0.6.24 precision patch — gate says NO-GO, oracle says +858 correct. The gate can't see honest abstention.

**Date:** 2026-06-08
**Candidate:** current `main` (v19 model + validation-veto-default-on + utc/url/measurement_unit/geohash schema-fail demotions). Binary `target/release/finetype` @ 11:31 — confirmed current-main, not the stale 0.6.20 on PATH, because the candidate parquet shows the measurement_unit demotion (5,995→26) which only exists post-9d83e13.
**Pass:** 33,250-file stratified sample, 852,463 columns, 26 min.
**Gate verdict:** **NO-GO** — `unknown` over_emit + oracle_fp (ratio 1.74, +266k projected), `categorical` over_emit (ratio 1.55, +23k), collapse bands on doi/mime_type/measurement_unit/decimal_comma.

## The headline the gate's bands hide

Measured **directly against the same gated-YDF oracle** on the 707,562 matched columns where the oracle has an opinion:

- v19 floor (pre-fixes): **0.4915**
- current main (veto + demotions): **0.4927**
- **Δ +0.0012 → +858 columns *more* correct, not fewer.**

The candidate is a genuine precision **improvement**. The NO-GO is a gate blind spot.

## Why the gate fires anyway

The fixes convert v19's confident over-emissions into honest `unknown` (and a little `categorical`). The gate counts those new `unknown`/`categorical` predictions as `oracle_fp` — because `unknown` can never *match* the oracle's specific label, every honest abstention reads as a fresh disagreement.

Decomposing the 49,018 veto demotions to `unknown` (matched sample) by what the oracle says about the **destroyed** label:

| outcome | cols | share | meaning |
|---|---|---|---|
| **harmful** — oracle *confirmed* the demoted label | **117** | **0.2%** | veto wrongly nulled a correct label |
| neutral — oracle refuted the base label too | 42,288 | 86.3% | confidently-wrong → honestly-unknown (a *gain* for analyst trust) |
| oracle abstains | 6,613 | 13.5% | no ground truth either way |

Same shape for the `categorical` inflation: of 2,985 new categoricals, **104 (3.5%)** harmful; the rest come from the intended utc/measurement_unit/geohash/url demotions landing on `categorical` instead of `unknown`.

So the **false-veto rate against the oracle is 0.2%** — comfortably inside the allowlist's <10% design target. The veto does exactly its job: it catches v19 over-emitting `integer`/`currency`/`full_name`/`url` on columns the oracle *also* says are something else (decimal, plain_text, entity_name), and replaces the confident-wrong label with `unknown`.

## The instrument finding

The corpus-honest gate is a **proven NO-GO detector for collapse-into-confident-wrong** (v22/v23/latdec). It has an unguarded blind spot in the other direction: it treats `unknown` as an ordinary label, so **demote-to-abstain reads identically to a regression**. It cannot distinguish:

- *right → unknown* (truly bad — destroys a correct label): **0.2%** here, and
- *wrong → honest unknown* (good — fewer confident mislabels, the Precision Principle): **86%** here.

CLAUDE.md already flags the symmetric caveat — "a NO-GO is blocking (H05); a GO is advisory ... GO-precision unvalidated." This is the **inverse**: a NO-GO false alarm on a precision-improving patch whose only "sin" is abstaining more honestly.

## Recommendation

The patch is good; the gate's `oracle_fp`/`over_emit` bands need to **exempt `unknown` (and abstention-like sinks) when the demoted base label was itself oracle-refuted** — i.e. only score a demote-to-unknown as net-harmful when it destroys an oracle-*confirmed* label. With that refinement the candidate clears (0.2%/3.5% harmful, +858 net-correct).

But this **modifies a BLOCKING release instrument (H05)** to pass our own candidate — exactly the move the gate exists to stop us rationalizing. So it needs an explicit decision, not a unilateral edit. Do not promote past the NO-GO until the gate refinement is reviewed and agreed.

## State
- Nothing promoted. `models/default` = v19 (0.6.24 is a Sharpen patch on the same model — no swap).
- Evidence parquet: `output/corpus-honest-gate/v0624_pass/corpus_pass/columns.parquet`; gate JSON: `output/corpus-honest-gate/gate_v0624.json/gate_v19-sharpen-0.6.24.json`.

## Resolution (2026-06-08) — Option A taken: gate refined, patch clears

The gate's `oracle_fp` and `collapse` bands were made **oracle-aware** (read the oracle
against both ends of each transition, so a demote-to-`unknown` of an already-refuted
label is no longer scored as a regression). The change was validated by re-running the
four-verdict regression: **v19 GO, v22/v23/latdec NO-GO all preserved** on their
load-bearing movers, and the 0.6.24 patch now reads **GO** honestly. Full write-up:
`output/corpus-honest-gate/refined/oracle_aware_bands.md`. 0.6.24 ships.
