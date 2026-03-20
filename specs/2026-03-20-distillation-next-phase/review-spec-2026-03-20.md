# Spec Review

**Date:** 2026-03-20
**Reviewer:** Context-separated agent (fresh session, Opus)
**Spec:** specs/2026-03-20-distillation-next-phase/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] F1: The "172 distilled / 78 synthetic" split is wrong
**Category:** assumption
**Description:** The spec counts all `final_label` types across all rows. When filtered to agreement rows only (`agreement == 'yes'`), which is what the spec requires (AC-2), there are only **92 distinct types**, not 172. Only **44 types** have 10+ agreement rows (fully distilled). The rest need generator fill.
**Evidence:** Reviewer parsed `sherlock_distilled.csv.gz`: 24,261 agreement rows, 92 types with ≥1 agreement row, 44 with ≥10.
**Recommendation:** Re-derive the split from actual agreement data. Update constraints, AC-1, AC-6, S-1/S-2 with correct numbers. Decide whether 92 agreement types changes the value proposition.

### [CRITICAL] F2: `finetype generate` produces individual values, not columns
**Category:** missing-requirement
**Description:** The spec assumes S-2 can "generate remainder via `finetype generate`" as columns. But `finetype generate` outputs NDJSON of individual `{classification, text}` values, not column-shaped arrays. The spec never specifies: how many values per synthetic column? Distilled columns vary (1–20 values, mean ~8). A synthetic column of 20 perfectly-matched values is easier to classify than a real column of 3 noisy values — systematic bias.
**Evidence:** `finetype generate --help` output format is NDJSON.
**Recommendation:** Specify values-per-column for synthetic. Consider matching distilled distribution (5–20 values, seeded). Document that `finetype generate` output must be grouped into columns.

### [WARN] F3: Distilled data has quality issues the spec ignores
**Category:** failure-mode
**Description:** 185 agreement rows have unparseable JSON in `sample_values`. 257 have empty arrays. 7,726 (32%) have fewer than 5 values. 167 have only 1 value. Classifying 1-value "columns" is fundamentally different from classifying 20-value columns.
**Evidence:** Reviewer parsed agreement rows from `sherlock_distilled.csv.gz`.
**Recommendation:** Add minimum value count filter (e.g., `len(values) >= 5`). Log exclusions. Handle JSON parse failures.

### [WARN] F4: AC-2 doesn't verify value provenance
**Category:** test-gap
**Description:** AC-2 checks agreement metadata but not that `values` in the benchmark actually came from the claimed source row. A bug misaligning values and labels wouldn't be caught.
**Recommendation:** AC-2 should verify `values` matches `sample_values` of the source row.

### [WARN] F5: No specification for how Python scripts invoke the Rust binary
**Category:** missing-requirement
**Description:** Scoring 2,500 columns by shelling out once per column would take ~58 minutes (1.39s model load each). Batch mode would take seconds.
**Recommendation:** Specify `finetype infer --mode column --batch` with JSONL stdin.

### [WARN] F6: Headers empty in distilled, potentially present in synthetic
**Category:** assumption
**Description:** Distilled data has nearly all empty headers (Sherlock is headerless). If synthetic columns get headers, the distilled-vs-synthetic accuracy split becomes apples-to-oranges.
**Recommendation:** Decide explicitly: synthetic columns headerless (matching distilled) or with headers? Document the choice.

### [INFO] F7: No example row for benchmark CSV schema
**Category:** missing-requirement
**Description:** CSV columns defined in AC-1 prose but ambiguities remain: JSON encoding for values, empty header representation, source enum values.
**Recommendation:** Add example rows to spec.

### [INFO] F8: Benchmark will be ~80% synthetic
**Category:** constraint-conflict
**Description:** With 92 agreement types (44 fully distilled, 48 partial), and 158 fully synthetic, the benchmark is predominantly synthetic. This is fine but should be acknowledged.
**Recommendation:** Set expectations: initial baseline measured against a mostly-synthetic benchmark. Rebuild note when pipeline completes.

---

## Honest Assessment

The spec has a clean structure and reasonable goals, but it's built on a factual error that invalidates its core arithmetic. The "172 distilled / 78 synthetic" split counts all rows, not agreement-only rows — but the spec constrains sampling to agreement rows only. The actual split is 92/158, meaning ~80% synthetic. That's still valid (synthetic is correct by construction), but changes what the baseline means. The `finetype generate` interface mismatch and column-size question are real implementation blockers. Fix F1 and F2, and this is ready.
