---
id: decision-002
title: Locale detection strategy — post-hoc validation vs model-based classification
date: '2026-02-26 00:01'
status: accepted
---
## Context

NNFT-126 trained tiered-v3 with 4-level locale labels (e.g. `identity.person.phone_number.EN_US`).
The infrastructure works correctly: `strip_locale_suffix()` collapses 4-level to 3-level for voting,
`detected_locale` field populates in `ColumnResult`, and locale appears in JSON output.

However, profile eval regressed **70/74 → 67/74** due to expanded T2 label space:
- VARCHAR/person: 94 labels at 51% accuracy (vs 13 labels in tiered-v2)
- VARCHAR/location: 65 labels at 55% accuracy
- Changed vote distributions break header hint disambiguation for 3 columns

The 3 regressions (tech_systems.os, books_catalog.url, datetime_formats.utc_offset) are all caused by
different T2 confusion patterns shifting which type gets plurality vote — not by code bugs.

The code infrastructure (`strip_locale_suffix`, `detected_locale`, locale-aware vote aggregation, CLI
output changes) is proven and committed regardless of which option is chosen.

Related: NNFT-126, NNFT-118, NNFT-132

## Options

### Option A: More training data and epochs

Increase to 1000–2000 samples/label and 10+ epochs for tiered-v3.

- **Effort:** 6–8h training time
- **Risk:** CharCNN may not scale to 94 classes; no guarantee of reaching 70/74
- **Gain:** Single unified model for both type and locale classification
- **Pillar alignment:** Future-proof (one model), but uncertain joy (may not converge)

### Option B: Post-hoc locale detection ⭐ recommended

Keep tiered-v2 for classification, add locale detection via `validation_by_locale` after classification.
When a type is classified (e.g. `phone_number`), run sample values against each locale's validation
pattern — the locale with the highest match rate becomes `detected_locale`.

- **Effort:** 2–3h coding
- **Risk:** Zero regression — proven model stays untouched
- **Gain:** Locale detection from precise validation patterns, aligned with the Precision Principle
- **Pillar alignment:** Does one thing well (type classification stays clean, locale is a composable add-on); joy (no accuracy loss); future-proof (validation patterns are extensible per locale)
- **Leverages:** Existing `validation_by_locale` infrastructure (NNFT-118, NNFT-132)

### Option C: Hybrid tier architecture

Split T2 into a 3-level type classifier + a separate locale classifier per type family.
T2 predicts the type (13 labels for person), then a lightweight locale model predicts locale from the same value.

- **Effort:** 4–6h design + training
- **Risk:** More models to maintain, coordination complexity between tiers
- **Gain:** Clean separation of concerns; each model stays small and accurate
- **Pillar alignment:** Composable (two focused models), but more complex to maintain

## Decision

**Option B: Post-hoc locale detection.** Keep tiered-v2 as the classification model. Detect locale
via `validation_by_locale` patterns after type classification is complete.

This was confirmed by comparative analysis of the old finetype prototype (`hughcameron/finetype`),
which revealed that the original 4-level locale-in-label design worked because it used a Transformer
model with much higher capacity. The CharCNN architecture hits a capacity ceiling at ~20 labels per
T2 model — the 94-label VARCHAR/person and 65-label VARCHAR/location models demonstrate this clearly.
Post-hoc validation is the right tool for locale detection given our model architecture.

Additionally, we will add richer designation metadata (NNFT-139) from the old prototype's taxonomy
(`broad_words`, `broad_characters`, `broad_numbers`) to codify which types the CharCNN can and
cannot reliably distinguish from character patterns alone.

Related: NNFT-139, NNFT-140, NNFT-141

## Consequences

- **Default model stays tiered-v2.** The tiered-v3 model artifacts remain in `models/tiered-v3/`
  for reference but are not used.
- **Locale is a composable add-on**, not a classification output. The `detected_locale` field is
  populated by validation pattern matching, not model prediction.
- **Locale accuracy depends on validation_by_locale coverage.** Types without locale patterns will
  report no locale. Expanding locale coverage (NNFT-141) is the incremental path to better detection.
- **No regression risk.** The proven model is untouched; locale detection is additive.
- **Future model upgrades** (e.g., Transformer or attention-based models) could revisit Option A or C
  if the capacity ceiling is lifted.

