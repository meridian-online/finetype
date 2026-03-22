# Interview: Overnight Training Run v2

**Date:** 2026-03-22
**Interviewer:** Nightingale

---

## Context

Setting up an overnight training run for the multi-branch model at scale (300k+ samples, 20+ epochs) with a training TUI dashboard inspired by Burn's ratatui renderer. Covers milestones M-3 through M-6 in a single automated pipeline.

## Interview Q&A

### Q1: TUI interaction mode
**Q:** Should the TUI be interactive (keyboard controls) or display-only?
**A:** Display-only. Safe for overnight/unattended runs. No keyboard capture.

### Q2: TUI default behaviour
**Q:** Should the TUI be default or opt-in?
**A:** Default on, `--no-tui` flag to disable.

### Q3: Label remap table location
**Q:** Should the 29 non-canonical distilled label mappings live in a separate file or inline?
**A:** Separate JSON file (`data/label_remap.json`).

### Q4: Training scope
**Q:** Train flat only or both flat + hierarchical?
**A:** Both flat + hierarchical in the same overnight run.

### Q5: TUI panels
**Q:** What should the training dashboard show?
**A:** Three panels: loss/accuracy chart (train + val), progress bar with ETA, epoch summary table.

### Q6: Eval in overnight script
**Q:** Include eval after training or train only?
**A:** Train + eval. Run `scripts/eval.sh` after each model. Wake up to accuracy numbers.

---

## Summary

### Goal
Overnight script that: (1) prepares 300k+ column-level training data with label remapping, (2) trains flat and hierarchical multi-branch models with 20+ epochs, (3) evaluates both against Tier 1 profile eval, (4) displays live training progress via ratatui TUI dashboard.

### Constraints
- Display-only TUI (no keyboard capture), default on, `--no-tui` to disable
- Label remap table as `data/label_remap.json`
- Both flat + hierarchical heads trained
- Runs unattended on M1 Pro with Metal acceleration
- Total time budget: ~3 hours (data prep ~45min, flat ~60min, hier ~90min, eval ~10min)
- Acknowledge Burn (tracel-ai) in README for TUI inspiration

### Success Criteria
- Script produces two evaluated models with Tier 1 accuracy scores
- TUI shows live training curves, progress bar with ETA, epoch table
- Label remap recovers ~131 columns of distilled training data
- Training data reaches 300k+ records across 249+ types

### Open Questions
- None — ready to spec and implement
