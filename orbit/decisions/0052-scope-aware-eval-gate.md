---
status: accepted
date-created: 2026-04-20
date-modified: 2026-04-20
---
# 0052. Scope-Aware Eval Promotion Gate

## Context and Problem Statement

The v17 retrain adds real distilled data for `user_agent` and `loinc`,
plus improved generators for four other types. These changes will almost
certainly change the evaluation corpus (new columns may be added, label
distributions may shift). If we gate promotion against an absolute score
— "v17 must beat 235/242 on the CURRENT corpus" — then any corpus
expansion that adds hard columns will falsely block a genuine
improvement. Conversely, if we gate against "v17 must beat v16 on v16's
corpus", we risk cherry-picking the corpus to make v17 look good.

The v16 retrain (decision 0049) itself hit a related trap: the initial
"v16 with synthetic" run was accidentally v14 being re-scored, because
`FINETYPE_MODEL_DIR` is read by the DuckDB extension but not the CLI.
An eval gate is only as trustworthy as the pinning of the thing it's
comparing against.

## Considered Options

- **A. Absolute floor** — require `v17_score ≥ 235/242` regardless of
  corpus. Simple but penalises honest corpus expansion.
- **B. Relative-to-v16 only** — require `v17_score ≥ v16_score` measured
  on the same corpus. No absolute floor — a corpus change that tanks
  both v16 and v17 to 180/242 would still pass.
- **C. Scope-aware `max()` of both** — require
  `v17_score ≥ max(235/242, v16_baseline_at_corpus_freeze)` where
  `v16_baseline_at_corpus_freeze` is a re-scored number captured on the
  exact same corpus v17 was trained against.

## Decision Outcome

Chosen option: **C — scope-aware `max()`**. This prevents both failure
modes: v17 cannot win by shrinking the corpus, and v17 cannot lose
because v16's old score was set on a different corpus.

Operational rules:

1. **Corpus freeze precedes baseline capture.** The eval corpus (GT
   labels, column list, schema mapping) is frozen at a specific git SHA
   before any v16-baseline or v17-train run.
2. **v16 baseline is re-measured on the frozen corpus.** `v16-baseline.md`
   records score, git SHA, timestamp, and the CLI version that produced
   it. This is ac-10 of the v17 spec.
3. **Corpus freeze is audited.** If corpus files change between baseline
   capture and v17 eval, the baseline is re-measured. No exceptions.
4. **Winner selection is deterministic.** Among the 3 seeds: highest
   profile eval > highest val_acc > lowest seed number. Documented in
   `specs/.../progress.md` at eval time.

### Consequences

- Good, because the gate is honest under corpus churn — adding hard
  columns doesn't falsely block v17, and removing hard columns doesn't
  falsely promote it.
- Good, because v16-baseline recapture closes the v16-era measurement
  loophole (wrong-model eval bug from decision 0049).
- Good, because the `max()` ensures absolute quality doesn't silently
  regress — if v16 drops below 235 on the new corpus, the 235 floor
  still applies.
- Bad, because re-measuring v16 is an extra ~5-minute step per spec
  cycle. Acceptable cost for gate honesty.
- Bad, because "corpus freeze" is a process discipline that depends on
  humans not editing eval files during the spec window. Documented but
  not machine-enforced (would need a lock file — deferred).
- Neutral, because the two numbers may be equal in practice (235/242 is
  both the absolute floor and the likely v16 recapture), in which case
  `max()` degenerates to the absolute floor.

## References

- Spec: `orbit/specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml` (v1.3)
- Prior incident: `orbit/decisions/0049-preserve-synthetic-for-bad-distilled-types.md`
  (eval env-var bug, `FINETYPE_MODEL_DIR` vs `FINETYPE_MODEL`)
- Baseline capture: `orbit/specs/2026-04-20-distilled-data-relabel-7-types/v16-baseline.md` (ac-10)
- Eval pipeline: `eval/profile_eval.sh`, `eval/schema_mapping.yaml`
