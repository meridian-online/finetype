# ac-01 — Dual-encoder native: implemented and verified. potion-8M HALTS at the native gate.

**Date:** 2026-06-23 · spec 2026-06-23-dual-encoder-native-ship-potion8m

## Headline for the author

The dual-encoder works — and that's exactly how we caught that potion-8M shouldn't
ship. Run through the real `finetype profile` pipeline (the one every user actually
hits), potion-8M gets **69 columns in 100 right on the gold corpus; v19 gets 80**.
The "tie with v19" we saw offline was a measurement that skipped the validation
veto — once the veto is in, potion-8M drops 11 points and v19 doesn't budge. The
encoder change is correct to the last decimal; the model behind it isn't good enough.
**Stay on v19. The next bet is fixing the fresh-retrain data drift, not the encoder.**

## What was built (ac-01 deliverable — correct and kept)

Dual-encoder native inference, localised to `MultiBranchClassifier`:
- New optional `value_model2vec` resource + `value_embed_model` config field. The
  value-aggregation branch uses potion-8M (256-dim → 1024 agg); header + Sense +
  entity + semantic + sibling all keep potion-4M. Absent field → single-encoder
  (v19, m2v-244) loads bit-identically.
- New `extract_embedding_aggregation_dyn` (4 × embed_dim, any width). Replaces the
  128-pinned const path in the four `MultiBranchClassifier` value-agg call sites;
  tensor width driven by `config.embed_dim`.
- Edits: `multi_branch/config.rs`, `embedding_aggregation.rs`, `multi_branch/mod.rs`,
  `lib.rs`, the release `from_bytes` caller (main.rs), the three m2v8m configs.
- Tests: `test_dyn_matches_const_on_128dim` (None path bit-identical), 
  `test_dyn_1024_on_potion8m` (256→1024). `clippy -D warnings` clean on the changed
  crates; `cargo test -p finetype-model` green.

**Fidelity proof (B07 / H02):** the real Rust value-branch output matches the offline
Python `potion_embed` to 7+ significant figures on real gold columns
(`msg_id`: 0.05340816 vs 0.053408157; `region`: 0.121262364 vs 0.121262364). The
embedding matrix is byte-identical to HF `minishlab/potion-base-8M` (max abs diff
0.0), and Rust `encode_batch` matches model2vec `.encode` at cosine 1.0000. The
dual-encoder is faithful; the implementation is not the problem.

## The verdict — native gold composed, best-of-3 (reframe headline, n=927)

| model | offline composed (no veto) | **native composed (faithful)** |
|---|---|---|
| v19 (shipped) | 0.793 | **0.797** |
| m2v8m-s42 | 0.764 | 0.676 |
| m2v8m-s43 | 0.792 | 0.690 |
| m2v8m-s44 | 0.794 | **0.689** |
| **m2v8m best-of-3** | **0.794 (ties v19)** | **0.690 (−11pp vs v19)** |

The HALT condition fired: native composed (0.690) does **not** reproduce the offline
0.794. Per the spec, STOP and diagnose. Diagnosis below.

## Why the offline number was wrong (the diagnosis)

The whole reason this spec existed: there is no faithful offline gate — the validation
veto lives only in native `finetype profile`. Now confirmed quantitatively:

- **Offline compose = Sharpen-only, no veto** (`compose_predictions`). It takes
  potion-8M's raw Sense 0.503 → 0.800.
- **Native = veto + Sharpen.** Same raw Sense 0.503 → 0.690.
- For **v19** the veto is neutral (0.793 → 0.797). For **potion-8M** the veto is
  destructive (0.800 → 0.690).

The veto only demotes a prediction when the column's values **fail that predicted
type's validator**. So potion-8M is predicting types its own values can't validate —
it over-asserts into types the data refutes. That is the **fresh-retrain data/recipe
drift** already documented for m2v-244 ([[m2v244-corpus-nogo-gold-reproducible-not-corpus-faithful]]:
numeric_code collapse, isbn/npi collapse, si_number/file_size over-emit onto
oracle-refuted columns). potion-8M shares the same data blend, inherits the same
drift, and the embed swap cannot fix a data-driven problem — exactly as predicted.
The corpus-honest gate would have said NO-GO; the native gold run now says it too,
11pp down, on the curated fixture, with the veto applied.

On the 168 native-vs-offline composed disagreements, offline is right 117× and native
15× against gold — i.e. the veto is throwing away predictions that *would* have scored,
because they don't validate. That isn't a v19-vs-potion quality story you fix with a
rule; it's the model asserting types the values don't support.

## Decision — HALT. Do not proceed to ac-02/03/04 with potion-8M.

- **ac-02 (corpus-honest gate):** would be NO-GO. m2v-244 (same recipe) already is;
  potion-8M now shows the same drift on native gold. No point spending the corpus pass.
- **ac-03 (rule fixes):** the spec caps this at ~4 rules and says ">4 → fix the data
  drift instead." An 11pp gold gap from systematic over-assertion is far past 4 rules.
- **ac-04 (ship):** documented fallback — **stay on v19**.

**v19 remains the default.** It is still 240-label + unreproducible (the binding
constraint is unchanged), but no reproducible candidate has cleared a faithful gate.

## What this unblocks / the real next bet

The dual-encoder is the standing, verified capability: it lets any future dual-encoder
model be tested natively (no more offline self-deception). The next bet is **not** an
encoder and **not** rules — it is **diagnosing and fixing the fresh-retrain data/recipe
drift** so a freshly trained reproducible model validates by construction (numeric_code
leading-zero preservation, isbn/npi, si_number/file_size over-emit). That is gold-blind
and only the veto/corpus gate sees it. Once a reproducible model is corpus-clean, the
dual-encoder (if a bigger value encoder is still wanted) is ready and proven.

One line: *the encoder is right, the data recipe is wrong — measuring honestly (with
the veto) turned a phantom tie into an 11-point loss, so the work moves to the training
data, and v19 stays put.*
