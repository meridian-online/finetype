# Spec Review

**Date:** 2026-05-04
**Reviewer:** Context-separated agent (fresh session)
**Bead:** finetype-7zi
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 1 |
| 2 — Assumption & failure | content signals (cron deployment, sidecar schema, smoke-gate halt) + Pass-1 LOW | 0 |
| 3 — Adversarial | not triggered | — |

---

## Status of Prior Reviews' Findings

Re-reviewing v1.3 against `review-spec-2026-05-04-v3.md` (1 HIGH / 1 MEDIUM / 1 LOW). Verified each finding is structurally resolved:

- **v3 HIGH 1 (smoke math depends on heuristic value the heuristic cannot deliver).** RESOLVED. Fixture switched from titanic `Sex` → canonical `email` column (`column_name="email"` in ac-06 verification line 151, `samples=[alice@example.com, ...]` line 156). With this fixture the smoke gate is robust to the implementation_notes' Jaccard-vs-overlap ambiguity:
  - Header tokens `{email}` vs canonical email type's tail tokens `{identity, person, email}` (verified at `labels/definitions_identity.yaml:132` — the canonical type ID is `identity.person.email`, not the spec's hedged `identity.contact.email_address`).
  - Under Jaccard: `h = |{email}| / |{identity, person, email}| = 1/3 ≈ 0.333`. Score = `0.4·1.0 + 0.6·0.333 = 0.60`. Confidence ≥ smoke_min_confidence (0.5) ✓. Rule 8 (prediction_confirmed) fires because predicted==inferred AND validator_pass_rate=1.0 ≥ 0.7 ✓.
  - Under overlap-on-min: `h = 1/1 = 1.0`. Score = `0.4·1.0 + 0.6·1.0 = 1.0`. Both gates pass even more comfortably.
  - The fixture is structurally robust to the heuristic-formulation ambiguity, which is exactly what v1.3 was meant to deliver. The accept-set narrowing to `{prediction_confirmed}` (line 144) tightens correctness from "any of three plausible mechanisms" to a single deterministic expectation.
  - Validator-broken cases (titanic Sex / airports timezone) explicitly relocated to ac-13 labelled-eval and ac-02 held-out floor (lines 145-148), preserving coverage outside the load-and-correctness smoke gate.

- **v3 MEDIUM 1 (ac-12 join-key uniqueness undercount).** RESOLVED. ac-12 verification (lines 334-356) now combines all three remediations: (a) FULL OUTER JOIN with no-NULL assertion on either side; (b) `COUNT(*) == COUNT(DISTINCT (cycle_id, file_path, file_content_sha256, column_name))` uniqueness check; (c) per-cycle row-count match across 3 consecutive cycles, aligned with exit_conditions line 486's smoke-gate window. The 3-cycle window ensures cross-cycle drift (sidecar accidentally backfilling cycle N-1 from cycle N output) cannot pass undetected.

- **v3 LOW 1 (f64 determinism leak via round-after-argmax).** RESOLVED. ac-04 (lines 91-116) now mandates the three-step floating-point determinism contract: (i) lexicographic iteration of candidates at scoring time (not just at tie-break — line 98); (ii) round score to 4dp BEFORE argmax (line 100, "BEFORE the argmax comparison"); (iii) lex tie-break on equal-rounded-score (line 104). Test plan adds a 100-round stress test on a column whose raw scores differ by less than 1e-5 from another candidate (line 112) — exercising exactly the sub-ulp window the rounding eliminates.

All three v3 findings are structurally resolved. No regression of v1/v2 findings detected on re-walk.

---

## Findings

### [LOW] ac-06 verification hedges the canonical email type ID
**Category:** test-gap
**Pass:** 1
**Description:** ac-06 verification (line 152-155) writes `predicted_type matches the canonical email taxonomy ID (resolve at implementation time — most likely `identity.contact.email_address`; the implementing agent confirms the exact ID against `labels/definitions_identity.yaml` and writes the resolved ID into the smoke script as a literal)`. The hedge is unnecessary: the canonical ID is `identity.person.email` (verified at `labels/definitions_identity.yaml:132`, not `identity.contact.email_address` as the spec speculates). The taxonomy has no `identity.contact.*` namespace at all (verified by `grep '^identity\.' labels/definitions_identity.yaml` returning only `identity.person.*`, `identity.commerce.*`, `identity.medical.*`, `identity.government.*`, `identity.academic.*`). The implementing agent will resolve correctly because the spec instructs them to confirm against the file, but burning their resolution loop on a mis-hedge that names a non-existent namespace is friction the spec can eliminate.

The smoke math elsewhere in the spec (line 162-163, implementation_notes line 430) is type-ID-agnostic — it relies on the header `email` tokenising to `{email}` and the label tail to contain `email`, both of which hold for `identity.person.email`. So this is a documentation accuracy issue, not a math-correctness issue.
**Evidence:** spec.yaml line 153 (`most likely identity.contact.email_address`); `labels/definitions_identity.yaml:132` (`identity.person.email:` is the canonical entry, with `aliases: [email_address]` at line 153 — explaining the hedge: `email_address` is the alias, not the type ID); `labels/definitions_identity.yaml:30,69,100,132,164,...` (full namespace listing under `identity.person.*` not `identity.contact.*`).
**Recommendation:** Replace lines 152-155 with: `predicted_type="identity.person.email"` (resolved against `labels/definitions_identity.yaml:132`; aliases include `email_address` but the canonical type ID uses the `person` sub-namespace). Drop the "resolve at implementation time" hedge — the resolution is a one-line lookup the spec author has already done implicitly. This shrinks the implementing agent's smoke-fixture work from "look up + write literal + double-check namespace" to "use the literal".

---

## Honest Assessment

v1.3 cleanly resolves all three v3 findings. The smoke fixture switch from titanic Sex to canonical email is the strongest fix in this iteration: it dissolves three reviews' worth of recurring smoke-math contradictions in a single move, because the new fixture is robust to the implementation_notes' header-match-heuristic ambiguity (Jaccard=0.333 and overlap-on-min=1.0 both clear all the gates). The narrowed accept set `{prediction_confirmed}` plus the validator-broken cases relocated to ac-13/ac-02 is the right separation of concerns — smoke is load+correctness, deeper coverage lives where it's evaluable.

The ac-12 join verification with FULL OUTER JOIN + DISTINCT-count + 3-cycle audit catches all three failure modes the v3 finding raised. The ac-04 determinism contract with lex iteration + round-before-argmax + 100-round sub-ulp stress test is a tight floating-point determinism story; this is exactly the kind of contract that prevents "deterministic in dev, non-deterministic in prod when a future PR swaps Vec for HashMap" silent breakage.

The single remaining finding is documentation drift — the smoke verification hedges on a type ID that the spec author has already implicitly resolved, naming a namespace (`identity.contact`) that doesn't exist in the taxonomy. This is friction, not a correctness gap, and it's strictly LOW severity. The spec is implementable as-written; the implementing agent will resolve `identity.person.email` correctly via the file-confirmation instruction.

One out-of-scope observation worth flagging for the implementing agent (not blocking spec approval): the bead's stored `acceptance_criteria` field (`bd show finetype-7zi --json`) is stale relative to spec.yaml v1.3 — bead ac-06 still says "titanic Sex column, mechanism ∈ {enum_overfit, validator_widening, prediction_confirmed}" while spec.yaml v1.3 ac-06 says canonical email + accept set narrowed to `{prediction_confirmed}`. exit_conditions line 487 already names `bd update finetype-7zi --acceptance` as a deliverable; the implementing agent should run that update early so the deterministic gate-AC checker (which reads bead state, not spec.yaml) sees the v1.3 surface. This isn't a spec-review finding because the spec correctly itemises the bead update as an exit condition.

The pattern across four reviews: every iteration tightened a specific contract that's now load-bearing (locked weights, decoupled thresholds, FULL OUTER JOIN audit, round-before-argmax, lex iteration, 100-round determinism stress, 3-cycle sidecar audit). The spec is denser and more verifiable than v1.0, with no observed contradictions remaining. Approve and ship.
