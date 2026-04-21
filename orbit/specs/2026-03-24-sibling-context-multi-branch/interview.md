# Interview: Sibling-Context Multi-Branch Training

**Date:** 2026-03-24
**Interviewer:** Nightingale

---

## Context

sherlock-v3-flat achieves 140/190 (73.7%) on profile eval — +12 over v2 but below target. The model has 98.3% val accuracy (not underfitting). Gap analysis shows 15 decimal→date/IP errors, 8+ header-fixable, 7 entity confusion. The multi-branch header branch sees headers in isolation — sibling-context attention exists but is explicitly bypassed for multi-branch (`column.rs:822`).

## Interview Q&A

### Q1: Scope
**Q:** Should the strategy focus on wiring sibling context into multi-branch, improving the header branch standalone, or both?
**A:** Wire siblings into multi-branch. This is the biggest untapped signal.

### Q2: Training approach
**Q:** Should sibling context be inference-only or trained into the model?
**A:** Train with sibling context. Must still handle single-value inference gracefully (`finetype infer -i "alice@example.com"` → no siblings, still works).

### Q3: Numeric disambiguation
**Q:** Feature-based numeric guard now, or defer to pipeline integration?
**A:** Defer to pipeline integration. When multi-branch replaces Sense, existing Sense masking handles numerics.

### Q4: Endgame (settled, decision 0041)
**Q:** Multi-branch replaces everything, or serves as better Sense within the pipeline?
**A:** Multi-branch as better Sense within the existing pipeline. **This is settled — do not re-ask.**

### Q5: Sprint priority
**Q:** Integrate first then improve, or improve model first then integrate?
**A:** Improve model first, then integrate.

### Q6: Training data format
**Q:** Table-grouped FTMB v3, or pre-computed enriched headers in existing format?
**A:** Table-grouped FTMB v3. New binary format where records are grouped by source table.

### Q7: Synthetic siblings
**Q:** How to handle siblings for synthetic training records?
**A:** Synthetic table assembly — generate synthetic "tables" by grouping 5-15 related types together with domain knowledge for realistic co-occurrence.

### Q8: Numeric fix timing
**Q:** Add F7 numeric guard now, or defer to pipeline integration?
**A:** Defer to pipeline integration.

### Q9: Frozen vs trainable attention
**Q:** Should sibling-context attention weights be frozen or trainable during multi-branch training?
**A:** Frozen attention, trainable MLP (recommended). Follows transfer learning best practice — reuse proven attention weights, let the header branch MLP learn to use the enriched signal.

### Q10: Implementation approach
**Q:** Run attention during training, or pre-compute enriched embeddings offline?
**A:** Run attention during training (frozen). Keep the sibling model in the loop rather than pre-computing.

### Q11: Success metrics
**Q:** Raw multi-branch accuracy, or post-processed (with pipeline disambiguation)?
**A:** Post-processed is the real target. What matters is multi-branch + existing pipeline rules.

### Q12: Ship bar
**A:** Stop asking about targets. Focus on improving the pipeline.

---

## Summary

### Goal
Wire sibling-context attention into multi-branch training so the header branch sees cross-column context. Train with table-grouped data (FTMB v3). Improve multi-branch model accuracy as measured by post-processed pipeline output.

### Key Decisions
- Decision 0041: Multi-branch as Sense replacement within existing pipeline (settled)
- Decision 0042: Remove regex header hints in favour of learned approaches (settled)
- Decision 0038: Strength through simplification — prefer training over rules (settled)

### Constraints
- Single-value inference must still work (`finetype infer -i "value"`)
- Sibling-context attention weights frozen during training
- FTMB v3 format with table-grouped records
- Synthetic data uses assembled tables (5-15 related types grouped by domain)
- Numeric disambiguation deferred to pipeline integration phase

### Success Criteria
- Multi-branch model with sibling context trained and evaluated
- Post-processed (pipeline-integrated) accuracy is the meaningful metric
- Model improvement prioritised over pipeline integration this sprint

### Open Questions
- None — direction is clear
