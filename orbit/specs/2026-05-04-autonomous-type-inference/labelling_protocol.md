# Labelling Protocol — `labelled_eval.tsv` for finetype-7zi

**Spec:** `orbit/specs/2026-05-04-autonomous-type-inference/spec.yaml` (v1.3)
**AC:** ac-13 (200-row hand-labelled eval for precision-on-labelled measurement)
**Sampling source:** measure half of `eval/gittables/failure_log.tsv` (rows where `file_content_sha256` SHA-bucket MOD 2 == 1, partitioned by `scripts/split_failure_log.py`)

## Purpose

The full failure_log floor (ac-02) measures the inference module's *decisiveness* — what fraction of B01/B04 entries get a non-`unknown` inference at confidence ≥0.7 with the locked weights. Decisiveness is not correctness. The labelled eval anchors precision-on-labelled — what fraction of decisive inferences are *correct* under human-judged ground truth.

This protocol exists so precision-on-labelled is a reproducible metric, not whatever-the-labeller-said.

## Sampling

1. Run `scripts/split_failure_log.py` to write `failure_log.measure.tsv` (rows where `file_content_sha256 MOD 2 == 1`).
2. Stratify the measure half by `predicted_type`: build a frequency table; require ≥30 distinct predicted types in the labelled subset.
3. Sample target: 200 rows total. Per-stratum allocation: at least 5 rows per predicted_type until the 30-type quota is met; remaining budget allocated proportionally to predicted_type frequency.
4. **Sampling seed: 20260504** (recorded in `progress.md`). Reproducible: the same seed yields the same row sample.

The sampling script SHOULD be `scripts/sample_labelled_eval.py` (deliverable of this AC). It writes `orbit/specs/2026-05-04-autonomous-type-inference/labelled_eval.unlabelled.tsv` — the same 9 columns as `failure_log.tsv` plus four empty columns ready for hand-labelling: `truth_inferred_type`, `truth_mechanism`, `labeller`, `note`. The labeller fills these.

## The columns

After labelling, each row of `labelled_eval.tsv` has these 13 columns:

| Column | Source | Notes |
|--------|--------|-------|
| `cycle_id`, `timestamp`, `file_path`, `file_content_sha256`, `column_name`, `predicted_type`, `observed_values_sample`, `inferred_correct_type`, `mechanism` | failure_log.tsv | Original 9 columns. `inferred_correct_type` and `mechanism` retain the values that were in failure_log AT SAMPLING TIME (likely "unknown"/"value-shape" for pre-7zi rows). They are NOT the inference module's output for this row — that comes from the precision-on-labelled measurement script. |
| `truth_inferred_type` | hand-labelled | Canonical taxonomy ID OR `representation.text.string` OR `unknown`. See "What to emit" below. |
| `truth_mechanism` | hand-labelled | One of the 10 closed mechanism tokens (MADR 0081). See "Mechanism semantics" below. |
| `labeller` | hand-labelled | Free text identifier. For autonomous Nightingale labelling: `"nightingale-2026-05-04"`. For Hugh review labels: `"hugh-2026-MM-DD"`. |
| `note` | hand-labelled | Free text. Empty if rule application is unambiguous. Required if `truth_inferred_type=="unknown"` (must say WHY the labeller couldn't decide). |

`labelled_at` is a single attestation block in `progress.md` rather than per-row. Saves space; matches how the labelling actually happens (one session, not per-row).

## Rubric — what to emit per column

### 1. Identify the column's true semantic type

Read `column_name` and `observed_values_sample` carefully. Apply this priority:

1. **Distinctive value-shape evidence dominates header.** A column named `"id"` with values that look exactly like UUIDs gets `truth_inferred_type = "identity.identifier.uuid"`, not whatever `id` might suggest.
2. **Header tokens disambiguate ambiguous values.** A column with values `"USA", "GBR", "FRA"` is country-code-shaped; if the header is `country` use `geography.location.country` (label form); if the header is `country_code` use `geography.location.country_code`. The header is the disambiguator.
3. **Generic super-types are last resort.** Only emit `representation.text.string` if no specific type fits. The fact that the validator for `representation.text.string` would pass on virtually any column does NOT make it the right truth label — it's the inference module's *fallback*, not the truth.

### 2. Cross-check against the taxonomy

Verify the chosen type ID exists in `labels/definitions_*.yaml`. Use the bigram search:

```bash
grep -rn "^<type_id_tail>:" labels/
```

If the chosen ID does not exist, either:
- pick the closest existing ID and explain in `note` (e.g. `"closest fit; observed shape suggests a new type would help"`), OR
- emit `truth_inferred_type = "unknown"` with a `note` that names the missing type (e.g. `"no canonical type; values look like SWIFT/BIC codes — taxonomy gap"`).

### 3. Resolve uncertainty

When two or more taxonomy IDs are plausible:

- **Same prefix, different leaf** (`country` vs `country_code`): the header decides per Rule 2.
- **Different prefix, plausible cross-domain** (numeric column predicted SEDOL vs identifier vs hs_code): pick the *narrowest* type whose validator would pass on the observed samples. Prefer specific over generic.
- **Truly ambiguous** (the labeller cannot decide between two specific types after 30 seconds of thought): emit `truth_inferred_type = "unknown"` with a `note` listing the candidates.

The 30-second budget is intentional. The rubric optimises for reproducibility and throughput, not deep judgment per row.

## Mechanism semantics — `truth_mechanism`

The 10 closed mechanism tokens (MADR 0081) describe **the failure mode the model exhibited on this column**, not how the labeller decided. Map your judgement to the closest token:

| If the labeller's read of the row is... | Emit `truth_mechanism =` |
|----------------------------------------|--------------------------|
| Model picked a wrong subtype within the right family (e.g. predicted `datetime.timestamp.iso8601` for a SQL-standard timestamp; both are `datetime.timestamp.*`) | `format_diversity_path_b` |
| Model picked a wrong family (e.g. predicted `finance.securities.sedol` for `ORD-00001` which is `representation.discrete.identifier`; different prefixes) | `misclassification` |
| Model picked an enum-typed predicted, validator would reject samples on enum, header confirms predicted | `enum_overfit` |
| Model picked a regex-typed predicted, validator would reject samples on regex, no seam (semicolon/comma list pattern) detected | `format_diversity_path_a` |
| Model and labeller agree on the type; validator passes | `prediction_confirmed` |
| Model and labeller agree on the type; validator rejects ≥50%; header strongly supports the predicted type (validator is broken, not model) | `validator_widening` |
| Labeller cannot type the column despite seeing values; no canonical taxonomy entry fits | `unknown_no_fit` (truth_inferred_type = `representation.text.string`) |
| `observed_values_sample` is empty / null / garbage | `fallthrough` (truth_inferred_type = `unknown`) |
| Cross-prefix code-vs-canonical pair (e.g. predicted `finance.securities.sedol` numeric, expected `finance.currency.amount` numeric — same numeric shape, different domain) | `code_vs_canonical_path_b` |
| Same-prefix code-vs-canonical pair (predicted `geography.location.country_code`, expected `geography.location.country`) | `code_vs_canonical_path_a` (rarer; document in note if used) |

The display roll-ups `format_diversity` and `code_vs_canonical` (without suffix) are NEVER emitted, in line with MADR 0075's rule-owned-trigger doctrine. `truth_mechanism` is one of the 10 suffixed/extended tokens.

## What to do when stuck

If a row resists labelling after 30 seconds:

1. Emit `truth_inferred_type = "unknown"`, `truth_mechanism = "fallthrough"` only if the values are genuinely unreadable (empty, garbage, null markers).
2. Otherwise emit `truth_inferred_type = "representation.text.string"`, `truth_mechanism = "unknown_no_fit"` and write a `note` explaining what the labeller saw and why no specific type fit.
3. Move on. Total time per row should average <60s; some rows are fast (clear emails, clear UUIDs), others are slower (industry-specific codes). Don't perfect any single row at the cost of finishing 200.

## Attestation

`progress.md` carries a single attestation block of the form:

```
## Labelled eval attestation

- Labeller: nightingale-2026-05-04
- Date range: 2026-05-04 (single session)
- Rows labelled: 200
- Stratification: ≥30 distinct predicted_types
- Sampling seed: 20260504
- Time budget: ≤30s/row (mean ~25s observed)
- Self-audit: ≥3 rows reviewed by Hugh post-labelling for spot-check;
  agreement rate recorded.
```

This block satisfies ac-13's verification requirement (labeller name + date + rubric reference) without per-row attestation noise.

## Anti-patterns — DO NOT

- **Do not** consult the inference module's output for any row before assigning `truth_inferred_type`. The eval is meaningless if the labeller anchors on the module's prediction.
- **Do not** cite `truth_mechanism` from the spec's cascade rules (ac-09). The labeller picks the mechanism that *describes the model's failure mode*; the rules are the module's predictions, not the labeller's reasoning.
- **Do not** use `truth_inferred_type = "unknown"` as a polite-disagreement vote. If a specific type fits, emit it — disagreement with the model is what the eval is for.
- **Do not** sample from the calibrate half (bucket 0). The calibrate half is for the descriptive sweep curve; the labelled eval lives entirely in the measure half (bucket 1) to keep the precision-on-labelled metric independent of any future weight tuning.

## References

- `orbit/specs/2026-05-04-autonomous-type-inference/spec.yaml` (ac-13)
- `orbit/decisions/0075-mechanism-bucket-coalesce.md` (mechanism vocabulary source)
- `orbit/decisions/0081-mechanism-vocabulary-aligned-with-madr-0075.md` (extended vocabulary)
- `orbit/decisions/0082-fallback-policy-text-string-vs-unknown.md` (fallback semantics)
