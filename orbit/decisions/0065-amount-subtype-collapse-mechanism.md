---
status: accepted
date-created: 2026-04-24
date-modified: 2026-04-24
primary_mechanism: other
post_fix_assertion: "In diagnostics/predictions_post.tsv (profile run against models/default on eval/datasets/csv/coverage_closure_phase_ab.csv after the ac-06 fix lands), at least 3 of the 11 target eval columns' top-1 predicted label flips from `finance.currency.amount` to the expected `finance.currency.amount_<variant>` label, versus the pre-fix diagnostics/predictions.tsv baseline."
---

# 0065. Amount-subtype collapse mechanism — hint-layer over-generalisation

## Context and Problem Statement

The 11 `finance.currency.amount_*` subtypes collapse to plain `finance.currency.amount` in
both v16 (shipped) and v18 (HELD). The v18 retrain carried 47/55 v16 failures unchanged —
retraining alone is not the lever (decision 0062). This MADR names the primary mechanism
that orbit/specs/2026-04-24-amount-variant-generators surfaces across ac-01..ac-04.

## Evidence (diagnostic arc)

Four diagnostic artefacts land under `orbit/specs/2026-04-24-amount-variant-generators/diagnostics/`:

- **ac-01 corpus counts** (`corpus_counts.tsv`, `v16_corpus_hash.txt`) — 12 rows (11 subtypes + plain-amount control). Row counts range 294 (amount_multisym) to 357 (amount_comma); max/min ratio = 1.214. **Corpus is near-balanced.**
- **ac-02 value-shape Jaccard** (`jaccard_matrix.tsv`) — 12x12, seed=42 pinned, 100 samples/subtype. Mean off-diagonal Jaccard = 0.0102, max = 0.1935 (amount ↔ amount_accounting), min = 0.0000. **Generated value shapes are nearly disjoint across subtypes.**
- **ac-03 confusion** (`confusion.tsv`, `predictions.tsv`) — v16 profile-pipeline output on the 11 eval columns. **All 11 predict plain `finance.currency.amount`** (0/11 correct). Disambiguation-rule trace attributes every prediction to `header_hint_*` — `hardcoded`, `cross_domain`, or `same_category`. Plain-amount control also reports `header_hint_hardcoded:amount`.
- **ac-04 raw softmax top-5** (`confidence_topk.tsv`) — multi-branch model softmax without Sharpen post-processing (via `MultiBranchClassifier::classify_column_topk`). Raw top-1 confidences range 0.33 (amount_crypto) to 0.99 (amount_code_prefix). Expected label in top-1: 1/11 (amount_code_prefix). In top-5: 3/11. **8/11 target subtypes have the correct label absent from the top-5 entirely** — this is a representation gap in the model, not a tie-breaking gap.

## Considered Options

- Option A — `imbalance`
- Option B — `overlap`
- Option C — `confident_wrong`
- Option D — `flat_confidence`
- Option E — `multi_cause`
- Option F — `other` (pipeline artefact: header-hint layer destructively over-generalises)

## Decision Outcome

Chosen option: **`other` — header-hint over-generalisation**.

**Named mechanism (post_fix_assertion frontmatter carries the machine contract):**

`crates/finetype-model/src/column.rs:4303-4314` contains a broad substring matcher in
`header_hint()`:

```rust
if h.contains("price")
    || h.contains("cost")
    || h.contains("amount")      // ← over-generalises
    || h.contains("salary")
    ...
{
    return Some("finance.currency.amount");
}
```

Every header containing the literal substring `amount` (including `amount_comma`,
`amount_lakh`, `amount_code_prefix`, ...) returns `finance.currency.amount` — the plain
parent label — regardless of the variant suffix. The Sharpen pipeline's hint-override
branches (`header_hint_hardcoded`, `header_hint_cross_domain`, `header_hint_same_category`
at column.rs:1111..1181) then apply this hint, which **overrides any correct variant
prediction the multi-branch model produces** (e.g. amount_code_prefix at 99.5% raw top-1,
amount_comma at 38.6% raw top-2 rising to top-1 post-Sharpen at 63.8% with `--no-header-hint`).

Without the destructive hint (`--no-header-hint`), 2/11 target subtypes classify correctly
(amount_code_prefix, amount_comma). With it, 0/11 classify correctly. The hint layer is
strictly destructive on the amount family.

Model-level representation weakness exists (ac-04: 8/11 correct label absent from top-5)
but is **not** the dominant pipeline failure — the hint override short-circuits the pipeline
before that weakness matters. Fixing the hint is the smallest unit-of-change that moves the
needle. Model retraining is the right v19-proper lever after the hint layer is fixed.

### Consequences

- Good, because the mechanism is surgical and verifiable without retraining — fix is a
  `header_hint()` table edit plus narrowing the destructive substring.
- Good, because the post_fix_assertion is directly measurable: re-run `finetype profile`
  on the same 12-column eval CSV, count top-1 flips from plain amount to variant labels.
- Bad, because it does not solve the underlying model representation gap — v19-proper
  retraining is still needed for robust accuracy beyond eval headers that happen to match
  the narrow hint matcher.
- Bad, because the narrowed hint relies on header strings matching the variant suffix
  exactly. Headers like "total amount comma" are still unhandled in production data.

## Ruled Out

- **`imbalance` — ruled out.** ac-01 corpus_counts.tsv shows max/min ratio = 1.214 across
  the 11 subtypes. Near-balanced. Imbalance-driven collapse requires ratios in double
  digits.
- **`overlap` — ruled out.** ac-02 jaccard_matrix.tsv shows mean off-diagonal Jaccard =
  0.0102 and max = 0.1935. Value shapes are overwhelmingly disjoint across subtypes;
  the model is not being confused by colliding value patterns.
- **`confident_wrong` alone — ruled out as primary.** ac-04 shows only 4/11 target
  subtypes with raw top-1 confidence > 0.6 on a wrong label (amount_lakh 0.78,
  amount_apostrophe 0.60, amount_nodecimal 0.64, amount_space 0.62). 7/11 are not in this
  pattern. This is a **contributing** mechanism, not primary. Dispatching the ac-07
  `confident_wrong` assertion (>=3 flips in confusion_matrix_post) would measure the
  right signal directionally but misattribute it — the flips come from removing the
  destructive hint, not from nudging confident-wrong predictions.
- **`flat_confidence` — ruled out.** Only 1/11 target subtypes (amount_crypto at 0.33 top-1)
  exhibits genuinely flat top-5 distribution. The rest are confident — just confident about
  the wrong label (or missing the right label entirely from top-5).
- **`multi_cause` — considered, ruled out as primary.** The pipeline-artefact (hint
  override) and model-weakness (representation gap) causes genuinely co-occur. `multi_cause`
  would require ac-07 to verify reduction in each sub-cause's post-fix stat — but the
  hint-layer fix does not move the model-weakness stat (ac-04 raw softmax is unchanged
  pre/post the hint edit; raw model is unchanged). Naming `multi_cause` would force the
  ac-07 assertion to fail on the unaddressed sub-cause, misreporting the outcome. The
  honest framing is: fix the dominant cause first; measure lift; if lift insufficient,
  the residual gap *is* the model-weakness sub-cause and a new spec addresses it via
  retrain. This is captured by the v19 hard gate MADR (ac-11) and v19-proper sprint
  eligibility.

## Diagnostic artefact paths cited

- `orbit/specs/2026-04-24-amount-variant-generators/diagnostics/corpus_counts.tsv`
- `orbit/specs/2026-04-24-amount-variant-generators/diagnostics/jaccard_matrix.tsv`
- `orbit/specs/2026-04-24-amount-variant-generators/diagnostics/confusion.tsv`
- `orbit/specs/2026-04-24-amount-variant-generators/diagnostics/predictions.tsv`
- `orbit/specs/2026-04-24-amount-variant-generators/diagnostics/confidence_topk.tsv`
