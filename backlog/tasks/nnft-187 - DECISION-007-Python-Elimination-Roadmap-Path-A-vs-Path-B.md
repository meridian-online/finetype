---
id: NNFT-187
title: 'DECISION-007: Python Elimination Roadmap - Path A vs Path B'
status: Done
assignee: []
created_date: '2026-03-02 07:29'
updated_date: '2026-03-02 08:48'
labels:
  - decision
  - architecture
  - python-elimination
  - pure-rust
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Decision Record: Pure Rust Return for FineType

**Context**: The FineType codebase has accumulated 20 Python files over time despite "build entirely in Rust" principle. Phase 0 spike evaluated whether HuggingFace Candle ML framework could replace PyTorch for training.

**Decision**: **Adopt Path A (Full Rust via Candle)** — confirmed by Phase 0 spike re-run

**Phase 0 Spike Results (Session 2)**:
- Candle 0.8.4 compiles with `half = "2.4"` pin (resolves known rand/half conflict)
- Sense Architecture A (cross-attention) and Entity classifier (Deep Sets MLP) both work
- 10/10 validation tests pass: forward pass, gradients, optimizer step, safetensors round-trip
- Cross-entropy loss expressible via log_softmax + gather
- Variable batch sizes (1-32) and sequence lengths (1-100) supported

**Option A (Full Rust via Candle) — CHOSEN**:
- Replace all 20 Python files with Rust equivalents
- Use Candle for model training instead of PyTorch
- Effort: 50-80 hours (build tools + eval + training)
- Dependency fix: pin `half = "2.4"` (community-documented workaround)

**Option B (Hybrid) — REJECTED**:
- Was recommended in Session 1 due to premature dependency failure analysis
- Reversed after Session 2 showed dependency fix is trivial

**Rationale**: The Candle dependency issue was a known, well-documented problem with a simple fix. All critical technical requirements validated: architecture expressiveness, gradient flow, optimizer updates, model serialization. Path A aligns with project principle of building entirely in Rust.

**Phases**: A (build tools, 6-8h) → B (eval Rust, 20-30h) → C (Candle training, 30-40h) → D (cleanup, 2-3h)
<!-- SECTION:DESCRIPTION:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
