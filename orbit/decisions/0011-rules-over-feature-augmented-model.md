---
status: accepted
date-created: 2026-03-08
date-modified: 2026-03-11
---
# 0011. Rules over feature-augmented model — feature_dim=0 + post-vote rules

## Context and Problem Statement

FineType's feature extractor (NNFT-247-250) computes 36 deterministic features per value (parse tests, character statistics, structural patterns). Two strategies for using these features were evaluated:

1. Fuse features into CharCNN at training time (`feature_dim=32/36`), letting the model learn to use them
2. Keep `feature_dim=0` (pure character CNN) and apply features as post-vote disambiguation rules (F1–F5)

A spike (NNFT-253) trained CharCNN v15 with `feature_dim=32` and compared against v14 with `feature_dim=0` + rules.

## Considered Options

- **Feature-augmented CharCNN (feature_dim=32)** — Training accuracy rose +5pp (86.6% → 91.6%), but profile eval *fell* -1.6pp (178/186 → 175/186). The model developed a "city attractor" — 6 columns incorrectly predicted as `city` due to overfitting on character statistics shared between city names, person names, and other short text.
- **Feature_dim=0 + post-vote rules (F1–F5)** — CharCNN operates on character patterns only. Deterministic features applied after vote aggregation for specific confusable type pairs. Combined with expanded header hints (NNFT-254): cross-domain hardcoded hint override with domain-aware thresholds (0.85 cross-domain, 0.5 same-domain).

## Decision Outcome

Chosen option: **Feature_dim=0 + post-vote rules**, because the empirical evidence shows that feature fusion causes cross-domain regressions while post-vote rules are surgically scoped and transparent.

The 5 rules:
- **F1** — Leading zeros present → upgrade postal_code/cpt to numeric_code (preserve as VARCHAR)
- **F2** — Slash-segment count → docker_ref over hostname
- **F3** — Digit ratio + dots + float parseability → hs_code disambiguation (with negative-prefix guard and dot-variance confidence check)
- **F4** — Zero length-variance + all hex + len=40 → git_sha over hash
- **F5** — numeric_code without leading zeros → demote to integer_number

Profile eval: 180/186 (96.8% label, 98.4% domain) — best achieved.

### Consequences

- Good, because rules are transparent and debuggable — each rule has a clear scope and can be independently verified
- Good, because no model retraining risk — rules operate post-vote on the proven CharCNN v14
- Good, because column-level feature aggregation (mean, variance, min, max via NNFT-266) enables distributional reasoning the model can't do
- Bad, because the feature fusion architecture (NNFT-248) is architecturally sound but empirically worse — the regression is likely a training data problem that *could* be solved with more data or feature selection
- Neutral, because future work could revisit feature fusion with selective features (only parse-test features, not character statistics) or curriculum training
