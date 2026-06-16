# Determinability probe — the contested columns ARE determinable; the gap is gold + taxonomy

**Date:** 2026-06-16 · 3 independent blind Claude panels (the distillation "blind pass"
mechanism) over 68 contested residual-label errors + 15 controls. Panels saw only
header + sample values — no gold, no model prediction. Substrate:
`output/determinability-probe/`.

## Panels are reliable (controls)

15/15 control columns (where model already agrees with gold): **100% unanimous**, and the
panel agreed with both gold and model on all 15. The instrument is trustworthy.

## The contested columns are NOT undeterminable

| | contested errors (n=68) |
|---|---|
| all-3 panels agree | **87%** |
| 2-of-3 agree | 13% |
| 3-way split | **0%** |
| **determinable (≥2 agree)** | **100%** |

"Not determinable" was the wrong call. A strong teacher labels these columns confidently
and consistently. The disagreement is not panel-vs-panel — it is **panel-vs-gold**.

## Where the disagreement actually is

- **panel-majority vs GOLD: 35 agree / 33 disagree** → the *gold label is contestable on
  ~49%* of these errors.
- **panel-majority vs MODEL: 7 agree / 61 disagree** → the model is genuinely wrong on
  most.

Breaking down the 33 gold-contestable errors:

| verdict | n | meaning |
|---|---:|---|
| **Recoverable** (panel sides with model) | 7 | gold wrong, model right → legitimate score gain on re-adjudication (e.g. `id`→uuid, `name`→entity_name, `BenchmarkName`→alphanumeric_id) |
| **Taxonomy gap** (panel says "other") | 7 | gold used `plain_text` as a fallback for a type FineType lacks: `street_name`, `filename`/file-path, `publicationyear`, `link`, `block` |
| **Panel-third** (panel ≠ gold ≠ model) | 19 | gold (mostly `plain_text`) is a lazy catch-all; panel says categorical / entity_name / alphanumeric_id. Gold wrong, model also wrong |

The 35 where panel = gold are the genuine, confirmed model errors — the hard residual core.

## What this means

1. **The "98% / not-determinable" tension dissolves.** These columns have defensible
   answers; what's capping the score is partly **gold-label quality** (~half the contested
   errors have a contestable gold label, almost all from the `ac-03` heuristic tier, not
   the two-panel-adjudicated tier) and partly **missing taxonomy types** hidden behind
   `plain_text`.
2. **Distillation-as-judge works.** The blind Claude panels are the existing distillation
   teacher; they reliably adjudicate the exact boundary the flat model fails. This is the
   honest analyst answer — a reasoned, confident call (with a named runner-up), not a shrug.
3. **The ambitious-but-honest path** is not "make the model hit 98% against today's gold."
   It is: re-adjudicate the heuristic gold (corrects ~33 labels here; +7 immediate, the
   rest become honest targets), close the taxonomy gaps (`street_name`, `file_path`,
   `publication_year`, …), then fix the residual model errors that survive — which are now
   *provably* determinable, so worth training/ruling.

## Caveat

The 3 panels share a base model, so "agreement" reflects shared priors, not fully
independent judgment (like human panels). But it matches the methodology behind the gold's
own two-panel tier, the controls validate it, and 0% three-way splits at high confidence is
a strong signal. For gold-grade re-adjudication, mix in a different teacher family
(e.g. the Qwen3:32b Ollama path already in the stack) as a third, genuinely-independent
panel.

## Recommended next moves

1. **Re-adjudicate the `ac-03`-heuristic gold** (the ~349 heuristic-labelled columns, not
   just these 68) with a mixed-teacher panel — per choice 0095 gold-evolution. Corrects
   contestable labels; the corrected gold is the honest ceiling.
2. **Open a taxonomy-discovery pass** for the recurring "other" types
   (`street_name`, `file_path`, `publication_year`).
3. **Surface panel reasoning + confidence as the analyst-facing output** for residual/
   contested columns.
