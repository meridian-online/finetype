---
status: accepted
date-created: 2026-04-21
date-modified: 2026-04-21
---
# 0061. Sweep Data-Seed Discipline — Fix Prep, Vary Training

## Context and Problem Statement

Sweep scripts at `scripts/sweep_v16.sh` and `scripts/sweep_v17.sh` pass the sweep's training seed (`$SEED`) to `scripts/prepare_multibranch_data.py` via `--seed $SEED`. The prep script threads that seed through its RNGs (lines 2592, 1468, 2810, 2824, 2837 of `prepare_multibranch_data.py`), producing different training data per training seed.

The effect is twofold:

1. **Redundant cost per sweep.** Each training seed re-runs the full prep pipeline: ~30–60 min wall-clock × 3 seeds ≈ 60 min redundant runtime + ≈ 1.9 GB redundant disk (three `.ftmb` files differing only in randomness). Measured on v17's `sweep_v17.sh` invocation history.
2. **Signal conflation.** A 3-seed sweep is meant to measure training-stability variance (weight-init + batch-order randomness). Per-seed prep contaminates that signal with data-distribution variance (sampling, synthetic generation order, loader shuffling). Gate decisions become ambiguous: is the variance we're seeing a property of training, or of the sampled corpus?

Both symptoms are load-bearing for the gate-promotion calculus (decision 0053: train-gate floor 88%, auto-accept 91.2%) and for v17's diagnostic trust (decision 0054: held on net-zero eval delta, but was "net-zero" partly a seed-sampled data variance?).

## Considered Options

- Option A — keep per-seed prep (status quo). Every seed gets its own data.
- Option B — fix data seed, vary training seed only. One prep run per sweep; all training seeds share a single `.ftmb`.
- Option C — treat data and training as separate sweep axes. Run e.g. 2 data seeds × 3 training seeds = 6 runs.

## Decision Outcome

Chosen option: **Option B (fix data seed, vary training only)**, because it eliminates the ≈ 60 min + 1.9 GB redundancy per sweep AND cleans the 3-seed variance signal at zero statistical cost. Option C doubles sweep wall-clock without clear benefit when data-variance is not the question; Option A is what we're already paying for with no offsetting gain.

### Implementation

- `scripts/sweep_v18.sh` (ac-02) invokes `prepare_multibranch_data.py --seed 42` exactly once per sweep, writing `output/multibranch-training/v18.ftmb`.
- The training loop then invokes `cargo run -- train-multi-branch --seed {42,43,44}` three times pointing at the same `.ftmb`.
- `DATA_SEED=42` is a load-bearing spec constraint (spec v1.3, constraints #1 and #13).

### v17 Waste Quantification

- **Wall-clock:** per-seed prep ≈ 30–60 min on the v17 sweep corpus. 3-seed sweep × 2 redundant preps = **+60 min** wall-clock (compared to single prep + 3 training seeds).
- **Disk:** each `.ftmb` ≈ 950 MB (v17 corpus). 3-seed-per-prep sweep stores 3 copies, only one of which is ever needed per training run. **+1.9 GB** redundant storage.
- **Training-signal ambiguity:** cannot measure training-only variance from v17's `sweep_v17.sh` output without reconstructing the prep seed per training run. The conflation is empirically observable in v17's epoch metrics — per-seed prep produces different `pre_filter_rows` / `post_filter_rows` counts (trackable via m-19 Phase A+B logging, but not at v17's time).

### Consequences

**Good, because**:

- Sweep runtime reduces by ≈ 60 min (≈ 1/6 of a 6h sweep).
- Disk usage reduces by ≈ 1.9 GB per sweep.
- 3-seed variance is now a clean training-stability signal. Promotion-gate decisions (decision 0053's auto-accept 91.2%) can be made against a single well-defined corpus.
- Debuggability improves — re-running a single training seed against the shared `.ftmb` is reproducible without re-prepping.
- Ties into decision 0056 (leakage prevention) — single prep invocation means row-hash firewall only verifies once per sweep, reducing the attack surface for "filter silently disabled on one of three prep runs" divergence.

**Bad, because**:

- Forfeits any noise-reduction benefit from averaging over data-seed variance. The v18 gate is evaluated against a single data sample; if that sample is unlucky, the variance signal won't surface it. Mitigation: the `.ftmb` is stored with its seed in the path (`v18.ftmb`), and future sweeps can vary `DATA_SEED` in a separate axis if data-variance is suspected.
- Breaks the implicit "each seed reruns everything" mental model that v16/v17 scripts established. Documentation in `scripts/sweep_v18.sh` header makes the new convention explicit.

### Adopters

- **v18 (first adopter)** — `scripts/sweep_v18.sh` + spec v1.3.
- **All future sweeps** — the expectation is that sweep_v19.sh and later inherit this pattern unless a specific sweep needs to measure data-variance, in which case the deviation is recorded in a new MADR.
