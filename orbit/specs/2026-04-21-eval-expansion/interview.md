# Discovery: Eval Expansion

**Date:** 2026-04-21
**Interviewer:** Nightingale
**Card:** none (discovery-mode entry)
**Mode:** discovery
**Trigger:** `orbit/specs/2026-04-20-distilled-data-relabel-7-types/handover.md` — v17 sprint
surfaced that 5 of 7 relabel targets had zero eval coverage, making the retrain
structurally unmeasurable.

---

## Context

The v17 relabel sprint trained a model identical on the eval score (235/242) to
v16, spent 9 hours of compute, and was held from promotion under
`orbit/decisions/0054-hold-v17-no-promotion.md` partly because 5 of 7 relabel
targets (`swift_bic`, `cpt`, `loinc`, `excel_format`, `user_agent`) had no eval
columns — success was unmeasurable by construction. Separately, Hugh has long
held concern that the current 242-column eval lacks realism: too much
synthetic-adjacent data, not enough messiness.

Prior art reviewed before the interview:

- `eval/datasets/manifest.csv` — 35 datasets, 338 raw rows → 242 columns after
  v16 audit, 160 unique short labels (≈67% of 240-type taxonomy).
- `orbit/specs/2026-04-18-v16-data-audit-retrain/eval-audit.md` — precedent for
  manual per-column audit (338 rows reviewed, 20 corrections, 1–2 days).
- `orbit/decisions/0050-per-type-sourcing-policy.md` — per-type
  synthetic/distilled/real sourcing rules already exist.
- `orbit/decisions/0052-scope-aware-eval-gate.md` — eval gate already
  distinguishes target types from non-targets.
- `orbit/decisions/0049-preserve-synthetic-for-bad-distilled-types.md` — the 7
  types with bad distilled coverage, which partly motivated v17.

## Q&A

### Q1: Primary goal
**Q:** What's the primary goal of eval expansion — what outcome makes this
sprint a success?
**A:** **All three** — measurability of retrains, taxonomy coverage audit, and
eval realism. Realism is the long-running concern; measurability is the
immediate trigger.

### Q2: Sequencing
**Q:** How should measurability, coverage, and realism be sequenced?
**A:** **Realism audit first, then coverage, then measurability as fallout.**
Realism is the ontological root — if the existing columns aren't realistic, the
coverage and measurability gains are built on sand.

### Q3: Realism definition
**Q:** What makes a column "realistic" — which signals matter?
**A:** **Provenance + messiness + distributional fidelity.** (Header
authenticity deliberately excluded from scope — flagged as a separate future
concern.)

### Q4: Audit output
**Q:** What does the realism audit produce for each of the 242 existing columns?
**A:** **Triage flag + action:** `keep` / `augment` / `replace`. Produces a
worklist that can be sized and budgeted.

### Q5: Audit method
**Q:** How do we actually perform the 242-column realism audit?
**A:** **Programmatic pre-screen + Hugh review.** Deterministic checks
(provenance ledger, messiness metrics, distributional tests) flag suspicious
columns; Hugh reviews the flagged subset. Aligns with CLAUDE.md Engineering
Principle 3 — *LLMs for parsing, programmatic checks for validation.*

### Q6: Coverage floor
**Q:** After expansion, what's the per-type minimum?
**A:** **≥1 realistic column + ≥1 edge-case column per type.** ~400 new columns
long-term (240 types × 2 − 160 already covered, modulo per-type depth). This is
a multi-sprint programme.

### Q7: Sourcing strategy
**Q:** Where do the ~400 new realistic columns come from?
**A:** **Delegated to Nightingale**, subject to two hard constraints:
1. Ethical sourcing for research purposes
2. Attribution required

Fan-out to Sonnet sub-sessions is explicitly authorised for per-source loader
work. Tactical mix (public datasets / public APIs / hand-curated / sanctioned
distillation) to be decided per type-domain in the spec.

### Q8: Leakage prevention
**Q:** How do we prevent training/eval contamination?
**A:** **Row-hash deduplication + source-level separation.** Source manifest
records each dataset's role (train/eval/both-forbidden); every eval row gets a
SHA256 over (header, sample-values); training pipeline filters any row matching
an eval hash. Belt-and-braces. Will need its own MADR.

### Q9: Sprint shape
**Q:** What does the first sprint aim to ship?
**A:** **Full Phase A + B in one sprint**: audit + all `replace` actions
executed + zero-coverage closure (every type gets ≥1 column). 1–2 weeks.
Phase C (edge-case pass to hit ~400 columns) is a future programme.

### Q10: Sprint done criterion
**Q:** Pass/fail gate for Phase A + B?
**A:** Three boxes must tick:
1. All 242 existing columns have provenance ledger entries (audit complete).
2. All columns flagged `replace` have been replaced.
3. All 240 taxonomy types have ≥1 eval column (zero-coverage closed).

The `augment` worklist may remain open at sprint end.

### Q11: Retrain coupling
**Q:** Does this sprint block new retrains?
**A:** **Yes — block retrains until expanded eval ships.** No v18 sweep starts
until Phase A + B is done. Cleanest stance: no model ships against an eval
we've flagged as unrealistic.

### Q12: Attribution home
**Q:** Where does per-column attribution live?
**A:** **Extend `eval/datasets/manifest.csv`** with `source_url`, `licence`,
and `fetched_date` columns. Single machine-readable source of truth, grows
from 4 columns to 7.

---

## Summary

### Goal
Rebuild the eval corpus so every type in the 240-type taxonomy has at least
one realistic eval column, and every retrain sprint can be measured
type-by-type. Realism (provenance + messiness + distributional fidelity) is
the ontological root; coverage and measurability fall out of a realism-first
audit.

### Constraints
- **Phasing:** realism audit precedes coverage expansion precedes
  measurability-as-fallout.
- **Audit method:** programmatic pre-screen + human review. No
  LLM-as-judge for the final call (CLAUDE.md Principle 3).
- **Sourcing:** ethical for research use, attribution mandatory.
- **Leakage:** row-hash + source-level separation between train and eval.
- **Retrain block:** no v18 sweep until Phase A + B ships.
- **Coverage long-term target:** ≥1 realistic + ≥1 edge-case per type
  (~400 columns; Phase C beyond this sprint).
- **Attribution home:** extend `manifest.csv` with `source_url`, `licence`,
  `fetched_date`.

### Success Criteria (Phase A + B sprint)
1. All 242 existing eval columns have provenance ledger entries.
2. All columns flagged `replace` during audit have been replaced with real
   sourced data meeting the realism bar.
3. Every type in the 240-type taxonomy has ≥1 eval column (zero-coverage
   closed for `swift_bic`, `cpt`, `loinc`, `excel_format`, `user_agent`,
   plus any others surfaced by the audit).

### Decisions Surfaced
Each of these will become a MADR during or after `/orb:spec`:

- **Realism dimensions for eval columns:** provenance + messiness +
  distributional fidelity. Header authenticity deliberately out of scope
  for this programme. (→ MADR candidate)
- **Audit methodology:** programmatic pre-screen + human review, not
  LLM-as-judge. (→ reinforces Principle 3; MADR optional)
- **Coverage floor:** ≥1 realistic + ≥1 edge-case per type long-term;
  Phase A + B closes zero-coverage only. (→ MADR candidate)
- **Leakage prevention:** row-hash dedup + source-level manifest roles.
  (→ MADR, high-priority — affects training pipeline)
- **Retrain block:** no new model sweeps until the expanded eval ships.
  (→ sprint policy; MADR optional)
- **Attribution schema:** extend `manifest.csv` with `source_url`,
  `licence`, `fetched_date`. (→ MADR or covered in spec)

### Open Questions
- **Header authenticity** is deliberately out of scope for this programme but
  the model relies heavily on headers; worth a follow-up card once eval
  expansion ships.
- **Distributional fidelity reference distributions** — who writes/curates the
  per-type reference for the distributional test? May need per-domain expert
  input (medical types especially).
- **Domain-expert involvement** for medical (LOINC, CPT, ICD), finance (SWIFT
  BIC, LEI, IBAN), and systems (http_method, user_agent, excel_format) types.
  Hugh's delegation authorises Sonnet fan-out but doesn't settle human
  domain-expert involvement — answer at spec time if needed.
- **Phase C sizing** — the full ~400-column target is a future programme; its
  scope, budget, and trigger will be decided after Phase A + B lands.

---

**Next step:** `/orb:spec eval-expansion` to crystallise this into a structured
specification. The spec will need to pin the programmatic pre-screen metrics,
the manifest.csv schema change, and the leakage-prevention MADR before
implementation begins.
