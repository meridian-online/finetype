# ac-03 diagnosis — what drives the 0.1494 reject rate (DO NOT FIX without author sign-off)

Measured 2026-06-05 with `scripts/roundtrip_metrics.sh` against the shipping
default Sense model (`sherlock-v19-relu-s42`, multi-branch). Reproduces the
spec's recorded baseline exactly:

- `non_trivial_pct = 1.0000` (recall guard holds — ac-02)
- `reject_rate_non_trivial = 0.1494`, grade F, 46,447 of 310,904 cells rejected.

## Which columns drive it

Per-column rejects from the `finetype_reject_errors` sidecar (total 46,447):

| column          | rejects | wrong type (Sense)                          | schema constraint that rejects        | what it should be                         |
|-----------------|--------:|---------------------------------------------|----------------------------------------|-------------------------------------------|
| `net`           | 14,132  | `representation.scientific.measurement_unit`| `enum` = 30 SI units {meter, second…}  | `representation.discrete.categorical`      |
| `locationSource`| 14,132  | `representation.scientific.measurement_unit`| same SI-unit enum                       | `representation.discrete.categorical`      |
| `magSource`     | 14,132  | `representation.scientific.measurement_unit`| same SI-unit enum                       | `representation.discrete.categorical`      |
| `id`            |  3,978  | `geography.coordinate.geohash`              | `pattern ^[0-9b-hjkmnp-z]{6,12}$`       | `representation.identifier.alphanumeric_id`|
| `place`         |     67  | `geography.address.full_address`            | long-tail (most rows pass)              | (acceptable — 0.5% reject)                 |
| `gap`           |      6  | `representation.numeric.integer_number`     | long-tail decimal-in-integer            | (acceptable — 0.04% reject)                |

Three network-code columns plus one event-id column produce **46,374 of the
46,447 rejects (99.8%)**. `place` and `gap` are noise. So the precision failure
is ~3.2 columns wide, not a smear.

### Why each is wrong
- **net / locationSource / magSource** are 11–14 distinct two/three-letter
  network codes (`us`, `ci`, `nc`, `nn`…). Sense labels them
  `measurement_unit`, whose JSON Schema is a closed `enum` of SI units, so
  **every** value rejects (100%). These are textbook low-cardinality
  categorical codes.
- **id** is 14,132 unique event identifiers like `us6000pgkh`. Sense labels it
  `geohash`, whose pattern is the geohash base32 alphabet
  (`[0-9b-hjkmnp-z]`, excludes a/i/l/o) bounded to 6–12 chars. The ids contain
  excluded letters / wrong shape, so they reject. It is a free-form
  alphanumeric identifier.

Both labels carry `disambiguation: multi-branch-sibling` in the profile — they
come straight from the model's sibling-context head, **not** from a Sharpen
rule. So nothing in the current pipeline second-guesses them.

## Why the existing Sharpen demotion doesn't catch this

`disambiguate_attractor_demotion()` (crates/finetype-model/src/column.rs:4954)
already implements exactly the right idea: validate the column's sample against
the predicted type's schema, and if >50% fail, demote to a representation.*
fallback (`attractor_demotion_validation:<label>`). On these columns it would
fire at a 100% fail rate.

But it only runs for labels in three hard-coded allow-lists
(column.rs:2460–2482):

```
NUMERIC_ATTRACTORS = ["geography.address.postal_code"]
TEXT_ATTRACTORS    = [first_name, phone_number, username, street_name]
CODE_ATTRACTORS    = [icao_code, ndc, cusip, top_level_domain]
```

Neither `representation.scientific.measurement_unit` nor
`geography.coordinate.geohash` is in any list — that single omission is the
whole reason the round-trip stays broken. The demotion machinery exists; these
two labels just aren't routed into it.

## Fix options (author decides — DO NOT apply here)

**Option A (recommended, smallest blast radius): add the two labels to the
attractor allow-lists.**
- Add `geography.coordinate.geohash` to `CODE_ATTRACTORS`.
- Add `representation.scientific.measurement_unit` to `TEXT_ATTRACTORS`.
- Signal 1 (validation failure, >50% fail) then demotes geohash →
  `alphanumeric_id` and measurement_unit → categorical, both via the existing
  `select_fallback()` path. This is a *value-based* demotion gated on the
  column's own schema-validation pass-rate — squarely within decisions
  0038/0048 (value-based, last-resort Sharpen) and the spec's "scoped
  value-based Sharpen demotion" lever.
- **Recall risk (ac-02): low.** Fallbacks are `categorical` and
  `alphanumeric_id` — both non-trivial taxonomy types, so `non_trivial_pct`
  stays 1.00. No collapse to `plain_text`. Confirmed both labels exist in the
  240-type taxonomy.
- **Precision risk: bounded but real.** Adding a label to an attractor list
  arms *all three* signals for it, not just the validation one. For
  measurement_unit, Signal 3 (cardinality ≤20 → categorical) and Signal 2
  (confidence <0.85 → demote) would also fire on genuine measurement-unit
  columns. measurement_unit is a low-cardinality type by nature (a units column
  often has few distinct values), so Signal 3 could over-demote real unit
  columns to categorical. Mitigate by gating the new entries to Signal 1 only,
  or by confirming on the gittables corpus that measurement_unit isn't a
  high-volume true-positive label before widening. geohash is safer — high
  cardinality, Signal 3 won't touch it.

**Option B (narrower, surgical): a dedicated value-based rule, not the shared
allow-list.** Add one rule that fires only when (predicted ∈
{measurement_unit, geohash}) AND (column schema-validation fail-rate > 0.5),
demoting to categorical (low-card) / alphanumeric_id (high-card). This isolates
the change from the attractor signals' side-effects, so it can't over-demote
real measurement_unit columns via cardinality. Costs a few more lines and a
test, but no collateral on the existing attractor behaviour. **Recall risk:
none** (same non-trivial fallbacks).

## Recommendation

If the author wants the blog demo green tomorrow with the least review surface,
**Option B** — it touches only these two labels and only via the
schema-validation signal, so it can't regress unrelated columns. Option A is
one-line-per-label but couples the fix to the attractor signals' cardinality
and confidence behaviour, which needs a corpus re-baseline before shipping.

Either way the model is the cleaner long-term fix (the spec notes both families
are LOW-safety for additive training, safety 0.29 / 0.32, so retraining is the
wrong lever) — a value-based Sharpen demotion is the sanctioned last resort and
is sufficient to close ac-03 / ac-04.

## Out of scope (left untouched, per task)
- No Sharpen rule or attractor list was modified.
- No model file touched.
- ac-03 and ac-04 left UNCHECKED — the fix is the author's call.
