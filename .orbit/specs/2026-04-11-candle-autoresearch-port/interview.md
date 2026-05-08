# Design: Candle Port of Autoresearch Findings

**Date:** 2026-04-11
**Interviewer:** Nightingale
**Card:** .orbit/cards/0002-semantic-type-detection.yaml

---

## Context

Card: *Semantic type detection* — 4 scenarios, goal: 155/190 → 170+/190 profile eval
Prior specs: 4 — sherlock architecture (shipped), pipeline integration (shipped at 155/190), data quality (fixes landed), autoresearch (80 experiments, 92.4%→96.6% val_accuracy, PR #32 open)
Gap: Autoresearch found 5 architecture improvements in PyTorch. They need porting to Candle, retraining, and profile eval validation.

## Q&A

### Q1: Model scale
**Q:** The autoresearch used smaller dimensions [300,300]/[200,200] than production [450,450]/[300,300]. Port findings onto production scale or reset to smaller?
**A:** Production scale. The autoresearch validated activations and normalization, not dimensions. Keep the capacity tuned for 250 classes.

### Q2: Normalization strategy
**Q:** Replace all BatchNorm with LayerNorm, or keep BatchNorm in merge and add LayerNorm after?
**A:** Replace BatchNorm with LayerNorm entirely. Cleaner code, matches autoresearch winner, removes the batch dimension squeeze/unsqueeze workaround.

### Q3: Training hardware
**Q:** Mac with Metal or another RunPod session?
**A:** Mac with Metal. Proven for prior overnight runs, no cloud cost, zombie protection not yet fixed.

### Q4: Backward compatibility
**Q:** Adding `use_layer_norm` and `activation` fields changes the config schema. How to handle old models?
**A:** Serde defaults. New fields get `#[serde(default)]` — old configs deserialize with ReLU+BatchNorm defaults. New configs opt into GELU+LN. Zero breakage.

### Q5: Validation gate
**Q:** What profile eval threshold to gate HuggingFace publication?
**A:** ≥160/190. Must beat current 155/190 by a meaningful margin. If it regresses, don't ship.

---

## Summary

### Goal
Port 5 autoresearch findings to Candle multi-branch trainer, retrain on Mac with Metal, validate via profile eval, publish to HuggingFace if ≥160/190.

### Constraints
- Production-scale config (sherlock-v5-scaled dimensions)
- Single file change: `crates/finetype-train/src/multi_branch.rs`
- Backward compatible via serde defaults
- Mac Metal training (no cloud)
- Header branch retained (with real Model2Vec features)

### Success Criteria
- Profile eval ≥160/190 label accuracy (currently 155/190)
- No regression in actionability eval
- Model published to HuggingFace for DuckDB extension

### Decisions Surfaced
- **LayerNorm replaces BatchNorm**: chose full replacement over additive (BatchNorm+LN). Rationale: matches autoresearch winner exactly, removes batch dimension workaround, simpler code.
- **Production scale over autoresearch dimensions**: findings are about activations/normalization, not width. Keep [450,450]/[300,300] capacity.
- **Serde defaults for compatibility**: `#[serde(default)]` on new config fields. Old models load unchanged, new models opt in.
- **≥160/190 publish gate**: conservative — ship improvement, iterate toward 170+ in follow-up sessions.

### Open Questions
- None — scope is tight.
