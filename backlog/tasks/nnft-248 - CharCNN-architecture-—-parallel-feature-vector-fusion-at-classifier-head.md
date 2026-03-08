---
id: NNFT-248
title: CharCNN architecture — parallel feature vector fusion at classifier head
status: To Do
assignee: []
created_date: '2026-03-07 23:55'
labels:
  - model
  - architecture
milestone: m-12
dependencies:
  - NNFT-247
references:
  - crates/finetype-model/src/char_cnn.rs
  - crates/finetype-train/src/training.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Modify the CharCNN architecture to accept a parallel feature vector alongside the learned character embeddings. The feature vector is concatenated with the CNN output **at the classifier head** (not at input embedding level).

Architecture change:
```
Input string → CharCNN conv layers → cnn_embedding (N dims)
Input string → Feature extractor → feature_vector (~30 dims)
[cnn_embedding ∥ feature_vector] → fusion layers → classifier → type prediction
```

The fusion layers are 1-2 fully connected layers that take the concatenated vector and produce the final logits. Both the CNN and the fusion layers train end-to-end jointly.

Also explore wider filters and deeper CNN layers within the 50MB binary budget.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CharCNN forward pass accepts both character input and feature vector
- [ ] #2 Feature vector concatenated with CNN output before classifier head (not at embedding)
- [ ] #3 Fusion layers (1-2 FC layers) connect concatenated vector to output logits
- [ ] #4 Model trains end-to-end — CNN weights and fusion weights update jointly
- [ ] #5 Architecture supports both flat (250-class) and tiered model variants
- [ ] #6 Compiled model + binary stays under 50MB
- [ ] #7 Backward-compatible model loading — can still load old CharCNN weights (feature vector zeroed/absent)
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Tests pass — cargo test + taxonomy check (cargo run -- check) confirm no regressions
- [ ] #2 Final Summary written (PR-quality — what changed / why / impact / tests)
- [ ] #3 CLAUDE.md updated if Current State / Architecture / Priority Order affected
- [ ] #4 Decision record created if plan involved choosing between approaches
- [ ] #5 Daily memory log updated with session outcomes
- [ ] #6 Changes committed with task ID in commit message
<!-- DOD:END -->
