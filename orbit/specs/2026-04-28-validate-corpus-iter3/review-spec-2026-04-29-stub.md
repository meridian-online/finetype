# Spec Review

**Date:** 2026-04-29
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-28-validate-corpus-iter3/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 1 (content signal: eval corpus + reject ontology) |
| 2 — Assumption & failure | content signal + cascade-rule complexity | 6 |
| 3 — Adversarial | structural concerns: implicit cascade contract, no mechanism-count baseline | 3 |

---

## Findings

### [HIGH] code_vs_canonical rule contradicts the iter-2 implementation already in tree
**Category:** constraint-conflict
**Pass:** 2
**Description:** ac-02 + implementation_notes describe `code_vs_canonical` as a
*value-shape vs canonical-form disagreement* where prediction and GT disagree
on broad-type (one side codey, one side texty). The existing rule at
`crates/finetype-eval/src/bin/validate_corpus.rs:325-329` fires
`CodeVsCanonical` only when **predicted == expected** AND the column-name is
in a hard-coded 5-seam table (`gender|country|currency|language|blood_type`).
Two incompatible rule shapes describe the same mechanism. The spec doesn't
acknowledge the existing rule, doesn't say "replace it" or "extend it," and
doesn't reconcile the seam-table heuristic against the new value-shape
heuristic. Without this resolution, an implementer guesses, and "no silent
re-classification" (constraint 2) is impossible to enforce.
**Evidence:**
- Spec ac-02 (line 48-62): "values are short-form codes... but whose GT label
  is a canonical/text type" — implies predicted ≠ expected.
- Existing code (validate_corpus.rs:325-329): `if !mismatch && any_semantic
  && any_pattern && column_in_seam_table(...) { CodeVsCanonical }` —
  requires predicted == expected.
- Spec metadata.notes flags this as a stub spec, but the contradiction is
  load-bearing for ac-02.
**Recommendation:** Add a constraint that explicitly states the disposition
of the existing 5-seam-table rule: *replaced*, *extended* (new disjunctive
clause), or *kept as a separate sub-bucket*. If replaced, exit_conditions
should require a regression test proving the gender/country/currency seam
columns still attribute correctly.

### [HIGH] format_diversity rule shape conflicts with the cascade's misclassification priority
**Category:** failure-mode
**Pass:** 2
**Description:** ac-01's worked example (GT=`datetime.timestamp.sql_standard`,
predicted=`datetime.timestamp.iso_8601`) is a `predicted ≠ expected` case.
The current cascade fires Rule 1 (Misclassification) on any
`mismatch && any_semantic` — which catches NYC Taxi and GDELT *first*,
before any format_diversity rule has a chance. The spec's
implementation_notes hint at this ("Trigger when prefix-match AND
subtype-mismatch") but never says: "this rule must run **before**
misclassification in the cascade, gated on broad-type-prefix-match." Without
explicit cascade ordering, ac-01 is unreachable: the existing Rule 1 will
keep claiming these columns.
**Evidence:**
- validate_corpus.rs:315-318: Rule 1 fires on any mismatch + SEMANTIC_TYPE.
- Iter-2 actual table (curation spec lines 435-439) confirms NYC Taxi and
  GDELT attribute to misclassification under the current cascade.
- Spec ac-01 verification asks for these exact columns to flip to
  format_diversity — that requires cascade reordering, not just rule
  addition.
**Recommendation:** Add an explicit `constraint` pinning cascade order, with
the new format_diversity rule placed before Rule 1 misclassification when
GT and predicted share a broad-type prefix. Make this orderings part of the
test contract (Finding K is the test-naming corollary).

### [HIGH] Cascade reorder risks over-promoting genuine misclassification to format_diversity
**Category:** failure-mode
**Pass:** 3
**Description:** Constraint 4 says "≥1 column attributing to format_diversity
AND ≥1 column attributing to code_vs_canonical" — that's a *floor*, not a
*ceiling*. A naive prefix-match rule would re-attribute *every* same-broad-type
mispredicition to format_diversity, including ones that should remain
misclassification (e.g. a date column predicted as `datetime.year` when it's
actually a full timestamp may be a real model error, not a "format diversity"
issue). The spec gives no per-mechanism-count baseline against which the
post-iter-3 report is checked, so a rule that over-promotes still passes ACs.
Iter-2's curation spec table (lines 435-439) names the expected attribution
*per dataset* — that table is the latent baseline.
**Evidence:**
- spec.yaml line 27-30: floor-only guarantee on format_diversity and
  code_vs_canonical; no upper-bound, no per-cell flip allowlist.
- Curation spec lines 435-439: dataset×mechanism expected map exists already
  but is not committed as a test fixture.
**Recommendation:** Promote the iter-2 expected-vs-actual table to a
committed JSON/YAML fixture (e.g.
`eval/validate_corpus_expected_attributions.yaml`) keyed by `(dataset,
column_name) → expected_mechanism`. Add an exit_condition: "the post-iter-3
report's per-column attributions match the committed fixture for the rows
listed there." This locks the cascade behaviour against silent
re-attribution.

### [MEDIUM] Reject-row schema lacks the value-shape signal ac-02 needs
**Category:** missing-requirement
**Pass:** 2
**Description:** The implementation_notes for `code_vs_canonical` rely on a
"column-mode is short-token (length ≤ 4 chars, all upper)" heuristic. The
current `RejectRow` (validate_corpus.rs:170-178) carries
`column_name | error_type | constraint_failed | expected_type` — no sampled
column value, no length statistic, no character-class summary. Constraint 3
says rules consume "per-column artefacts already on hand," but
column-value-shape is *not* on hand. Either the rule is achievable from
labels alone (broad-type-mismatch with one side flagged code-typed in the
taxonomy) or the spec must add a value-sample artefact. The choice has real
implementation cost.
**Evidence:**
- validate_corpus.rs:170-178 — RejectRow fields list.
- spec.yaml lines 102-105 — value-shape rule shape.
- Constraint 3 (line 22-26) — claims artefacts are sufficient.
**Recommendation:** Pick one path explicitly:
- **Label-only:** code_vs_canonical fires when predicted broad-type is
  `representation.text.*` and GT broad-type is `*.code.*`-tagged in the
  taxonomy (or vice versa). Simple, deterministic, no new artefact.
- **Value-shape:** add a `column_mode_value_shape: {len, upper_ratio}`
  artefact to the per-column input. New plumbing through `process_dataset`.
The spec should say which.

### [MEDIUM] No negative tests pin rule boundaries
**Category:** test-gap
**Pass:** 3
**Description:** ac-03 calls for "≥4 tests passing (one per mechanism)" —
implicitly positive cases. There's no test that pins the *boundary*: e.g.
"given GT=`datetime.timestamp.iso_8601`, predicted=`representation.numeric.integer`,
format_diversity does NOT fire (this is misclassification)." Without
negative tests, a rule that over-fires (Finding C) passes ac-03 without
detection. The existing iter-2 tests at validate_corpus.rs:690-755 are all
positive — this gap is inherited, not introduced, but iter-3 is the right
place to close it.
**Evidence:** ac-03 verification specifies test count and discoverability,
not test polarity.
**Recommendation:** Add to ac-03: "≥4 positive tests (one per mechanism) and
≥4 negative tests (one per mechanism asserting it does NOT fire under a
plausible adjacent input)."

### [MEDIUM] ac-01 verification couples to corpus columns and "Datasets-affected ≥2"
**Category:** test-gap
**Pass:** 2
**Description:** ac-01 names specific columns (NYC Taxi `tpep_pickup_datetime`,
GDELT `SQLDATE / MonthYear / Year / DATEADDED`) and requires
"Datasets-affected ≥2" — meaning ≥2 datasets must trigger format_diversity.
The verification couples rule correctness to *current model predictions*. If
GDELT's `SQLDATE` predicts as `representation.numeric.integer` (not a
`datetime.*` subtype), the new prefix-match rule won't fire on it — but the
*rule* might still be correct. ac-01 then fails for corpus/model reasons
rather than rule reasons.
**Evidence:** spec.yaml line 42-46.
**Recommendation:** Split ac-01 into:
- **ac-01a (rule-correctness):** unit test with synthetic GT/predicted/reject
  rows asserting format_diversity attribution (covered by ac-03 already; can
  collapse).
- **ac-01b (corpus-anchored):** Datasets-affected ≥1 (not ≥2) on the iter-2
  manifest, with the report text quoted as evidence rather than column-name
  hard-coded.

### [MEDIUM] ac-02's bidirectional rule shape isn't pinned in implementation_notes
**Category:** assumption
**Pass:** 2
**Description:** ac-02 describes two sub-rules connected by "OR conversely":
(a) values are short codes, GT is canonical text; (b) values are long names,
GT is code-typed. The implementation_notes describe only one direction
("codey vs texty on one side and word/text on the other") without saying
which side gets which mechanism label. The unit test under ac-03 will need
fixtures for both directions, and the cascade may need to disambiguate when
the two sub-rules would both fire on different evidence.
**Evidence:** ac-02 lines 50-56; implementation_notes lines 102-105.
**Recommendation:** Add a structured rule statement:
- direction-A: predicted is `representation.text.*`, GT is `*.code.*`,
  column values mode-shape is short-upper → code_vs_canonical.
- direction-B: predicted is `*.code.*`, GT is `representation.text.*`,
  column values mode-shape is word-text → code_vs_canonical.
- otherwise: not code_vs_canonical.

### [MEDIUM] Constraint 2 (no silent re-classification) is unenforceable without a baseline snapshot
**Category:** constraint-conflict
**Pass:** 2
**Description:** Constraint 2 says re-attribution must be "intentional, not
incidental." But the spec exit_conditions don't include a committed
mechanism-count baseline against which iter-3 deltas are measured. Without
that, the constraint is aspirational — any implementer can ship rules that
re-attribute silently and still pass exit_conditions. This is the same
shape as Finding C, viewed from the constraint side.
**Evidence:** spec.yaml lines 19-22 (constraint 2) vs lines 121-124
(exit_conditions).
**Recommendation:** Capture iter-2's per-mechanism cell counts as a
committed snapshot file. Add exit_condition: "post-iter-3 mechanism counts
match the snapshot or every delta is documented in a row of an
'Iter-3 attribution diff' section in the report."

### [LOW] Test-naming verification doesn't match existing test prefix
**Category:** test-gap
**Pass:** 3
**Description:** ac-03's verification runs `cargo test -p finetype-eval
mechanism_attribution`. The existing iter-2 tests are named
`pvc_attribute_rule_*` (validate_corpus.rs:690, 703, 716, 729, 742, 754) —
they would not match. The spec's `metadata.test_prefix: vci3` is a third
naming convention. An implementer faces three plausible choices: rename old
tests, add new tests under `vci3_*`, or add new tests under
`mechanism_attribution_*` to match the verification command.
**Evidence:** spec.yaml line 73-76 (verification) vs line 128
(test_prefix) vs validate_corpus.rs:690-755 (existing tests).
**Recommendation:** Pick one. Either:
- update verification to match `vci3_attribute_*` and rename old tests to
  the same prefix for consistency; or
- keep the `vci3_*` prefix and add a constraint: "iter-3 adds new tests
  under the `vci3_` prefix; existing `pvc_attribute_*` tests stay as
  iter-2 regression."

### [LOW] Gap-spec→AC-08 lineage understated
**Category:** assumption
**Pass:** 1
**Description:** Iter-2's AC-08 verification (curation spec line 282) checks
for *any* directory matching `orbit/specs/*validate-corpus-iter3*` to
declare AC-08 satisfied via gap-downgrade. The current stub spec already
satisfies that check by existing. The spec's metadata.notes flags this
historically but doesn't make explicit: landing iter-3 closes the work, but
*this stub already discharged iter-2's exit criterion*. Worth noting so
nobody assumes iter-3 needs to ship before iter-2 can ship.
**Evidence:** curation spec line 282; this spec metadata.notes lines 133-138.
**Recommendation:** One-line addition to metadata.notes: "iter-2's AC-08
escape-hatch is satisfied by this stub's existence; iter-3 implementation
is the planned follow-up, not an iter-2 blocker."

---

## Honest Assessment

This stub spec lays out the right shape — four mechanisms, deterministic
rules, regression tests — but it has a load-bearing latent contradiction
with the iter-2 code already in `validate_corpus.rs`: the existing
`code_vs_canonical` rule requires `predicted == expected` plus a 5-seam
column-name match, while the new ac-02 describes a `predicted ≠ expected`
value-shape disagreement. The same holds for `format_diversity`: ac-01's
worked example flips a current-misclassification column to format_diversity,
which means the new rule must run *before* misclassification in the cascade.
Neither contradiction is acknowledged in the spec, neither has a constraint
pinning the resolution, and neither has a test that would fail loudly if
the existing rule got silently dropped.

The biggest risk is **silent cascade re-ordering**: a fast-and-loose
implementation that adds the new rules and gets them placed early enough to
catch ac-01 / ac-02's named columns will likely over-promote misclassified
date / numeric columns into format_diversity and break the un-stated
mechanism-count invariant. Constraint 4's floor-only guarantee makes that
slippage impossible to detect from exit_conditions alone.

Concrete asks before going to `/orb:design` or `/orb:implement`:

1. **Reconcile with the existing iter-2 rules** — say *replace*, *extend*,
   or *separate sub-bucket* explicitly. Decide for both `code_vs_canonical`
   (5-seam table) and `format_diversity` (current `predicted == expected`
   shape).
2. **Pin cascade ordering** as a constraint, not an implementation detail.
3. **Commit a per-cell mechanism-attribution baseline** (the iter-2
   expected-vs-actual table from curation spec lines 435-439) and require
   the post-iter-3 report match it modulo a documented diff.
4. **Decide reject-row schema:** label-only rules (no new plumbing) or add
   value-shape artefacts (new plumbing through `process_dataset`).
5. **Add negative tests** to ac-03 — one per mechanism asserting it does
   NOT fire on a plausible adjacent input.

Once those resolve, the spec is implementable. As-written, an implementer
could ship in three different directions and pass all three ACs.
