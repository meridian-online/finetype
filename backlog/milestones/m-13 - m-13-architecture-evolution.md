---
id: m-13
title: "m-13: Architecture Evolution"
---

## Description

Next-generation inference pipeline informed by literature review and three discovery spikes (NNFT-262/263/264). Four phases:

Phase 1: Expanded column-level features + disambiguation rules (no retrain)
Phase 2: Hierarchical classification head — 7→43→250 tree softmax (retrain)
Phase 3: Sibling-context attention — cross-column self-attention for bare-name ambiguity (multi-column training data)
Phase 4: LLM distillation — Qwen3 32B teacher labelling via Ollama (parallel data effort)

Goal: Eliminate all 7 remaining errors (3 bare-name ambiguity, 3 visually-similar confusion, 1 GT edge case) while staying within 10–50 MB model budget. Composite stack: Model2Vec + features + attention + hierarchical head = 12–42 MB.

Key references:
- discovery/sense-architecture-challenge/FINDINGS.md
- discovery/sense-architecture-challenge/ARCHITECTURE_EVOLUTION.md
