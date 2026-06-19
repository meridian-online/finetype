# Deterministic datetime parser — findings (spec 2026-06-19-deterministic-datetime-parser)

**Headline:** a delimited datetime string resolves to exactly one taxonomy leaf with no
model needed. We replaced the Sense model's guess between near-identical datetime
sub-formats (iso_8601 vs …_milliseconds vs sql_standard vs rfc_3339) with a deterministic
value-based reader, gated so it can only ever ASSERT a leaf the column's values actually
pass under the taxonomy's own validator. Result: it recovers timestamps the model dropped
to `unknown` and fixes sub-leaves, and — by construction — it cannot relocate non-datetime
columns into datetime.

## What shipped

- **`finetype_core::datetime_format::detect_datetime_format`** — an ordered table of
  exact-anchored datetime regexes (each with a field-range check: month ≤ 12, hour ≤ 23,
  plausible year). Because every pattern is anchored and exact, a value matches at most one
  format, so there is no specificity-ordering hazard — detection is "which single format
  does ≥95% of the column match". A mixed column matches none at threshold and asserts
  nothing. Locale-ambiguous `dd/mm/yyyy`↔`mm/dd/yyyy` is resolved at column level by which
  positional field exceeds 12; undecidable or contradictory → no assertion. 17 unit tests.
- **`datetime_format_refinement`** Sharpen rule (value-based, decision 0048, RHH-disableable)
  on the three multi-branch/fusion classify paths. Two safety gates:
  1. **Corroboration gate** — a bare-integer reading (epoch, 4-digit year) is asserted ONLY
     when the model already predicted `datetime.*`. A bare 10-digit integer is honestly
     epoch-or-id-or-phone, so the rule never *creates* datetime mass from integers.
  2. **Veto-consistency gate** — the rule asserts a leaf ONLY if the sample passes that
     leaf's taxonomy validator at ≥ 0.9 (well above the veto's 0.5 floor). This was added
     after the first eval showed the detector's looser format-family regexes could assert a
     leaf the downstream validation veto then hard-rejects into `unknown`/`alphanumeric_id`
     — strictly worse than the model's guess. Gating on the same validator the veto uses
     keeps the two a single source of truth.

## ac-03 — gold + representative (blocking truth gates): GO

Isolated by rule-on vs rule-off on the same binary (the honest A/B):

- **Gold: +1, 0 losses, 0 neutral regressions.** `agency_start_date` (`2001-09-17 00:00:00`
  …): `unknown` → `datetime.timestamp.sql_standard`. The model dropped a clean SQL
  timestamp; the rule recovered it. Headline 0.804, datetime exact-leaf held/up.
- **Representative: 0.691, identical to v19** — no regression on production-random columns.

The gold datetime set is small (141 rows) and the model is already ~90% exact-leaf on it,
so the curated headline moves little. The rule's real value is corpus-scale sub-leaf
precision (the model constantly guesses between value-near-identical ISO/SQL sub-formats),
which the 141-row gold set cannot see — measured in ac-04.

## ac-04 — corpus over-emission (directional, non-blocking): clean

Method: ONE instrumented binary, rule ON (default) vs rule OFF
(`RHH_DISABLE_HINTS=datetime_format_refinement`) over 2,000 random non-trivial corpus
columns, single-column profiles (matches gold methodology). Same model, same Sharpen chain,
same sibling context — the only difference is our rule, so every delta is attributable to
it exactly. (`output/deterministic-datetime/ac04_result.txt`.)

**Result: the rule changed 0 of 2,000 columns.** Zero non-datetime→datetime
(over-emission), zero datetime→non-datetime, zero datetime sub-leaf shifts. Every
datetime-leaf marginal is byte-identical off→on (~90 datetime columns in the sample across
21 leaves: sql_standard 11, hm_24h 15, year 11, iso 8, mdy_slash 6, epoch_seconds 6, …).

**What this measures vs assumes.** *Measured:* on a uniform-random corpus sample the rule
introduces no over-emission and no relocation — the corroboration gate (bare integers) and
veto-consistency gate (taxonomy ≥0.9) hold exactly as designed. *Assumed:* the 2,000-sample
generalises to the full ~500k corpus; over-emission risk is structurally bounded (a bare
integer can never *create* datetime mass; a delimited assertion requires the taxonomy
validator to confirm the values genuinely are that datetime), so a larger sample is expected
to stay clean.

**Honest scope.** 0 changes in 2,000 also means the rule fires *rarely* on common data — the
model already gets the common datetime sub-leaf right, so the rule only acts on the tail
(the model-dropped/mis-sub-leafed timestamp, like the gold `agency_start_date` recovery).
This is a precision safety-net for the tail, not a broad headline mover. At true corpus
scale a 0-in-2,000 rate still recovers on the order of hundreds of silently-dropped
timestamps — a real analyst win (a date read as `unknown` is worse UX than a date read as
the wrong sub-format) — but the big lever for datetime breadth is the taxonomy zoneless-ISO
gap below, not this rule.

## Known taxonomy gap (follow-up, not this spec)

The taxonomy's ISO patterns REQUIRE a trailing `Z` (`^…\.\d{3}Z$`), so a zoneless
`2013-06-04T01:02:03.123` (extremely common real data) matches NO datetime leaf. The
veto-consistency gate correctly declines these (no regression), but the feature cannot help
them until the taxonomy accepts zoneless ISO. Filed as a separate taxonomy task — it has
global validation implications (the veto + `validate` materialisation) and should go through
a taxonomy spec, not a Sharpen rule. The rare obscure leaves (jp_era, julian, chinese,
rfc_2822, ctime, syslog, clf, quarter, fiscal) are intentionally left to model+veto.
