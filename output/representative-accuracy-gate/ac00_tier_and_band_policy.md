# ac-00 — representative fixture: truth tier + advisory-band policy

**Spec:** 2026-06-18-representative-accuracy-gate (card 0020 scenario 1)
**Date:** 2026-06-18
**Status:** design gate — settles what the fixture's number does and does not authorise.

## Headline decision

**Freeze the fixture at PANEL tier and run it as a strictly ADVISORY band. The
relative comparison (candidate vs v19, same labels) is the load-bearing signal;
the absolute number is a soft band, not a point.**

A substrate gap forces the tier choice and it is worth stating plainly up front
(it was not known when the spec was drafted an hour earlier):

> The cited adjudicated 0.68 is **not reproducible per-column.** Only the raw
> blind-Sonnet panel labels were persisted (`panel_labels.json`). The two-Opus
> adjudication that lifted 0.648→0.68 survives only as an **aggregate** in
> `finding.md` (88 disagreements: 78 model-lost, 8 panel-wrong, 2 tie) — *which* 8
> columns flipped was never written down. 0.68 = 0.648 + 8/250. So the standing
> fixture can faithfully freeze the panel labels, but not the adjudicated ones.

## (a) Truth tier

**Tier = `repr-panel-sonnet` (blind single-model panel + confidence), no author tier.**

- It is the only per-column truth that exists on disk. 260 columns, blind Sonnet
  labelled from values+header (no model prediction shown), each carrying a
  confidence (`high` 215 / `med` 25 / `low` 20) and a `runner_up` where the panel
  hesitated (66 columns).
- This is *softer* than gold's author tier and slightly softer than gold's
  `llm-2panel` tier (single model, single round vs gold's multi-model + Opus
  adjudication). The provenance string records this honestly so no future promoter
  mistakes it for gold-grade truth.
- **Known bias: panel-tier is ~3pp anti-model.** The 8 columns where the
  adjudicators found the *panel* wrong and the model right are frozen here as
  model-wrong. A panel-tier headline therefore *under*-reports true accuracy by
  ~8/250 ≈ 3pp. This is acceptable for the advisory purpose (see below) and is the
  single reason an adjudicated upgrade is worth doing later.
- **Upgrade path (out of scope here, needs its own opt-in):** re-run a blind
  2-model panel + adjudicator over the ~88–98 model/panel disagreements (values
  retrievable from `columns.parquet`), persist the per-column verdict, and promote
  the fixture to an `llm-2panel`/author tier. That is an LLM-in-the-loop sub-spec,
  not a fixture-freeze step.

## (b) Advisory band

**The representative headline is REPORTED at every promotion and an advisory FLAG
(never a block) fires when a candidate's representative headline drops more than
the fixture CI below the v19 baseline.**

- **CI / "materially":** at n≈260 the Wilson 95% half-width is ≈ ±6pp. The advisory
  flag fires when `candidate_repr < v19_repr_baseline − 0.06` (a drop beyond the
  noise floor). Smaller moves are reported but not flagged.
- **Why advisory, not blocking — three reasons, all real:**
  1. **n is small.** 260 columns cannot separate marginal candidates; a blocking
     gate here would false-alarm on noise.
  2. **panel-tier softness + the ~3pp bias.** A blocking gate must not rest on
     single-round single-model labels with a known anti-model skew.
  3. **the bias cancels where it matters.** Every candidate is scored against the
     *same* panel labels, so the ~3pp skew is common-mode — it subtracts out of the
     candidate-vs-v19 *delta*, which is exactly what the flag reads. The absolute
     0.65-ish is soft; the relative drop is trustworthy. This is precisely why the
     band is advisory-on-the-delta, not blocking-on-the-absolute.
- **What stays blocking (unchanged):** gold + the corpus-honest relocation gate
  (H05) remain the only blocking gates in promotion order 0095. The representative
  band never overrides them and never blocks on its own.

## Consequence for downstream ACs

- **ac-02 baseline expectation is corrected:** v19 will reproduce the **panel-tier**
  number (raw-panel ≈ 0.648, recomputed exactly under `--reframe`), **not** the
  adjudicated 0.680–0.688. The correctness check is a clean end-to-end reproduction
  of the *panel* number from the frozen fixture — the adjudicated band is recorded
  as context, not as the fixture's baseline.
- **ac-04** keeps its job unchanged: whether the (panel-tier) representative band
  moves independently of gold across candidates.

## One line

The representative fixture is an honest, reproducible *advisory* ruler — soft on
the absolute, trustworthy on the delta — and we say so in its provenance rather
than dressing a single-round panel up as gold-grade truth.
</content>
