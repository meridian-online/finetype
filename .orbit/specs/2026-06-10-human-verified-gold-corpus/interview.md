# Discovery interview — human-verified gold corpus

**Date:** 2026-06-10
**Participants:** author (Hugh), agent
**Origin:** author direction — "we're not delivering on our vision… we need a breakthrough", naming three success criteria (convincing profile/validation results, very fast inference, simplified coherent codebase) and three concerns (artefact bloat, long code files, eval inaccuracies).

## Diagnosis presented and accepted

The three concerns are one disease: four failed retrains (v22/v23/v24/latdec) each left scaffolding behind — five mutually-suspicious proxy instruments, 110 Python scripts, 17 GB of eval artefacts, ~100 model directories, 13 open specs, ~77k lines of Rust (CLAUDE.md claimed ~20k). Root cause: no trusted ground truth. Every instrument is a proxy scored against another proxy; total human-verified labels in the project were 240 curated columns plus one unreviewed ~120-row sample. The breakthrough bet is the gold corpus, not another retrain or another gate.

## Questions and answers

**Q1. Gold corpus size / author review budget?**
A: "Please check for me." → The agent derives the size empirically in ac-01 (gated sizing memo): per-type quotas from an explicit CI target (≤5pp half-width per contested type ≈ 80–100 verified columns each, ~10–13 contested types from the campaign record plus a common-type backbone, landing ~1,200–1,500 columns total), with a hard author-adjudication cap of ~350 columns (~3 hours). The author reviews the memo before any build spend.

**Q2. Where do gold columns come from?**
A: "GitTables and some other open source datasets." → Mix: majority GitTables (reusing the ac-01 stratified-sample machinery, directly comparable to existing instrument history) plus ~30% fresh external open data (data.gov / Kaggle-class) to test generalisation beyond the corpus everything was tuned on. Leakage firewall (sources.yaml + row-hash check) applies to both.

**Q3. When does simplification start?**
A: "Safe cleanup now, risky after gold." → Companion spec `2026-06-10-fossil-cleanup` proceeds immediately for changes needing no eval evidence (artefact archive, stale-doc fixes, dirty-tree hygiene, author-signed model-dir pruning). Code deletions, dead-rule removal, and giant-file splits wait for gold as their regression gate (card 0019 scenario 5).

## Out of scope (named, not forgotten)

- **Inference speed**: listed as a success criterion but unmeasured — no benchmark evidence either way. A quick measurement belongs in fossil-cleanup follow-up or its own memo before assuming a problem.
- **Taxonomy size**: whether 240 types is itself part of the credibility gap (corpus is 52% plain integer/decimal; contested types are 0.0003% of columns). A "fewer types, rigorously validated" question for a later discovery once gold can measure per-type value.
