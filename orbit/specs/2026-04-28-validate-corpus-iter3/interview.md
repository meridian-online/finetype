# Design: Validate-corpus iter-3 — mechanism-attribution rules

**Date:** 2026-04-29
**Interviewer:** Nightingale
**Card:** orbit/cards/0014-profile-validate-precision.yaml

---

## Context

**Card:** *Profile-validate precision corpus* — 8 scenarios. Goal: profile→validate
behaves as a self-consistent pair; failures attribute to one of four named
mechanisms (enum-overfit, validator-format-diversity, misclassification,
code-vs-canonical).

**Prior specs (3):**
- **iter-1** (`2026-04-28-validate-precision-corpus/`, shipped PR #58) — Built
  the harness, the 5-rule cascade (5 mechanisms + Unknown + NoGt), and 2
  in-scope fixes (enum-threshold 50→32, decimal_number scientific-notation).
  Headline: 3/7 datasets pass at P=99%.
- **iter-2** (`2026-04-28-validate-corpus-curation/`, shipped PR #59) — Added 5
  mechanism-coverage datasets curated specifically to exercise format_diversity
  and code_vs_canonical (NYC Taxi, GDELT, FIFA, OECD, S&P 500). Headline 3/12
  (delta +0). **AC-08 gap-downgraded** because all 5 attributed to the catch-all
  `misclassification` bucket — iter-2 GT sidecars stay byte-unchanged as the
  test surface for iter-3's rules.
- **iter-3** (this stub) — Wire mechanism attribution to surface what iter-2
  curated. Stub flagged for design before driving.

**Gap:** iter-2 curated the test surface; iter-3 wires the rules to surface
what was curated. Pure attribution-logic change in `validate_corpus.rs`.

**Review-spec verdict:** REQUEST_CHANGES (saved
`review-spec-2026-04-29.md`). Three HIGH findings:
1. iter-3's `code_vs_canonical` rule shape contradicts the existing iter-2
   rule at `validate_corpus.rs:325-329` (which fires on `predicted == expected`
   + 5-seam table).
2. iter-3's `format_diversity` rule (predicted ≠ expected, shared broad-type
   prefix) requires cascade reorder ahead of Misclassification.
3. Cascade reorder risks over-promoting genuine misclassification; spec lacks
   per-cell baseline lock.

The design session resolves these as intent-level decisions where they are
intent-level, and records them as implementation notes where they're not.

---

## Q&A

### Q1: Iter-3 success bar in the dashboard
**Q:** What does iter-3 success look like to you in the dashboard? This sets
the bar for whether the cascade reorder is conservative or aggressive, and
tells the implementer how hard to push on shrinking the misclassification
bucket.

Options offered:
- A — Curated 5 attribute correctly (narrow)
- B — Curated 5 + meaningful misclassification shrink (recommended)
- C — Every column has a defensible mechanism (broadest)

**A:** **B — Curated 5 + meaningful misclassification shrink.** The curated 5
iter-2 datasets attribute correctly AND the misclassification bucket across
the full 12-dataset corpus shrinks proportionally. Genuinely-misclassified
columns stay in misclassification; subtype-drift and code-canonical cases
route to their named buckets. Higher blast radius than narrow option, but the
per-(dataset, column) fixture lock is the safety net against over-firing.

### Q2: Scope boundary — adjacent cleanups
**Q:** Iter-2 surfaced two adjacent cleanups: (a) test-naming convention split
across `pvc_attribute_rule_*` / `mechanism_attribution` / `vci3`; (b) AC-06
pre-screen floor recalibration deferred. Fold either in?

Options offered (multi-select):
- Test-naming reconciliation
- AC-06 floor recalibration

**A:** **Test-naming reconciliation only.** AC-06 floor recalibration explained
post-hoc as a calibration job on `eval/pre-screen_floors.yaml` for real-world
dimensional data; mechanism-orthogonal to iter-3 attribution rules. Stays
deferred for a future spec under the m-19 / eval-expansion arc.

### Q3: Rigour upgrades — strategic framing
**Q:** Hugh framed iter-3 strategically: *"This eval method is likely the best
measure of success this project will ever have. Assuring analysts that we can
detect and validate data with ease is crucial to our mission. I'm not trying
to ship this quickly — I want this flow to be the thing we're the BEST at."*
Given that bar, fold three rigour upgrades into iter-3 before driving?

Options offered (multi-select):
- Wider fixture (every failing column across all 12 datasets, not just
  iter-2's curated 5)
- 2-3 negative tests per mechanism (boundary cases, not just one)
- Analyst-facing markdown doc explaining the 4 mechanisms

**A:** **All three.** Iter-3 isn't just closing the AC-08 gap-downgrade — it's
defining what "best-at-this" means for the round-trip diagnostic capability.
The fixture becomes authoritative ground-truth on every failure (not just a
regression test for new rules). Negative tests pin rule boundaries against
future drift. The markdown doc makes the report self-describing for analysts
who don't read Rust. Estimated scope: ~3.5-4h vs the 3h baseline.

---

## Summary

### Goal
Define what "best-at-this" looks like for FineType's round-trip diagnostic —
the headline capability the project will be measured by. Concretely:
- Curated 5 (NYC Taxi, GDELT, FIFA, OECD, S&P 500) attribute to their target
  buckets (format_diversity / code_vs_canonical).
- Misclassification bucket shrinks proportionally across the full 12-dataset
  corpus — genuinely-misclassified columns stay; subtype-drift and
  code-canonical cases route correctly.
- Per-(dataset, column) fixture is **authoritative ground-truth** on every
  failure across all 12 datasets — not just iter-2's curated 5.
- 2-3 negative tests per mechanism pin rule boundaries against future drift.
- Analyst-facing markdown doc explains the 4 mechanisms in plain language —
  what each means, what triggers it, what the fix path is. Linked from the
  report header.
- Test-naming convention reconciled across iter-1 / iter-3.

### Constraints
- Pure attribution-logic change. Iter-2 GT sidecars and validate-corpus CSVs
  stay byte-unchanged. The harness binary at
  `crates/finetype-eval/src/bin/validate_corpus.rs` gains attribution rules
  and a fixture-driven regression test; the manifest, sources.yaml, and
  row_hashes.tsv are untouched.
- Card 0014 scenario 2 pins the partition at **four buckets** (enum-overfit,
  validator-format-diversity, misclassification, code-vs-canonical). Iter-3
  must coalesce — not split into 6 buckets — even though the existing iter-2
  rules and the new iter-3 rules describe operationally distinct mechanisms
  inside each bucket.
- Cascade ordering must be explicit and tested. New rules placed ahead of
  Rule 1 (Misclassification) when GT and predicted share a broad-type prefix.
- Per-(dataset, column) fixture locks the cascade behaviour. Silent
  re-attribution is impossible — drift requires a fixture-diff in the PR.
- Attribution stays deterministic, label-only, no model calls. Rules consume
  per-column artefacts already on hand: GT label, predicted label,
  validator-failure reason, and column header.
- AC-06 pre-screen floor recalibration is explicitly out of scope. Stays
  deferred under m-19 / eval-expansion.

### Success Criteria
- Iter-2's expected-vs-actual table (curation spec lines 435-439) matches
  iter-3's actual report — every (dataset, column) pair labelled in the
  iter-2 thesis attributes to its target bucket.
- Per-mechanism breakdown shows `format_diversity ≥1` AND `code_vs_canonical
  ≥1` (lifted from iter-2 AC-08 which downgraded these to 0).
- Misclassification bucket shrinks across the full 12-dataset corpus
  proportional to (curated 5 + same-broad-type-prefix subtype drift cases +
  code/canonical disagreement cases) moving out.
- **Wider fixture: every failing column across all 12 datasets** has a row
  in `eval/datasets/validate_corpus_expected_attributions.yaml` with its
  expected mechanism + rationale. The fixture is authoritative, not partial.
- All 4 mechanisms have rustdoc + ≥1 positive test + **2-3 negative tests
  per mechanism** documenting boundary cases (e.g. format_diversity does NOT
  fire on cross-domain mismatch; code_vs_canonical does NOT fire when both
  sides are code-typed but different codes). Net: ≥12-16 attribution tests.
- **Analyst-facing markdown doc** at `docs/mechanism-attribution.md` (or
  equivalent path chosen by implementer) explains each of the 4 mechanisms
  in plain language: definition, trigger conditions, example failure,
  recommended fix path. Linked from `eval/eval_output/validate_corpus.md`
  report header.
- CI test asserts per-(dataset, column) fixture match for every row.
- All `pvc_attribute_rule_*` / `mechanism_attribution` / `vci3_*` test names
  collapsed to a single convention (likely `vci3_*` to match
  `metadata.test_prefix`).
- `make ci` exits 0.

### Decisions Surfaced

1. **Coalesce, don't split: 4 mechanism buckets, two trigger paths each.**
   Card 0014 scenario 2 names exactly four buckets — that pins the partition.
   Each bucket dispatches on two trigger paths internally:
   - `format_diversity`: (a) predicted == expected + SEMANTIC_TYPE pattern-reject
     (iter-2 path); (b) predicted ≠ expected + same-broad-type prefix +
     subtype mismatch (iter-3 path).
   - `code_vs_canonical`: (a) predicted == expected + SEMANTIC_TYPE pattern +
     5-seam column-name (iter-2 path); (b) predicted ≠ expected + GT
     broad-type tagged code-typed (iter-3 path).
   Per-column report row carries a `trigger:` notes field so debug isn't
   lost. → Will record as MADR.

2. **Per-(dataset, column) fixture file is the anti-regression lock.**
   Promote iter-2's expected-vs-actual table at curation spec lines 435-439
   to a committed fixture at
   `eval/datasets/validate_corpus_expected_attributions.yaml`. Add a unit
   test asserting per-column attribution matches the fixture. Silent
   re-attribution becomes impossible — drift requires a fixture diff in the
   PR. The fixture is also the regression test for ac-03. → Will record as
   MADR.

3. **Label-only first for code_vs_canonical (iter-3 path).** Rule fires when
   predicted broad-type ≠ GT broad-type AND one side is taxonomy-tagged as
   code-typed (`*.code.*` domain or explicit allowlist in attribution.rs).
   No new plumbing through `process_dataset` / RejectRow. If the rule
   under-fires on iter-2's curated datasets at first run, escalate to
   value-shape (column mode length, upper-ratio) as a follow-up — not in this
   spec.

4. **Test-naming reconciles to `vci3_*` prefix.** Iter-3 renames iter-1's
   `pvc_attribute_rule_*` tests to `vci3_attribute_*` (or a similar `vci3_`-prefixed
   name chosen by the implementer). Spec verification text updated to match.
   Single naming convention going forward.

### Implementation Notes

(For the implementing agent — these were considered during interview but are
implementation-level and don't need author input.)

- **Existing cascade lives at `crates/finetype-eval/src/bin/validate_corpus.rs:259-340`.**
  Module `attribute` with `Mechanism` enum (5 + NoGt), 5-seam table at line
  290, `attribute()` fn at line 300. Read this before changing anything.

- **iter-2 rules are *correct in their domain*.** The `predicted == expected`
  + pattern-reject case is genuinely a format_diversity / code_vs_canonical
  failure (validator pattern is too narrow for real-world data). Don't
  delete these rules — extend the matching shape so each bucket fires on
  both trigger paths.

- **Cascade order after iter-3:**
  1. enum_overfit — `predicted == expected` AND any enum-constraint failure.
  2. format_diversity (path B — iter-3) — `predicted ≠ expected` AND GT and
     predicted share broad-type prefix (e.g. both `datetime.*`) AND subtype
     differs.
  3. code_vs_canonical (path B — iter-3) — `predicted ≠ expected` AND one
     side is taxonomy-tagged code-typed.
  4. format_diversity (path A — iter-2) — `predicted == expected` AND
     SEMANTIC_TYPE pattern AND not in seam table.
  5. code_vs_canonical (path A — iter-2) — `predicted == expected` AND
     SEMANTIC_TYPE pattern AND in 5-seam table.
  6. misclassification — `predicted ≠ expected` AND any SEMANTIC_TYPE reject
     (catch-all for prediction errors not handled by 2 or 3).
  7. unknown — anything else.

  Note that paths B run *before* misclassification (Finding B from
  review-spec). Paths A keep their iter-2 ordering.

- **Code-typed taxonomy set.** Curate an explicit allowlist of domains/types
  treated as code-typed: `geography.code.*`, `finance.classification.*`,
  `identity.code.*` (CPT, LOINC), `finance.banking.swift_bic`,
  `geography.location.country_code`, etc. Walk `labels/definitions_*.yaml`
  for `broad_type: VARCHAR` + short-token format-string as a heuristic for
  the initial set, then hand-curate. Document the set in attribution.rs
  rustdoc.

- **Same-broad-type prefix detection** is `predicted_label.split('.').next()`
  vs `expected_label.split('.').next()` — both return the domain (`datetime`,
  `finance`, etc.). Subtype mismatch is the full-label inequality after
  domain match.

- **Fixture file shape:**
  ```yaml
  # eval/datasets/validate_corpus_expected_attributions.yaml
  - dataset: nyc_taxi
    column: tpep_pickup_datetime
    expected_mechanism: format_diversity
    rationale: "SQL-standard timestamp predicted as iso_8601 — same broad-type, subtype mismatch"
  - dataset: gdelt_events
    column: SQLDATE
    expected_mechanism: format_diversity
    rationale: "compact-integer date YYYYMMDD predicted as datetime.year — same broad-type prefix"
  - dataset: oecd_employment
    column: REF_AREA
    expected_mechanism: code_vs_canonical
    rationale: "ISO country code predicted as representation.text.word"
  # ... etc, populated from iter-2 expected-vs-actual table
  ```
  Test iterates the fixture and asserts `attribute(...)` returns the
  expected mechanism for each row, given the actual reject rows from the
  iter-2 db files.

- **Test plan (per ac-03 + review-spec medium finding):**
  - 4 positive tests (one per mechanism) — minimum baseline.
  - 4 negative tests (one per mechanism asserting it does NOT fire under a
    plausible adjacent input). E.g. format_diversity does NOT fire when
    GT=`datetime.timestamp.iso_8601` and predicted=`representation.numeric.integer`
    (cross-domain mismatch — that's misclassification).
  - 1 fixture-iteration test asserting per-(dataset, column) match.
  - 1 cascade-order test asserting rules fire in the documented order.
  - Net: ≥10 tests under the unified `vci3_*` (or chosen) prefix.

- **Report layout.** Per-mechanism breakdown table stays 4-bucket. Add a
  `trigger:` column to the per-column failures table to disambiguate which
  trigger path fired (e.g. `format_diversity / subtype-mismatch` vs
  `format_diversity / pattern-reject`). The dashboard headline stays the
  same; the breakdown shows the 4 buckets cleanly; the per-column table
  shows the trigger reason.

- **Rustdoc requirement (ac-03 doc constraint).** Each rule function gets a
  rustdoc block describing: trigger condition, example input, example
  output, what the rule does NOT fire on. Module-level rustdoc on
  `attribute` summarises the cascade.

- **`cargo doc -p finetype-eval --no-deps` is the rustdoc-existence check.**
  Already runs in CI via `cargo doc` — no new infrastructure needed.

### Open Questions
None at intent level. The spec is ready for `/orb:spec` after this design.

---

## Decisions to record as MADRs

After `/orb:spec` finalises the spec, three MADRs land alongside iter-3:

1. **MADR (next number) — Mechanism-bucket coalesce: 4 buckets, two trigger
   paths.** Card 0014 scenario 2 pins the partition; this decision records
   that the operational split (predicted == expected vs predicted ≠ expected)
   is internal, not surfaced in the bucket count. Refines / extends iter-1's
   implicit cascade decision.
2. **MADR (next number) — Per-(dataset, column) fixture as anti-regression
   lock.** Records the choice over per-mechanism count snapshot or
   PR-eyeball. Notes that the fixture is also the test surface for ac-03.
3. **MADR (next number) — Label-only attribution for code_vs_canonical.**
   Records the "no new plumbing" decision and the escalation path to
   value-shape signals if the rule under-fires.
