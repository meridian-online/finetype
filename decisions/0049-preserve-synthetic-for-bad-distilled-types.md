---
status: accepted
date-created: 2026-04-20
date-modified: 2026-04-20
---
# 0049. Preserve Synthetic Training Data for Types with Bad Distilled Data

## Context and Problem Statement

Seven types in the v16 taxonomy have distilled training data that is entirely
mislabeled (e.g. `identity.government.ssn` rows are partial dates, `technology.
internet.http_method` rows are random uppercase text, `identity.medical.cpt`
rows are mixed integers). The original v16 pipeline dropped these types from
**both** distilled and synthetic data via `_DROP_ALL_TYPES`, on the premise
that synthetic patterns overlap with common types (uppercase BIC codes →
`country_code`, 5-digit CPT → `postal_code`) and introduce noise.

That premise was wrong. It was supported by an empirical comparison
(`232 types → 233/242` vs `239 types → 229/242`) that turned out to be
produced by a misconfigured eval: `scripts/sweep_v16.sh` set `FINETYPE_MODEL_DIR`,
which is honoured only by the DuckDB extension, not the CLI. The CLI's
`--model` flag defaulted to `models/default` — which was v14 throughout.
Every "v16 with synthetic" run was actually v14 being re-scored.

When the env var was fixed (`FINETYPE_MODEL`), the first real v16 run scored
226/242 — a 7-point regression vs v14's 233/242. Six of the 16 errors were
self-inflicted: types with zero training examples cannot be predicted at all,
so every eval column labelled `ssn`, `http_method`, `cpt`, or `loinc`
automatically counted as wrong.

## Considered Options

- **A. Keep dropping from both** — accept the regression and document
  header-hint dependence as the coverage strategy for the 7 types.
- **B. Drop only from distilled** — remove the 7 types' mislabeled distilled
  rows but keep their synthetic generators in the training blend.
- **C. Invest in fixing the distilled data** — manually relabel the ~45 bad
  rows so they become usable. Higher effort, fixes root cause.

## Decision Outcome

Chosen option: **B — drop only from distilled**. Implemented by splitting
`_DROP_ALL_TYPES` into:

```python
_DROP_DISTILLED_TYPES = {7 types}    # drop mislabeled distilled rows
_DROP_SYNTHETIC_TYPES: set[str] = set()   # generators are trustworthy
```

with `_DROP_ALL_TYPES` retained as a backwards-compat alias for the
distilled-drop code paths so callers elsewhere don't break.

Evidence after the fix: v16 seed 43 scored 235/242 (97.1%) — net +2 over v14,
with three columns fixed (phone→telephone, method→http_method,
hostname→hostname) and one regressed (fiscal_year→year).

Option C is deferred as a separate card; it would further improve accuracy
but wasn't necessary to ship v16.

### Promotion deferred to release PR (2026-04-20)

Flipping `models/default → sherlock-v16` in this PR breaks CI because
`.github/scripts/download-model.sh` reads `models/default` and fetches
the pointed-to model from HuggingFace — where sherlock-v16 hasn't been
published yet (that's a release-scope task). The spec for m-18 was
"train + evaluate v16", not "release v16", so promotion is deferred.

This PR therefore keeps `models/default → sherlock-v14` and ships only
the training-pipeline fixes. The promotion steps (symlink flip, golden
test update, `eval/eval_output/report.md` refresh, HuggingFace publish,
version bump, release tag, Homebrew bump) are tracked in
`specs/2026-04-20-v16-release/`.

### Consequences

- Good, because the 7 types now have ~1200 synthetic columns each to learn
  from, eliminating the "zero training examples" class of error.
- Good, because the split makes future drop decisions explicit and auditable
  — `_DROP_SYNTHETIC_TYPES` is empty by default and populated only with
  justification per type.
- Bad, because synthetic `swift_bic` and `cpt` patterns genuinely do overlap
  with `country_code` and `postal_code`; we accept this cost in exchange for
  coverage. Future regressions in those types would motivate populating
  `_DROP_SYNTHETIC_TYPES`.
- Neutral, because the eval-env-var fix (`FINETYPE_MODEL` vs
  `FINETYPE_MODEL_DIR`) is also captured in the same commit and prevents the
  measurement loop from lying again.

## References

- Spec: `specs/2026-04-18-v16-data-audit-retrain/spec.yaml`
- Handover: `specs/2026-04-18-v16-data-audit-retrain/handover-2026-04-19.md`
- Sweep results: `results/sweep-v16-summary.csv` (seed 43 promoted)
- Code: `scripts/prepare_multibranch_data.py` (`_DROP_DISTILLED_TYPES`,
  `_DROP_SYNTHETIC_TYPES`)
