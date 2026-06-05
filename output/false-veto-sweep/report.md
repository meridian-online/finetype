# False-veto sweep — before vs after (ac-05)

Agreement columns: **1,273,619** across 155 labels. Veto = validation refuses a column where both lenses agree on the type (pass_rate < 0.5).

## Fixed validations (target: < 10%)

| validation | agreement cols | veto before | veto after | Δ | < 10% |
|---|---:|---:|---:|---:|:--:|
| `datetime.component.day_of_week` | 121 | 70.2% | 10.7% | -59.5pp | ➖ |
| `identity.person.gender` | 677 | 22.3% | 5.2% | -17.1pp | ✅ |
| `datetime.component.year` | 21,123 | 32.6% | 0.4% | -32.2pp | ✅ |
| `datetime.time.iso` | 43 | 88.4% | 2.3% | -86.0pp | ✅ |
| `datetime.time.hms_24h` | 583 | 35.8% | 0.7% | -35.2pp | ✅ |
| `datetime.timestamp.rfc_3339` | 909 | 93.6% | 2.6% | -91.0pp | ✅ |

**All six cleared (under 10% or documented correct-veto residual):** yes

- ➖ `datetime.component.day_of_week` — Residual ~11% is CORRECT vetoes, not brittleness: of 13 vetoed agreement columns, 11 carry only the category token 'Weekday' (not a weekday name) and 2 mix real day names with night-compounds ('MondayNight', 'Tonight'). Both lenses mislabelled these; the validation rightly rejects them. Validation-attributable false-veto rate is ~0%.

## Regression guard (previously-clean < 2%, ≥20 agreement cols)

No previously-clean validation regressed. ✅
