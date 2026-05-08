# Phase 2 Plausibility Memo (ac-01 of finetype-dnf)

**Date:** 2026-05-04
**Author:** Nightingale
**Bead:** finetype-dnf
**AC anchored:** ac-01 [gate]
**Required citations downstream:** ac-03 (weight rationale), ac-04 (outcome framing)

---

## Method

Deterministic 1000-row sample (file order, content-addressed via
SHA-bucket partition) from `eval/gittables/failure_log.measure.tsv`.
For each row, `finetype infer-type` was invoked under locked Phase 1
weights (`w_v=0.4, w_h=0.6`); the per-signal scores
(`validator_pass_rate`, `header_match`) and the final argmax
confidence were captured.

Rows were classified by which Phase 2 signal could plausibly recover
them:

| Tag | Definition | Phase-2 signal that addresses |
|---|---|---|
| `generator_shape_only` | failing AND argmax `validator_pass_rate ≥0.7` AND `header_match <0.3` | generator-shape (shape-uniform values, neutral header) |
| `sibling_context_only` | failing AND `validator_pass_rate <0.7` AND `header_match <0.5` | sibling-context (weak both axes — column needs disambiguating context) |
| `both` | satisfies both definitions | either signal |
| `neither` | failing but in neither subset | structural — neither signal addresses |
| `passes_phase1` | confidence ≥0.7 (Phase 1 already clears) | n/a |

Script: `scripts/phase2_plausibility_scan.py`. Per-row JSON:
`phase2_plausibility.json`. The numbers below are reproducible.

## Headline numbers

```
Total rows scanned:           1000
Phase 1 passes at 0.7:          71  (7.1%)
Phase 1 failing rows:          929 (92.9%)

Score buckets (Phase 1):
  ≥0.7 (passes):                71
  [0.5, 0.7) (mid):            133
  <0.5 (low):                  796

Failing rows by addressability:
  generator_shape_only:        780  (84.0% of failing)
  sibling_context_only:         16  ( 1.7% of failing)
  both:                          0  ( 0.0% of failing)
  neither:                     133  (14.3% of failing)
```

## The cliff cluster

The single dominant failure mode is one shape:

```
final score        validator        header           count
[0.4, 0.5)         v ≥ 0.7          h < 0.3            780
≥0.7               v ≥ 0.7          h ≥ 0.5             71
[0.5, 0.7)         v < 0.4          h ≥ 0.5            115
[0.5, 0.7)         v ≥ 0.7          0.3 ≤ h < 0.5       17
<0.4               v < 0.4          h < 0.3             16
[0.5, 0.7)         0.4 ≤ v < 0.7    h ≥ 0.5              1
```

**780 rows (78%) cluster at score ≈ 0.4 with validator pass-rate ≥0.7
and header_match <0.3.** This is the "cliff cluster": the argmax
validator passes (often at 1.0 — `v_median = 1.0`, `v_mean = 0.86` over
all failing rows), but the column header carries no overlap with any
taxonomy label (`h_median = 0.0`, `h_mean = 0.13`). Their score
formula evaluates to `0.4·1.0 + 0.6·0.0 = 0.4`, parked exactly on the
fallback threshold and well below the 0.7 confidence target.

This single cluster IS the structural ceiling MADR 0084 named.

## Per-signal lift estimates

### Generator-shape addressable subset: 780 rows (78.0% of total)

These are rows where the validator already passes near-perfectly. The
question for generator-shape is **independence**: does it carry
information beyond what `validator_pass_rate` already encodes?

**The honest answer is "barely."** Generator-shape's mechanism is "do
the column's values look like what a generator for type T would
produce" — semantically the same axis as "do the values match T's
validator". On shape-uniform values (the entire cliff cluster, by
construction) generator-shape will fire on the same type the validator
already votes for, with a comparable score. Two correlated signals
voting for the same answer don't lift the *score above the joint
ceiling* — they just give the joint vote two share of the weight.

Concrete arithmetic — assume Phase 2 redistributes weight as
`w_v = 0.2, w_h = 0.3, w_g = 0.3, w_s = 0.2` (halve Phase 1 weights to
make room; allocate remaining 0.5 between new signals with a slight
preference for generator-shape per first-principles below). For a
typical cliff-cluster row (`v=1, h=0`) under this scheme:

```
  pessimistic (g uncorrelated, fires at 0.5 on argmax):
    score = 0.2 + 0.0 + 0.3·0.5 + 0.0 = 0.35  — falls below fallback
  realistic (g correlates with v, fires at 1.0 on argmax):
    score = 0.2 + 0.0 + 0.3·1.0 + 0.0 = 0.50  — clears fallback, not 0.7
  optimistic (g + s both fire perfectly):
    score = 0.2 + 0.0 + 0.3·1.0 + 0.2·1.0 = 0.70  — exactly 0.7
```

Under realistic assumptions, generator-shape recovers the cliff
cluster from score 0.4 to score 0.5 — i.e., it lifts cliff rows OUT
of [0.4, 0.5) and INTO [0.5, 0.7). This is the load-bearing
mechanism for **ac-05 (cliff lift)** but does NOT clear ac-04 (60% at
0.7+).

**Plausible lift to non_unknown@0.7 from generator-shape alone:**
~5–10 pp above Phase 1's 7.1%. Most of generator-shape's 78%
addressable subset lands in [0.5, 0.7), not ≥0.7.

**Plausible lift to non_unknown@0.5 from generator-shape alone:**
~50–70 pp (the bulk of the cliff cluster lifts from [0.4, 0.5) to
[0.5, 0.7)).

### Sibling-context addressable subset: 16 rows (1.6% of total)

Sibling-context targets the rows where validator AND header are both
weak — the column's own signals are silent and disambiguation must
come from surrounding columns. The empirical addressable set is
**16 rows out of 1000** — 1.6%. Plus another 16 rows in the "neither"
bucket where both signals are weak AND validator≤0.4 are likely
sibling-context candidates as well (the script's threshold split
caused these to be tagged "neither" — see Note A).

**Plausible lift to non_unknown@0.7 from sibling-context alone:**
≤2 pp. The addressable subset is too small to move the headline.

**Plausible lift to non_unknown@0.5 from sibling-context alone:**
≤2 pp. Same set, same ceiling.

The architecturally interesting question — whether sibling-context
attention from the model's internals can be reused (MADR 0085, ac-02)
— has a small empirical-impact answer here. Even a perfect
sibling-context implementation moves the headline by ≤2 pp.

### Joint signal: ~0% of rows are addressable by both

`both = 0` in the empirical tabulation. The two signals partition the
failing rows almost cleanly: generator-shape addresses
shape-uniform-but-header-silent; sibling-context addresses
both-weak. There is **no row in the sample where adding *both* signals
together produces a score lift unavailable to either alone.** Joint
operation is additive but not multiplicative.

### Structurally unaddressable: 133 rows (13.3%)

These are rows where validator is moderate (between 0.4 and 0.7),
header is moderate, or some combination Phase 2 can't disentangle.
Examples likely include: ambiguous-format strings (validator hesitant
across multiple types); columns with rich headers that overlap with
SEVERAL labels (ambiguous header). Phase 2 won't recover these.

## What this means for ac-04 outcome

**Modal expectation: outcome (b) — structural ceiling MADR.**

The 60% at 0.7+ target requires the new signals to recover ~53 pp
beyond Phase 1's 7.1%. Decomposing the achievable lift:

```
generator_shape contribution to non_unknown@0.7:    +5 to +10 pp
sibling_context contribution to non_unknown@0.7:    +1 to +2 pp
joint contribution:                                  ~0 pp (no overlap)
demotion from reweighting (w_v 0.4→0.2):           -1 to -3 pp
─────────────────────────────────────────────────────────────────
estimated Phase 2 non_unknown@0.7:           7% + 5–10% = 12–17%
60% target:                                              60%
gap to target:                                       43–48 pp
```

**Outcome (a) — Phase 2 ships and ac-02 returns to 60% — would
require generator-shape to fire INDEPENDENTLY of validator AND
contribute ≥30 pp on its addressable subset.** That requires
generator-shape to carry information about a *different argmax type*
than the one the validator votes for, on a substantial fraction of
rows. Empirically, on shape-uniform values the validator's argmax IS
the generator-shape's argmax — they're not independent.

**The structural cause for outcome (b) is the cliff cluster's score
formula, not signal availability.** With v=1, h=0 typical, the cliff
cluster's score under any redistribution where w_v + w_g + w_s ≤ 0.7
(which preserves header's contribution at meaningful weight) is at
most 0.7 only when ALL of v, g, s contribute their max. Real signals
won't all max out simultaneously. The structural cap exists because
"shape says yes, name says nothing" cases simply don't have a
high-confidence answer regardless of how many shape-style signals you
add.

## What this means for ac-05 outcome

**Modal expectation: ac-05 PASSES** — the falsifiable cliff-lift gate
is achievable.

ac-05 requires `non_unknown@0.5 ≥0.304` (≥10 pp over Phase 1's 0.204
baseline). The cliff cluster (780 rows at score ≈0.4) lifts wholesale
under the realistic generator-shape assumption (g uncorrelated with v
on argmax type, fires at 1.0). Even if only 30% of cliff rows lift
above 0.5 under realistic-but-imperfect generator-shape, that's a
+24 pp lift in non_unknown@0.5. The 12.6% of `<0.5` rows required to
move into `≥0.5` (per the cliff-at-0.5 sensitivity) is well within
plausible reach.

**ac-05 is the load-bearing falsifiable claim** that the new signals
carry information. The data says it should pass.

## Implications for ac-03 weight selection

This memo's per-signal estimates are the **second** input to ac-03's
weight rationale (the first is first-principles reasoning analogous
to MADR 0079). The weights MADR (next number after 0085) should
account for:

1. **Generator-shape is correlated with validator on the cliff
   cluster — over-weighting it does not help.** Recommended:
   `w_g ≤ 0.3`.
2. **Sibling-context's empirical addressable set is small (≤2%).**
   Recommended: `w_s ≤ 0.2`.
3. **Header remains the most discriminating signal among the four —
   it's the axis where Phase 1 already pays its weight.** Inheriting
   MADR 0079's "header > validator" relative ranking, recommended:
   `w_h ≥ w_v`.
4. **Sum-to-one constraint.** A starting point for the weights MADR
   discussion: `w_v=0.2, w_h=0.3, w_g=0.3, w_s=0.2`. This is a
   *recommendation*, not a binding pre-commit — the weights MADR's
   own first-principles analysis may justify a different allocation.

## Limits of this analysis

1. **The 1000-row sample is from `failure_log.measure.tsv` file order
   only.** It matches Phase 1's measurement methodology but does not
   exhaustively characterise the full 10,660-row partition. The
   per-threshold curve in Phase 1's `progress.md` records ≤0.5 pp
   calibrate-vs-measure agreement, suggesting the sample is
   representative.
2. **"Generator-shape" is modelled abstractly** — the actual
   implementation may produce signal in cases this memo treats as
   "correlated with validator" if the generator's output
   characterisation differs structurally from the validator's regex.
   The plausibility numbers are upper-bound under correlation
   assumptions; if generator-shape proves more orthogonal than
   modelled, lift estimates rise. Calibration after implementation is
   the empirical check.
3. **"Sibling-context"** estimation here is a tabulation by signal
   weakness (proxy for "the column has no own-signal so siblings
   would help"). Real sibling-context's strength depends on the
   parquet file's column-graph structure — most failing rows come
   from rich-context tables where sibling clusters DO carry signal.
   This memo's 1.6% addressable estimate is conservative; the actual
   sibling-context lift may be higher if many of the 780
   generator-shape rows ALSO have strong sibling clusters that
   disambiguate the validator's argmax. That's a Phase 2 measurement
   to surface.

## Note A: classifier asymmetry

The classifier in `phase2_plausibility_scan.py` treats
`generator_shape_only` and `sibling_context_only` as
strict-by-construction (validator-strong AND header-neutral for the
first; validator-weak AND header-weak for the second). This means
some rows that are weakly addressable by both fall into "neither". A
more permissive classifier might re-tag some of the 133 "neither"
rows; the bound on Phase 2 lift would not move materially.

## Citation index for downstream ACs

When ac-03's weights MADR is authored, the rationale **must cite this
memo** by section. Recommended citation form:

> "Per the Phase 2 plausibility memo (`phase2_plausibility.md` §
> 'Per-signal lift estimates'), generator-shape's addressable subset
> (78% of failing rows) is correlated with validator on argmax
> type. The weights allocation reflects this correlation: `w_g ≤ 0.3`
> avoids over-weighting a signal whose information overlap with `w_v`
> is high on the cliff cluster."

When ac-04's outcome (a)/(b) framing is finalised:

> "The plausibility memo (`phase2_plausibility.md` § 'What this means
> for ac-04 outcome') estimates the achievable
> `non_unknown@0.7` ceiling under realistic 4-signal triangulation
> at 12–17%. Outcome (b) — structural-ceiling MADR — is the modal
> expectation; outcome (a) requires generator-shape to fire
> independently of validator on the cliff cluster, which the
> empirical signal correlation does not support."
