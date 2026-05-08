# Spec Review

**Date:** 2026-05-04
**Reviewer:** Context-separated agent (fresh session)
**Bead:** finetype-7zi
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 4 |
| 2 — Assumption & failure | content signals (training-data, cycle deployment, schema migration) + Pass-1 MEDIUMs | 5 |
| 3 — Adversarial | structural concerns (vocabulary mismatch, smoke-test contradiction, schema/contract conflict, calibration leakage) | 2 |

## Findings

### [HIGH] Mechanism vocabulary doesn't match the MADR it claims to extend
**Category:** constraint-conflict
**Pass:** 1
**Description:** ac-09 defines its closed mechanism set as `format_diversity`, `code_vs_canonical`, `enum_overfit`, `misclassification` "(from 0075)". MADR 0075's actual rule-emitted closed enum has six labels: `enum_overfit`, `format_diversity_path_a`, `format_diversity_path_b`, `code_vs_canonical_path_a`, `code_vs_canonical_path_b`, `misclassification`. The unsuffixed `format_diversity` / `code_vs_canonical` are the 4-bucket *display* labels, not the labels the cascade actually emits per MADR 0075's "rule-owned trigger label" doctrine.
**Evidence:** `.orbit/choices/0075-mechanism-bucket-coalesce.md:60-66` enumerates the closed cascade. The 4-bucket form discussed at lines 50-56 is the display roll-up; lines 54-56 explicitly say "the trigger label is rule-owned — never inferred from the variant — this keeps the rules and the report column in lock-step." The spec's ac-09 collapses six tokens to four without naming the path suffix policy.
**Recommendation:** Decide explicitly: (a) align with the 6-label rule-emitted vocabulary (preferred for traceability — preserves the path_a/path_b discrimination); or (b) declare a deliberate divergence and amend MADR 0075 (or D3) to coalesce both vocabularies. Either way, ac-09's enum and the smoke-test assertion in ac-06 must reference the same canonical token set. As written, an implementer who reads ac-09 will emit `format_diversity` and a downstream `validate-corpus` consumer aligned to MADR 0075 will reject it.

### [HIGH] Smoke-test mechanism expectation contradicts the rules that produce mechanisms
**Category:** test-gap
**Pass:** 2
**Description:** ac-06 requires the canned titanic Sex column to produce mechanism ∈ {`enum_overfit`, `validator_widening`}, with non-zero exit halting the cycle (H05 family). But by ac-08's definition, `validator_widening` only fires when "no other type scores higher" than the predicted type — and `enum_overfit` (per MADR 0075 rule 1) requires `predicted == expected`. The Sex case has the validator rejecting 100% of `male`/`female` because the canonical enum is `Male`/`Female` (case-sensitive); a triangulator that case-folds, normalises, or just header-matches `gender` exactly is *expected* to score `identity.person.gender` highest. If the inferred type equals the predicted type, `enum_overfit` is correct; if a sibling gender variant scores higher, `validator_widening` is gated out by ac-08's clause (c) and the catch-all becomes `prediction_confirmed` or `subtype_drift` — neither of which is in ac-06's accept set. The smoke test will halt the cycle on the *correct* answer.
**Evidence:** ac-06 verification block (lines 102-107) hardcodes the two-element set. ac-08 (lines 130-139) defines `validator_widening` as requiring "no other type scores higher". MADR 0075 rule 1 (line 60) gates `enum_overfit` on `predicted == expected`. The titanic Sex column with header-match score ≈ 1.0 on `identity.person.gender` is exactly the corner where the score-tying rule of ac-07 ("argmax score") and ac-08's clause (c) interact.
**Recommendation:** Either (a) widen ac-06's accept set to {`enum_overfit`, `validator_widening`, `prediction_confirmed`} and cite which rule the implementer must use to tie-break, or (b) replace the canned column with one whose mechanism is unambiguous under ac-08's gating. As written, ac-06 is more likely to halt the cycle than to validate the module.

### [HIGH] ac-12 schema "additive" change collides with the H08 append-only contract
**Category:** constraint-conflict
**Pass:** 2
**Description:** ac-12 calls the `confidence` column add "additive" and "append-only contract (H08/H09) honoured". But adding a 10th column to an existing TSV requires either (a) a one-time migration that rewrites all 21,789 existing rows with `confidence=0.0` (which is a rewrite, not an append), or (b) a header-versioning scheme where pre-v2 rows are read with a missing column and post-v2 rows have it. ac-12's verification accepts (a) ("recommend `0.0` for pre-integration rows, with a one-time migration note in `progress.md`"). The contract's H08 trigger fires on "failure_log corruption" — there is no carve-out for an authorised one-time schema rewrite. Whether H08 considers a column-add a corruption event is undecided in the spec.
**Evidence:** Spec ac-12 verification (lines 196-201) describes a backfill. Contract `2026-05-10-gittables-90-percent-roundtrip.yaml` line 475-490 defines H08/H09 around schema integrity. The bead description's "Constraint: must be deterministic" is unrelated to the schema migration question.
**Recommendation:** The spec must either (a) explicitly amend H08 to allow a one-time, audited schema migration as part of this bead's exit, with the audit recorded in `progress.md`, or (b) carry pre-integration rows forward without backfill (i.e., the cycle worker reads "9 columns + optional 10th" and the inference module owns confidence only for new rows). Pick one and document. Don't leave the migration verb undefined — it is the bead's first contact with the contract's halt machinery.

### [HIGH] Calibration weights are chosen on the same set the floor is measured against
**Category:** assumption
**Pass:** 2
**Description:** ac-07 says weights `w_v` and `w_h` are "determined by the calibration sweep against `failure_log.tsv`". ac-02 measures the 60% non-unknown floor on the *same* `failure_log.tsv`. Since the implementer can sweep weights to maximise the count of rows clearing the 0.7-confidence threshold, the floor is hill-climbable on the same surface that defines pass/fail. ac-13's labelled eval is the only out-of-fold check, but the spec doesn't bind the chosen weights to performance on the labelled subset — only to performance on the full 21,789. This is a textbook calibration-on-the-test-set leak.
**Evidence:** ac-02 (lines 36-51) defines the metric on `failure_log.tsv`. ac-07 (lines 109-124) says weights are picked from the sweep on the same file. ac-13 (lines 203-219) introduces the labelled subset but doesn't gate weight selection on labelled-subset precision.
**Recommendation:** Either (a) lock the weights before the sweep (e.g., `w_v=0.6, w_h=0.4` baseline, justify deviations), or (b) require weights to be selected on a held-out partition — split `failure_log.tsv` into calibrate/measure halves with a documented seed, sweep weights on the calibrate half, report the floor metric on the measure half. The labelled subset (ac-13) is too small (200 rows, ≥30 types) to anchor weights against the 21,789-row floor; it serves precision-on-labelled, not weight selection.

### [MEDIUM] Bead acceptance text and spec ac-03 disagree on integration sites
**Category:** constraint-conflict
**Pass:** 1
**Description:** The bead's `acceptance_criteria` field reads `ac-03: B01/B02a branch handlers integrate the module`. The spec's ac-03 reads `B01 and B04 failure_log append sites`. The cycle worker (`scripts/cron_cycle_work.py`) does append failure_log entries from B01 (line 376-391) and B04 (line 403-418); the B02-candidate path at line 425 explicitly *avoids* appending to failure_log. The spec correctly tracks the code; the bead text is stale (it pre-dates the pass-2 contract's B02 split). But the bead is the spec's authority and an unresolved divergence is a contract bug.
**Evidence:** Bead `acceptance_criteria` field. `scripts/cron_cycle_work.py` lines 368, 391, 399, 418, 425. Contract `2026-05-10-gittables-90-percent-roundtrip.yaml` lines 725-727 ("B02 split into B02a_validator_narrow_detected... codifies pass-1 worker's invented B02-candidate"). Spec constraint at line 18 ("Stay on B01/B04 logging surfaces").
**Recommendation:** Either update the bead's acceptance text to read `B01/B04` (matches reality + spec + constraint), or the spec must justify why it diverges from the bead's stated AC. Suggest the former — bead-text update is one `bd update --acceptance` call. As-is, an auditor reading bead-vs-spec-vs-code sees three different trios of branch labels.

### [MEDIUM] Latency benchmark protocol is ambiguous about subprocess fork cost
**Category:** test-gap
**Pass:** 2
**Description:** ac-05 says "if invoked as subprocess-per-column, fork+JSON cost counts toward the budget" and prescribes `scripts/bench_infer.py`. But the verification doesn't specify whether the benchmark calls the subprocess once per row (counting fork cost) or invokes the binary once and times the inner loop (amortising fork). The cycle worker is the production-reality consumer and shells out per column — so the per-column subprocess form is the budget that matters. If the benchmark amortises, the ac passes while production breaches.
**Evidence:** ac-05 (lines 81-94). Implementation_notes line 240: "Cycle worker (Python) shells out per column. If subprocess fork breaches ac-05, escalate to long-lived `finetype infer-server`."
**Recommendation:** Tighten ac-05 to: "Benchmark MUST invoke the same code path the cycle worker uses (one subprocess invocation per column for subprocess form; one socket round-trip per column for server form). The benchmarked invocation pattern is recorded in `progress.md`."

### [MEDIUM] Labelled-eval ground-truth provenance is unspecified
**Category:** missing-requirement
**Pass:** 2
**Description:** ac-13 produces a 200-row hand-labelled eval subset with `truth_inferred_type` columns. But the spec doesn't say *who* labels, *against what reference*, *with what tie-break protocol*, or *what to do when the labeller is uncertain*. Without a rubric, "precision-on-labelled" becomes whatever the implementing agent says it is — and the metric loses value as a calibration check on the full-corpus floor.
**Evidence:** ac-13 (lines 203-219). The verification block requires the file to exist and have ≥30 distinct predicted_types, but says nothing about labelling provenance.
**Recommendation:** Add to ac-13 verification: "(a) labelling rubric documented in `labelled_eval.tsv` header comment or sibling `labelling_protocol.md`; (b) labeller name + date per row OR a single attestation block; (c) protocol for rows where the column has no clear canonical type (resolution: emit `unknown` as truth, with reason). Sampling seed already required."

### [MEDIUM] ac-09 enum extensions don't all have a defined trigger condition
**Category:** test-gap
**Pass:** 2
**Description:** ac-09 enumerates `validator_widening`, `enum_completeness`, `prediction_confirmed`, `subtype_drift`, `unknown_no_fit` as inference-specific extensions and requires "each new label has a docstring naming when it fires". But only `validator_widening` (ac-08) and `unknown_no_fit` (ac-10) have explicit trigger rules in the spec. `enum_completeness`, `prediction_confirmed`, and `subtype_drift` are named but not gated. An implementer can satisfy ac-09's verification (closed enum + emitted label is in set) without ever firing those three labels — silently leaving them dead.
**Evidence:** ac-08 (defines `validator_widening`), ac-10 (defines `unknown_no_fit`), ac-09 (line 144-156 lists all five but defers trigger definition to "docstring").
**Recommendation:** Either (a) add ACs that define when `enum_completeness`, `prediction_confirmed`, `subtype_drift` fire (mirroring ac-08), or (b) drop those three from the closed enum until Phase 2 needs them. As-is they're vocabulary debt — defined enough to pass ac-09 but not enough to be testable.

### [LOW] Latency cost model assumes a fixed sample bound the spec doesn't enforce
**Category:** assumption
**Pass:** 2
**Description:** Implementation_notes line 245 derives the ~25ms per-column estimate from "240 validator regex/enum runs × 8 sample values = 1920 regex evaluations". But the spec's input shape (`samples`) is unbounded; the cycle worker's `OBSERVED_SAMPLE_LIMIT` is set elsewhere. If the inference module receives 100 samples per column (reasonable for high-cardinality columns), the estimate becomes ~250ms — exceeding the 100ms budget without subprocess overhead.
**Evidence:** spec implementation_notes line 245. `scripts/cron_cycle_work.py` line 76 (TRIVIAL_FRACTION_FLOOR) — sample limits are constants but not surfaced into the spec.
**Recommendation:** Add a constraint: "Inference module truncates `samples` to N=8 (or the cycle worker's OBSERVED_SAMPLE_LIMIT, whichever is smaller) before validator scan. N is the same for the calibration sweep, the smoke test, and production." Otherwise ac-05 passes on benchmark distributions but breaches on real cycles.

### [LOW] D6 MADR is unnecessary
**Category:** missing-requirement
**Pass:** 1
**Description:** ac-14 requires six MADRs. D6 ("B01/B04 only; B02-candidate logging unchanged") is a non-decision: it preserves the existing MADR 0078 policy. A MADR records a *change*; "we didn't change anything" is not a decision worth a record. Drafting it produces a no-op artefact that future readers will encounter as noise.
**Evidence:** Spec D6 (line 99 of interview.md, ac-14 verification line 232-234). MADR 0078 already covers B02-candidate logging policy.
**Recommendation:** Drop D6 from the MADR list. The spec's constraint on line 18 ("Stay on B01/B04 logging surfaces. Do not extend logging to B02-candidates in this bead. MADR 0078 policy stands.") is sufficient — it cites MADR 0078, which is the existing policy artefact. Update ac-14 to require five MADRs (D1-D5).

### [LOW] Gate AC descriptions all pass deterministic checks
**Category:** test-gap
**Pass:** 1
**Description:** All three gate-typed ACs (ac-04, ac-05, ac-06 — `ac_type: gate`) have non-empty, non-placeholder, ≥20-character descriptions. Pass 1's gate-AC description check passes deterministically. (Recorded for completeness — this is a clean spot in the spec.)
**Evidence:** ac-04 line 71-79, ac-05 line 83-94, ac-06 line 98-107.
**Recommendation:** None. This is a positive finding.

### [LOW] Bead acceptance field uses prose, not orbit AC convention
**Category:** missing-requirement
**Pass:** 1
**Description:** The bead's `acceptance_criteria` field is prose-formatted (`ac-01: ...` per line) rather than the orbit convention (`- [ ] ac-NN [gate]: description`). `parse-acceptance.sh acs finetype-7zi` returns no output (silent skip). Downstream tooling that depends on the parser (gate-block enforcement in `/orb:implement`, AC checking) will see zero ACs and behave as if all are checked. The spec.yaml carries the real ACs — but the bead is the runtime gate substrate.
**Evidence:** `bd show finetype-7zi --json` acceptance_criteria field. `parse-acceptance.sh` empty output for this bead. Spec metadata (line 297-309) references the bead but doesn't normalise the acceptance field.
**Recommendation:** Before implementation, run `bd update finetype-7zi --acceptance` with the orbit-convention version of the AC list (14 lines, one per spec AC, with `[gate]` markers on ac-04/05/06). Otherwise `/orb:implement`'s gate enforcement is silently disabled for this bead.

---

## Honest Assessment

The spec is well-considered and shows real engagement with the problem — the triangulator framing, the read-only validator authority, and the labelled-eval recognition of the decisiveness-vs-precision gap are all genuine architectural calls. But four substantive issues will bite during implementation: the mechanism-vocabulary token mismatch with MADR 0075 (HIGH), the smoke-test mechanism expectation contradicting its own gating rules (HIGH), the schema-migration ambiguity against the H08 append-only contract (HIGH), and the calibration-on-the-test-set leakage in weight selection (HIGH). The biggest risk is the smoke test: as written, it will halt the cycle on a *correct* inference because the accept-set doesn't include the mechanism the rules will actually emit on the canned input. That turns the bead's exit gate against itself. Resolve the four HIGHs and the spec is implementable; ship as-is and the implementing agent will re-litigate them under cycle-halt pressure.
