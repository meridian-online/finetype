# Spec Review

**Date:** 2026-04-27
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-27-sharpen-rule-audit/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Review Depth

| Pass | Triggered by | Findings |
|------|-------------|----------|
| 1 — Structural scan | always | 3 |
| 2 — Assumption & failure | content signals: model promotion, CI changes, gate amendment | 3 |
| 3 — Adversarial | not triggered | — |

## Findings

### [LOW] AC-04 may produce zero work

**Category:** missing-requirement
**Pass:** 1
**Description:** AC-04 requires editing all NARROW-verdict rules, but the audit might produce zero NARROW verdicts (only KEEP and REMOVE). The AC has no "N/A if no NARROW verdicts" escape hatch, making it ambiguously testable — does it pass vacuously or fail for lack of action?
**Evidence:** AC-04 description says "All NARROW-verdict rules edited" but the constraint "No Sharpen rule additions — only removals and narrowing" suggests narrowing is secondary. The interview (Q4) focuses on removal as the primary action.
**Recommendation:** Add a clause: "If no rules receive a NARROW verdict, AC-04 is satisfied vacuously and recorded as such in the audit verdicts TSV."

### [MEDIUM] AC-03 assumes deletions but has no minimum or fallback

**Category:** assumption
**Pass:** 1
**Description:** AC-03 verification requires "Deleted rule count > 0" — at least one rule must be net-negative. If the ablation shows all rules are net-positive or net-zero on v19, the spec has no path forward. The interview (Q5) says net-zero rules should be removed, but the ablation TSV categorisation in AC-02 says REMOVE is for "net-negative or net-zero." If v19 is genuinely better at everything and no rule is net-negative or net-zero, AC-03 blocks. This is unlikely given the -6 delta, but the spec should acknowledge the possibility.
**Evidence:** AC-02 categorisation: "REMOVE (net-negative or net-zero)". AC-03 verification: "Deleted rule count > 0". Interview Q5 confirms net-zero rules are removed.
**Recommendation:** This is low-risk given the known -6 delta, but add to implementation_notes: "If ablation reveals no REMOVE candidates, revisit the gate amendment (ac-07) independently — the model may still be promotable at 365/448 if future rule work is decoupled."

### [MEDIUM] Ablation methodology gap — interaction effects not addressed

**Category:** test-gap
**Pass:** 1
**Description:** The ablation measures each rule's impact individually (disable one, score). But rules can interact — removing rule A alone might regress, removing rules A+B together might improve. The spec does not address how to handle interaction effects when multiple rules are removed simultaneously.
**Evidence:** AC-01 says "disable it" (singular), AC-02 categorises individually, AC-03 deletes all REMOVE-verdict rules at once. The combined removal in AC-03 is not re-measured before AC-05's gate check.
**Recommendation:** AC-05 already gates the combined result (v19 + cleaned pipeline >= 371). This is the interaction safety net. Add an implementation note making this explicit: "Individual ablation is for triage; AC-05 is the interaction-aware gate. If AC-05 fails after AC-03 removals, re-examine interaction effects between removed rules."

### [MEDIUM] The -6 delta recovery assumption is unvalidated

**Category:** assumption
**Pass:** 2
**Description:** The spec's central thesis is that Sharpen rules account for the -6 label regression in v19-relu-s42 (365 vs 371). If rules account for only 3-4 of those 6, the cleaned pipeline lands at 368-369 and fails the >= 371 gate in AC-05. The interview frames this as "Sharpen rules fight [v19] on 16 columns" but MADR 0068 shows the -6 is net (some columns improved, some regressed). The 16 regressions may not all be rule-caused.
**Evidence:** MADR 0068: "Best ReLU (s42): 365/448 — net_label_delta = -6". Interview: "v19 model has better val_acc (91.3% vs ~91%) but Sharpen rules fight it on 16 columns -> net -6 profile eval." The 16 regressions include "7 coverage_closure, 3 datetime subtype, 3 cross-domain, 3 scientific/text" — coverage_closure regressions may be model-intrinsic, not rule-caused.
**Recommendation:** Add a constraint or implementation note acknowledging this risk: "If rule cleanup recovers fewer than 6 labels, the fallback is the gate amendment in AC-07 (>= 0 when rule count decreases). In that case, AC-05's threshold of 371 is replaced by the amended gate's >= 365 (tie with v19-relu-s42's raw score)." Alternatively, sequence AC-07 before AC-05 so the amended gate is in effect when the score is measured.

### [MEDIUM] AC ordering creates a chicken-and-egg with the gate amendment

**Category:** constraint-conflict
**Pass:** 2
**Description:** AC-07 amends the gate to accept ties (>= 0). AC-05 checks the gate at >= 371 (the v16 baseline, requiring recovery of all 6 lost labels). If the amendment is the fallback when full recovery fails, these two ACs need explicit sequencing — the amendment should be drafted before AC-05 is evaluated, so the evaluator knows which threshold applies.
**Evidence:** AC-05 verification: "Label count >= 371 (tie or better)." AC-07: "amend MADR 0066 gate to accept net_label_delta >= 0." If the cleaned pipeline scores 368, AC-05 fails under the current threshold but would pass under the amended gate.
**Recommendation:** Either (a) make AC-05's threshold explicitly "per the gate in effect at evaluation time" (so AC-07 can lower it), or (b) add a decision point: "If AC-05 fails at 371 but passes at >= 365, AC-07 is accepted first, then AC-05 is re-evaluated under the amended gate." The current spec reads as if both thresholds coexist without resolving which wins.

### [LOW] HuggingFace upload not captured as an AC

**Category:** missing-requirement
**Pass:** 2
**Description:** AC-06 covers the symlink and CI env var but not the HuggingFace upload, which is step 1 of the 3-step promotion flow documented in CLAUDE.md. The implementation_notes mention it, and the exit_conditions say "PR created with rule removals + model promotion," but no AC gates the HF upload succeeding.
**Evidence:** CLAUDE.md promotion flow: "1. Publish to HuggingFace... 2. Bump FINETYPE_CI_MODEL... 3. Flip models/default." AC-06 covers steps 2-3 only.
**Recommendation:** Either add a verification clause to AC-06 ("HuggingFace model page exists at meridian-online/finetype-model for sherlock-v19-relu-s42") or add an implementation note that HF upload is a pre-requisite performed manually before the PR merges.

---

## Honest Assessment

This is a well-motivated spec with clear diagnostic methodology and a solid eval gate. The central risk is the unvalidated assumption that Sharpen rules account for all 6 of v19's label regressions. The spec partially addresses this with the gate amendment (AC-07), but the relationship between AC-05's hard threshold (371) and AC-07's softer gate (tie acceptable) needs to be resolved — right now they can contradict each other. The ablation methodology is sound for triage but the spec correctly gates the combined result in AC-05, which handles interaction effects. I recommend resolving the AC-05/AC-07 sequencing ambiguity and acknowledging the partial-recovery scenario before implementation begins. None of these findings are blockers — they are clarifications that prevent mid-implementation confusion about what "passing" means.
