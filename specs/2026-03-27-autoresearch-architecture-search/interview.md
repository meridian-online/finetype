# Interview: Python-based Autoresearch Loop for FineType Architecture Search

**Date:** 2026-03-27
**Interviewer:** Nightingale

---

## Context

The branch diagnostic spike (2026-03-27) found that FineType's early-fusion multi-branch architecture destroys 84 correct branch answers while creating only 5 — the header branch (44.4% eval accuracy, highest of any branch) contributes only 10% of the merged vector and gets systematically outvoted.

Rather than manually specifying fusion variants, we want to adapt Karpathy's autoresearch pattern: a `program.md` instructs an autonomous agent to iterate on a Python training script overnight, searching for better fusion architectures. The winning architecture gets ported back to Rust for production.

The autoresearch pattern: `program.md` (agent instructions) + `train.py` (single editable file) + `prepare.py` (fixed eval/data) + loop: modify → train → eval → keep/discard → repeat forever.

## Interview Q&A

### Q1: Deliverable
**Q:** What's the primary deliverable from the overnight research loop — a single winning architecture config, or a Pareto frontier of architecture/accuracy trade-offs that you review in the morning?
**A:** Single best architecture. Agent keeps the highest-scoring config. Wake up to one winner ready to port to Rust.

### Q2: Eval metric
**Q:** For the keep/discard metric: our situation has two evaluation layers — raw model accuracy (fast) and model+Sharpen accuracy (full pipeline). Which should the agent optimise against?
**A:** Raw model accuracy. Isolates what the architecture actually learns — Sharpen rules are applied later in Rust regardless.

### Q3: Time budget
**Q:** Training time budget per experiment: v7 trains 30 epochs in ~30 min on Metal. Should we use shorter probing runs or full 30-epoch runs?
**A:** Initially leaned towards 10 epochs (~10 min). Revised after Q10/Q11 discussion to **10-minute wall-clock budget** (matching autoresearch's time-based approach). See Q10 and Q11 for rationale.

### Q4: Data format
**Q:** The FTMB data file is binary. Should we write a Python FTMB reader, export to numpy, or export to PyTorch tensors?
**A:** Python FTMB reader (Nightingale recommended). Single source of truth — no conversion step. Reader goes in prepare.py (fixed, read-only), keeping it out of the agent's reach. ~50 lines of Python.

### Q5: Search scope
**Q:** Should we constrain the agent to architecture-only changes, or let it also explore training hyperparameters?
**A:** Architecture only. Keeps the search focused on the fusion problem diagnosed by the branch probe. Training hyperparams are fixed at v7-proven values (LR 0.0001, dropout 0.35, batch 32, weight decay 0.0001).

### Q6: Keep/discard threshold
**Q:** Should the agent keep any improvement, or require a minimum delta to filter noise?
**A:** Any improvement (Nightingale recommended). Training is deterministic with fixed seed (42). No random seed variation to filter. +0.001 from an architectural change is real signal. Greedy-keep matches the autoresearch pattern.

### Q7: Eval method
**Q:** Should we build a Python eval on the same 214 columns, export weights to Rust eval, or use validation accuracy only?
**A:** Python eval on same data. Pre-extract 214 eval column features into an eval FTMB file. Python forward pass scores them directly. Fast (~30s), self-contained, no Rust compilation needed.

### Q8: Starting point
**Q:** Should we pre-implement fusion variants, start from baseline only, or start from late fusion only?
**A:** Baseline only, agent builds. Start with v7 early-fusion architecture. Agent modifies the model class directly to invent new architectures. True autoresearch style — the program.md includes diagnostic evidence to guide exploration. Maximum creativity.

### Q9: Location
**Q:** Where does this research loop live in the repo?
**A:** `finetype/research/` subdirectory. Inside the FineType repo for easy access to FTMB data and eval columns. Gitignored model outputs. Clear that it's a research spike, not production code.

### Q10: Autoresearch conflicts
**Q:** Does anything we've decided conflict with the autoresearch method?
**A:** One real conflict identified: **epoch-based vs time-based budget**. Autoresearch uses wall-clock time (5 min, period) — this naturally penalises bloated architectures because a slower model gets fewer training steps in the same window. Our epoch-based budget (10 epochs) doesn't have this property — an agent could add a massive attention mechanism, take 3x longer, and still "win" on accuracy. Minor tension: architecture-only constraint is a soft program.md guideline, not enforced mechanically — this is consistent with autoresearch's own soft constraints ("don't add dependencies"). Decision: switch to time-based budget (see Q3 revision).

### Q11: Inference speed
**Q:** How can we be sure the model will prioritise inference speed, not just accuracy? Both are important.
**A:** Solved by switching from epoch budget to **10-minute wall-clock time budget**. This creates implicit pressure toward efficient architectures: a model with 2x parameters gets ~5 epochs in 10 min (vs ~10 for baseline), so it must learn faster to win — not just be bigger. Additionally, log inference time per sample in results.tsv as a secondary metric for human review, but the time budget does the heavy lifting. This matches autoresearch exactly and resolves the inference speed concern without adding a second optimisation target.

---

## Summary

### Goal
Build a Python-based autoresearch loop that autonomously searches for the best multi-branch fusion architecture for FineType, running overnight on Metal. The agent iterates on `train.py`, evaluating each variant against raw model accuracy on the 214-column profile eval set. The single best architecture is ported back to Rust for production training.

### Constraints
- Agent modifies only `train.py` — `prepare.py` (data/eval) and `program.md` (instructions) are fixed
- Architecture-only changes — training hyperparams locked at v7-proven values
- 10-minute wall-clock time budget per experiment (not epoch-based) — naturally penalises bloated architectures
- Deterministic training (seed 42) — any improvement is real signal
- Raw model accuracy as the keep/discard metric (no Sharpen rules)
- Data loaded via Python FTMB reader from canonical `.ftmb` file
- Lives in `finetype/research/` subdirectory
- Single best architecture as deliverable

### Success Criteria
- Research loop runs autonomously overnight without human intervention
- Agent produces a ranked results.tsv logging all experiments
- Winning architecture achieves higher raw model accuracy than v7 baseline (172/214 = 80.4%) within a 10-minute training window
- Architecture is transferable to Rust (documented layer structure, dimensions, forward pass)
- program.md includes branch diagnostic evidence to guide agent exploration

### Open Questions
- Exact structure of the eval FTMB file (how to pre-extract 214 eval column features)
- Whether to include the branch probe results.json as context for the agent
- Git branch naming convention for experiment sessions
- Inference time logging format in results.tsv (ms/sample? ms/batch?)
