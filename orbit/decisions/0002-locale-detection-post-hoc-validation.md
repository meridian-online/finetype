---
status: accepted
date-created: 2026-02-26
date-modified: 2026-03-11
---
# 0002. Locale detection strategy — post-hoc validation vs model-based classification

## Context and Problem Statement

NNFT-126 trained tiered-v3 with 4-level locale labels (e.g. `identity.person.phone_number.EN_US`). The infrastructure works correctly: `strip_locale_suffix()` collapses 4-level to 3-level for voting, `detected_locale` field populates in `ColumnResult`, and locale appears in JSON output.

However, profile eval regressed 70/74 → 67/74 due to expanded T2 label space: VARCHAR/person at 94 labels hit 51% accuracy (vs 13 labels in tiered-v2), VARCHAR/location at 65 labels hit 55% accuracy. The CharCNN architecture hits a capacity ceiling at ~20 labels per T2 model.

## Considered Options

- **Option A — More training data and epochs.** Increase to 1000-2000 samples/label and 10+ epochs for tiered-v3. Risk: CharCNN may not scale to 94 classes.
- **Option B — Post-hoc locale detection.** Keep tiered-v2 for classification, detect locale via `validation_by_locale` patterns after classification. Zero regression risk.
- **Option C — Hybrid tier architecture.** Split T2 into a 3-level type classifier + separate locale classifier per type family. Clean separation but more models to maintain.

## Decision Outcome

Chosen option: **Option B — Post-hoc locale detection**, because it provides zero regression risk and leverages the existing `validation_by_locale` infrastructure (NNFT-118, NNFT-132). When a type is classified (e.g. `phone_number`), sample values are run against each locale's validation pattern — the locale with the highest match rate >50% becomes `detected_locale`.

Confirmed by analysis of the original finetype prototype, which succeeded with 4-level locale labels because it used a Transformer model with much higher capacity.

### Consequences

- Good, because the proven tiered-v2 model stays untouched — zero regression risk
- Good, because locale detection accuracy depends on validation pattern coverage, which is incrementally extensible
- Good, because locale is a composable add-on, not coupled to the classification model
- Bad, because types without `validation_by_locale` patterns report no locale
- Neutral, because future model upgrades (Transformer-based) could revisit model-level locale integration
