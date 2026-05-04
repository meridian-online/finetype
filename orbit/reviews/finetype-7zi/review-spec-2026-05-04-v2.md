# Spec Review

**Date:** 2026-05-04
**Reviewer:** Context-separated agent (fresh session)
**Bead:** finetype-7zi
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 3 |
| 2 — Assumption & failure | content signals (cycle deployment, sidecar schema, calibration partition) + Pass-1 MEDIUM | 2 |
| 3 — Adversarial | structural concerns (selection-rule contradiction, smoke-test math contradiction, cascade unreachable terminal) | 1 |

## Status of Prior Review's Findings

Re-reviewing v1.1 against `review-spec-2026-05-04.md` (REQUEST_CHANGES, 4 HIGH / 3 MEDIUM / 4 LOW):

- **HIGH 1 (vocabulary token mismatch):** RESOLVED. ac-09 now lists the 7 rule-emitted tokens from MADR 0075 (verified against `orbit/decisions/0075-mechanism-bucket-coalesce.md:60-66`) plus 3 triangulator extensions. Constraint at line 24 explicitly forbids display roll-ups.
- **HIGH 2 (smoke-test mechanism contradiction):** PARTIALLY RESOLVED. Accept set widened to {`enum_overfit`, `validator_widening`, `prediction_confirmed`}, but a new contradiction emerges from the score-fusion math (see HIGH 1 below).
- **HIGH 3 (H08 schema-migration conflict):** RESOLVED. failure_log.tsv 9-col schema unchanged; new sidecar `inference_signals.tsv` is append-only from creation. Verified against contract `2026-05-10-gittables-90-percent-roundtrip.yaml:475-486` — H08 trigger is line-count drop, which a never-existed-before sidecar cannot trip.
- **HIGH 4 (calibration-on-test-set leak):** PARTIALLY RESOLVED. Bucket split by `file_content_sha256 MOD 2` is clean and deterministic, but ac-07's "deviations require a one-line rationale citing measure-half precision-on-labelled" re-introduces a soft leak (see MEDIUM 1 below).
- **MED 1 (bead-spec branch-label divergence):** RESOLVED. Bead `acceptance_criteria` now uses orbit convention with B01/B04 wording matching the spec.
- **MED 2 (latency benchmark protocol):** RESOLVED. ac-05 mandates production code path (subprocess-per-column or socket-round-trip-per-column) and records the pattern in `progress.md`.
- **MED 3 (labelling rubric):** RESOLVED. ac-13 requires `labelling_protocol.md` with four named rubric points; sampling seed and labeller attestation required.
- **MED 4 (mechanism extension triggers undefined):** RESOLVED. `enum_completeness` and `subtype_drift` dropped (verified by `grep` returning zero matches in spec.yaml). Each remaining triangulator-specific token (`validator_widening`, `prediction_confirmed`, `unknown_no_fit`) has an explicit cascade rule.
- **LOW 1 (unbounded sample assumption):** RESOLVED. ac-15 enforces N=8 truncation matching `OBSERVED_SAMPLE_LIMIT` (verified at `scripts/cron_cycle_work.py:80`).
- **LOW 2 (D6 redundant):** RESOLVED. Dropped from ac-14; only mentioned as "dropped as non-decision."
- **LOW 3 (gate AC descriptions clean):** STILL CLEAN. Re-verified deterministically: ac-04 (83 chars), ac-05 (84 chars), ac-06 (149 chars). All three gate ACs pass the non-empty / non-placeholder / ≥20-char check.
- **LOW 4 (bead acceptance prose):** RESOLVED. `parse-acceptance.sh acs finetype-7zi` now emits 15 ACs with correct gate markers on ac-04/05/06.

## Findings

### [HIGH] Smoke test cannot pass under default weights — score-fusion math contradicts the accept set
**Category:** constraint-conflict
**Pass:** 3
**Description:** ac-06 requires the canned titanic Sex inference to satisfy BOTH `confidence ≥ 0.5` AND mechanism ∈ {`enum_overfit`, `validator_widening`, `prediction_confirmed`}. ac-07 specifies default weights `w_v=0.6, w_h=0.4` and confidence = max-type-score. The titanic Sex column rejects 100% under the case-sensitive `identity.person.gender` enum (samples are lowercase `male`/`female`; canonical enum is `Male`/`Female`), so `validator_pass_rate(gender) = 0`. The maximum possible confidence for `identity.person.gender` under default weights is therefore `0.6·0 + 0.4·header_match ≤ 0.4`. Below the 0.5 threshold for both ac-06's confidence requirement AND Rule 9's no-fit threshold (ac-10), the cascade emits `unknown_no_fit` — which is NOT in the accept set. The smoke test halts the cycle (H05) on a *correct-by-the-spec's-rules* inference. This is a re-emergence of the prior review's HIGH 2, with the accept-set fix made but the underlying score-fusion math left unreconciled.
**Evidence:** ac-06 lines 128-131 (confidence ≥0.5 AND 3-element accept set); ac-07 lines 156, 158 (defaults w_v=0.6, w_h=0.4, confidence = max_type score); ac-10 line 252 (Rule 9 fires when no candidate scores ≥0.5); implementation_notes line 375 (enumerates the same Sex case as expected mechanism ∈ {`validator_widening`, `enum_overfit`, `prediction_confirmed`} — without checking the math). The cascade Rule 1 (`validator_widening`) per ac-08 clause (a) requires ≥50% reject (satisfied), clause (b) requires header_match ≥0.7 (plausibly satisfied), clause (c) requires `inferred == predicted` — which fails when the argmax flips to a different type whose validator passes any sample, since `validator_widening` candidate's own score is 0.4·header_match while a distractor type's score can be 0.6·any_pass + 0.4·any_match. Confidence `≥ 0.5` (ac-06) and the cascade reaching Rule 1, 2, or 7 are jointly unattainable under the spec's own default weights for this fixture.
**Recommendation:** Pick one and explicitly: (a) raise the smoke-test confidence threshold floor to 0.4 and add `unknown_no_fit` to the accept set, OR (b) override the score-fusion specifically for the smoke test (e.g. set the smoke test's weights to `w_v=0, w_h=1`), OR (c) replace the canned column with one whose validator passes — e.g. an all-uppercase `MALE`/`FEMALE` fixture or a column predicted as a different type whose validator passes. Whichever option ships, walk the math through the spec text so a future reader can verify the smoke test is reachable under the chosen weights. As written, ac-06 fails on a correct module.

### [MEDIUM] Selection-rule contradiction between constraint and ac-07
**Category:** constraint-conflict
**Pass:** 1
**Description:** Constraint line 25 says: "the inferred type is selected from the triangulator's argmax with deterministic tie-breaking (lexicographically smaller taxonomy ID wins), and the model's prediction is overridden only when triangulator confidence ≥0.7 on a different type." ac-07 line 159 says: "Inferred type is the argmax with lexicographic tie-break (ac-04)." These are different selection rules. Under the constraint, if the argmax is a different type than `predicted_type` AND its confidence is <0.7, the *predicted* type wins (override gated). Under ac-07, the argmax always wins regardless of agreement with the prediction. An implementer reading constraint-vs-AC sees two contradictory selection rules and an auditor cannot tell which `inferred_correct_type` should appear in failure_log.tsv for the same column.
**Evidence:** spec.yaml line 25 vs line 159. Compounds with ac-09 Rule 7 (`prediction_confirmed`) which assumes `predicted == inferred` is a possible cascade outcome — true under the constraint's "override gated" rule, less obviously true under ac-07's "always argmax" rule (which would make `prediction_confirmed` only fire when the argmax happens to equal the prediction by coincidence).
**Recommendation:** Pick one rule. Recommended: drop the override-at-≥0.7 clause from the constraint (line 25) and let ac-07's "always argmax with lexicographic tie-break" stand. Rationale: the cascade in ac-09 already discriminates `prediction_confirmed` (Rule 7) from `misclassification` (Rule 8) based on predicted-vs-inferred equality, so the override threshold is redundant. Alternatively, encode the override threshold explicitly into ac-07's score function (`if argmax != predicted AND argmax_score < 0.7: return predicted`) — but that materially changes the cascade math and the calibration sweep grid. Simpler to drop the constraint clause.

### [MEDIUM] Calibration-leak fix has a soft-peek hole — ac-07 deviation rationale cites measure-half labelled precision
**Category:** assumption
**Pass:** 2
**Description:** Constraint at line 29 says "the labelled eval (ac-13) is sampled from the held-out half only and measures precision-on-labelled, never weight selection." ac-07 lines 156-158 says "Default starting weights are `w_v=0.6, w_h=0.4`; deviations require a one-line rationale citing measure-half precision-on-labelled (ac-13)." These contradict. If a "rationale" can cite measure-half labelled precision, the implementer can sweep weights, observe measure-half precision at each (w_v, w_h), and deviate from defaults to whatever maximises measure-half precision — which IS weight selection on the held-out set. The leak is softer than the prior review's HIGH 4 (the FLOOR metric ac-02 is properly held out), but it survives in the labelled-precision side.
**Evidence:** spec.yaml line 29 (constraint forbids weight selection on measure half) vs line 157-158 (deviation rationale explicitly cites measure-half precision-on-labelled).
**Recommendation:** Decide which is authoritative. Recommended: replace ac-07's deviation rationale clause with "deviations require a one-line rationale citing **calibrate-half** non_unknown_rate at conf ≥0.7 (the calibration metric)." If labelled-precision must be the rationale source, then split the labelled set into a calibrate-labelled subset (rationale-allowed) and a measure-labelled subset (reporting only) — but this is more complex than just keying the rationale on the calibrate half.

### [MEDIUM] Cascade Rule 10 (`fallthrough`, no observed values) is unreachable from inside the cascade
**Category:** test-gap
**Pass:** 2
**Description:** ac-09 Rule 10 fires when "no observed values; inferred=unknown" — but Rules 1-9 all require evaluating `validator_pass_rate(type)` and/or `header_match(type)` for at least one candidate type. With `samples=[]`, validator pass-rate is undefined (0/0), which means either: (a) the rules short-circuit to Rule 9 (`unknown_no_fit`, score < 0.5 trivially), making Rule 10 unreachable; or (b) the implementer must add a precondition guard that emits Rule 10 BEFORE entering the cascade — but ac-09's "first-fire wins" priority makes Rule 10 last, not first. ac-10 line 252-256 verifies Rule 10 by asserting an empty-samples column produces `inferred=unknown, confidence=0.0, mechanism=fallthrough`. That assertion presupposes a precondition guard the cascade structure doesn't define. As written, an implementer following the cascade order will never emit `fallthrough`; they'll emit `unknown_no_fit` instead — and ac-10 will fail.
**Evidence:** ac-09 lines 234-236 (Rule 10 placed last, "no observed values; inferred=unknown"); ac-10 lines 254-256 (verification expects `mechanism="fallthrough"` for empty samples). Per MADR 0075's actual cascade (`orbit/decisions/0075-mechanism-bucket-coalesce.md:64`), `fallthrough` is "Rule 7: terminal unknown" — a true cascade-tail catch-all, not an empty-samples guard.
**Recommendation:** Either (a) restructure ac-09 to label Rule 10 as a Rule-0 precondition guard ("if `len(samples) == 0`, emit `fallthrough` and skip the cascade"), or (b) align with MADR 0075's semantics and move the empty-samples case into Rule 9's `unknown_no_fit` branch with a sub-condition (`unknown_no_fit` always fires for both no-fit AND no-values, with confidence 0.3 when there were values to score and confidence 0.0 when there weren't). Pick one; document the choice in D4's MADR.

### [MEDIUM] Sidecar join-key undercounts when the same column appears under different file_content_sha256
**Category:** test-gap
**Pass:** 2
**Description:** ac-12 verification asserts the sidecar row count equals failure_log.tsv row count after one cycle, joined on `(cycle_id, file_path, column_name)`. But failure_log.tsv's join-natural primary key includes `file_content_sha256` (column 4 of the 9-col schema) — not just `file_path`. Two different parquet files with the same column name and same file_path (e.g. re-uploaded with different content over time within the same cycle) would collide on the proposed join key. The cycle worker writes file_content_sha256 explicitly; the sidecar schema includes it (ac-12 line 284). The verification just doesn't use it. Minor — but the DuckDB one-liner in `progress.md` will report a false match (or miss) under file-content drift within a cycle.
**Evidence:** ac-12 lines 282-301 (sidecar schema includes `file_content_sha256` but verification join key omits it).
**Recommendation:** Tighten ac-12 verification join key to `(cycle_id, file_content_sha256, column_name)` — uniquely identifies the column-version-in-cycle, matches both schemas, and survives cross-cycle file mutation. file_path becomes a denormalised reporting column.

### [LOW] Constraint line 25's "≥0.7 override" duplicates ac-07's confidence definition
**Category:** missing-requirement
**Pass:** 1
**Description:** The constraint at line 25 introduces a "≥0.7 override" threshold for the prediction-vs-argmax selection rule. ac-07 separately uses "confidence ≥0.7" as the ac-02 floor threshold. Same number, two different uses. Future readers may conflate them. Cosmetic given MEDIUM 1 above already proposes dropping the override clause; flagging here in case the override survives MEDIUM 1's resolution.
**Evidence:** spec.yaml line 25 (override threshold) vs ac-02 line 50 ("confidence ≥0.7" as floor metric).
**Recommendation:** If the override clause survives, rename its threshold (e.g. `OVERRIDE_THRESHOLD = 0.6`) so it's not the same number as the floor. If it doesn't (recommended fix in MEDIUM 1), this finding is moot.

---

## Honest Assessment

v1.1 is a substantial improvement over v1.0 — three of the four prior HIGHs are cleanly resolved, all three MEDIUMs are addressed, and the LOWs are tidied up. The vocabulary alignment with MADR 0075's rule-emitted tokens, the deterministic SHA-bucket calibration split, and the sidecar-not-migration schema strategy are all the right calls. But the smoke-test fix patched the symptom (widening the accept set) without re-checking the score-fusion math — under default weights, the fixture column produces a confidence of at most 0.4 against an ac-06 floor of 0.5, so the smoke test halts the cycle on a correct module. Compounding that, the constraint block introduces a "≥0.7 override" selection rule that ac-07 doesn't honour (always-argmax), and the cascade's Rule 10 is structurally unreachable because the empty-samples case is checked nowhere before the rules evaluate. The biggest risk is the smoke-test math: as in v1.0, the bead's own exit gate will turn against the implementer under cycle-halt pressure. Resolve the smoke-test math (HIGH 1) and the two selection-rule / cascade-structure contradictions (MEDIUMs 1 and 3) and the spec is implementable; ship as-is and the implementing agent re-litigates them under H05 halt.
