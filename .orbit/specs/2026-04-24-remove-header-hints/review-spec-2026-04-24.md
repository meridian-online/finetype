# Spec Review

**Date:** 2026-04-24
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-04-24-remove-header-hints/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

```
| Pass                          | Triggered by                                              | Findings |
|-------------------------------|-----------------------------------------------------------|----------|
| 1 — Structural scan           | always                                                    | 2        |
| 2 — Assumption & failure      | content signals (eval datasets, model instrumentation)    | 3        |
| 3 — Adversarial               | hint_id-vs-rule_family granularity ambiguity threatens AC | 2        |
```

---

## Findings

### [HIGH] hint_id granularity is load-bearing but ambiguously defined
**Category:** assumption
**Pass:** 2
**Description:** The entire roadmap pivots on `hint_id` being finer-grained than `rule_family`. ac-01 says expected row count ≥ 60 ("~50 match arms + ~10 branch families + ~70 substring matchers — some collapse"). But ac-03 measures hits via `disambiguation_rule` output, and the runtime emitter does NOT label hits at hint_id granularity — every arm of `header_hint()` emits the same `header_hint_hardcoded` tag; every substring branch in `apply_header_sharpen` emits a coarser family tag like `header_hint_measurement` or `header_hint_location`. The spec silently assumes a mapping between compile-time hint_ids (from grep/AST walk in ac-01) and runtime rule tags (from disambig_rule parsing in ac-03) that does not currently exist in the code.
**Evidence:**
- ac-01 describes hint_id as stable identifiers like `hh_email_exact`, `hh_amount_substring` — requiring per-arm granularity.
- ac-03 measurement strategy: "parses the disambiguation_rule output" — but `grep -oE "header_hint_[a-z_]+"` across column.rs returns ~36 distinct family-level tags, not per-arm tags. The `match h { "email" | "e mail" ... => Some("identity.person.email") }` arm at column.rs:4007 emits `header_hint_hardcoded`, not `hh_email_exact`.
- ac-02's instrumentation hook (RHH_DISABLE_HINTS with hint_id list) requires the same per-hint_id granularity — which means either (a) the hook taps every match arm individually, which is invasive, or (b) the hook operates at family level, which means ac-01's "≥60 rows" collapses to ~15 rule_family rows.
**Recommendation:** Pick a granularity and make it consistent end-to-end. Two options:
- **Option A (per-arm, as written):** ac-02 instrumentation must tag every match arm with its hint_id at emission time (e.g., extend `disambiguation_rule` to carry the specific arm matched). This is a non-trivial refactor of `header_hint()` and substring branches. Add an explicit constraint acknowledging this.
- **Option B (per-family, simpler):** Redefine hint_id == rule_family. Drop the "≥60" floor to "≥15." AC-01 becomes a rule-family inventory (the 15-ish families in the ontology_schema). Counterfactual measurement runs 15 disablement passes, not 60+. Downstream per-domain specs remove at family granularity, not arm granularity.

### [HIGH] "12 header_hint_* rule-family tags" under-counts actual surface
**Category:** missing-requirement
**Pass:** 1
**Description:** Constraint 3 says the roadmap covers "all 12 header_hint_* rule-family tags." Grep of column.rs for `header_hint_[a-z_]+` returns 36 distinct suffixes (including examples like `header_hint_blocks_rescue`, `header_hint_embarked`, `header_hint_fare`, `header_hint_survival_columns`, `header_hint_ticket_cabin`, `header_hint_measurement_keywords`, `header_hint_measurements_nnft`, `header_hint_categorical_nnft`, `header_hint_class_columns`, `header_hint_count_columns`, `header_hint_coverage`, `header_hint_names`, `header_hint_publisher`, `header_hint_timezone`, `header_hint_no_match`, `header_hint_priority_hardcoded_first`, `header_hint_email`, `header_hint_phone`, `header_hint_identity`, `header_hint_tech`, `header_hint_postal`, `header_hint_geo`, `header_hint_date`, `header_hint_numeric`, `header_hint_class_keyword_matching`). The ontology_schema correctly enumerates 15 rule_family values, but the spec's prose constraint says 12. The interview's surface audit table listed 12 — but that was the interview's hand-count of mechanism categories, not a grep-verified list of emitted tags.
**Evidence:** `grep -oE "header_hint_[a-z_]+" crates/finetype-model/src/column.rs | sort -u` returns 36 lines. ontology_schema.fields.rule_family enumeration lists 15 values. Constraint text says 12.
**Recommendation:** Reconcile to one number. Either (a) ship ac-01 with the actual grep output and let the inventory be the source of truth (drop the "12" in constraint 3), or (b) specify that the 24 extra tags are dataset-specific one-offs (titanic, etc.) that get batched separately. Either way, the constraint prose should not pre-commit to a number that a 30-second grep refutes.

### [MEDIUM] ac-04 "spot-check 20 rows" is a soft verification in an otherwise strict spec
**Category:** test-gap
**Pass:** 2
**Description:** ac-04 verification: "baseline_prediction matches the profile eval baseline (spot-check 20 rows against eval/eval_output/profile_results.csv)." Spot-checking is manual and non-deterministic. Every other verification in the spec is a hard assertion (file exists, schema matches, row counts exact, sha256 identical).
**Evidence:** Contrast with ac-09's "sha256 diff" gate — that's deterministic and CI-runnable. ac-04's spot-check is not.
**Recommendation:** Replace with a full diff: "baseline_prediction column in rhh_counterfactual.tsv equals the predicted_type column in eval/eval_output/profile_results.csv for every (dataset, column_name) pair that appears in both files. Any mismatch is a test failure." Codify as `rhh_ac04_baseline_consistency`.

### [MEDIUM] ac-02 "wall-time within 5%" is a flaky gate
**Category:** test-gap
**Pass:** 2
**Description:** Zero-overhead test runs `finetype profile` once and compares wall-time. macOS/Metal inference timing varies by ≥10% run-to-run due to thermal, scheduler, and Model2Vec cache state. A single-run 5% threshold will false-fail in CI.
**Evidence:** The eval harness itself already accommodates this elsewhere by running multi-seed (see `scripts/sweep_v17.sh`). Binary-size check ±5% is fine (deterministic); wall-time is not.
**Recommendation:** Either (a) drop the wall-time check (binary-size ±5% is enough evidence of zero-overhead compile-out), or (b) require median-of-5 runs with ≥10% tolerance. Record both median and p95 in `rhh_timing.tsv`.

### [MEDIUM] ac-06 verification contains a self-referential failure mode
**Category:** constraint-conflict
**Pass:** 2
**Description:** ac-06 verification states: "At least one domain has removal_readiness == 'READY' (else the interview's recommended-sequence ordering would need revision)." If the data genuinely shows zero READY domains, the correct spec-phase response is to **re-sequence**, not to fail the verification. Encoding "data must support the interview's recommendation" as a test assertion inverts the evidence-first principle stated in `evaluation_principles[0]`.
**Evidence:** evaluation_principles line 277: "Evidence over judgement — 80% threshold is computed, not debated" (weight 0.35). ac-07 description line 183 explicitly accommodates re-sequencing: "may be overridden if ac-06 reveals evidence to re-sequence." ac-06 verification contradicts ac-07 description.
**Recommendation:** Rewrite the ac-06 verification clause to: "If zero domains are READY, rhh_methodology.md must document the revised sequence and its rationale in the 'Limitations' section." That keeps the evidence-first posture without turning the verification into a trap.

### [LOW] Instrumentation hook risks cross-contamination with default-build behaviour
**Category:** failure-mode
**Pass:** 3
**Description:** ac-02 requires a feature-flag-gated hook that "compiles out to zero overhead" when off. In practice, adding `if !should_skip("hh_xxx") { ... }` around every match arm / substring branch in column.rs (the hottest file in the pipeline) requires either: (a) cfg-gated code blocks that fork every hint site — maintenance burden and readability loss, or (b) a single runtime dispatch check per column that enumerates active hints — measurable overhead even when no hints are disabled. The spec says "zero overhead" but does not mandate a design.
**Evidence:** column.rs is 10,853 lines. ~100 disambiguation_rule assignments. ~36 header_hint_* tags. A naive wrapper at every site is a large diff.
**Recommendation:** Add a constraint: "The hook design must be reviewed before ac-02 implementation. Preferred pattern: cfg(feature = 'rhh-instrumentation') gating at the function entry point (single dispatch check), not per-arm." Optionally require a short design note in rhh_methodology.md §Instrumentation Design before code lands.

### [LOW] ac-03 "every hint either fires or is correctly bypassed" rule can false-flag
**Category:** test-gap
**Pass:** 3
**Description:** ac-03 verification: "No hint_id has columns_hit == 0 AND columns_unused < 352 (every hint either fires or is correctly bypassed — a hint with hit=0 and unused<352 indicates a parsing bug)." The invariant `hit + unused == 352` is stated, but a hint with `hit == 0` means `unused == 352` by arithmetic — the "AND unused < 352" branch is unreachable if the first invariant holds. The test is tautological unless the arithmetic invariant is violated by a bug — in which case the test as written won't catch the bug either.
**Evidence:** Line 106: "Sum of columns_hit + columns_unused == 352 for every row." Line 110: "No hint_id has columns_hit == 0 AND columns_unused < 352." Second clause is a consequence of first.
**Recommendation:** Drop the redundant clause, or rewrite as a distinct check: "For every hint_id, if columns_hit == 0, then type_targets must appear as a predicted_type for ≥ 1 column in the baseline eval (else the hint covers a type that is absent from the eval corpus — flag for Phase C coverage expansion)." This is a meaningful, non-tautological test.

---

## Honest Assessment

The spec is well-structured, traceable, and correctly scoped as measurement-only. The ambiguity_score of 0.074 is plausible for the skeleton, but the two HIGH findings (hint_id-vs-rule_family granularity, and the "12 tags" constraint that grep disproves) sit on the spec's load-bearing axis. Without resolving granularity first, ac-01 through ac-07 could ship with internally consistent files that downstream specs cannot act on — "remove hh_email_exact" is not a meaningful instruction if the actual artefact only measures at `header_hint_hardcoded` family level.

Biggest risk: the downstream per-domain specs (7 follow-up specs already listed in metadata) are all gated on this roadmap. If the granularity ambiguity is not resolved here, every downstream spec inherits it. Resolve by picking Option A (per-arm, with explicit instrumentation refactor scope) or Option B (per-family, simpler, matches the ontology_schema as written). Both are fine; the spec must pick one.

Two MEDIUM findings (ac-04 spot-check, ac-02 wall-time) are easy fixes — replace soft verifications with strict diffs or multi-run medians. One MEDIUM (ac-06 self-referential) reverses an evidence-first principle and should be rewritten. The LOW findings are polish.

Recommend REQUEST_CHANGES: resolve granularity (HIGH #1), reconcile the tag count (HIGH #2), tighten ac-04 and ac-02 (MEDIUM #1 and #2), and rewrite ac-06 verification (MEDIUM #3). Re-review after changes; the skeleton is sound and should pass a second review quickly.
