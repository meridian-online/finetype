# Discovery: profile→validate precision — corpus, harness, and what "good" means

**Date:** 2026-04-28
**Interviewer:** Nightingale
**Card:** none yet (this discovery feeds card creation)
**Mode:** discovery

---

## Context

`finetype profile <csv>` and `finetype validate <csv> <schema>` should be a self-consistent pair: profile a clean CSV, take its inferred schema, validate the same CSV — and most rows should pass.

In practice (2026-04-28), they aren't. Two evidence cases at session start:

- `airports.csv` (7698 rows × 14 cols) → **0 valid rows / 18,493 rejects / Grade F**
- `medical_records.csv` (60 rows × 14 cols) → **0 valid rows / 161 rejects / Grade F**

Diagnosis on these two CSVs surfaced **four distinct failure mechanisms**:

1. **Validator format-diversity gap.** Inferred label is correct but the validator only accepts a narrow format. E.g. `utc_offset` data is `"10"` (integer hours); validator wants `"UTC +10:00"` string form.
2. **Enum-from-sample over-fit.** Profile sees ≤N distinct values in a 100-row sample, freezes them as an enum constraint; subsequent rows fail. E.g. `timezone` enum frozen to 12 IANA values; real data has hundreds.
3. **Misclassification.** Profile picked the wrong label outright. E.g. `dst="U"` → `representation.scientific.rna_sequence`; `name="Goroka Airport"` → `geography.address.full_address`.
4. **Code-vs-canonical mismatch.** Validator expects canonical forms but data uses codes. E.g. `gender="M"` validates against an enum of `["Male", "Female", ...]`.

The session goal is to define what "good" looks like, scope a corpus + harness to measure it, and decide which mechanisms are in scope for sprint-level fixes.

### Prior art consulted

- **MADR 0055** (eval realism dimensions) — pinned floors, restricted-registry carve-out. Validate corpus extends this, doesn't reinvent.
- **MADR 0001** (locale-only confirmation) — universal validation can only reject for locale-specific types. Implication: missing/incorrect locale detection can break round-trip even when the label is right.
- **MADR 0037** (eval serves the engine) — eval expectations update when fixes are demonstrably correct. Same principle applies to the new round-trip eval.
- **MADR 0064** (validate as DuckDB reject pipeline) — current validate engine; reject ontology gives us the diagnostic surface we need.
- **MADR 0071** (validate absorbs load) — `error_type='SEMANTIC_TYPE'` vs `'TRANSFORM_FAILED'` distinction is the seam between mechanism (2)/(4) and mechanism (1).
- **m-19 sprint** (eval-corpus expansion) — already shipped: `eval/datasets/sources.yaml` (35 sources, role=eval), MADR 0055 floors, row-hash leakage firewall, 240/240 type coverage. Validate corpus reuses this infrastructure.

---

## Q&A

### Q1: Success shape — what does "good profile→validate" mean?
**Q:** When a user runs profile→validate on a clean, real-world CSV, what should happen?
**A:** **Round-trip pass.** profile(csv) → validate(csv, schema) should achieve ≥X% valid rows for clean data. Anything below threshold is a FineType bug — schema was wrong or validator was wrong. The pair is self-consistent by construction.

> Implication: profile must not emit constraints it can't honour at validate-time; validate must not be stricter than profile's expectations. Misclassification by profile becomes a validate failure that we own, not a user problem.

### Q2: Corpus shape
**Q:** What corpus do we build the round-trip metric against?
**A:** **Whole-CSV corpus, fresh build.** New corpus of full real-world CSVs (airports, medical_records, taxi trips, etc.) — not column-by-column extracts. Profile/validate operate on whole tables; the harness should mirror reality.

> Implication: each manifest row is a CSV file, not a column. Per-CSV metrics (rows valid / rows total, columns rejecting everything). MADR 0055 realism floors apply per-column within the CSVs.

### Q3: Corpus size and provenance
**Q:** How big and how sourced should the validate-precision corpus be?
**A:** **Broad (30–50 CSVs, mixed sources).** Mix of Kaggle, government open data (Data.gov, ONS, EU Open Data), GitHub-hosted reference CSVs. Prioritise diversity over depth — different domains, languages, header conventions, data ages. Each CSV is real with provenance.

> Comparable scale to m-19 (35 sources, 448-row column-level manifest), but the unit of work is the CSV, not the column. Sprint-shippable if curation is disciplined.

### Q4: Aggregation metric
**Q:** How is round-trip pass aggregated across the corpus?
**A:** **Dataset pass rate.** A dataset "passes" if ≥P% of rows validate. Headline: "N of 40 datasets pass at threshold P." Like CI test counts — each dataset is a binary signal. Per-dataset reports show which ones fail and why.

> Spec-time refinement: default P=99% (allows tiny noise). Harness may emit at multiple thresholds (P=99 / P=95 / P=90) for distribution visibility, but the headline is at P=99.

### Q5: Sprint deliverable
**Q:** What's the sprint deliverable — just the harness, or harness plus first-pass fixes?
**A:** **Harness + obvious fixes.** Ship the harness, baseline, plus the "obvious" fixes that fall out during corpus building. Sprint shows movement.

> Discipline: "obvious" is bounded by Q6 (in-scope mechanisms). Fixes outside those mechanisms are out of scope by definition, regardless of how easy they look.

### Q6: Fix scope by mechanism
**Q:** Which failure mechanisms are in-scope for sprint fixes — vs. deferred to follow-up cards?
**A:**
- **IN SCOPE:**
  - **Enum-overfit (profile-side).** Stop emitting enum constraints when sample cardinality suggests it's not actually an enum.
  - **Validator format diversity (taxonomy-side).** Tighten or widen specific validators where corpus reveals real-world format variance.
- **DEFERRED:**
  - **Misclassification (model-side).** Governed by MADR 0066 (v19 retrain hard gate). Defer to a follow-up retrain card.
  - **Code-vs-canonical mismatches (taxonomy-side).** Cross-cutting; needs decision-level work first. Defer to a follow-up decision card.

> The deferred mechanisms still surface in the harness's per-mechanism breakdown — they just don't get fixes this sprint.

### Q7: Sprint success criteria
**Q:** What does "sprint succeeded" look like for the validate-precision corpus?
**A:** **Movement-based.** Sprint succeeds if: harness ships + baseline measured + after-fix measurement shows net +N datasets passing at P=99%. No absolute target. Success is the documented improvement attributable to the in-scope fixes.

> Avoids the trap of a fixed target that's blocked by deferred mechanisms.

### Q8: Infrastructure reuse
**Q:** How much m-19 infrastructure does the validate-precision corpus reuse?
**A:** **Full reuse, extend schema.** validate-corpus CSVs join `eval/datasets/sources.yaml` with `role=validate` (alongside `role=eval`). Per-column MADR 0055 floors apply. Row-hash leakage check extended to cover training-data leakage. One source-of-truth manifest.

> Means-level: needs a manifest schema decision — do whole-CSV entries live in the same `manifest.csv` (which is column-keyed) or a parallel `validate_manifest.csv`? Spec-time question.

### Q9: Consumer / output surface
**Q:** Where does the validate-corpus harness run and what's its output surface?
**A:** **`make` target + eval dashboard.** `make validate-corpus` runs locally; output joins `make eval-report` alongside profile-eval and actionability. No CI gate yet — CI gating waits until score stabilises.

> Mirrors how profile-eval works today. CLI subcommand explicitly out of scope (we just consolidated CLI surface in v0.6.19).

### Q10: Ground truth
**Q:** Do corpus CSVs carry per-column ground-truth labels, or just the CSV file?
**A:** **Full GT per column.** Each corpus CSV ships with a sidecar mapping (column → expected x-finetype-label). Harness reports both round-trip pass AND mechanism breakdown (e.g. "this column was misclassified, that one was enum-overfit"). Curation cost: ~14 cols × 40 datasets ≈ ~560 labels.

> The GT also doubles as an extension of the profile-eval — whole-CSV ground truth that the existing column-extract eval doesn't have. Diagnostic value justifies curation effort.

---

## Summary

### Goal

Make `finetype profile <csv>` and `finetype validate <csv> <schema>` a self-consistent pair: profiling a clean real-world CSV and validating it against the inferred schema should yield a high round-trip pass rate. Build a corpus + harness that measures this, and ship the obvious fixes that fall out of building it.

### Constraints

- **Corpus:** 30–50 whole-CSV real datasets, mixed sources (Kaggle / open data / GitHub), per-column ground-truth labels, full reuse of m-19 infrastructure (`eval/datasets/sources.yaml` extended with `role=validate`, MADR 0055 floors per column, row-hash leakage firewall extended).
- **Fix scope (IN):** enum-overfit on the profile side; validator format-diversity on the taxonomy side.
- **Fix scope (OUT):** misclassification (deferred to retrain card under MADR 0066 gate); code-vs-canonical mismatch (deferred to decision-level work).
- **Output:** `make validate-corpus` target, integrated into `make eval-report` dashboard. No CI gate yet. No new public CLI surface.
- **Threshold default:** P=99% per-dataset row-pass for headline; multi-threshold reports permitted.

### Success Criteria

- Harness ships with `make validate-corpus`.
- Corpus assembled at 30–50 CSVs, all in `sources.yaml` with `role=validate`, all passing MADR 0055 per-column floors.
- Ground-truth sidecar shipped for every corpus CSV (~560 labels).
- Baseline measurement run and recorded in sprint artefacts.
- After in-scope fixes ship, re-measurement shows net +N datasets passing at P=99% relative to baseline.
- Per-mechanism breakdown report exists and attributes every failing column to one of the four mechanisms.

### Decisions Surfaced

1. **Round-trip pass is the validate metric.** profile(csv) → validate(csv, schema) ≥ P% rows valid is the contract. Failures are FineType bugs, not user problems. → MADR candidate.
2. **In-scope vs deferred fix mechanisms.** Enum-overfit + validator-format-diversity in; misclassification + code-vs-canonical deferred to follow-up cards. → MADR candidate.
3. **Whole-CSV validate corpus reusing m-19 infrastructure.** Extends `sources.yaml` with `role=validate`; applies MADR 0055 floors per column; extends row-hash leakage firewall. → MADR candidate (extension/refinement of MADR 0055 + 0056 + 0057).

### Implementation Notes

- **Manifest layout (open):** existing `eval/datasets/manifest.csv` is column-keyed (one row per column). validate-corpus is CSV-keyed (one row per file). Likely a parallel `eval/datasets/validate_manifest.csv` with columns: `dataset, file_path, source_url, licence, fetched_date, gt_sidecar_path`.
- **Ground-truth sidecar format (open):** small JSON or YAML alongside each CSV: `{column_name: expected_label}`. Format choice spec-time.
- **Reject-ontology mapping to mechanisms:** MADR 0071's reject ontology already gives us the seam:
  - `error_type='SEMANTIC_TYPE'` + label matches GT → mechanism 1 (validator format-diversity)
  - `error_type='SEMANTIC_TYPE'` + enum-constraint failure → mechanism 2 (enum-overfit)
  - `error_type='SEMANTIC_TYPE'` + label disagrees with GT → mechanism 3 (misclassification)
  - `error_type='TRANSFORM_FAILED'` → likely mechanism 1 or 4
  Spec time: harden this mapping into a deterministic classifier in the harness.
- **Leakage extension:** existing `scripts/eval_leakage/__init__.py` normalises (header, sample-values). Validate corpus rows must be excluded from training data via the same row-hash table (not just from profile-eval).
- **Output report:** likely `eval/eval_output/validate_corpus.md` with per-dataset pass/fail, per-mechanism breakdown, and a top-level "N of M datasets pass at P=99%" headline.
- **Engine-side:** harness shells the existing `finetype profile` and `finetype validate` binaries — no new engine code in this sprint. Fixes go into the existing crates (profile-emission for enum-overfit; taxonomy YAML for validator format-diversity).
- **Day-1 datasets to seed corpus:** airports, medical_records (already in tree); add ~10 well-known reference CSVs (Olympic athletes, NYC taxi sample, IMDB ratings, World Bank indicators, ONS census tracts, etc.).

### Open Questions

- **Threshold P pinning.** Default P=99% but spec should justify the choice and decide whether to report multi-threshold (99/95/90) or single.
- **Row-cap per dataset.** A 1M-row CSV vs 60-row CSV — do we cap rows for harness runtime? Or run full datasets and accept long harness times? (Likely cap at 100k rows for the harness; full-file analysis is opt-in.)
- **GT curation rule.** When a column's "right" label is debatable (e.g. `name` in airports — `entity_name` vs `place_name` vs `point_of_interest`), what's the adjudication process? MADR 0037 says eval expectations update when fixes are correct — same principle, but spec should define the workflow.
- **Carve-out for legitimately failing datasets.** A dataset that fails entirely on the deferred mechanisms (e.g. a CSV dominated by misclassification) — does it stay in the corpus marked as "expected fail until retrain card"? Or is it pulled out and re-added later? Spec-time.
- **Threshold for enum-overfit fix.** What sample-cardinality / sample-size combination triggers enum emission vs not? Needs a numerical rule. Spec-time.

---

**Next step:** `/orb:spec` to generate a structured specification from this discovery.
