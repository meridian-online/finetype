# Interview: Training TUI Dashboard Improvements

**Date:** 2026-03-25
**Interviewer:** Nightingale

---

## Context

The overnight v4 training pipeline has a ratatui-based TUI dashboard for monitoring training progress. Before the next overnight run, we want to: (1) verify the TUI renders correctly, and (2) ship two improvements identified in the Burn reference analysis (`analysis.md` in this directory).

## Interview Q&A

### Q1: Primary pain point
**Q:** The analysis identified P1 (batch-level loss charts) and P3 (panic hook guard) as the immediate wins. But you mentioned wanting to 'test that they're effective' — what's the primary pain point driving this? Is it the 2-minute dead screen during epochs, or something else about how the overnight results get presented?
**A:** I just want a way to make sure the TUI meets our intentions, i.e. text is readable on the progress bar etc.

### Q2: Verification method
**Q:** So the core need is visual verification — making sure the TUI renders correctly before committing to an overnight run. How should we verify this? A short synthetic training run, a demo/mock mode, or a snapshot test?
**A:** Demo/mock mode — a `--demo` flag that feeds synthetic metrics to the TUI without actual training.

### Q3: Demo fidelity
**Q:** For the demo mode: should it simulate the full training lifecycle (progressing through epochs) or just render a static 'mid-training' snapshot with representative data?
**A:** Static snapshot — render one frame with realistic-looking data in all panels. Quick visual check that layout and text are correct.

### Q4: Scope
**Q:** Which TUI improvements to ship alongside demo mode?
**A:** Demo + P1 (batch-level loss charts) + P3 (panic hook guard). Moderate effort, addresses all immediate pain points.

### Q5: Layout for batch-level loss
**Q:** Should the batch loss sparkline replace the current epoch-level loss sparkline, or appear as an additional panel?
**A:** Replace epoch sparkline. Batch-level loss IS the loss chart. Epoch markers shown as vertical ticks or colour changes. Simpler layout, one chart to read.

### Q6: Demo mode entry point
**Q:** How should the demo mode be invoked?
**A:** Standalone script — a separate Rust binary or shell script that imports the TUI module and feeds it synthetic data. Doesn't touch training code.

---

## Summary

### Goal
Ship a standalone TUI demo mode for visual verification, plus two training TUI improvements (batch-level loss charts, panic hook guard) before the next overnight training run.

### Constraints
- Demo mode is a standalone script/binary, not a flag on `finetype train`
- Demo renders a static snapshot (one frame), not an animated simulation
- Batch-level loss replaces the epoch-level loss sparkline (not an additional panel)
- Panic hook is ~15 lines in `render_loop` — minimal scope
- Must preserve display-only mode (no `enable_raw_mode()`) for unattended overnight safety

### Success Criteria
- Demo mode renders all TUI panels with realistic data, text is readable
- Batch-level loss chart updates during epochs (eliminates 2-min dead screen)
- Panic hook restores terminal on crash
- Existing overnight pipeline still works unmodified

### Open Questions
- None — requirements are clear
