# Design: Autonomous type-inference module for cycle worker

**Date:** 2026-05-04
**Interviewer:** Nightingale
**Bead:** finetype-7zi (P1 — bead serves as card per bead-native flow)
**Spec output:** `orbit/specs/2026-05-04-autonomous-type-inference/`

---

## Context

**Bead summary:** P1 deliverable surfaced in 2026-05-10 pass-2 tabletop. The 2026-05-03 GitTables 90% round-trip contract's E01 escalation fired 7 times across 7 full cycles to no effect because its trigger expression assumes inference capability the cycle worker doesn't have:

> failure_log accumulates ≥ 20 distinct (predicted_type, observed_inferred_type) pairs

All 21,789 failure_log entries from pass-1 cycles have `inferred_correct_type = "unknown"`. The pass-2 contract split E01 into E01a (autonomous, structurally blocked until this bead ships) and E01b (calibration backstop, human-attended tabletop). Without 7zi every retrain forever happens via E01b.

**Prior specs (closest matches by topic):**
- `orbit/cards/0014-profile-validate-precision.yaml` — precision card, parent context
- `orbit/cards/0015-status-orderid-misclassification.yaml` (status: proposed, no specs yet) — same misclassification family (status→periodicity, order_id→SEDOL); proposes training-data-side fix path
- `orbit/decisions/0075-mechanism-bucket-coalesce.md` — established 4-bucket / 6-trigger cascade for `validate-corpus` mechanism attribution; cascade takes (predicted, expected, samples) → mechanism
- `orbit/decisions/0078-validator-alternations-compose-canonical-sibling-patterns.md` — composition-over-invention doctrine; validators are canonical pattern source

**Architectural insight surfaced during design:** This inference module is the *forward direction* of the MADR 0075 cascade — same discrimination axis (validator-broken / subtype-drift / model-error), but inferring the unknown `expected` rather than explaining a known `(predicted, expected)` pair.

**Gap to close:** Cycle worker writes `inferred_correct_type = "unknown"` at `scripts/cron_cycle_work.py:382, 410`. Need module that fills this column with a real taxonomy ID (or a justified fallback) plus a mechanism tag, deterministically, in <100ms per column.

---

## Q&A

### Q1: Mechanism tag vocabulary
**Q:** Which mechanism vocabulary should the inference module emit — bead's (header-signal / value-shape / prefix-shape / sibling-context), MADR 0075's existing cascade vocabulary, or hybrid?
**A:** Hugh did not pick from the offered options; pushed back on the framing. Pointed out via titanic and airports examples that **validators are NOT always orthogonal ground truth**. Sex column rejects 891/891 because `identity.person.gender` enum is case-sensitive and incomplete. timezone column rejects 6761/7698 because `datetime.offset.iana` enum has 12 entries vs ~600 IANA zones. Name column rejects 177/891 because the regex disallows parentheses. The validators themselves can be the broken signal.

This pushback re-shaped Q1: the question isn't vocabulary, it's whether the module is sophisticated enough to *distinguish* validator-broken from model-error. The 0075 cascade already encodes that discrimination. Implication: align with 0075 vocabulary because the cascade IS the discrimination architecture, not just a label set. Confirmed via Q3 answer (triangulator architecture).

### Q2: Calibration target
**Q:** Is ac-02 (≥60% non-unknown at confidence ≥0.7) a floor (optimise upward) or a target (calibrate to that exact corner)?
**A:** **60%/0.7 is a floor — optimise upward (Recommended).** Hit the AC, then push higher-precision subset at higher confidence. Empirical sweep against harvest_pool determines the achievable corner. Spec progress reports both numbers.

### Q3: Architectural premise
**Q:** Given validators are themselves error-prone, what's the inference module's stance toward the model's prediction — from-scratch type-finder, triangulator over multiple signals, confirm-or-deny, or validator-audit-first?
**A:** **Triangulator over multiple signals (Recommended).** Fuse validator pass-rate + generator-shape + header signal + sibling-context. Use model's prediction as structural prior. Output mechanism distinguishes validator-broken / subtype-drift / model-error. Naturally aligns with MADR 0075's cascade vocabulary.

### Q4: Validator authority
**Q:** When the module concludes the validator (not the model) is the problem, what authority does it have?
**A:** **Read-only signal: emit mechanism = `validator_widening`, no automated fix (Recommended).** Module flags the case. Cycle worker logs it. Human review (or future bead) decides whether to amend the validator. Stays inside the contract's halt/escalation framework via E04 (taxonomy amendment) which is already manual-attended.

### Q5: Fallback policy
**Q:** When no taxonomy validator scores well on a column, what does the module emit — generic fallback, strict unknown, or cascade?
**A:** **Generic fallback with low confidence (Recommended).** Emit `representation.text.string` (or closest generic) at confidence ~0.3-0.5. Rationale: "unknown" should mean "I cannot read this column" (empty/null/garbage), not "I read it but no canonical type fits". Keeps E01a viable for genuinely-novel shapes.

### Q6: B02 scope
**Q:** Does this bead change MADR 0078 policy on B02-candidate logging, or stay narrowly on B01/B04?
**A:** **Stay on B01/B04 only (Recommended).** Inference module fills `inferred_correct_type` for entries the cycle worker already logs. Does not change which branches log. B02-candidate logging is a separate decision deserving its own card.

### Q7: Phase 1 signals
**Q:** Which of the four triangulator signals must Phase 1 ship — validator pass-rate, header-name, generator-shape, sibling-context — to satisfy ac-02?
**A:** **Validator pass-rate + header-name (Recommended).** Cheap pair. Enough for the Sex/timezone/order_id/status cases. Fits the <100ms budget easily. Generator-shape and sibling-context become Phase 2 levers if 60%/0.7 isn't met empirically.

---

## Summary

### Goal

Ship a deterministic, autonomous type-inference module that fills `failure_log.tsv`'s `inferred_correct_type` column with a meaningful taxonomy ID (or principled fallback) plus a mechanism tag, so that E01a in the 2026-05-10 GitTables 90% round-trip contract can fire on real (predicted, inferred) pair-distinctness instead of degenerating to "20 distinct predicted types".

### Constraints

- **Deterministic.** Same inputs → byte-identical output (ac-04).
- **<100ms per column on M1**, median over 1000 columns (ac-05).
- **Phase 1 signals limited to validator pass-rate + header-name match.** Defer generator-shape and sibling-context unless calibration sweep shows they're needed to hit ≥60%/0.7.
- **Read-only on the taxonomy.** Inference may flag `validator_widening` as a mechanism but does not auto-amend definitions. E04 remains human-attended.
- **Stay on B01/B04 logging surfaces.** Do not extend to B02-candidates in this bead. MADR 0078 policy unchanged.
- **Mechanism vocabulary aligned with MADR 0075's cascade buckets** (`format_diversity`, `code_vs_canonical`, `enum_overfit`, `misclassification`) extended with inference-specific labels (`validator_widening`, `prediction_confirmed`, etc.) where the cascade vocabulary doesn't cover the case.
- **Triangulator architecture, not from-scratch type-finder.** Use the model's prediction as a structural prior; output mechanism explains agreement or disagreement with that prior.
- **Generic fallback policy.** When no validator scores well, emit `representation.text.string` at low confidence (≈0.3-0.5). Reserve `unknown` for genuinely unreadable columns.

### Success Criteria

- ac-01: spec doc names "triangulator over validator pass-rate + header-name match (Phase 1)" as the candidate approach
- ac-02: ≥60% non-unknown `inferred_correct_type` on harvest_pool's 21,789 B01/B04 entries at confidence ≥0.7 — **this is a floor, not a target**; report achievable (non_unknown_rate, confidence) curve
- ac-03: B01 (lines 376-391) and B04 (lines 403-418) handlers in `scripts/cron_cycle_work.py` call the module before failure_log append; mechanism tag recorded in existing `mechanism` column
- ac-04: deterministic — golden-input regression test in cycle preamble or unit-test layer
- ac-05: <100ms median per column, M1, over 1000-column benchmark
- ac-06: smoke test wired into `scripts/cron_preamble.sh` — module loads, sanity-infers one canned column, returns expected type

### Decisions Surfaced

These need MADR records during/after the spec phase:

- **D1 — Triangulator architecture for autonomous inference.** Chose multi-signal fusion (validator pass-rate + header + sibling + generator) using model prediction as prior, over (a) from-scratch validator-retrieval, (b) confirm-or-deny binary, (c) validator-audit-first split. Rationale: titanic/airports evidence shows validator pass-rate alone confuses validator-broken with model-error. Triangulator is the smallest architecture that distinguishes the three failure modes.
- **D2 — Read-only validator authority.** Inference module flags `validator_widening` as a mechanism but does not auto-amend definitions. Validator widening remains human-attended via E04 and tabletop. Deferred even with high-confidence widening proposals — risk of taxonomy drift outweighs throughput gain.
- **D3 — Mechanism vocabulary aligned with MADR 0075 cascade buckets.** Re-use `format_diversity` / `code_vs_canonical` / `enum_overfit` / `misclassification`. Extend with inference-specific labels (`validator_widening`, `enum_completeness`, `prediction_confirmed`, `subtype_drift`, `unknown_no_fit`) where 0075 doesn't cover. Avoids parallel-vocabulary drift.
- **D4 — Generic fallback policy.** `representation.text.string` at confidence ≈0.3 when no validator passes well. `unknown` reserved for empty/null/garbage columns.
- **D5 — Phase 1 ships validator-pass-rate + header-name only.** Generator-shape and sibling-context deferred. Empirical sweep determines whether Phase 2 is needed.
- **D6 — B01/B04 only; B02-candidate logging unchanged.** MADR 0078 policy stands.

### Implementation Notes

Means-level observations from codebase exploration — starting context for the implementing agent:

- **Append sites in cycle worker:** `scripts/cron_cycle_work.py` lines 376-391 (B01) and 403-418 (B04) are structurally identical — both build a failure_row dict with hardcoded `"inferred_correct_type": "unknown"` and `"mechanism": "value-shape"`. Refactor into a single `_infer_and_append_failure_row()` helper that calls the inference module.
- **Existing primitives in Rust:**
  - `crates/finetype-core/src/validator.rs` (48KB) — already loaded by the runtime as "240 validators cached, 6 with locale validators". Pre-compiled, ready to call.
  - `crates/finetype-core/src/taxonomy.rs` (55KB) — 240 type definitions, including labels usable for header-name match.
  - `crates/finetype-core/src/generator.rs` (306KB) — Phase 2 lever for generator-shape signal; not needed in Phase 1.
- **Surface recommendation:** Rust binary subcommand `finetype infer` reading JSON from stdin (`{column_name, predicted_type, samples, [siblings]}`) and returning JSON to stdout (`{inferred_correct_type, confidence, mechanism, signals: {...}}`). Cycle worker (Python) shells out per column.
  - **Latency caveat:** subprocess fork + JSON serialisation may breach 100ms on small columns. If empirical measurement shows it does, escalate to a long-lived `finetype infer-server` (Unix socket) started from `scripts/cron_preamble.sh`. Decide empirically — start with subprocess.
  - The validator runtime already loads in `finetype validate`; reusing the loader for `finetype infer` keeps cold-start cost amortised by the cycle preamble.
- **Header-match heuristic:** taxonomy label IDs (`identity.person.gender`, `datetime.offset.iana`) decompose into bigrams that match column-name normalised tokens. Lightweight string-distance + token-set Jaccard. No ML needed in Phase 1.
- **Validator pass-rate signal:** for each candidate type, run its validator over the observed samples; record pass count / sample count. Top-k candidates feed downstream scoring.
- **Score fusion:** weighted sum or rank-aggregated (Borda count) over the two phase-1 signals. Confidence = max-signal-agreement. Specifics determined by calibration sweep.
- **failure_log schema is additive-friendly.** Existing columns: cycle_id, timestamp, file_path, file_content_sha256, column_name, predicted_type, observed_values_sample, **inferred_correct_type** (gets filled), **mechanism** (gets richer vocabulary), confidence (NEW — append column).
- **Calibration corpus:** `eval/gittables/failure_log.tsv` (21,789 rows). Run inference module against the unlabelled rows, sweep confidence threshold, report (non_unknown_rate, threshold) curve. See Open Question §1 for ground-truth gap.
- **Smoke test (ac-06):** add a canned-input invocation to `scripts/cron_preamble.sh` after the existing model-load check. Input: a fixture column from `eval/datasets/csv/titanic.csv` (Sex). Expected output: `inferred_correct_type=identity.person.gender`, mechanism includes `enum_overfit` or `validator_widening`. Failing the canned input halts the cycle (H05 family).

### Open Questions

Intent-level only — implementation questions resolved in spec phase.

1. **Ground-truth gap for ac-02 measurement.** failure_log's 21,789 rows are unlabelled. Without a hand-labelled subset (200-500 entries), the calibration sweep can measure non_unknown_rate but not whether the inferences are *correct*. ac-02's "60% non-unknown at confidence ≥0.7" measures the module's decisiveness, not its accuracy. Spec phase must answer: does the bead's acceptance want decisiveness, or does it want labelled-precision? If the latter, scope expands to include labelling. Recommendation to surface in spec: hand-label 200 entries during implementation and report precision-on-labelled alongside non_unknown_rate-on-full.

2. **Confidence units across signals.** Validator pass-rate is a fraction in [0, 1]. Header-name match is a string-distance score in [0, 1]. They're not commensurate. The fusion weights and the meaning of "confidence ≥0.7" depend on the chosen aggregation. Recommendation: spec phase prescribes the aggregation (weighted sum vs rank-aggregation vs lexicographic) before the bead lands.
