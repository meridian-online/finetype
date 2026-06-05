# Spec Review

**Date:** 2026-06-05
**Reviewer:** Context-separated agent (fresh session)
**Spec:** 2026-06-05-gold-eval-anchor
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 0 |
| 2 — Assumption & failure | content signals (ground truth, eval datasets, training inputs, leakage firewall) | 1 |
| 3 — Adversarial | not triggered (no untestable ACs, no cascading structural failures) | — |

---

## What changed since cycle 1

This is the cycle-2 re-review. Cycle 1 returned REQUEST_CHANGES on three findings; all three are addressed in the current AC text (notes.jsonl confirms the revision intent, and the AC bodies bear it out):

- **[MEDIUM] value-tuple-hash under-catches same-column-different-sample leak** → **fixed.** ac-06 now mandates the guard key on `(file, column)` identity, not the value tuple. It names the exact defect it must avoid: "train_ydf.py's existing exclusion … hashes a column's sampled value TUPLE — so the same column sampled with a different window yields a different hash and slips past the filter." The guard is correctly re-grounded on the identity the fixture already carries (ac-03).
- **[MEDIUM] ac-06 had no standalone closure path; depended on B2** → **fixed.** The AC is split. ac-06 keeps the now-closable half (wire the fixture into `train_ydf.py`'s exclusion against today's corpus + a test asserting zero `(file, column)` overlap) and is `code`-typed, so it blocks close and gets real verification now. The B2-dependent cross-corpus audit is split out as **ac-07**, typed `observation` — it defers rather than blocks, exactly so it can't be vacuously ticked before the harvested corpus exists.
- **[LOW] >= 20-per-family floor unreachable for rare families** → **fixed.** ac-03 now requires that where N < 20, "the achieved count and the rarity rationale" are recorded in the fixture's provenance column "so the floor exception is auditable, not silent." The self-judging gap is closed with an audit trail.

Substrate cited by the revised ACs verifies: `scripts/train_ydf.py:84-98,133` is the `_labelled_eval_hashes` / `_value_hash` tuple-hash path the spec correctly identifies as the thing **not** to copy; `.orbit/specs/2026-05-04-autonomous-type-inference/labelled_eval.tsv` exists (87.8K) and is the existing leakage infra ac-03 extends; B2 (`2026-06-05-reference-data-inventory`) is still `open`, which is exactly why ac-07's deferral is correct.

---

## Findings

### [LOW] ac-02's two cited memory keys do not resolve

**Category:** assumption
**Pass:** 2
**Description:** ac-02 grounds the confusion-family scope in "memories v23-ac01-finding / v23-ac08-outcome". Neither key resolves via `orbit memory show` or `orbit memory search v23` — the search surfaces `charcnn-probe-latitude-cascade` and `charcnn-vs-multibranch-data-scaling` but not the two cited keys. This is citation drift, not a substantive gap: the confusion families themselves (msg_id → iso6346, stock_id → mgrs, team codes → country_code, exchange codes → country_code, plus the shared-shape numerics) are real, well-attested in the corpus-pass diagnostic, and the families — not the memory keys — are what the curator enumerates. The fixture's contents won't change because a citation is stale.

**Evidence:** `orbit memory show v23-ac01-finding` and `… v23-ac08-outcome` both return empty; `orbit memory search v23` does not list them. ac-02 text references both keys as scope evidence.

**Recommendation:** During implementation, either fix the two keys to the memories that actually carry the v23 cell-2 findings, or drop the parenthetical and cite `eval/gittables/corpus_pass/report.md` directly (which ac-03/ac-05 already lean on). Cosmetic — does not block, and the curator should not stall on it.

---

## Honest Assessment

Ready to implement. The cycle-1 leakage-guard risk — a guard that passes while leaking — was the one finding that genuinely threatened the spec's reason for existing, and the revision fixes it at the root: the guard now keys on column identity, so a gold column is excluded no matter how it is later sampled, and the audit that can only run after B2 exists is split into a deferrable observation rather than a blocker that would either stall the spec or get ticked vacuously. The independence contract stays correctly first and gated (ac-01), the families are real, and reusing the existing `labelled_eval.tsv` infra rather than forking a parallel one is the right discipline. The only remaining finding is a stale memory citation that changes nothing about what gets built.

In product terms: once YDF flips from judge to miner, scoring Sense against YDF-derived labels would be circular — Sense would pass by construction. This gold set is the independent tie-breaker for the handful of column types the two lenses fight over, and the leakage guard is now strong enough that "held out" actually means held out.

One-line for a stakeholder: the gold-set plan addressed every cycle-1 concern — the leakage guard now keys on column identity instead of sample values, the B2-dependent audit is split out as a non-blocking observation, and it's clear to implement; the only nit is a stale memory citation.
