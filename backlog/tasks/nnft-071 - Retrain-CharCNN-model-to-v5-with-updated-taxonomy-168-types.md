---
id: NNFT-071
title: Retrain CharCNN model to v5 with updated taxonomy (168 types)
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-15 10:04'
updated_date: '2026-02-15 10:04'
labels:
  - model
  - training
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Retrain the CharCNN model with the expanded taxonomy (168 types, up from 159/v4) including new generators from NNFT-055 (phone numbers), NNFT-056 (addresses), NNFT-059 (excel formats), and NNFT-063 (categorical, ordinal, boolean, alphanumeric_id).

Training config: batch_size=64, epochs=10, lr=1e-3, max_seq_length=128, embed_dim=32, num_filters=64, hidden_dim=128. Generated 83,000 training samples (500/label, priority >= 1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 New model trained with all 168 taxonomy types
- [x] #2 Model saved to models/char-cnn-v5/
- [x] #3 Default symlink updated to char-cnn-v5
- [x] #4 All 169 tests pass
- [x] #5 Generator-taxonomy alignment 100% (8400/8400)
- [x] #6 Inference produces correct results on spot checks
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Retrained CharCNN model from v4 (159 classes) to v5 (168 classes) with improved training data.

Changes:
- Generated 83,000 training samples (500/label × 166 types at priority >= 1)
- Trained char-cnn-v5 for 10 epochs: loss 2.03→0.24, accuracy 49.2%→90.1%
- Updated models/default symlink: char-cnn-v4 → char-cnn-v5
- Model size: 347KB weights, 5.7KB labels, 117B config

Training progression:
| Epoch | Loss  | Accuracy |
|-------|-------|----------|
| 1     | 2.033 | 49.17%   |
| 2     | 0.609 | 79.05%   |
| 3     | 0.434 | 84.08%   |
| 5     | 0.308 | 87.65%   |
| 7     | 0.272 | 89.11%   |
| 10    | 0.240 | 90.09%   |

New types now classifiable:
- representation.file.excel_format (100% confidence on "#,##0.00")
- representation.code.alphanumeric_id (99% confidence on "SKU-12345-A")
- representation.discrete.categorical, ordinal, boolean
- technology.code.isbn, issn, doi

Verification:
- 169 tests pass (73 core + 96 model)
- 168/168 definitions, 8400/8400 samples validated
- Spot-check inference: email 99.8%, phone 100%, IPv4 99.98%, ISO date 99.9%
<!-- SECTION:FINAL_SUMMARY:END -->
