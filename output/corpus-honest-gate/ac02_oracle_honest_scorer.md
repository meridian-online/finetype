# ac-02 — oracle-honest FP scorer

Spec `2026-06-07-corpus-honest-quality-gate`, ac-02. Closes flaw 2 (the
oracle-gameable metric). Tool: `scripts/corpus_honest_gate.py`. Reads the ac-01
stratified sample, scales each per-column transition by the v19 source-label sample
rate, and adjudicates every candidate prediction against the **stable baseline
gated-YDF oracle** — including the columns where the candidate's own YDF abstains.

## The bug the latdec bet hid behind — and why the fix is the *baseline* oracle

The latdec bet scored itself on "sense=latitude AND ydf=decimal" and read **zero**.
Two compounding reasons it was a mirage, both closed here:

1. **The oracle has to be the baseline's, not the candidate's.** YDF is a property of
   the column's *data*, not the Sense model — so it is identical across models and is
   read once from the v19 pass. The latdec candidate parquet never ran `--fill-ydf`:
   its `ydf_prediction` is **100% NULL** (6.56M rows). Any metric keyed on the
   *candidate's* YDF therefore reads a structural zero. That artdefact alone drove the
   bet's FP count to zero.

2. **The oracle has to be GATED.** Raw YDF is demonstrably noisy (msg_id→iso6346,
   team-codes→country_code), so raw contradiction floods the soft-text labels with
   tens of thousands of bogus refutes. The canonical `ydf_prediction_gated` lens NULLs
   any YDF label fewer than 50% of the column's values pass — the same lens the corpus
   scoring already trusts. Against the gated v19 oracle, latdec's new latitude calls
   are **not** "hidden on ydf=NULL": on the sample, 421 of 457 `decimal→latitude` moves
   sit on columns the gated oracle positively labels **decimal**; only 36 are silent.

So the honest signal is **oracle contradiction against the stable gated baseline**,
not abstain-bucket inflow. It is also algebraically the right quantity: per label B,
`net_contra_in = contra_in − contra_out` equals `candidate_refuted(B) − v19_refuted(B)`
— the **net new** oracle-refuted predictions of B the candidate introduces over v19.
v19-vs-itself has zero moves, so it scores zero net contra by construction → no false
alarm (ac-03a).

## Demonstration on latdec — it counts what the bet drove to zero

| | bet's original metric | this scorer (sample, scaled) |
|---|---:|---:|
| latitude false positives | **0** (candidate ydf = NULL) | **2,481 net new** oracle-refuted |
| observed sample evidence | ~18 cols (proxy) | **446** contradicted `→latitude` moves |
| band fired | — | `oracle_fp` |

The 2,481 is the scaled corpus estimate of the relocation the gold anchor, m-19 and
the drift proxy all cleared. The scorer counts it because it adjudicates against the
column's own data (the gated v19 oracle says decimal), not against the candidate's
empty YDF.

## Scorer mechanics

- **Source-rate scaling.** The sample is stratified on v19's calls, so a raw marginal
  reads the wrong sign (latitude DROPS on the latitude-rich sample while it RISES on
  the corpus). Each transition `A→B` is scaled by `1 / sample_rate[A]`, un-biasing it:
  421 `decimal→latitude` / 0.115 ≈ 3,670 estimated corpus moves.
- **Three bands.** `over_emit` (est marginal / v19 marginal ≥ `--rel-mult`, default 3),
  `collapse` (≤ `--collapse-frac`, default 0.6), `oracle_fp` (net contradicted inflow
  ≥ floor AND ≥ `--oracle-fp-ratio` of v19 marginal AND ≥ `--oracle-fp-obs-floor` raw
  observed columns). The observed-column floor suppresses rare-source scaling
  amplification (a handful of columns × a large `1/rate` would otherwise trip tiny
  labels like `short_mdy`).
- Output: `output/corpus-honest-gate/gate_<label>.json` — report + full 243-label
  table; exit 0 GO / 1 NO-GO.

## Next

ac-03 runs this scorer against all four labelled parquets and confirms it reproduces
every known verdict from the sample alone.
