# Spec Review

**Date:** 2026-05-04
**Reviewer:** Context-separated agent (fresh session)
**Bead:** finetype-7zi
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 1 |
| 2 — Assumption & failure | content signals (cron deployment, sidecar schema, smoke-gate halt) + Pass-1 HIGH | 2 |
| 3 — Adversarial | structural concern (smoke-test math depends on a heuristic value the spec's own heuristic cannot deliver) | 0 |

---

## Status of Prior Reviews' Findings

Re-reviewing v1.2 against `review-spec-2026-05-04.md` (v1.0→v1.1, 4 HIGH / 3 MEDIUM / 4 LOW) and `review-spec-2026-05-04-v2.md` (v1.1→v1.2, 1 HIGH / 4 MEDIUM / 1 LOW). Verified each of the v1.2-targeted findings:

- **v2 HIGH 1 (smoke-test math)** — RESOLVED at the math-walking layer. Default weights flipped to `w_v=0.4, w_h=0.6` (constraint line 27, ac-07 line 156). With `header_match=1.0`, `score(gender) = 0.4·0 + 0.6·1.0 = 0.6` ≥ smoke_min_confidence (0.5). Cascade Rule 2 (validator_widening) reachable per ac-08 conditions. Math is now traced in implementation_notes line 401. *But* the math assumes a `header_match` value the spec's own heuristic cannot produce — see HIGH 1 below; this is a re-emergence at a deeper layer.
- **v2 MEDIUM 1 (selection-rule contradiction)** — RESOLVED. Constraint line 25 dropped the "≥0.7 override" clause; ac-07 line 163 ("Inferred type is the argmax with lexicographic tie-break (ac-04)") is the single source of truth. Verified by `grep "override\|≥0.7"` returning no surviving conflicting clauses.
- **v2 MEDIUM 2 (soft calibration leak via labelled-precision rationale)** — RESOLVED. Constraint line 27 explicitly LOCKS weights and forbids implementing-agent weight selection against any subset. ac-07 lines 158-161 confirm "The implementing agent does not run a weight-selection sweep". ac-07 verification: "NO rationale block for weight 'selection' — the weights are locked." Leak structurally eliminated.
- **v2 MEDIUM 3 (Rule 10 unreachable)** — RESOLVED. ac-09 lines 209-214 make empty-samples a PRECONDITION before the cascade ("short-circuit with mechanism=`fallthrough`, ... Cascade does not run"). Cascade renumbered to 9 rules; ac-10 verification asserts the rule slice was not entered for empty-samples (line 284-285). ac-09 unit-test plan now includes one precondition test.
- **v2 MEDIUM 4 (sidecar join-key undercount)** — RESOLVED. ac-12 line 322-327 join key now `(cycle_id, file_path, file_content_sha256, column_name)` — strict superset of the prior recommendation, harmlessly preserves `file_path` for reporting. Includes the SHA so same-path-different-content collisions cannot occur.
- **v2 LOW (threshold collision)** — RESOLVED. Constraint line 28 names three named constants with three distinct numbers: `fallback_threshold = 0.4`, `smoke_min_confidence = 0.5`, `ac02_threshold = 0.7`. ac-02 line 53 references `ac02_threshold` by name; ac-06 uses `confidence ≥ 0.5` matching `smoke_min_confidence`; ac-10 cites `fallback_threshold (= 0.4)`. No reuse of the same number for different purposes.

All six v2 findings are addressed. The new HIGH below is a deeper layer of v2's HIGH 1 / v1's HIGH 2 — same fixture, same gate, same shape (smoke-test math contradicts spec's own heuristic), but now showing at the heuristic-implementation level rather than the score-fusion level.

---

## Findings

### [HIGH] Smoke-test math assumes `header_match(gender|"Sex") ≥ 0.834` but the spec's own heuristic cannot deliver it
**Category:** constraint-conflict
**Pass:** 3
**Description:** The v1.2 smoke-test math (implementation_notes line 401) walks `score(gender) = 0.4·0 + 0.6·1.0 = 0.6 ≥ smoke_min_confidence (0.5)` — and Rule 2 (validator_widening) gating requires `header_match for predicted ≥ 0.7` (ac-08 clause b, ac-09 Rule 2). Both depend on the column-header `Sex` matching the taxonomy label `identity.person.gender` at a score ≥ 0.834 (for ac-06's confidence floor) or at minimum ≥ 0.7 (for Rule 2 to fire). The header_match heuristic specified in implementation_notes line 396 is "tokenise column_name (snake_case → tokens, lowercase), tokenise the taxonomy label tail (`identity.person.gender` → tokens [identity, person, gender]), score by token-set Jaccard or weighted-overlap." For column `Sex` (tokens: `{sex}`) versus label tail tokens `{identity, person, gender}`, the token-set Jaccard is `|{} ∩ {}| / |union| = 0/4 = 0`. Plain weighted-overlap is also 0. Neither variant reaches 0.7, let alone 0.834. The smoke test will produce `score(gender) = 0.4·0 + 0.6·0 = 0`, fall to Rule 1 (`unknown_no_fit`, max_score < 0.4), emit `representation.text.string` at confidence 0.3 with mechanism `unknown_no_fit` — none of which is in ac-06's accept set, AND confidence (0.3) < smoke_min_confidence (0.5). Smoke gate halts the cycle on a correct module. This is the same shape as v1.1 review's HIGH 2 and v1.2 review's HIGH 1: the spec walks the math at one abstraction (score-fusion) but the underlying heuristic at the next layer down cannot produce the assumed input value. Note: the taxonomy DOES support `aliases` (verified at `crates/finetype-core/src/taxonomy.rs:234` and across `labels/definitions_identity.yaml`), but `identity.person.gender` has `aliases: null` (`labels/definitions_identity.yaml:601`) — there is no `sex → gender` alias in the taxonomy. Without that alias, no purely-lexical header heuristic can bridge `Sex` and `gender`.
**Evidence:** spec.yaml line 396 (heuristic = "token-set Jaccard or weighted-overlap" on label-tail tokens); spec.yaml line 401 (smoke math assumes `header_match=1.0`); ac-06 (confidence ≥ 0.5 floor); ac-08 clause (b) (header_match ≥ 0.7); `labels/definitions_identity.yaml:578-601` (gender block has `aliases: null`); `crates/finetype-core/src/taxonomy.rs:234` (TypeDef has aliases field — heuristic could consult it, but spec doesn't say to). Token sets `{sex}` ∩ `{identity, person, gender}` = ∅, so Jaccard and overlap are both 0.
**Recommendation:** Pick one of the following and walk the math through to ground truth in implementation_notes:
  (a) Augment the header-match heuristic to consult `TypeDef.aliases` (already loaded by `taxonomy.rs:234`) and add `sex` to `identity.person.gender`'s aliases in `labels/definitions_identity.yaml`. With `aliases: [sex]`, the column header `Sex` (token `{sex}`) hits the alias set, lifting `header_match` to 1.0. Cheap, scope-appropriate, and structurally re-usable beyond the smoke test.
  (b) Replace the canned smoke-test fixture with a column whose plain-token Jaccard *does* clear 0.7 — e.g. titanic's `Age` column (tokens `{age}` ∩ `{identity, person, age_years}` etc.) where token overlap is non-zero. Walk the math through the new fixture's expected outputs.
  (c) Lower ac-06's confidence floor and Rule 2's header_match gate to a value the bare token-Jaccard heuristic can deliver from `Sex` → `gender` (which is 0). This effectively defeats the gates' purpose; not recommended.
  (d) Pre-process column headers through a shared synonyms table (`sex` ↔ `gender`, `gender` ↔ `sex`, etc.) and document it as a Phase 1 sub-component. Scope creep relative to ac-11's Phase-1 lock; mention it as a one-line registry only.
Whichever option ships, the implementation_notes line 401 math walk MUST cite the heuristic mechanism that produces the `header_match` value used (e.g. "with aliases lookup hitting `sex → gender`, header_match=1.0").

### [MEDIUM] ac-12 sidecar verification doesn't enforce join-key uniqueness against multiple cycles
**Category:** test-gap
**Pass:** 2
**Description:** ac-12 line 322-327 asserts row count of `inference_signals.tsv` equals row count of `failure_log.tsv` "for that cycle_id" after the integration cycle, joined on `(cycle_id, file_path, file_content_sha256, column_name)`. Three concerns: (1) the assertion is per-cycle, but the sidecar is append-only across all cycles — equal row counts within one cycle don't catch a pattern where the sidecar misses rows from cycle N but accidentally includes duplicate rows from cycle N+1, masking a defect. (2) The join doesn't assert the natural unique key actually IS unique — a duplicate-row defect (e.g. helper called twice for the same column due to retry logic) would inflate the sidecar count without breaking the equality assertion if both sides duplicate. (3) "row-count equality on both sides" is what's asserted, but a directional assertion (`every failure_log row has exactly one matching sidecar row`) is stronger than counting. With a partial join failure plus a phantom duplicate, counts can match by coincidence.
**Evidence:** ac-12 lines 320-327 ("After the integration cycle, every B01/B04 row in failure_log.tsv has a corresponding row in inference_signals.tsv keyed by `(cycle_id, file_path, file_content_sha256, column_name)` — verified by a DuckDB join in `progress.md` that asserts row-count equality on both sides").
**Recommendation:** Tighten ac-12 verification to (a) FULL OUTER JOIN with assertion that no row on either side has a NULL counterpart (catches missing rows in either direction); (b) assert `count(distinct (cycle_id, file_content_sha256, column_name))` equals the row count on each side independently within the cycle (catches phantom duplicates); (c) run the assertion across at least 2 cycles (catches cross-cycle drift). One-liner DuckDB query covers all three.

### [LOW] Score determinism may leak through f64 rounding when used for argmax tie-break
**Category:** test-gap
**Pass:** 2
**Description:** Constraint line 19 specifies "byte-identical output (taxonomy ID, confidence rounded to 4 decimal places, mechanism, signal subscores)" — output is rounded. ac-04 verifies byte-identical stdout on rerun. ac-04's lex-tie-break test constructs two types with "equal score". But if scores are computed in f64 and the argmax+tie-break happens BEFORE rounding, two scores that round to the same 4dp value (e.g. `0.60001` and `0.60002`) will not tie internally even though their persisted form is `0.6000` for both. Then iteration order over types determines which wins — and stable iteration order isn't specified. The byte-identical-rerun test (ac-04 path 1) will pass within a single deterministic build (same iteration order), but a future change (e.g. switching from `Vec<TypeDef>` to `HashMap<String, TypeDef>` for taxonomy) would silently break determinism. Implementation_notes line 398 says "closed-set tie-break on argmax: when two types have equal score, pick the lexicographically smaller taxonomy ID" — doesn't specify pre-rounding or stable-iteration. Minor for the immediate ship; potential time bomb.
**Evidence:** spec.yaml line 19 (rounding spec only on persisted output); spec.yaml line 398 (tie-break rule doesn't specify pre-rounding); ac-04 verification (rerun-byte-identical + synthetic equal-score test, but synthetic test has explicit equal scores so doesn't catch the f64-rounding-vs-tie-break gap).
**Recommendation:** Either (a) require scores to be rounded to 4dp BEFORE argmax/tie-break (`score_rounded = (score * 10000.0).round() / 10000.0`; argmax operates on rounded scores), so pairs that persist as the same value tie-break consistently; or (b) require iteration over types to be sorted lexicographically by taxonomy ID (deterministic regardless of internal storage), so iteration order is the tie-break for any near-tie. Both are one-line implementation choices; pick one and document in ac-04 verification.

---

## Honest Assessment

v1.2 cleanly resolves all six findings raised in v1.1's review. The locked weights eliminate the soft-calibration-leak entirely (the strongest fix in this iteration), the precondition / cascade re-baselining for empty samples is structurally sound, the threshold-token register is now correctly decoupled, the join-key includes the SHA, and the selection-rule contradiction is gone. These are all the right calls. **However**, the smoke-test math walk surfaces a deeper layer of the same recurring contradiction: the math now correctly traces `0.4·0 + 0.6·1.0 = 0.6`, but the assumed `header_match=1.0` requires synonymy that the spec's own heuristic ("token-set Jaccard or weighted-overlap" on label-tail tokens) cannot deliver for column `Sex` against label `identity.person.gender` (Jaccard = 0/4 = 0, since `{sex}` and `{identity, person, gender}` are disjoint). The taxonomy supports `aliases` but the gender entry has none. The implementer following the spec literally will produce `score(gender) = 0` and trip Rule 1 (`unknown_no_fit`), failing both the confidence floor and the mechanism accept-set. The fix is small (add `sex` to gender's aliases plus consult `TypeDef.aliases` in the heuristic, OR pick a different fixture, OR add a synonyms table) — but it must be picked in the spec, not left for the implementer to discover under H05 cycle-halt pressure. Resolve this third pass at the smoke-test math (HIGH 1) and tighten ac-12's join uniqueness (MEDIUM 1) and the spec is implementable. The pattern across three reviews suggests a deeper convention worth adopting: any concrete fixture with assumed numeric outputs in a spec MUST cite the heuristic mechanism that produces each input number, not just the score-fusion equation.
