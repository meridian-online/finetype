# ac-00 — the prize + the latency budget

**Spec:** 2026-06-18-fine-tuned-encoder-discovery (GATE)
**Date:** 2026-06-18 · v19 / 0.6.34 · `--reframe` lens

## (a) The prize — the semantic over-emission mass this lever targets

Misses that need **meaning, not value-shape** (the only mass in scope — structural and
datetime are deterministic/handled, pure-residual is already correct):

| instrument | scored | semantic prize | % | oracle upper-bound lift |
|---|--:|--:|--:|--:|
| **gold** | 927 | **83** | 9.0% | 0.803 → ~0.89 |
| **representative** | 259 | **31** | 12.0% | 0.691 → ~0.81 |

Breakdown (gold): residual→specific over-emission **62** (`PA→country_code`, `Si→region`,
`aranzebia→city`), geography/person semantic confusion **16**, currency-by-header **5**.

**These are ORACLE upper bounds** — what a *perfect* semantic resolver would recover.
Realistic capture is far lower: the determinability probe found even blind LLM panels
resolve only ~87% of contested columns, and a real model captures a fraction of that.
So the honest prize is "up to +9pp gold / +12pp representative, almost certainly much
less" — the discovery exists to find how much of it a fine-tuned encoder can actually take.

## (b) The latency budget

Measured marginal per-column cost of the **current** encoder (Model2Vec static encode +
multi-branch classify + duckdb read), best-of-3, isolating the ~0.2s fixed process startup
by differencing a 1000-col vs 100-col single-invocation profile:

- **Current: ≈ 0.9 ms/column.** Full 6.6M-column corpus pass ≈ 99 min of marginal model time.
- The current encoder is so cheap it sits *below the process-startup noise floor* on small
  files — startup (~0.2s) dominates interactive single-file use.

A fine-tuned LM encoder on CPU is realistically ~10 ms/column (small quantised transformer),
i.e. **~11× the current cost**. Projected:

| regime | what runs the LM | corpus-pass cost @10ms | verdict |
|---|---|---|---|
| **every-column** | all 6.6M | ~18 h (vs ~1.6 h today) | **likely off the table** — 10–20× the corpus budget |
| **low-band-only** | ~30% (quality_band `low`) | ~5.5 h | **the viable target** — and interactive cost is trivial (only the uncertain columns in a file escalate) |

## Budget ceiling + regime decision (the gate output)

- **Regime the bet MUST hit: low-band-only escalation.** The shipped confidence band
  (`quality_band < 0.70`, memory `confidence-quality-band-shipped`) is the gate that decides
  which columns get the richer encoder — the cheap static path serves the confident ~70%, the
  LM serves only the uncertain rest. Every-column is essentially ruled out at LM speeds.
- **Hard ceiling for ac-01:** a candidate must come in at **≤ ~10 ms/column on CPU
  (quantised)** to clear low-band-only; **≤ ~3–5 ms/column** would be needed to even reconsider
  every-column. Above ~10 ms/col it's a speed NO-GO regardless of accuracy.
- The integration pattern already exists — FineType embeds and runs three local neural models
  (Model2Vec, multi-branch, entity-classifier) compiled into the binary, offline and
  deterministic — so this is a speed/accuracy question, not a "can we run a local model" question.

## Hand-off to ac-01 / ac-02

ac-01 measures real CPU latency of candidate small encoders against the ≤10 ms/col ceiling.
ac-02 measures whether those candidates' representations *separate* the 83-/31-column semantic
prize better than Model2Vec. Both gate the ac-04 GO/NO-GO. Note: ac-01/02 need a Python ML
environment (torch + a small-transformer / sentence-transformer stack) the repo's eval venv
does not currently carry — standing that up is the first concrete step of ac-01.
</content>
