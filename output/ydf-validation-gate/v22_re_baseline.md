# v22 re-baseline against the gated YDF eval

Per spec `2026-05-26-ydf-validation-gate` ac-05.

## Headline

v22 **moves from Failed to Partial** once the cell-2 metric stops
penalising it for disagreeing with demonstrably-wrong YDF labels.

| Baseline | v22 cell-2 vs v19 | Band (per v22 spec ac-08) |
|---|---:|---|
| Noisy (raw YDF) | −8.9% | Failed (< 10%) |
| Gated (clean YDF) | **−10.4%** | **Partial (10–20%)** |

Net change: +1.5 pp once the metric is honest. Just inside the
Partial band; still short of Met (≥ 20%).

## What changed and why

The cell-2 metric counts columns where Sense disagrees with YDF and
YDF says "geography". Before this spec, YDF's predictions were
unvalidated — so the metric penalised v22 every time it disagreed
with calls like:

- 2,828 `msg_id` columns labelled `geography.transportation.iso6346`
- 547 `TEAM_ABBREVIATION` columns labelled `country_code`
- 237 `exchange` columns labelled `country_code`
- credit-card / phone / IBAN / EIN columns labelled with finance or
  identity types they don't structurally belong to

The gate (ac-01) refuses any YDF prediction where fewer than 50% of
the column's sampled values pass the predicted label's JSON Schema
validation. Across the v22 corpus pass, 3.06% of YDF predictions
were refused — but those refused predictions were concentrated in
the noisy types, so the impact on cell-2 is larger:

- v19 noise dropped: 3,163 cell-2 misses (3.9%)
- v22 noise dropped: 4,056 cell-2 misses (5.5%)

v22 had *more* noise pressure than v19 in absolute terms — so
removing it helps v22 relatively more. That's where the +1.5 pp
shift comes from.

## Per-subtype shape (v19 → v22, gated)

The Sense improvements v22 was actually making — masked before by
the noise — now show clearly:

- `location.city`: 55,281 → 49,642 (−10.2%)
- `location.region`: 10,449 → 9,110 (−12.8%)
- **`location.country`: 4,297 → 2,945 (−31.5%)**
- `address.full_address`: 5,728 → 5,658 (−1.2%)

The "regressed" subtypes the v23 spec chased (iso6346, mgrs,
plus_code, country_code) are entirely absent from the gated cell-2
table — the gate refused their YDF labels at near-100% rates, so
Sense's disagreement no longer counts as a miss. That confirms
the v23 ac-01 finding from the other direction: those weren't real
regressions to begin with.

## Honest scope of this finding

What we now know:
- v22's true position is Partial-band (−10.4%), not Failed.
- The v23 Sharpen-rule experiment was the wrong tool — but the
  v22 spec's pre-committed branching (Failed → "architectural
  surgery next") was triggered by a measurement artefact, not a
  real model deficiency.
- The gate is a precision-positive correction across all four
  tracked corpus passes (v19 −3.9%, v20 −3.0%, v21 −3.1%,
  v22 −5.5% noise drop).

What we don't know yet:
- Whether v22 reaches Met (−20%) against an even cleaner
  baseline — the gate is conservative (50% threshold; length-only
  validations skipped). A stricter gate may still leak noise we
  haven't measured.
- Whether the per-subtype gains (city −10%, region −13%,
  country −31%) hold up against a hand-audited sample of the
  remaining "missed geography" columns.

## What ships from this spec

- `ydf_prediction_gated` becomes the canonical scoring lens
  (ac-06 wires it into the corpus-pass pipeline).
- Future Sense retrains are judged against the gated baseline.
- The original `ydf_prediction` column stays in the parquet for
  diagnostic purposes — consumers that need raw YDF for lens-
  disagreement diagnostics still have access.

## One-line for a stakeholder

*v22 moves from Failed (−8.9%) to Partial (−10.4%) once the cell-2
metric stops scoring against demonstrably-wrong YDF labels;
v23's Sharpen-rule experiment was chasing a measurement artefact.*
