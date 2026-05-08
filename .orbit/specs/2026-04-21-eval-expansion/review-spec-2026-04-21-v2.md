# Review: eval-expansion spec (Phase A+B) — v1.1

**Spec:** `.orbit/specs/2026-04-21-eval-expansion/spec.yaml` (v1.1)
**Reviewer:** Nightingale (fork session, context-separated)
**Date:** 2026-04-21
**Passes run:** 1 (Structural) + 2 (Assumption/failure) — Pass 3 not triggered

---

**Verdict:** APPROVE

**Findings by severity:** 0 blocking, 0 high, 2 medium, 2 low.
v1.1 cleanly resolves every blocker and every high from v1.0. The
additions (ac-15 consumer patching, constraint #10 carve-out, pinned
floors in ac-01, diagnostic-only framing in ac-12) are concrete,
verifiable, and do not introduce new structural risk. The remaining
mediums are refinements, not blockers.

---

## Resolution of v1.0 findings

```
| Finding | Severity | Status            | Rationale                                                                                                                                     |
|---------|----------|-------------------|-----------------------------------------------------------------------------------------------------------------------------------------------|
| F-B1    | blocking | FULLY_RESOLVES    | ac-15 inventories consumers (profile_eval.sh:78,148 named), mandates patching before schema change lands, and adds ±1 baseline regression.     |
| F-B2    | blocking | FULLY_RESOLVES    | Constraint #10 introduces `synthetic-necessary` provenance carve-out; ac-04 and ac-09 name the carve-out table. 100% coverage now achievable.  |
| F-B3    | blocking | FULLY_RESOLVES    | ac-01 pins concrete default floors (null_rate ≤ 0.20, unique_ratio bands, entropy, top-1 skew); ac-09 requires a floors table, ac-04 refs it.  |
| F-H1    | high     | FULLY_RESOLVES    | ac-04 now mandates `provenance_status ∈ {real, hand-curated}` AND pass_floors=true per row; carve-out is the only exception.                   |
| F-H2    | high     | FULLY_RESOLVES    | ac-10 explicitly enumerates format-drift, header-synonym, and whitespace blind spots under Consequences/Bad; verified by section check.        |
| F-H3    | high     | FULLY_RESOLVES    | ac-12 requires per-row `previously_covered`/`newly_covered` tag AND separate subset scores for each slice.                                     |
| F-H4    | high     | FULLY_RESOLVES    | ac-08 now encodes "eval keeps; training relocates"; unresolvable cases are listed in progress.md with replacement plan.                         |
| F-H5    | high     | FULLY_RESOLVES    | ac-12 declares v16 re-score diagnostic-only and names v18's true baseline as the first filter-trained model.                                   |
| F-M1    | medium   | FULLY_RESOLVES    | ac-14 adds a day-5 checkpoint with on-track-or-escalate outcome.                                                                               |
| F-M2    | medium   | FULLY_RESOLVES    | Constraint #11 pins MADR-proposed → code → MADR-accepted ordering and ties it to ac-01/07 start gates.                                         |
| F-M3    | medium   | FULLY_RESOLVES    | ac-02 requires SPDX or allowlist membership; `eval/licence_allowlist.txt` committed; validator checks membership.                              |
| F-M4    | medium   | FULLY_RESOLVES    | ac-05 verification adds file-exists AND ≥ 5 non-null values per (dataset, column).                                                             |
| F-L1    | low      | UNRESOLVED        | `FINETYPE_CI_MODEL` still not mentioned in ac-13; minor doc gap.                                                                                |
| F-L2    | low      | PARTIALLY_RESOLVES| ac-03 adds `gt_label_change` flag; constraint line 17 still reads as a contradiction in letter but the flag handles the substance.             |
| F-L3    | low      | UNRESOLVED        | ac-12's verification does not cite which env var (`FINETYPE_MODEL`) to use for the v16 re-score; decision 0049 post-mortem warned about this.   |
```

Resolution counts: **12 FULLY_RESOLVES / 1 PARTIALLY_RESOLVES / 0 NEW_PROBLEM / 2 UNRESOLVED** (both low).

---

## Pass 1 — Structural scan (v1.1)

### AC testability per-row

```
| AC     | type   | testable | change since v1.0                                           |
|--------|--------|----------|-------------------------------------------------------------|
| ac-01  | code   | yes      | stronger — pre-screen_floors.yaml deliverable + defaults    |
| ac-02  | config | yes      | stronger — SPDX allowlist + licence_allowlist.txt committed |
| ac-03  | doc    | yes      | unchanged from v1.0 (adds gt_label_change flag)             |
| ac-04  | code   | yes      | was weak → strong: provenance + pass_floors + carve-out     |
| ac-05  | gate   | yes      | stronger — file-exists + ≥5 rows                            |
| ac-06  | code   | yes      | unchanged                                                    |
| ac-07  | code   | yes      | sharper — removal count captured in progress.md             |
| ac-08  | doc    | yes      | stronger — resolution rule pins outcomes                    |
| ac-09  | doc    | yes      | stronger — floors table + carve-out table required          |
| ac-10  | doc    | yes      | stronger — known blind spots enumerated                     |
| ac-11  | doc    | yes      | unchanged                                                    |
| ac-12  | gate   | yes      | was weak → strong: diagnostic-only + dual subset scores     |
| ac-13  | config | yes      | unchanged                                                    |
| ac-14  | doc    | yes      | stronger — day-5 checkpoint                                 |
| ac-15  | code   | yes      | NEW — consumer inventory + ±1 regression gate               |
```

All 15 ACs are now testable with concrete verification. Gate ACs (ac-05, ac-12) both clear the deterministic-verification bar.

### Constraint conflicts

Constraints #10 (carve-out) and #11 (MADR ordering) are both non-trivial; neither conflicts with the existing nine. #10 is narrow (restricted-registry only) and gated through an explicit table in ac-09 — it does not become a side-door for "synthetic because convenient". #11's "proposed before code, accepted after verify" cycle matches the project's decision-register convention.

### Scope vs goal alignment

Unchanged — still cleanly Phase A+B with C explicitly out of scope. Carve-out does not expand scope; it handles a known impossibility.

### Content signals (triggers Pass 2)

Same as v1.0: training pipeline, eval schema, cross-system boundaries. Pass 2 runs.

---

## Pass 2 — Assumption & failure analysis

### Re-examining v1.0 assumptions

**A1 (consumer breakage).** Resolved by ac-15. The bash-read-folds-fields mechanism is explicitly named; the remediation is gated by a ±1 baseline regression. Verified by reading `eval/profile_eval.sh` — lines 78 and 148 are exactly as described in ac-15.

**A2 (row-hash blind spots).** Resolved by ac-10 blind-spot enumeration. The filter's limits are now written down before it ships, not after someone notices.

**A3 (100% coverage).** Resolved by constraint #10 + synthetic-necessary provenance. Decision 0050 (which I read to confirm) is a *training* policy, not an eval policy, but the spec reasonably treats restricted-registry constraints as binding for both — the carve-out names CPT and SSN explicitly, matching 0050's spirit.

**A5 (contaminated baseline).** Resolved by ac-12's diagnostic-only framing. The v18 promotion baseline is now correctly defined as "the first model trained with the filter active."

**A6 (sources.yaml retroactive roles).** Resolved by ac-08's "eval keeps; training relocates" rule. Deadlock broken.

**A7 (undefined floors).** Resolved by ac-01's default values + ac-09's floors table. A future session can run ac-04's verification without inventing floors on the spot.

**A8 (MADR post-hoc).** Resolved by constraint #11 ordering.

### New assumptions introduced in v1.1

**NA1. `synthetic-necessary` is not a loophole.**
*Failure mode:* A future sprint finds sourcing hard for a non-restricted type and files it under `synthetic-necessary` anyway. *Mitigation in spec:* constraint #10 says "only authoritative source is behind a restricted registry" and requires a named entry in ac-09's carve-out table with rationale. Auditability is in place. Low risk.

**NA2. The ±1 column regression bar is appropriately tight.**
*Failure mode:* A real silent-parse bug moves the score by exactly 1 column and passes. *Mitigation:* unlikely on a 242-column corpus — a bash read misparse would fold 3 extra fields into every gt_label, moving the score dramatically, not by one. The ±1 tolerance is for spurious noise (e.g. timestamp-family re-classification), not for hiding a parse bug. Acceptable.

**NA3. MADR "proposed" state gates code start.**
*Failure mode:* The "proposed" MADR is thin — just a skeleton — and rubber-stamps the code anyway. *Mitigation:* ac-09 requires the floors table populated before `accepted`, and ac-04's verification depends on those floors. The cycle holds if ac-04's verification is run honestly. Low-to-medium residual risk, flagged below as F-M5.

### Failure-mode deltas

- **ac-02** — previously weak on licence format, now validates against `eval/licence_allowlist.txt`. Good. New risk: the allowlist file is committed but its curation process isn't specified (who approves additions?). Minor — see F-M6.
- **ac-05** — previously paper-only, now checks file-exists + ≥5 rows. Good. The threshold of 5 is lower than the profile eval's typical sample size (100); acceptable for a coverage floor.
- **ac-12** — previously narrative-only, now structured with a boolean flag and dual subset scores. Strongest of the high fixes.

---

## New findings (v1.1-specific)

### Medium

**F-M5 — MADR-proposed state is load-bearing but underspecified.**
Constraint #11 requires MADRs in `proposed` status before code ACs start. The spec does not say what a minimal `proposed` MADR contains. If a skeleton MADR with TBD sections qualifies, constraint #11 becomes ceremony. Recommend: proposed-state MADRs must contain at least the Context and Considered Options sections (not just a title). Not blocking — ac-09 separately requires the floors and carve-out tables before `accepted`, which is the load-bearing artefact.

**F-M6 — `eval/licence_allowlist.txt` curation process is unspecified.**
ac-02 treats the allowlist as committed state but does not say how additions are reviewed (e.g. PR approval, decision register). Given that the point of the allowlist is to prevent ad-hoc free-form licence strings, the bar for adding entries should be stated. Recommend a single line in ac-02 or ac-09. Low-cost fix.

### Low

**F-L4 — `FINETYPE_CI_MODEL` still absent from ac-13.**
Carried over from v1.0's F-L1. ac-13 explicitly lists `FINETYPE_MODEL` and `FINETYPE_MODEL_DIR` as untouched but doesn't mention `FINETYPE_CI_MODEL`. Given the recent CI-decoupling work (PR #39), worth one line for completeness.

**F-L5 — ac-12 does not cite the env var for the v16 re-score.**
Carried over from v1.0's F-L3. Decision 0049 flagged a `FINETYPE_MODEL_DIR` vs `FINETYPE_MODEL` mix-up that caused an eval run to test the wrong model. ac-12's verification should name `FINETYPE_MODEL=models/sherlock-v16` (or equivalent) explicitly.

---

## Summary

v1.1 is a strong revision. Every blocker and every high from v1.0 is
genuinely resolved — the fixes are specific, testable, and avoid the
common failure mode of waving at a concern without pinning a
verification. No new blockers emerge; the new findings are minor
refinements.

Implementation may proceed. The two lows and two mediums above can be
folded in during implementation (or rolled into a v1.2 mini-revision)
without blocking start. The sprint-focus gate says: ship it.

---

## Appendix — files inspected

- `.orbit/specs/2026-04-21-eval-expansion/spec.yaml` (v1.1)
- `.orbit/specs/2026-04-21-eval-expansion/review-spec-2026-04-21.md` (v1.0 review)
- `.orbit/specs/2026-04-21-eval-expansion/interview.md`
- `eval/profile_eval.sh` (lines 70–160 — confirmed the bash read-fold mechanism named in ac-15)
- `.orbit/choices/0050-per-type-sourcing-policy.md` (confirmed carve-out is consistent with 0050's spirit)
