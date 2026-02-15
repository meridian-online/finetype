---
id: NNFT-061
title: Train char-cnn-v4 and release v0.1.3 with 7 finance types
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-15 01:14'
updated_date: '2026-02-15 01:14'
labels:
  - model
  - release
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generate v4 training data (129K samples, 159 types), train char-cnn-v4 model, evaluate, update default symlink, bump version to 0.1.3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 v4 training data generated with 159 types
- [x] #2 char-cnn-v4 trained (5 epochs, 90%+ accuracy)
- [x] #3 Evaluation run on test set with results saved
- [x] #4 Default model symlink updated to v4
- [x] #5 Version bumped to 0.1.3
- [x] #6 CHANGELOG updated with v0.1.3 section
- [x] #7 All tests pass
- [x] #8 Committed and pushed
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Trained char-cnn-v4 model on expanded 159-type taxonomy and released v0.1.3.

Changes:
- Generated train_v4.ndjson (129K samples, 1000/type, priority >= 2, seed 44) and test_v4.ndjson (25.8K samples, seed 99)
- Trained char-cnn-v4: 5 epochs on CPU, final training accuracy 90.36%
- Test evaluation: 91.62% accuracy, 99.21% top-3, macro F1 91.3%
- New finance type performance: LEI 96.6% F1, currency_code 94.3%, SEDOL 89.9%, CUSIP 84.6%, SWIFT/BIC 73.5%, ISIN 62.7%
- Known issue: currency_symbol 4.9% F1 (confused with emoji — single Unicode characters are ambiguous)
- Updated models/default symlink from char-cnn-v2 to char-cnn-v4
- Bumped workspace version to 0.1.3
- Updated CHANGELOG.md with full v0.1.3 release notes
- All 135 tests pass (73 core + 62 model)
<!-- SECTION:FINAL_SUMMARY:END -->
