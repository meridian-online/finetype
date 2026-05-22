---
pre_screener: claude (Claude Opus 4.7)
attestor_required: @hughcameron
generated_for: ac-12 attestation acceleration
---

# ac-12 — pre-screen recommendations

This document accompanies `spot_check.md`. I've read all 22 sampled gaps
and applied the three pass conditions (mechanism fits evidence, lenses
genuinely disagree, token correct). My verdict per gap is below. **You
remain the attestor**: scan the headline summary, deep-read only the
cells I flag as failing, override where I've got it wrong, and commit
your verdicts into `spot_check.md`.

## Headline

**Per my reading: 4 cells PASS, 4 cells FAIL the 90% per-cell
threshold.** Failing cells would need to be demoted from `report.md` to
`single_lens_signals.tsv` per the spec's failure-consequence procedure.

| cell | my verdict | per-cell pass rate (my read) |
|---|---|---:|
| `non_trivial_floor × format_diversity_path_b` | **FAIL** | 2/3 (67%) |
| `non_trivial_floor × misclassification` | **PASS** | 3/3 (100%) |
| `reject_rate_ceil × code_vs_canonical_path_a` | **FAIL** | 0/1 (0%) |
| `reject_rate_ceil × enum_overfit` | **PASS** | 3/3 (100%) |
| `reject_rate_ceil × format_diversity_path_a` | **FAIL** | 2/3 (67%) |
| `reject_rate_ceil × format_diversity_path_b` | **PASS** | 3/3 (100%) |
| `reject_rate_ceil × misclassification` | **PASS** | 3/3 (100%) |
| `reject_rate_ceil × validator_widening` | **FAIL** | 0/3 (0%) |

## The common failure pattern

The cells I flag as failing share one shape: **the mechanism token
emitted by the cascade doesn't match what's actually wrong with the
column.** The lenses correctly flag *that* something's wrong, but the
cascade's classification of *why* is sometimes off.

Specifically:
- `validator_widening` gets emitted on columns that aren't widenable
  (EMAIL column containing full addresses; URL column containing
  integers) — widening the email validator wouldn't make
  `"42294 Foster Plaza West…"` a valid email. The right token is
  `misclassification` or `unknown_no_fit`.
- `code_vs_canonical_path_a` gets emitted on an atom-label column with
  no code/canonical distinction at all — should be `enum_overfit`.
- `format_diversity_path_a` gets emitted on a `WEIGHT (%)` column where
  Sense identified `identity.person.weight` (wrong) and the data is
  percentages — should be `misclassification`.

This is a **cascade-rule precision** finding, not a YDF or lens-stack
finding. The AND-filter machinery (ac-09) is doing its job — both
lenses agree something is wrong. The cascade's mechanism assignment
(ac-08) is where the misclassifications concentrate.

## Per-gap details

### Cell: `non_trivial_floor × format_diversity_path_b`

**Sample 1** (`a57a3a01…`) — `text` column of fill-in-the-blank prompts.
Sense: plain_text; YDF: sentence; cascade: format_diversity_path_b.
- **My verdict: PASS** (low confidence). The data is sentence-like
  English prose with a specific pattern; `format_diversity_path_b`
  ("predicted type is narrow, broader variant exists") is defensible
  for plain_text → sentence.

**Sample 2** (`0c2f31b51309…`) — `Text` column of email body text. Sense:
plain_text; YDF: entity_name; cascade: format_diversity_path_b.
- **My verdict: FAIL**. Two concerns: (i) YDF's `entity_name` guess
  doesn't fit email body content — the lens disagreement is real but
  ungrounded; (ii) `format_diversity_path_b` doesn't fit a plain text
  body — the issue isn't format diversity. Right token would be
  `misclassification` or `fallthrough`.

**Sample 3** (`e0eb6890ae08…`) — `text` column of Magic: The Gathering
card rules text. Sense: plain_text; YDF: sentence; cascade:
format_diversity_path_b.
- **My verdict: PASS**. Domain-specific sentence-shaped text;
  format_diversity_path_b fits.

### Cell: `non_trivial_floor × misclassification` — all PASS

**Sample 1** (`63db51c7…`) — 4 columns literally named `Title` with title
content. Sense: plain_text. YDF: entity_name. DBpedia: ontology/title.
- **PASS** (strong). Textbook misclassification → training_data_addition.

**Sample 2** (`08593c1937d3…`) — `addition machine` column with CS terms
list. Sense: plain_text; YDF: sentence.
- **PASS** (mild). The data isn't sentences (YDF's read is off), but
  the cascade's `misclassification` claim — "Sense's prediction is
  wrong" — is true.

**Sample 3** (`b4128af44888…`) — column header that's itself a long
sentence excerpt; values are English narrative. Sense: plain_text;
YDF: sentence.
- **PASS** (mild). Both lenses concur the data is sentence-shaped;
  misclassification fits.

### Cell: `reject_rate_ceil × code_vs_canonical_path_a` — FAIL

**Sample 1** (`839ed8e4221b…`) — `atom_id` column with chemistry atom
labels (`C1'`, `C2`, `C2'`, `C3'`, `C4`). Sense: alphanumeric_id; YDF:
discrete.categorical; cascade: code_vs_canonical_path_a.
- **My verdict: FAIL** on condition (c). These atom labels are a fixed
  enum, not a code/canonical-form pair. The token should be
  `enum_overfit` — Sense's `alphanumeric_id` overfit because the values
  are categorical labels. `code_vs_canonical_path_a` doesn't fit any
  reading of this data.

### Cell: `reject_rate_ceil × enum_overfit` — all PASS

**Sample 1** (`cdf4571a30e4…`) — `display_unit` with mass units
(`kg`, `g`, `dg`, …). Sense: measurement_unit; YDF: categorical.
- **PASS** (strong). Fixed enum of mass units; Sense's measurement_unit
  validator overfit to a narrower set.

**Sample 2** (`211c7d391798…`) — `BooleanValue` with `boolean` and
`FALSE` as the two values. Sense: boolean.terms; YDF: categorical.
- **PASS** (defensible). The literal value `"boolean"` isn't a boolean
  term; enum_overfit fits.

**Sample 3** (`ecd834154222…`) — `unit.symbol` with mixed mass+time
units (`kg`, `g`, `lbs`, `s`, `h`). Sense: measurement_unit;
YDF: categorical.
- **PASS** (strong). Heterogeneous unit categorical; enum_overfit fits.

### Cell: `reject_rate_ceil × format_diversity_path_a`

**Sample 1** (`7925dc9e13e9…`) — `customer_id` with Stripe-formatted IDs
(`cus_*`). Sense: alphanumeric_id; YDF: entity_name.
- **My verdict: PASS** (mild). Sense's alphanumeric_id validator
  likely fails on the underscore; widening it (format_diversity_path_a)
  fits. YDF's `entity_name` read is slightly off but the cascade's
  mechanism claim holds.

**Sample 2** (`2453b24e2e5e…`) — `VERSION_DATE` column with
`2017-01-27` and `TRUE` mixed. Sense: datetime.date.iso;
YDF: plain_text.
- **My verdict: PASS** (mild). Sense's date.iso validator is correct
  for `2017-01-27` but rejects `TRUE` — that's the format-diversity
  failure pattern, even if the right fix is "remove the corrupt row"
  rather than "widen the validator".

**Sample 3** (`cd8ec93a4780…`) — `WEIGHT (%)` column with values `-90.0`
and `100.0`. Sense: `identity.person.weight`; YDF: decimal_number.
- **My verdict: FAIL** on condition (c). Sense said *person.weight*;
  values are clearly percentages (column literally named `WEIGHT (%)`),
  and a person weight can't be -90. This is misclassification, not
  format diversity. The token is wrong.

### Cell: `reject_rate_ceil × format_diversity_path_b` — all PASS

**Sample 1** (`7555f3869e52…`) — `World / Drawdown Region` with values
like `Middle East and Africa`, `OECD90`. Sense: continent;
YDF: entity_name.
- **PASS** (strong). Sense's `continent` is narrower than the actual
  concept (geopolitical region). Textbook format_diversity_path_b.

**Sample 2** (`0a86df7d3263…`) — `ADDRESS ZIP` with full addresses.
Sense: postal_code; YDF: full_address.
- **PASS** (strong). Sense's postal_code captures the narrowest
  interpretation; data is full_address. Both lenses agree.

**Sample 3** (`ddb54a6cdf58…`) — same shape as Sample 2.
- **PASS** (strong).

### Cell: `reject_rate_ceil × misclassification` — all PASS

**Sample 1** (`66010133df53…`) — `loses` column containing `(Reuters)`,
`Inc`, `filed`. Sense: word; YDF: categorical.
- **PASS** (mild). Data is corrupt/misaligned but the diagnostic
  correctly flags that something's wrong; misclassification is the
  closest available token.

**Sample 2** (`0860c99acb10…`) — `Namespace` column with .NET namespaces
(`Dapper.SimpleCRUDTests`). Sense: word; YDF: entity_name.
- **PASS** (strong). The right type isn't in the taxonomy yet — these
  are code identifiers — so misclassification → training_data_addition
  fits.

**Sample 3** (`b863f8172da1…`) — `authors` column with biology paper
author citations (`Subram. & Lodha`). Sense: full_name;
YDF: entity_name.
- **PASS** (mild). Author abbreviations are similar to but not exactly
  full names; misclassification is defensible.

### Cell: `reject_rate_ceil × validator_widening` — all FAIL

**Sample 1** (`75550339ad4b…`) — `EMAIL` column containing full
addresses (`42294 Foster Plaza West Danny, IA 06826`). Sense:
identity.person.email; YDF: full_address.
- **My verdict: FAIL** on condition (c). The values aren't emails;
  widening the email validator wouldn't make them valid emails. The
  right token is `misclassification` (column is misnamed) or
  `unknown_no_fit`. `validator_widening` doesn't apply.

**Sample 2** (`099beb68241d…`) — identical shape to Sample 1.
- **FAIL** — same reason.

**Sample 3** (`dda85e6d07f2…`) — `URL` column with integers like `57954`.
Sense: technology.internet.url; YDF: integer_number.
- **FAIL** — same shape. URL validator can't be widened to accept
  bare integers as URLs. Token should be `misclassification`.

## Summary for the attestor

**If you agree with my reading**: 4 cells fail (the validator_widening
cell wholesale, plus one sample each in format_diversity_path_a /
format_diversity_path_b / code_vs_canonical_path_a). The
failure-consequence procedure kicks in: demote all gaps in those four
cells to `single_lens_signals.tsv`, log the demotion in `progress.md`,
then close ac-12.

**If you disagree with my reading**: override per-gap in `spot_check.md`
directly. The strict pre-screen here errs toward FAIL on token
mismatches; a more lenient reading (mechanism *roughly* fits the
failure pattern, lenses *technically* disagree) would PASS more.

**If you want me to actually demote**: I can write the
`progress.md` update + remove failing-cell gaps from `report.md`
(rebuilding it with a filter) once you confirm the verdicts.

## What this finding means in plain terms

The mechanism cascade — the rule-based engine that classifies *why* a
column failed FineType's prediction — gets the broad category right
(both lenses agree something's wrong) but sometimes labels the failure
with the wrong specific mechanism token. The 4 failing cells aren't
randomly wrong; they share a pattern: **the cascade reaches for a
specific mechanism (`validator_widening`, `code_vs_canonical_path_a`,
`format_diversity_path_a`) when the actual answer is the more generic
`misclassification`**.

This is a cascade-rule precision issue, not a corpus diagnostic
failure. The corroborated_gaps.parquet still surfaces real columns
worth investigating; the mechanism *label* on them is sometimes
off-by-one in the closed-10 token set. For the v20 retrain plan, this
matters only for the 4 failing cells (which collectively have ~390
clusters out of 64,565 — under 1% of the total report). The 95.2%
ac-11 precision on labelled_eval already evidences that the cascade
gets it right at population scale; the spot-check just surfaces that
some specific cells are weaker.
