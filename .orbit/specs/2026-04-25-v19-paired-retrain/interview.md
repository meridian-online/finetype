# Discovery: v19 paired retrain — ReLU+BN vs GELU+LN

**Date:** 2026-04-25
**Interviewer:** Nightingale
**Card:** .orbit/cards/0002-semantic-type-detection.yaml
**Mode:** discovery

---

## Context

Diagnostic top-k analysis of 14 failing columns (8 container, 6 datetime)
revealed that 13/14 have the ground-truth label absent from the model's raw
softmax top-5 — a genuine model representation gap, not a hint-layer problem.
Container types have generators but are excluded from training data entirely.
Datetime subtypes are in training but the model can't distinguish them.

The GELU+LN architecture (MADR 0046, closed April 12) showed a -5 label
regression vs ReLU+BN, but was measured against a Sharpen layer that has since
changed substantially (demotion guard PR #44, amount-variant fix PR #47,
header-hint regex removal PR #48, eval corpus expansion). The interaction
between GELU+LN output distribution and today's Sharpen may be different.

## Q&A

### Q1: Primary goal
**Q:** The v19 retrain has two independent variables: training data and
architecture. Is this about closing the 14-column gap, a broader "best model
we can ship" sweep, or primarily an architecture decision?
**A:** Best model we can ship. Success = highest overall score on the 448-row
manifest through the MADR 0066 gate. The 14 columns are motivation but net
improvement is what matters.

### Q2: Corpus base
**Q:** Should v19 adopt the v4 corpus (UA loader, excel_format improvements)
or stay on v3?
**A:** v4 base + new additions. The v4 regression risk (MADR 0054: 3 fixes /
3 regressions) may not reproduce given pipeline changes since v17.

### Q3: Container training data strategy
**Q:** Container types have generators but aren't in TABLE_TEMPLATES. How
should we add them?
**A:** Nightingale's call — chose TABLE_TEMPLATES (option 1). Table-context
co-occurrence signal teaches the model realistic column relationships.

### Q4: MADR 0046 disposition
**Q:** Should we supersede MADR 0046 (GELU+LN not adopted) now or after
results?
**A:** Supersede now. Record that the rationale for revisiting is substantial
Sharpen layer changes since the original measurement.

### Q5: Sweep schedule
**Q:** 6 runs (3 seeds × 2 architectures) = ~15 hours. Single overnight,
two nights, or parallel on two machines?
**A:** Single overnight. Start evening, results by morning.

### Q6: Mechanism attribution
**Q:** How should we attribute which change (data vs architecture) moved the
numbers?
**A:** Three-way diff. v16 baseline (already measured at 297/352) vs ReLU-v19
vs GELU-v19. Both architectures train on identical data, so the ReLU↔GELU
diff isolates architecture effect; the winner↔v16 diff isolates combined
effect.

### Q7: Scope
**Q:** Should training data work be limited to container + datetime, or also
fold in other v4 improvements?
**A:** Everything v4 gives us + container + datetime. v4 base inherently
brings UA + excel_format, no extra scoping needed.

### Q8: Promotion path
**Q:** If both architectures pass MADR 0066, what decides? What if GELU+LN
wins — any concerns about changing the default architecture?
**A:** Winner takes all. GELU+LN is already in the inference crate behind
config flags. DuckDB/MCP users won't notice the difference.

---

## Summary

### Goal
Ship the best-scoring model on the 448-row eval manifest through the MADR 0066
hard gate, using a paired ReLU+BN vs GELU+LN sweep on improved training data.

### Constraints
- v4 corpus base + container/datetime training data additions
- 3-seed sweep (42, 43, 44) × 100 epochs × 2 architectures = 6 runs
- Single overnight execution (~15 hours on M1 Pro Metal)
- MADR 0066 hard gate applies to both candidates independently
- Three-way diff for mechanism attribution (v16 vs ReLU-v19 vs GELU-v19)

### Success Criteria
- At least one architecture passes all 6 MADR 0066 gate conditions
- Winner promoted to `models/default` (winner takes all, no margin requirement)
- Per-column diff and mechanism attribution published in PR

### Decisions Surfaced
- **Corpus base: v4 + additions** — chose v4 over v3 because pipeline changes
  since v17 may have neutralised the regression risk that held v17 (MADR 0054).
  Supersedes the v3-only choice in MADR 0060 for this sweep.
- **Supersede MADR 0046 now** — GELU+LN revisited because Sharpen layer has
  changed substantially. New MADR will record the paired comparison outcome.
- **Container training data via TABLE_TEMPLATES** — matches how all other types
  get training coverage; preserves table-context co-occurrence signal.
- **Winner takes all promotion** — no margin requirement for GELU+LN over
  ReLU+BN. GELU+LN inference path already ships in the crate.

### Implementation Notes
- Container generators exist in generator.rs but need TABLE_TEMPLATES entries
  in prepare_multibranch_data.py to appear in training data
- v4 corpus lives on branch `distilled-data-relabel-7-types-v17` — needs
  rebase or cherry-pick onto main for the v19 prep script
- GELU+LN is activated via `activation: "GELU"` + `use_layer_norm: true` in
  MultiBranchConfig — no code changes needed, just config
- The overnight script needs to produce `results.json` and `epochs.jsonl` for
  each of the 6 runs, plus a post-sweep eval comparison script
- v16 baseline is already measured at 297/352 (84.4% label, 91.8% domain) on
  the 448-row manifest — no re-run needed for the three-way diff

### Open Questions
- How many TABLE_TEMPLATES entries for container types? (implementation detail)
- Should datetime generator improvements target all 6 failing subtypes or
  prioritise the 3 that are in the wrong domain entirely? (implementation detail)
- v4 branch rebase strategy — cherry-pick vs merge (implementation detail)
