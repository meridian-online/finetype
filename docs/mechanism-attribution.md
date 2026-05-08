# Mechanism Attribution — Reading the Validate-Corpus Report

## What this document is for

When `make validate-corpus` runs, it round-trips every dataset in
`eval/datasets/validate_manifest.csv` through the FineType pipeline:
`profile` → JSON Schema → `validate`. Any dataset that fails to land
back at ≥99% row validity is a failure to explain — and the explanation
matters because the fix path is different for each kind of failure.

The harness's **attribution cascade** classifies every failing column
into one of four mechanism buckets so analysts can act on the report
without reading Rust. This document is the analyst-facing companion to
that cascade. It tells you what each bucket means in plain language,
what failure shape triggers it, a concrete example from the iter-2
corpus, and what to do about it.

A second concept the report exposes is the **trigger label** — a
short string showing which rule inside the cascade fired. Two rules
can attribute to the same mechanism via different paths (for example,
`format_diversity` fires both when the predicted type matches the
expected type but the validator pattern rejects the values, AND when
the predicted type is in the same broad family as the expected type
but the subtype is wrong). The trigger label disambiguates these
without forcing you to re-derive which rule won.

## How to read the per-mechanism breakdown table

```
| Mechanism             | Failing columns | Datasets affected |
|-----------------------|-----------------|-------------------|
| enum_overfit          |               2 |                 1 |
| format_diversity      |               5 |                 3 |
| misclassification     |               9 |                 4 |
| code_vs_canonical     |               4 |                 2 |
| unknown               |               0 |                 0 |
| no_gt                 |               1 |                 1 |
```

- **Failing columns** — total count of `(dataset, column)` pairs that
  attributed to this bucket.
- **Datasets affected** — how many distinct datasets contained at
  least one column in this bucket.

The `unknown` bucket is the cascade's safety net. A non-zero count
there is a signal the cascade has a gap — file a follow-up card to
extend the rule set.

The `no_gt` bucket counts columns where the dataset has no ground-truth
label in its `.gt.yaml` sidecar. These are not mechanism failures —
they're coverage gaps in the curation surface.

---

## Mechanism: enum_overfit

### Definition

The model predicted the right type, but the validator's enum constraint
is over-fit to a closed value set that real-world data exceeds. The
type taxonomy says "this is a categorical with these 6 allowed values"
and the data has a 7th. The prediction is correct; the validator's
allowlist needs widening.

### Trigger conditions

- Predicted type **equals** expected type.
- At least one reject row carries `constraint_failed: enum`.

### Example failure

A `Currency` column whose model predicts `finance.currency.currency_code`
correctly, but the dataset includes `XOF` (West African CFA franc) and
the validator's enum doesn't list it. Currency code is the right type;
the closed-list enum is the wrong constraint.

### Recommended fix path

1. Inspect the rejects' values. Are they legitimate values of the
   predicted type? If yes, **widen the taxonomy enum** in the relevant
   `labels/definitions_*.yaml` file and bump the validator regression
   tests.
2. If the rejects look like they don't belong (typos, synthetic noise),
   the data quality is the issue — flag it back to the data owner.
3. Watch for over-correction: an enum that admits everything is no
   enum at all (Precision Principle, project CLAUDE.md).

---

## Mechanism: format_diversity

### Definition

The model identified the right kind of data (date, timestamp, currency,
etc.) but disagrees on the precise sub-format the values use. Either
the predicted subtype is correct and the validator's regex is too
narrow (path A), or the prediction picked the wrong subtype within the
correct broad family (path B).

### Trigger conditions

Two trigger paths share this bucket:

- **path-a-pattern**: predicted type **equals** expected type, a
  SEMANTIC_TYPE pattern reject is present, and the column name is NOT
  in the seam table (which would otherwise indicate a code-vs-canonical
  seam). Means the type is right but the validator's regex doesn't
  cover the values' format variant.
- **path-b-prefix**: predicted type **does not equal** expected type,
  but the two share a broad-type prefix (the first segment when split
  on `.`). Means the model picked the wrong subtype within the correct
  broad family — `datetime.timestamp.iso_8601` predicted against
  `datetime.timestamp.sql_standard`, for example.

### Example failure

`nyc_taxi.tpep_pickup_datetime` carries timestamps formatted as
`2024-01-01 00:57:55` (SQL-standard). The model predicts
`datetime.timestamp.iso_8601` because most prior corpus timestamps
were ISO-8601 with the `T` separator. Same broad family
(`datetime.timestamp`), wrong subtype → path-b-prefix.

### Recommended fix path

1. **Path A (validator pattern is the gap):** Widen the validator
   regex in the relevant `definitions_*.yaml` to cover the new format
   variant. Add a regression case to `crates/finetype-core/tests/`.
2. **Path B (model picked the wrong subtype):** This usually means
   training data under-represents the variant. File a card under the
   relevant model-improvement umbrella; document the variant with a
   minimal CSV in `eval/datasets/validate_corpus/csv/`.
3. Format diversity is the most common bucket on real-world data —
   it's the cost of supporting variant formats. Don't reflexively
   widen everything; check that the variant is genuinely valid.

---

## Mechanism: misclassification

### Definition

The model picked the wrong type and the validator caught it, but the
failure isn't subtype drift (path B format diversity) and isn't a
code-vs-canonical seam (path B code-vs-canonical). The prediction
crossed a domain boundary — `representation` predicted where
`identity` was expected, or similar.

### Trigger conditions

- Predicted type **does not equal** expected type.
- At least one SEMANTIC_TYPE reject is present.
- Earlier rules (path-B format diversity, path-B code-vs-canonical)
  did not fire — i.e. the predicted and expected types are in
  different broad families AND neither side is in the code-typed
  allowlist.

### Example failure

A `Photo` column carrying URLs that the model predicts as
`representation.text.plain_text` against expected
`technology.internet.url`. Different broad domains
(`representation` vs `technology`); neither in the code-typed
allowlist; the validator rejects on URL pattern mismatch.

### Recommended fix path

1. **First instinct should NOT be a new rule.** The Strength Through
   Simplification principle (MADR 0038) says: prefer retraining over
   adding disambiguation rules. Rules are a last resort.
2. Look at the model's prediction confidence — if it's low, the
   training data is thin for the expected type. Log a card to expand
   that type's training corpus.
3. If the prediction is high-confidence wrong, it's a signal the
   training data is contaminated — file a data quality card.
4. If a Sharpen rule could legitimately catch this case (e.g. a
   header hint that's already removed for being net-negative), do not
   add it back without a full ablation per MADR 0069.

---

## Mechanism: code_vs_canonical

### Definition

A column carries short-form codes where the schema expected canonical
text, OR vice versa. `Country` column with `US`/`GB`/`FR` against an
expected `geography.location.country` (canonical-name) type.
`oecd_employment.REF_AREA` carrying ISO-3 country codes against
`geography.location.country_code` (correct) but paired with a sibling
`Reference area` column carrying full names — the seam between the
two is the failure surface.

### Trigger conditions

Two trigger paths share this bucket:

- **path-a-pattern**: predicted type **equals** expected type, a
  SEMANTIC_TYPE pattern reject is present, AND the column name is in
  the seam table (`country`, `nationality`, `language`, `gender`,
  `blood_type`, etc.). The type is right; the column's name signals
  it's a code/canonical seam where the validator's pattern doesn't
  match.
- **path-b-codetype**: predicted type **does not equal** expected
  type, AND exactly one side (predicted XOR expected) is in
  `CODE_TYPED_LABELS` — the curated allowlist of taxonomy labels
  treated as short-form codes (country_code, swift_bic, currency_code,
  cpt, loinc, etc.). Means the predicted and expected sides disagree
  on whether the column is the code form or the canonical-text form.

### Example failure

`fifa_players.Value` carries values like `€110.5M` and `€565K`. The
GT label is `representation.text.plain_text` because the M/K suffix
breaks numeric parsing. The model, primed by the column name and the
currency symbol, predicts `finance.currency.amount` (in the
allowlist). Different prefixes (`representation` vs `finance`),
exactly one side in the allowlist → path-b-codetype.

### Recommended fix path

1. **Pair-aware data:** When the column is half of a code/canonical
   pair (REF_AREA / Reference area), the schema decision is upstream:
   either accept that the canonical-text side validates as text and
   adjust expectations, or extend the taxonomy with a paired-format
   type that admits both shapes.
2. **Allowlist tuning:** If the harness predicts a label that should
   not be in `CODE_TYPED_LABELS` (or omits one that should be),
   adjust the allowlist in `validate_corpus.rs`. Re-run the
   `vci3_code_typed_*` tests.
3. **Value-shape signals:** When label-only signals are insufficient
   (the GICS Sector case in the iter-2 corpus is the canonical
   example), file a follow-up card to plumb value-shape data into
   `RejectRow` — currently the cascade only sees labels and reject
   metadata, not the values themselves.

---

## When to file a follow-up card

The validate-corpus report is a diagnostic. The fix is always
upstream: the model, the taxonomy, the validator, the curation, or
the cascade itself. File a card when:

- A bucket count surprises you. If `enum_overfit` jumps from 0 to 12
  between runs, the model promotion or taxonomy edit between those
  runs is the suspect.
- The same `(dataset, column)` row appears with a different
  mechanism between runs. The fixture
  (`eval/datasets/validate_corpus_expected_attributions.yaml`) is the
  anti-regression lock — if a row's attribution changes,
  `vci3_fixture_attribution_regression_match` will fail and surface
  the diff in CI.
- The `unknown` bucket has any rows. The cascade has a gap — file a
  card to extend the rule set with an explicit handler.
- A row is currently marked `pending_escalation: true` in the
  fixture. These are known taxonomy gaps (e.g. GICS Sector with no
  FineType label). File a follow-up card to widen the taxonomy or
  plumb value-shape signals.

The fixture is the authoritative answer to "what does this failure
mean?" When you fix the underlying issue, update the fixture in the
same PR — the fixture row's `expected_mechanism` should change to
the new (correct) mechanism, and the `rationale` should explain the
fix.

---

## See also

- `crates/finetype-eval/src/bin/validate_corpus.rs` — cascade
  implementation and rule rustdocs.
- `eval/datasets/validate_corpus_expected_attributions.yaml` — the
  fixture (anti-regression lock).
- `eval/datasets/validate_corpus/*.gt.yaml` — per-dataset ground
  truth sidecars; the `notes:` section is the authoritative
  curation thesis for each dataset.
- `.orbit/choices/` — MADRs for the cascade design (bucket
  coalesce, fixture anti-regression lock, label-only
  code-vs-canonical attribution).
