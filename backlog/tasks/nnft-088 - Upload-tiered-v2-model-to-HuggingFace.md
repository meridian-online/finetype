---
id: NNFT-088
title: Upload tiered-v2 model to HuggingFace
status: Done
assignee:
  - '@nightingale'
created_date: '2026-02-17 22:44'
updated_date: '2026-02-17 22:57'
labels:
  - release
  - model
  - infrastructure
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The tiered-v2 model (30 epochs, batch_size=64, 100 samples/type) is trained locally in models/tiered-v2/ but not yet published. Upload to HuggingFace so download-model.sh can fetch it and release builds can embed it.

Model contains: 34 safetensors files, 35 JSON files, 34 YAML configs, 1 tier_graph.json. Total ~9.1MB.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tiered-v2 model artifacts uploaded to HuggingFace noon-org/finetype repo
- [x] #2 download-model.sh updated to fetch tiered-v2 model
- [x] #3 download-model.sh tested — fetches and verifies model integrity
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Uploaded tiered-v2 model (34 safetensors, 35 JSON, 34 YAML, 1 tier_graph.json, 1 manifest.txt) to HuggingFace noon-org/finetype-char-cnn repo. Updated download-model.sh to support tiered models via manifest.txt file listing. Commit: 42de6264fea9bf88d50902d88727195968182cef on HuggingFace."
<!-- SECTION:FINAL_SUMMARY:END -->
