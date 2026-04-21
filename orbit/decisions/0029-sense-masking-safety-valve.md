---
status: accepted
date-created: 2026-03-01
date-modified: 2026-03-11
---
# 0029. Sense masking safety valve — fallback when confidence <0.75 and >40% votes removed

## Context and Problem Statement

The Sense→Sharpen pipeline uses Sense's broad category prediction to mask CharCNN votes — only types within the predicted category are considered. This dramatically improves accuracy when Sense is correct, but when Sense misroutes (predicts the wrong category), masking removes the correct type entirely, causing a guaranteed misclassification.

The question: when should the pipeline distrust Sense and fall back to unmasked votes?

## Considered Options

- **Always mask (Sense is authoritative)** — Simple but fails catastrophically when Sense is wrong. No recovery path.
- **Confidence-only threshold** — Fall back to unmasked when Sense confidence < threshold. Misses cases where confidence is moderate but masking is still harmful.
- **Dual-condition safety valve** — Fall back when *both* Sense confidence < 0.75 *and* masking removes > 40% of CharCNN votes. Catches the specific failure mode: Sense is uncertain AND masking is aggressive.

## Decision Outcome

Chosen option: **Dual-condition safety valve (confidence < 0.75 AND > 40% votes removed)**, because it targets the specific failure mode without being overly conservative. The two conditions must both be true:

1. Sense confidence < 0.75 — the category prediction is uncertain
2. Masking removes > 40% of CharCNN votes — the mask is aggressive enough to risk removing the correct type

Additionally, if masking removes ALL votes (empty mask), always fall back to unmasked regardless of confidence.

### Consequences

- Good, because Sense misrouting is caught before it causes guaranteed misclassification
- Good, because the dual condition is conservative — high-confidence Sense predictions always mask, even if aggressive
- Good, because unmasked fallback preserves the pre-Sense accuracy baseline as a floor
- Bad, because the thresholds (0.75, 40%) were empirically calibrated on current data — may need recalibration after model retraining
- Neutral, because the safety valve fires rarely in practice (~5% of columns in profile eval) — most Sense predictions are confident and correct
