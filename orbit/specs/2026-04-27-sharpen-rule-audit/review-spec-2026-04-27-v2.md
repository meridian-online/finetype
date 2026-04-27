# Spec Review

**Date:** 2026-04-27
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-27-sharpen-rule-audit/spec.yaml
**Verdict:** APPROVE

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 0 |
| 2 — Assumption & failure | content signals: model promotion, CI changes, gate amendment | 1 |
| 3 — Adversarial | not triggered | — |

## Prior Review Findings — Resolution Check

The v1.0 review (REQUEST_CHANGES, 6 findings) has been fully addressed in spec v1.1:

1. **AC-04 vacuous satisfaction** (LOW) — Added: "If no rules receive a NARROW verdict, AC-04 is satisfied vacuously." Resolved.
2. **AC-03 minimum threshold** (MEDIUM) — Changed from "Deleted rule count > 0" to ">= 0" with vacuous satisfaction clause and implementation note about decoupling gate amendment from rule work. Resolved.
3. **Ablation interaction effects** (MEDIUM) — Implementation note added: "Individual ablation is for triage; ac-06 is the interaction-aware gate." Resolved.
4. **Partial recovery assumption** (MEDIUM) — AC-06 now has dual thresholds (>= 365 OR >= 371) and implementation note acknowledges coverage_closure regressions may be model-intrinsic. Resolved.
5. **AC ordering chicken-and-egg** (MEDIUM) — AC-05 (gate amendment) now explicitly precedes AC-06 (gate evaluation), with description stating "Drafted BEFORE the gate evaluation in ac-06." Resolved.
6. **HuggingFace upload** (LOW) — AC-07 implementation_notes clause added: "HuggingFace upload is a manual pre-requisite performed before PR merge." Resolved.

## Findings

### [LOW] Ablation net_delta sign convention is counterintuitive

**Category:** assumption
**Pass:** 2
**Description:** The ontology schema defines net_delta as "fixes - regressions" where "positive = rule is harmful, should remove." This means a positive number signals a bad rule — the opposite of the usual convention where positive = good. The AC-02 categorisation logic (KEEP/REMOVE/NARROW) depends on interpreting this sign correctly. The spec is internally consistent, but the ablation script implementer must not confuse "positive net_delta" with "positive contribution."
**Evidence:** ontology_schema.fields[4]: `net_delta` — "positive = rule is harmful, should remove". AC-02: "REMOVE (net-negative or net-zero)." These use opposite sign conventions: a rule that is "net-negative" (bad for accuracy) produces a "positive" net_delta in the schema.
**Recommendation:** No spec change needed — the ontology schema is clear for anyone who reads it. Consider adding a one-line comment in the ablation script when implemented: "net_delta > 0 means disabling the rule improves the score, i.e., the rule is harmful."

---

## Honest Assessment

The spec is ready for implementation. All six findings from the v1.0 review have been cleanly addressed in v1.1. The AC sequencing is now unambiguous (gate amendment AC-05 before gate evaluation AC-06), the dual-threshold approach in AC-06 handles partial recovery without contradiction, and the vacuous satisfaction clauses for AC-03 and AC-04 eliminate dead-end paths. The only remaining observation is a sign convention note that is internally consistent but worth a comment during implementation. The biggest real risk — that Sharpen rules account for fewer than 6 of v19's regressions — is explicitly acknowledged in the implementation notes and handled by the amended gate. This is a well-structured audit plan.
