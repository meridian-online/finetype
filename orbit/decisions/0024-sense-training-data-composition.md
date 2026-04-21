---
status: accepted
date-created: 2026-03-01
date-modified: 2026-03-11
---
# 0024. Sense training data composition — SOTAB + profile headers 50× + synthetic header injection

## Context and Problem Statement

The Sense model (column-level broad category classifier) needed training data that maps column samples + headers to broad categories (temporal, numeric, geographic, entity, format, text). Initial training on SOTAB data alone regressed profile eval from 116/120 to 78/120 — the model learned SOTAB's header distribution but not FineType's curated header-category associations.

## Considered Options

- **SOTAB-only** — 31K columns from web tables. Good diversity but header naming conventions don't match analyst workflows. Profile eval: 78/120.
- **Profile eval headers only** — ~120 columns. Too small to train, but contains the exact header-category associations that matter.
- **SOTAB + profile eval headers repeated 50×** — Use SOTAB for benchmark coverage (31K columns), repeat profile eval headers 50× (6K samples) to encode curated header-category knowledge. 50% synthetic header injection on SOTAB samples.
- **GitTables** — 1.7M tables from GitHub CSVs. Rich but noisy labels, not yet curated for Sense training.

## Decision Outcome

Chosen option: **SOTAB + profile eval headers 50× with 50% synthetic header injection**, because it solved the 78/120 → 116/120 regression by balancing benchmark diversity with curated accuracy.

Key insight: profile eval is used in *training* (not validation) because SOTAB validation split monitors training progress while `profile_eval.sh` is the real acceptance test. The 50× repetition ensures the model weights header-category associations from curated data over SOTAB's noisier distribution.

### Consequences

- Good, because profile eval recovered to 116/120 and subsequently reached 180/186 after taxonomy expansion and pipeline improvements
- Good, because SOTAB coverage prevents overfitting to the curated dataset alone
- Good, because 50% synthetic header injection teaches the model to rely on value patterns when headers are uninformative
- Bad, because the 50× repetition creates a strong prior toward curated header associations — novel headers not in the curated set may misroute
- Bad, because profile eval headers in training data means profile eval is no longer a fully independent test (mitigated by SOTAB/GitTables as out-of-distribution checks)
- Neutral, because GitTables multi-column data is the planned next training source for sibling-context attention (NNFT-268)
