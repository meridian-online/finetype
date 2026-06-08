# Header-hint deletion: not yet — multi-instrument evidence

**Date:** 2026-06-09
**Decision:** keep the hardcoded header-hint table as-is. Ship only the coordinate veto. Revisit deletion only after training-data fortification closes the model-gap families.

## The question
Following the corpus-scale ablation (`header_hint_ablation.md`), the plan was to *delete* the hardcoded header-hint arms the model already covers (keeping the load-bearing ones: url/datetime/isbn/postal/amount). Before deleting shipping behaviour we measured the delete config — and defer, and hints-on — on **all** instruments, not just the corpus gate.

## Result — delete regresses on two instruments

| config | corpus-honest gate | m-19 curated (352 cols) | rare-type scoreboard |
|---|---|---|---|
| hints on (baseline) | GO | 303 / **86.1%** | lat 0.0012 / url 0.925 |
| **coord-veto** (default) | **GO** | 303 / **86.1%** (neutral) | **lat 0.0001** / url 0.925 |
| defer (per-family) | GO | 297 / 84.4% (**−1.7pp**) | lat 0.0001 / url 0.923 |
| **delete** | **NO-GO** (postal_code) | 289 / 82.1% (**−4.0pp**, −14 cols) | lat 0.0001 / url 0.923 |

- **Delete fails the corpus gate (postal_code) AND loses 14 curated columns (−4.0pp).**
- **Defer** scrapes a gate GO but still loses 6 curated columns (−1.7pp).
- **Neither is free.** The hardcoded hints are doing real work on the hard/rare types — removing or softening them costs measurable accuracy the model can't yet recover.

## The load-bearing lesson: no single instrument was sufficient
The corpus-honest gate **caught delete** (postal_code) but **green-lit defer** — yet m-19 showed defer regressing −1.7pp. The curated breadth eval caught what the oracle-bound gate could not (the gate is blind to the rare/hard curated types). **Only running the gate + scoreboard + m-19 together gave the true picture.** A single GO is not safety. This is the concrete justification for the multi-instrument promotion discipline (instrument map in CLAUDE.md / choice 0093).

## Why deletion doesn't pay for itself (yet)
The ablation's headline upside — "+3.7 oracle agreement from removing hints" — was mostly **honest abstention** on bulk numerics (more `unknown`), not new correctness, and it came largely from families we keep anyway (url/datetime/isbn). Against that marginal bulk gain sits a real −4pp curated-accuracy loss and a gate NO-GO. Bad trade.

## What ships, what waits
- **Ships:** the **coordinate veto** — a clean *add*, GO on every instrument, latitude FP 12× better, m-19 neutral. (Default-on header hint `header_hint_coord_veto`.)
- **Waits:** the header-hint *deletion*. Precondition for revisiting it is the RHH roadmap's real blocker — **training-data fortification** that turns the model-gap families (url/datetime/isbn/postal/amount, `header_hint_cross_domain`, …) into model-covered. *Then* the deletions become free, and we re-measure on all four instruments before cutting.
- **Removed:** the `FINETYPE_HINTS_DEFER` / `FINETYPE_HINTS_DELETE` measurement scaffolding (commit a7fee8d) — findings preserved here + in memory.

## Substrate
This finding; memory `header-hint-deletion-blocked-multi-instrument`; ablation `header_hint_ablation.md`; gate reports `gate_{delete,perfamily-defer,coordveto}.json`; m-19 captures `/tmp/m19_{default,defer,delete}.txt` (transient).
