# Interview: LLM Distillation v2 — Claude Code as Labeller

**Date:** 2026-03-18
**Interviewer:** Nightingale

---

## Context

The first LLM distillation run (specs/2026-03-11-llm-distillation/FINDINGS.md) used Qwen3 8B locally via Ollama on 5,359 columns from 508 CSVs. Results: 97% valid labels, 20% exact agreement with FineType, 14 hours runtime.

Key weaknesses: the 8B model defaulted to `container.array.*` on 1,744 cases, couldn't distinguish FineType's representation subtypes, and had only 3% agreement on finance types. However, it was genuinely better than FineType on ~990 cases (currency amounts, usernames, HTML content).

Separately, the earthquakes dataset profiling (specs/2026-03-13-disambiguator-spike/interview.md) exposed that the `id` header hint was overriding correct model predictions. This reinforced the value of real-world "wild" data in evaluation — columns that don't fit neat assumptions.

Hugh wants to revisit distillation using his Claude Max 20x subscription, leveraging Claude Code agents as batch labellers instead of a local Ollama model.

## Interview Q&A

### Q1: Access method
**Q:** Claude Max gives you unlimited chat UI usage but no direct API access. How are you thinking about programmatic access?
**A:** Claude Code as labeller — use spawned agents to classify columns, leveraging the Max subscription's Claude Code usage.

### Q2: Dataset scope
**Q:** The previous run covered 5,359 columns from 508 CSVs (96% GitTables). What's the target?
**A:** Solid coverage on GitTables and SOTAB first, then real-world datasets (Kaggle, government open data). The only constraint is cost and impact to the Max plan.

### Q3: Primary goal
**Q:** What's the primary use for the Claude-generated labels — training data, eval expansion, or gap analysis?
**A:** All three. Training data improvement, eval expansion, and gap analysis are complementary goals.

### Q4: Execution approach
**Q:** Batch agents (parallel, each handles a file) vs single-session sequential vs hybrid script?
**A:** Batch agents — spawn N parallel agents, each handles a CSV file or group of files.

### Q5: Scale and plan impact
**Q:** How many columns? Start with 1K pilot or go big?
**A:** Goal is to go big (10K+), but determine cost impact first. Max 20x plan gives ~220K tokens per 5-hour window. Phased approach preferred, incorporating prompt design improvements from v1 findings.

### Q6: Phasing
**Q:** Three-phase approach: Pilot (500) → Scale (5K+) → Wild expansion?
**A:** Yes, three phases. Each phase informs the next. Conservative on plan usage.

### Q7: Prompt design
**Q:** Direct classification (two-stage domain→type) vs adjudication (Claude reviews FineType's prediction) vs test both?
**A:** Adjudication mode — but critical that the adjudicator isn't biased by FineType output. An adjudicator that agrees too much is a waste; too much disagreement just to be "critical" leads to bad training data. All 250 labels must be visible (no 84-type cut-down).

### Q8: Output format
**Q:** Keep the existing CSV format or add fields?
**A:** Add confidence (high/medium/low) and brief reasoning. Helps filter for eval-quality labels.

### Q9: Bias control
**Q:** Two-pass design — blind classification first, then adjudication if disagreement — to avoid anchoring bias?
**A:** Yes, blind-first approach. Independent classification first, adjudication second.

### Q10: Output location
**Q:** Where should results land?
**A:** `output/distillation-v2/` — new directory alongside the existing `output/llm_labels.csv`. One CSV per batch, plus a merged summary.

---

## Summary

### Goal
Re-run LLM distillation using Claude Code agents (via Max 20x subscription) as batch labellers on 10K+ columns across GitTables, SOTAB, and real-world datasets. Triple objective: improve training data, expand eval manifest, and identify systematic FineType gaps.

### Approach: Blind-First Adjudication

Each batch agent processes a CSV file with two passes per column:

1. **Pass 1 (blind):** Classify from header + sample values + full 250-label taxonomy. No FineType prediction shown. Records: `blind_label`, `confidence`.
2. **Pass 2 (adjudicate):** FineType prediction revealed. If disagreement, write brief reasoning for which label is correct. Records: `final_label`, `reasoning`.

This produces three signals per column: independent LLM opinion, FineType prediction, and informed adjudication on disagreements.

### Phases

| Phase | Scope | Goal |
|-------|-------|------|
| Phase 1 — Pilot | 500 columns from existing 508 CSVs | Test prompt design, measure actual Max plan impact, compare Claude vs Qwen3 on same data |
| Phase 2 — Scale | 5K+ columns (full GitTables + SOTAB) | Best prompt from Phase 1 at scale. Training data + gap analysis |
| Phase 3 — Wild | Kaggle, government open data, new datasets | Eval manifest expansion with real-world data |

### Constraints
- Max 20x plan: ~220K tokens per 5-hour window, weekly caps apply
- Use Sonnet (not Opus) to preserve Opus caps for other work
- ~10-12 batch agents per 5-hour window → 500-1,200 columns per window
- Blind-first design to prevent anchoring bias
- All 250 taxonomy labels visible (no domain cut-down)

### Output
- Location: `output/distillation-v2/`
- Format: CSV with columns: `source_file`, `column_name`, `sample_values`, `blind_label`, `blind_confidence`, `finetype_label`, `agreement`, `final_label`, `reasoning`
- One CSV per batch agent, plus merged summary

### Success Criteria
- Phase 1: >50% exact agreement (vs Qwen3 8B's 20%), <5% invalid labels, measurable plan impact assessment
- Phase 2: Coverage of all 7 domains with representative disagreement data
- Phase 3: ≥50 new "wild" columns added to eval manifest with verified ground truth
- Overall: Actionable list of FineType gaps + silver-standard labels for retraining

### Open Questions
- Exact token usage per agent (measure in Phase 1 pilot)
- Weekly cap impact at scale — may need to spread Phase 2 across multiple weeks
- How many sample values per column to include (v1 used 10; more may help Claude)
- Whether to include sibling column context (other column headers from the same file) in the prompt
- Threshold for promoting Claude labels to eval manifest ground truth (human review required?)
