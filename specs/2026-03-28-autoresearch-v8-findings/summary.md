# Autoresearch v8: Architecture Search Findings

**Date:** 2026-03-28
**Duration:** ~12 hours (10:00–22:00)
**Experiments:** 29 (8 Opus + 21 Sonnet agents, plus 1 manual full-training run)
**Branch:** `feat/autoresearch-v8-sonnet`
**Baseline:** 108/214 (v7 early-fusion Python baseline, 20-min budget)

---

## Executive Summary

The autoresearch agent ran 29 experiments exploring architecture modifications to
the multi-branch fusion model. All agent-initiated experiments explored "drop char
branch" variants. **Best result: 126/214** (commit `09db63c`), an improvement over
the 108/214 baseline but well below the 172/214 Rust production model.

Error analysis proved **dropping char is a dead end** — the char branch carries
essential signal for ~35 type categories. The recommended next direction is
**late fusion with all 4 branches**, which the agent never explored.

---

## Top 5 Experiments (by eval_label_accuracy)

```
| Commit  | Eval Label | Eval Domain | Val Acc | Params | Description                                            |
|---------|-----------|-------------|---------|--------|--------------------------------------------------------|
| 09db63c | 126/214   | 167/214     | 0.8911  | 679K   | no char + slim embed (150) + plain L1 + residual L2    |
| 84fbe56 | 125/214   | 169/214     | 0.8893  | 880K   | no char + residual skip connections in trunk            |
| a6e56ab | 125/214   | 164/214     | 0.8907  | 715K   | no char + plain L1 then one residual L2                |
| 186ecf0 | 125/214   | 162/214     | 0.8907  | 746K   | embed (300,150) wider first hidden + residual          |
| bf94367 | 124/214   | 165/214     | 0.8905  | 972K   | no char + residual + wider stats branch (256,128)      |
```

## Full Training Ceiling (30 epochs)

The winning architecture (09db63c) was retrained for 30 uncapped epochs:
- **Result:** 121/214 eval_label (val_accuracy 0.8938 at epoch 25)
- **Verdict:** More epochs didn't help eval — the 126/214 at 18 epochs was likely
  a lucky eval split. The architecture's ceiling is ~125/214.

## What the Agent Explored

All 29 experiments were variants of "drop the char branch":
- Trunk modifications: residual connections, wider/deeper, funnel, dual-projection
- Activation functions: ReLU, GELU, SiLU
- Branch modifications: wider/slimmer embed, wider stats, boosted header
- Normalization: BatchNorm placement, LayerNorm in branches
- Fusion: hierarchical (semantic group first), attention gating
- Ablation: drop char+stats (embed+header only), tiny char (64,50)

## What the Agent Did NOT Explore

The diagnostic evidence (program.md) recommended these approaches in order:
1. **Late fusion with per-branch classification heads** — never tried
2. **Attention-based fusion (all 4 branches)** — never tried with all 4
3. **Branch confidence gating** — never tried
4. **Hierarchical fusion (all 4 branches)** — tried once without char
5. **Ensemble/voting** — never tried

The agent fixated on item #6 ("drop weakest branch") from the directive list,
which was intended as a minor ablation, not the primary direction.

## Error Analysis: Why Dropping Char Fails

The no-char model's 88 errors cluster into clear categories:

**Types that need char-level features (35+ categories):**
- Coordinates (lat/lon patterns), postal codes (digit structure)
- URLs, IP addresses, MAC addresses (delimiter patterns)
- Hashes (hex character distribution), UUIDs
- Phone numbers, credit cards (digit grouping)
- Dates in various formats (separator patterns)

**Key confusion patterns without char:**
- `integer_number` → `entity_name` (4 instances) — char distinguishes numeric patterns
- `region` → `full_name` (3) — char sees capitalization patterns
- `postal_code` → `integer_number` (2) — char sees digit structure
- `longitude` → `ndc` (2) — char sees decimal patterns

**Per-domain impact:**
- Finance: 40.0% accuracy (worst rate — credit cards, currencies need char)
- Geography: 48.2% accuracy (worst absolute — coordinates, postal codes need char)

## Operational Learnings

### Agent Discipline Issues
1. **Opus was too slow** — 8 experiments in 8 hours (30+ min thinking between runs)
2. **Opus hallucinated** — claimed "72 experiments" and "146/214 best" in exit report
3. **results.tsv constantly clobbered** — agent reset it before each run
4. **Wrong exploration direction** — both Opus and Sonnet fixated on "drop char"

### Structural Fixes That Worked
1. **Auto-logging to JSONL** — `experiment_log.jsonl` (append-only, gitignored).
   Agent can't clobber it because it doesn't know about it. 29/29 experiments logged.
2. **20-minute training budget** — doubled from 10 min, gave 15-18 epochs vs 7.
3. **Sonnet over Opus** — 3x faster iteration (21 experiments in ~10 hours).

### What Didn't Work
1. **Directive prompts** — ordering recommendations 1-5 didn't prevent the agent
   from fixating on option 6. The agent found early success with "no char" and
   kept hill-climbing in that local optimum.
2. **Convergence detection** — "40 consecutive non-improvements" threshold was never
   reached because the agent found small variations that occasionally beat baseline.

## Recommendations for Next Session

1. **Try late fusion with all 4 branches** — per-branch classification heads that
   each predict 239 classes independently, then a learned ensemble combines the
   probability distributions. This is the #1 recommendation from diagnostic evidence
   and was never attempted.

2. **Consider direct architecture specification** — rather than autonomous search,
   implement specific architectures manually based on the diagnostic evidence:
   - Per-branch heads + weighted ensemble
   - Attention-based branch weighting (per-sample)
   - Header-branch confidence gating

3. **The Python research loop has value but needs guardrails** — the auto-logging
   and JSONL tracking worked well. The search direction needs tighter constraints
   or manual steering checkpoints.

4. **Remember the production numbers** — Rust v7 model: 172/214 raw, 187/214 with
   Sharpen. The Python research baseline (108/214) is lower due to different data
   splits and no Sharpen post-processing. Apples-to-apples comparison requires
   running the winning Python architecture through the Rust eval pipeline.
