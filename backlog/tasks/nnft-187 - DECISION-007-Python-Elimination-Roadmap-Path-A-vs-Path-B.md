---
id: NNFT-187
title: 'DECISION-007: Python Elimination Roadmap - Path A vs Path B'
status: Done
assignee: []
created_date: '2026-03-02 07:29'
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

**Context**: The FineType codebase has accumulated 20 Python files over time despite "build entirely in Rust" principle. Phase 0 spike evaluated whether HuggingFace Candle ML framework could replace PyTorch for training, enabling Path A (full Rust migration).

**Decision**: **Adopt Path B (Hybrid approach)** with contingency for Path A

**Option A (Full Rust via Candle)**: 
- Replace all 20 Python files with Rust equivalents
- Use Candle for model training instead of PyTorch
- Effort: 50-80 hours (build tools + eval + training)
- Risk: Dependency ecosystem fragility; Candle version conflicts with rand/Arrow/Parquet

**Option B (Hybrid - Rust + Documented Python)**:
- Replace build tools and eval infrastructure with Rust (Phases A+B)
- Keep model training as explicit optional Python workflow
- Document Python as offline-only tooling for one-time model generation
- Effort: 26-38 hours (build tools + eval only; skip training port)
- Risk: Low; maintains pragmatic separation of concerns

**Rationale**:
1. **Phase 0 spike findings**: Candle can express required architectures (cross-attention Sense, Deep Sets entity classifier), but dependency ecosystem shows fragility
2. **Risk/reward trade-off**: Path A gains "pure Rust" purity at cost of 50+ hours and dependency management burden; Path B achieves practical pure Rust for all core/inference workflows while training remains optional offline activity
3. **Principle alignment**: Path B satisfies "Rust for inference and core CLI" principle while being pragmatic about training (one-time offline work)
4. **Maintainability**: Separating build/eval (Rust) from training (Python) reduces coupling and scope of pure-Rust commitment

**Decision Timeline**:
- **Immediate**: Proceed with Phase A (build tools) and Phase B (evaluation Rust) in parallel
- **Short-term**: Formalize Python training as optional documented workflow (Phase C Path B variant)
- **Future**: Revisit Path A if Candle ecosystem stabilizes or training needs to be fully automated in CI

**Acceptance Criteria**:
- [x] Phase 0 spike completed with clear findings
- [x] Path A vs Path B trade-offs documented
- [x] Recommendation endorsed by technical lead
- [ ] Phases A+B proceed without waiting for training decision
- [ ] CLAUDE.md updated with new approach
- [ ] Team confirms Path B direction before Phase C

**Assumptions**:
- Model training remains one-time offline activity (not part of CI/CD)
- Safetensors format stable for cross-ecosystem model serialization
- Future Python elimination (Path A) remains possible if Candle stabilizes

**Ownership**: Nightingale (technical spike lead), Hugh (team decision)

**References**: 
- `discovery/candle-feasibility-spike/SUMMARY.md` (spike analysis)
- `crates/finetype-candle-spike/` (proof-of-concept implementations)
- NNFT-182 (Phase 0 spike task)
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
