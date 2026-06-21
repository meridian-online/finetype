# ac-00 — gate-zero: does the Sharpen layer already heal the two-view range regressions?

**Verdict: NO. Composition does NOT recover them. ac-01 (numeric-range representation) is PRIMARY.**

## What two-view (seed 42) gets wrong
On the regressed types it predicts a WRONG LABEL, not a vetoable false positive:
- **latitude → `representation.numeric.decimal_number` ×28** (the bulk of the −0.30; it sees "a decimal", misses the latitude range)
- postal_code → numeric_code ×1, unlocode ×1, increment ×1 (the −0.40, only 3 cols)

## Why the Sharpen rules cannot fix it (code-confirmed)
The coordinate rules in `crates/finetype-model/src/column/value_sharpen.rs` both gate on
`result_label == COORDINATE_PAIR.0 || COORDINATE_PAIR.1` — they fire ONLY when the Sense
model has ALREADY said latitude or longitude:
- `coordinate_plausibility_gate` (l.66): coord with >10% values |v|>180 → **DEMOTE to decimal_number**.
- `coordinate_disambiguation` (l.97): coord → pick lat vs lon by range (|v|>90 → longitude).

Neither fires on a `decimal_number` prediction. **There is no decimal→latitude promotion rule** —
the engine uses value range only to REJECT false coords (subtractive), never to RECOGNISE true ones.
That asymmetry is deliberate (coord-promote-guard exists to PREVENT false coord promotion). The
postal rules are `NUMERIC_ATTRACTORS` guards in the same anti-false-positive shape. l.418 confirms the
veto/demotion family "can [n]ever correct" a wrong label.

So composing the two-view model would leave all 28 latitude-as-decimal errors uncorrected (at best a
plausibility gate keeps them as decimal). The regression is a missed PROMOTION, which the rule layer
does not do.

## Decision
**ac-01 is PRIMARY, not secondary.** The latitude-vs-decimal distinction must live in the LEARNED Sense
representation — and it is exactly a value-distribution problem: real latitudes cluster in [−90,90] with a
characteristic spread; arbitrary decimals do not. That is what numeric-range FEATURES (range buckets,
distribution shape, length/charset) capture and what a single out-of-range veto cannot. The spec proceeds
at full weight on ac-01; ac-02 (robust gating) and ac-03 (static path) unchanged.

Bonus design input for ac-01: the engine already computes value range for the plausibility VETO — ac-01
should lift that same signal into a positive, distribution-aware FEATURE the model can learn to promote on.
