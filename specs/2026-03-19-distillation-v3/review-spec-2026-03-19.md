# Spec Review

**Date:** 2026-03-19
**Reviewer:** Context-separated agent (fresh session)
**Spec:** specs/2026-03-19-distillation-v3/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] 1. Sherlock 687K columns is unvalidated against token budget
**Category:** assumption
**Description:** The AC "all 687K columns distilled" has no token budget calculation backing it. At ~100 cols/batch that's ~6,900 batches. Phase 2 did 107 batches. This is a 64× increase with no evidence the weekly cap can absorb it.
**Evidence:** The open question "Token budget allocation: 687K columns at ~100 cols/batch = ~6,900 batches" is flagged but unanswered. The spec ships with an AC that hasn't been checked against the binding constraint.
**Recommendation:** Estimate tokens per batch (prompt for 250-label taxonomy + 100 values + two-pass response). Multiply by 6,900. Compare against actual weekly cap. Either validate the AC or revise to "Sherlock test split distilled" (~137K columns) or "representative sample across all 78 types."

### [CRITICAL] 2. No durability guarantee — same class of incident can destroy v3
**Category:** missing-requirement
**Description:** v2 output was lost in an incident. The spec contains no analysis of what went wrong or what v3 does differently to prevent recurrence.
**Evidence:** Resume mechanism only handles mid-run restarts, not the incident class that destroyed v2. No statement about root cause, no backup strategy, no commit-after-batch policy.
**Recommendation:** State what caused the v2 loss. Add durability measures: per-batch CSVs committed to repo or synced to stable storage, checkpoint log independent of filesystem state, merge step separated from agent loop.

### [CRITICAL] 3. Sherlock headerless columns fundamentally affects comparison validity
**Category:** assumption
**Description:** Sherlock columns have no headers. FineType's column-mode inference uses header hints as a first-class signal. Running FineType on headerless Sherlock data measures "FineType without headers vs Sherlock" — not production-mode FineType. Agreement rates will be systematically different from real-world performance.
**Evidence:** Flagged as open question but unresolved. The extraction JSONL sets `header: ""`. FineType's pipeline includes semantic.rs header hints, Model2Vec header encoding, financial header hints.
**Recommendation:** Pick one approach before implementation: (a) use Sherlock type label as synthetic header (leaks ground truth — document explicitly), (b) leave empty and document that results measure value-only inference, (c) use generic placeholder. Note the effect on three-way comparison interpretation.

### [WARN] 4. SOTAB extraction is underspecified and blocking
**Category:** assumption
**Description:** The extraction step is `blocking: true` but the JSON.gz format hasn't been verified. SemTab JSON tables have variable schemas.
**Recommendation:** Add extraction validation AC: schema check on 100-table sample before full run. Add fallback: if extraction complexity exceeds 2 hours, defer SOTAB to Wave 2.

### [WARN] 5. GitTables AC says "all ≤50-column files" but plan is 50/topic sample
**Category:** constraint-conflict
**Description:** AC says "all ≤50-column files extracted and distilled" (1M files, 10.9M columns). The actual plan is ~4,700 files (~50K columns).
**Recommendation:** Fix AC to match the sampled scope: "~50 files per topic (94 topics, ~4,700 files) extracted and distilled."

### [WARN] 6. "Invalid label rate <5%" has no validation mechanism
**Category:** test-gap
**Description:** No script or check is specified to compute invalid label rate against the 250-label taxonomy.
**Recommendation:** Specify a post-merge validation script that joins labels against taxonomy key list and outputs counts. Run incrementally during distillation.

### [WARN] 7. Sherlock→FineType type mapping methodology unspecified
**Category:** missing-requirement
**Description:** "Programmatic" mapping is stated but the method (hand-authored, LLM-assisted, keyword matching) isn't specified.
**Recommendation:** Pick a method. State expected unmapped fraction. Ensure "unmapped" is counted in gap analysis.

### [WARN] 8. Resume mechanism is fragile under partial writes
**Category:** failure-mode
**Description:** Skip logic checks for CSV file existence. Partial writes (crash mid-batch) create incomplete files that get merged as complete.
**Recommendation:** Add completion marker (`.done` file) written atomically after CSV validation. Skip logic checks marker, not CSV.

### [INFO] 9. PR-2 weakness categorization step not specified
**Category:** missing-requirement
**Description:** AC requires "disagreement breakdown by PR-2 weakness category" but no categorization mechanism exists in the output schema.
**Recommendation:** Add a post-merge tagging script that maps type transitions to PR-2 categories, or note this is a separate analysis step.

### [INFO] 10. Sherlock train+val+test vs test-only is unresolved
**Category:** assumption
**Description:** Open question left unresolved, but AC commits to full 687K. Test split (137K) is the standard benchmark partition.
**Recommendation:** Distill test split first (standard benchmark), then expand if budget allows.

---

## Honest Assessment

This spec is well-structured and reflects genuine prior learning. The blind-first protocol is sound and the output schema is concrete. However, it ships with two ACs internally inconsistent with constraints (687K Sherlock vs unknown token budget; 1M GitTables vs sampled scope), one open question that is a precondition for the primary AC (headerless Sherlock), and no durability improvement over the setup that lost v2. The biggest risk is that distillation runs for days consuming token budget, then results are unusable because headerless FineType runs aren't comparable to production inference, or because the Sherlock AC was never achievable. Before implementation: resolve the token budget calculation, pick a Sherlock scope (test-only vs full), answer the header question, and fix the GitTables AC. Those four changes would make this approvable.
