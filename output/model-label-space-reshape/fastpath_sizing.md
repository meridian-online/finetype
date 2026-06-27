# Fast-path inference sizing (reshape lever #8)

How much of the corpus can skip the model via a value-only deterministic fast-path
built from the cede set's conclusive validators. Method: real shipped validators
(`extract-features --validation`) over a uniform-random sample of non-trivial corpus
columns; fast-path fires when ≥90% of values pass exactly one value-self-sufficient
(CEDE_CLEAN, 125-leaf) validator. Provenance: scripts in scratchpad, seed 7.

## Result (n=566 non-trivial columns)

| outcome | n | % | meaning |
|---|---|---|---|
| clean single-leaf HIT | 38 | 6.7% | skip the model (embed + forward pass) |
| — agree with model | 32 | 5.7% | pure latency win |
| — model missed it | 6 | 1.1% | latency win + usually more correct |
| ambiguous (≥2 conclusive) | 7 | 1.2% | needs tightest-wins tie-break |
| no hit (needs model) | 521 | 92.0% | open-vocab / broad-shape — model's job |
| **fast-path-able total** | **45** | **~8%** | (95% CI ≈ 6–10%) |

Dominant leaves: url (40% of hits), boolean.terms, uuid, si_number, the ISO datetimes.

## Interpretation

- **~1 in 13 substantive columns skips the model** — a real but MODEST latency win,
  not transformative. 92% still need the model (they are exactly the open-vocab
  KEEP set the reshape protects).
- **Model-view proxy was ~14%** (columns the model *predicts* as cede types); the
  validator-confirmed value-only rate is ~half that. The gap IS the model's
  over-emission on cede leaves — the reshape thesis, measured: ~half of the model's
  cede-type predictions don't survive a strict ≥90% validator check.
- The fast-path is also a small CORRECTNESS win (6/38 catch columns the model
  fumbled: html, url), though a few divergences are contestable (si_number vs
  amount) — confirming the conservative conclusive set + tie-break discipline.

## So: the fast-path is worth wiring, but it is not the big inference lever

It is cheap (machinery exists: `crates/finetype-core/src/fast_path.rs`), correct-by-
construction, and rides the reshape. But the levers that touch EVERY column —
dual→single encoder (−20–40ms uniform) and dropping the validation branch (saves the
per-column ~245-validator feature computation) — are the larger prize. Sizing those
needs a per-stage latency profile (embed vs validation-features vs forward vs sharpen),
not a column-skip count. That is the next measurement.
