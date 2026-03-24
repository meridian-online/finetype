---
status: accepted
date-created: 2026-03-24
date-modified: 2026-03-24
---
# 0042. Remove regex header hints in favour of learned approaches

## Context and Problem Statement

FineType has two header hint systems: hardcoded exact/substring match rules (`header_hint()` in semantic.rs, ~200 lines) and Model2Vec semantic similarity matching. Decision 0028 established hardcoded-first priority. Decision 0006 noted that "the Sense model will eventually subsume both hint systems by treating the header as a direct model input."

The multi-branch model now includes a dedicated header branch (128-dim Model2Vec embedding). Sibling-context attention (NNFT-268) enriches headers with cross-column context. Both are learned approaches that handle novel headers the regex system cannot. Continuing to maintain regex header hints alongside these learned systems creates competing signal sources and debugging complexity.

## Considered Options

- **Keep regex header hints** — Continue maintaining hardcoded rules alongside learned approaches. Known-good for covered headers but doesn't generalise, creates conflicts with model predictions, and is a maintenance burden.
- **Remove regex header hints, rely on learned approaches** — The multi-branch header branch, Model2Vec semantic matching, and sibling-context attention handle header signal. Remove regex-based `header_hint()` and associated hardcoded rules.
- **Gradual deprecation** — Keep regex hints as fallback but deprioritise behind learned approaches. Risk of them interfering with model predictions.

## Decision Outcome

Chosen option: "Remove regex header hints, rely on learned approaches", because:

1. Regex header hints are a maintenance rabbit hole — each new header pattern requires manual curation and risks unintended interactions.
2. The multi-branch header branch learns header→type associations from training data, generalising to novel headers.
3. Sibling-context attention provides cross-column disambiguation that regex hints cannot.
4. Model2Vec semantic similarity remains as a learned fallback for novel headers.
5. This aligns with decision 0038's "strength through simplification" — fewer hand-crafted heuristics, stronger models.

The removal should be coordinated with the multi-branch-as-Sense integration (decision 0041). Regex hints are removed when the multi-branch model handles header signal at least as well as the current hint system.

### Consequences

- Good, because the header signal path is simplified to a single learned system
- Good, because novel/unseen headers are handled by generalisation, not manual curation
- Good, because debugging is clearer — header influence comes from model weights, not regex rules
- Bad, because some well-known headers (email, phone, postal_code) may temporarily lose precision during transition
- Mitigation: validate against the 190-column eval before removing hints; if specific headers regress, add them to training data rather than re-adding rules
