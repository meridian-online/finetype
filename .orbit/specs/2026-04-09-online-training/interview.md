# Design: Online Training (Autoresearch on RunPod)

**Date:** 2026-04-09
**Interviewer:** Nightingale
**Card:** .orbit/cards/0002-semantic-type-detection.yaml

---

## Context

Card: *Semantic type detection* — 4 scenarios, goal: close 155/190 → 170+/190 profile eval accuracy. References: multi-branch pipeline integration spec, overnight v6 data quality spec, Sherlock architecture spec.

Inspiration: [karpathy/autoresearch](https://github.com/karpathy/autoresearch) — autonomous experiment loop where an agent modifies code, trains for a fixed time budget, keeps or discards based on metric, repeats indefinitely.

## Evidence Brief

Prior training runs inform the design constraints:

- **v4 baseline:** 121/190 raw → 155/190 post-Sharpen (81.6%)
- **v5 overnight (8h, M1 Pro Metal):** 154/190 raw (+33 columns), but 154/190 post-Sharpen (regression by 1) — error profile shifted incompatibly with v4-tuned Sharpen rules
- **Data quality wall:** 92% of types <20% distilled data, 50% have zero distilled data. Augmentation delivered 12.2% vs 35% target (double-gate bug). Oversampling undershot.
- **Decision 0038:** Prefer retraining over new rules. 5 superseded PR-2 fixes become test cases.
- **Decision 0041:** Multi-branch is the default classifier (replaces Sense + CharCNN).
- **Decision 0042:** Remove regex header hints in favour of learned approaches.
- **RunPod account:** $0 balance, $80 spend limit. RTX 4090 at ~$0.40/hr = ~200 GPU hours max.

Known accuracy gaps: numeric_code false positives (F5), entity confusion, date format ambiguity.

## Q&A

### Q1: What should the agent be allowed to modify?
**Q:** What should the agent be allowed to modify between experiments? Config-only is safest but limits the search space. Config + data pipeline captures where v5→v6 made gains. Full Rust source is powerful but compilation errors burn paid GPU time.
**A:** Config + data + architecture — full autoresearch scope. Safety via compile gate before GPU training. (Made practical by Q2's PyTorch decision.)

### Q2: Research language — PyTorch or Rust/Candle?
**Q:** (Volunteered by Hugh before being asked.) Earlier overnight trainings established a pattern: design and validate architecture in PyTorch, then port to Candle for release.
**A:** PyTorch for research, Candle for release. This eliminates the Rust compilation cost concern entirely. The autoresearch agent modifies Python, just like Karpathy's original.

### Q3: Where does the orchestrating agent run?
**Q:** Local Mac (agent thinks for free, pays only for GPU training) vs on-pod (simpler but GPU idles during think time)?
**A:** Local agent, remote GPU. Claude Code runs on Mac, edits configs/code locally, pushes to pod, kicks off training via SSH, reads back results. GPU only runs during actual training.

### Q4: What's the primary metric?
**Q:** Training val_accuracy (immediate), profile eval (real target, +5 min), or both?
**A:** Two-tier: val_accuracy as fast keep/discard gate during the loop. Full profile eval only on 'keep' results as confirmation. Alert if proxy drifts from real eval.

### Q5: Time budget per experiment?
**Q:** 5 min (max throughput), 10 min (balanced), or 20 min (full convergence)?
**A:** 10 min fixed training + ~2 min eval overhead = ~12 min per experiment. Gives ~40 experiments overnight at ~$3.20 total GPU cost.

### Q6: Code structure?
**Q:** Single-file train.py (Karpathy pattern) or multi-file with framework?
**A:** Single-file `train.py` (~300 lines) with the multi-branch model, optimizer, training loop, and eval. Agent modifies this file. Fixed `prepare.py` for data loading and eval harness. Human-edited `program.md` for agent instructions.

### Q7: Search focus?
**Q:** Broad exploration from the start, or staged from known gaps to radical?
**A:** Staged: Phase 1 (experiments 1–15) targets known gaps (data mix, augmentation, LR, dropout). Phase 2 (16–30) explores architecture (branch dims, merge strategy, loss functions). Phase 3 (31+) goes radical (attention, contrastive learning, novel approaches).

### Q8: Pod lifecycle?
**Q:** Semi-automatic (start/stop scripts), fully automatic (agent manages everything), or manual?
**A:** Fully automatic. Agent provisions pod via API, waits for ready, syncs code, runs experiments, syncs results on completion/timeout, terminates pod, handles preemption with auto-restart. Human just says "start autoresearch."

### Q10: Where does the agent run overnight?
**Q:** The "local agent" design assumed your Mac stays awake. If you close the laptop, the loop dies. Where does the agent actually run for overnight autonomy?
**A:** Beelink Mini PC. It's always-on, already running Claude Code (this session is on the Beelink), has the finetype repo checked out, and SSH configured. The agent edits train.py locally on the Beelink, syncs to the RunPod pod via SSH, kicks off training, reads back results. Mac can sleep. GPU pod is purely a compute slab. No additional setup needed on the Beelink side.

### Q9: Synthetic data for types not in Sherlock?
**Q:** Pre-generate + upload to HF, build finetype on pod, or cache on network volume?
**A:** Pre-generate locally using `finetype generate`, upload to HuggingFace as `meridian-online/finetype-synthetic`. Pod pulls both datasets (sherlock-annotated + finetype-synthetic) via HF. No Rust toolchain needed on the pod. Clean separation of data prep from training.

---

## Summary

### Goal
Establish an autonomous experiment loop (autoresearch) that runs overnight on RunPod, exploring model architecture, hyperparameters, and data strategies to close the 155/190 → 170+/190 profile eval accuracy gap for FineType's multi-branch semantic type classifier.

### Constraints
- $40 RunPod budget for first training session (~100 GPU hours on RTX 4090)
- 10 min fixed training time budget per experiment
- PyTorch for research; Candle port for release
- Single-file `train.py` — the only file the agent modifies
- Training data from HuggingFace (sherlock-annotated + finetype-synthetic)
- No multi-GPU, no distributed training
- Model budget: 10–50 MB final artefact
- Beelink Mini PC orchestrates (always-on), remote GPU trains
- SSH + rsync for code sync and result retrieval

### Success Criteria
- Autoresearch loop runs autonomously overnight (~40 experiments)
- val_accuracy improvement tracked per experiment with keep/discard decisions
- Profile eval confirmation on kept models
- At least one model exceeds 155/190 baseline on profile eval
- Stretch: model reaches 170+/190 on profile eval
- Full results.tsv log of all experiments with metrics

### Decisions Surfaced
- **Full search scope**: config + data + architecture, all via `train.py` edits (enabled by PyTorch decision)
- **PyTorch for research, Candle for release**: fast iteration in Python, port stable architecture to Rust
- **Beelink orchestration**: agent runs on always-on Beelink Mini PC, dispatches GPU work to RunPod via SSH — Mac can sleep, no GPU idle during think time
- **Two-tier metric**: val_accuracy for fast gate, profile eval for confirmation on keeps
- **10 min time budget**: ~40 experiments overnight, balanced convergence vs throughput
- **Autoresearch pattern**: single-file `train.py` + fixed `prepare.py` + human `program.md`
- **Staged search**: known gaps → architecture → radical departures
- **Fully automatic pod lifecycle**: agent provisions, runs, tears down
- **Pre-generated synthetic data on HuggingFace**: clean data/training separation

### Open Questions
- Exact feature extraction pipeline for `prepare.py` — does it replicate the Candle FTMB format or use a simpler PyTorch-native approach?
- How to run profile eval from the local Mac against a model trained in PyTorch (needs a bridge to the Candle/CLI eval pipeline)
- Network volume region selection (should match cheapest 4090 availability)
- Whether the agent should use W&B or keep metrics local to network volume
- Preemption handling details — how to detect and auto-restart cleanly
