# Spec Review

**Date:** 2026-06-05
**Reviewer:** Context-separated agent (fresh session)
**Spec:** 2026-06-05-gold-eval-anchor
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 1 |
| 2 — Assumption & failure | content signals (ground truth, eval datasets, training inputs, leakage firewall) | 3 |
| 3 — Adversarial | not triggered (no untestable ACs or cascading structural failures) | — |

---

## Findings

### [MEDIUM] ac-06's leakage guard inherits train_ydf's column-tuple hashing, which under-catches partial-sample overlap

**Category:** failure-mode
**Pass:** 2
**Description:** ac-06 says the gold columns are "hash-excluded from any training/mining corpus (value-hash filter, mirroring train_ydf.py's labelled_eval exclusion)" and that an audit "asserts zero overlap between the gold fixture and the B2 harvested corpus." The mechanism being mirrored — `_labelled_eval_hashes` / `_value_hash` in `scripts/train_ydf.py` (lines 84–98, 133) — hashes the *whole column's selected sample tuple*. Two facts make this a weak guard for the gold set:
1. The hash matches only when the *same set of sample values, in the same truncation* appears in both corpora. The B2 mining factory and the gold curation will almost certainly draw different sample windows from the same source column. Same column, different sample slice → different hash → no exclusion. The "zero overlap" the audit asserts would be true while the column itself leaks.
2. The independence contract (ac-01) is about *label provenance and shared inputs*, not value identity. A gold column can be value-disjoint from the corpus yet still be circular if its label came from a YDF-adjacent source. ac-06's value-hash audit cannot detect that — it only mechanically enforces value non-overlap, not provenance independence.

**Evidence:** `scripts/train_ydf.py:84–98` hashes `observed_values_sample` per row; `:133` excludes on tuple-hash equality. ac-06 text: "value-hash filter, mirroring train_ydf.py's labelled_eval exclusion" and "asserts zero overlap … once that exists."

**Recommendation:** Make ac-06 enforce overlap at the *column-identity* grain the gold fixture already carries (ac-03 mandates `file` + `column`), not (or in addition to) the value-tuple grain. The audit should assert no `(file, column)` from the gold fixture appears in the B2 harvested corpus — which is robust to differing sample windows. Keep the value-hash filter as a belt-and-braces secondary if cheap, but the primary key is `(file, column)`. State this explicitly so the implementer doesn't copy train_ydf's tuple-hash wholesale and ship a guard that passes while leaking.

### [MEDIUM] ac-06 is verifiable only after B2 exists; the spec gives it no standalone closure path

**Category:** test-gap
**Pass:** 2
**Description:** ac-06's substantive half — "asserts zero overlap between the gold fixture and the B2 harvested corpus once that exists" — is gated on `spec 2026-06-05-reference-data-inventory` (B2) producing a harvested corpus. B2 is currently `open` (confirmed via `orbit spec show`). As written, ac-06 cannot be closed by this spec's work alone: there is no corpus to diff against. The AC bundles two separable things — (a) the gold fixture carries a value-hash column / the exclusion is wired into the existing `train_ydf.py` path now, and (b) the cross-corpus zero-overlap audit, which is a future dependency. Without separating them, ac-06 either blocks this spec on B2 or gets checked on a vacuous "no corpus yet, zero overlap trivially holds" — which is exactly the failure mode that lets the guard rot silently before the corpus lands.

**Evidence:** `orbit spec show 2026-06-05-reference-data-inventory` → `status: open`, goal is to *produce* the catalogue B2 consumes; the harvested corpus does not yet exist. ac-06 phrase "once that exists" concedes the dependency.

**Recommendation:** Split ac-06. Keep the now-closable half here: gold fixture carries the leakage-key column and is wired into `train_ydf.py`'s existing exclusion so today's training corpus already honours it (closable, testable now). Move the cross-B2-corpus zero-overlap audit to a deferred/`observation`-typed AC or a follow-up task explicitly blocked on B2, so it doesn't get vacuously ticked. The `ac_type` field on ac-06 is currently unset (defaults to `code`, which blocks `spec.close`) — a half that cannot close until B2 ships will stall the spec.

### [LOW] ac-03's >= 20-per-family floor may be unreachable for the rare families ac-02 enumerates

**Category:** assumption
**Pass:** 2
**Description:** ac-02 fixes the confusion-family scope and ac-03 sets a floor of ">= 20 curated columns per family, or a justified smaller N where the family is genuinely rare." The corpus diagnostic surfaces some of these families (gender_code, country_code) abundantly, but others (msg_id → iso6346, stock_id → mgrs, exchange-code → country_code) are narrow tails — the report shows them as individual mis-labels, not as 20-column populations. The "justified smaller N" escape hatch exists, but the spec gives no criterion for what justifies it, so the floor is effectively author-discretion at curation time. That's acceptable for a curated fixture, but it leaves ac-03 partially self-judging.

**Evidence:** ac-02 enumerates rare tail families (iso6346, mgrs); `eval/gittables/corpus_pass/report.md` shows these as scattered single mis-labels, not dense populations. ac-03: ">= 20 … or a justified smaller N."

**Recommendation:** Either name the per-family floors explicitly (e.g. dense families >= 20, named rare families >= 5 with rationale captured in the fixture's `provenance` column), or state that the curator records the achieved N and rationale per family in the fixture so the "justified smaller N" is auditable after the fact. Minor — the fixture's `provenance/rationale` column already gives a home for the justification.

---

## Honest Assessment

This is a well-conceived spec and the right move: once YDF flips from judge to miner, scoring Sense against YDF-derived labels is circular by construction, and a small independent gold set is the only honest anchor. The independence contract is correctly placed first as the load-bearing constraint (ac-01, gated), the confusion families are real and substantiated by the corpus-pass report, and reusing the existing `labelled_eval.tsv` leakage infra rather than inventing a parallel one is the right discipline.

The biggest risk is ac-06: the leakage guard is the *mechanical* enforcement of the whole spec's reason for existing, and as written it (a) mirrors a value-tuple hash that under-catches the same-column-different-sample leak, and (b) depends on a B2 corpus that doesn't exist yet. A guard that passes while leaking is worse than no guard — it manufactures false confidence in the independence contract. Tighten ac-06 to a `(file, column)` identity check and split out the B2-dependent audit, and this is ready. The remaining findings are refinements, not blockers.

One-line for a stakeholder: the gold-set plan is sound and necessary, but the leakage guard needs to key on column identity (not sample values) and split its B2-dependent half out before implementation starts.
