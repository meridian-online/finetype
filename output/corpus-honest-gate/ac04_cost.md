# ac-04 — cost: cheap enough to run in the loop

Spec `2026-06-07-corpus-honest-quality-gate`, ac-04 (`ac_type: observation`).
Measured wall-clock of the stratified honest pass end-to-end (sample profile + score)
on one candidate, M1 (Metal), `--jobs 8`, 2026-06-07.

## Headline — 22 minutes, not 9 hours

| stage | wall-clock | share |
|---|---:|---:|
| sample profile (`gittables_corpus_pass.py --execute`) | **1,348.8 s (22.5 min)** | 99.97% |
| score (`corpus_honest_gate.py`) | **0.46 s** | 0.03% |
| **end-to-end** | **~22.5 min (0.375 h)** | — |

**0.375 h = 4.1% of the 9.08 h full corpus pass.** The target was ≤ ~20% (≤ ~2 h).
Met with a **5× margin**. The corpus-honest verdict is now a routine post-train gate,
not an overnight event.

- Throughput: 852,665 cols / 1,348.8 s = **632 cols/s** (jobs=8).
- Files: 33,054 profiled, 852,665 columns. 196 files (0.59%) errored — source parquets
  no longer on disk (`FileNotFoundError`); immaterial to the verdict.
- **What dominates: the profile subprocess, ~100%.** Scoring is free (DuckDB over the
  sample, sub-second). Any future cost cut targets the profiler, never the scorer.

## Why the gate never pays the YDF-fill cost

The 9.08 h full pass is dominated by its sequential `--fill-ydf` Pass B. The honest
gate does **not** need the candidate's YDF — it reads the stable GATED oracle once from
the v19 baseline (column-intrinsic). So a candidate's gate cost is the `--execute`
(profile + validate) pass only. Even projecting *execute-only* to the full corpus at
this throughput (~2.9 h) the sample is ~13% — still under the 20% bar; against the
real 9.08 h reference it is 4.1%.

## Bonus — a real no-false-alarm signal

This pass profiled **v19 itself** fresh through the exact pipeline a candidate uses,
then scored it against the v19 gated baseline. Verdict: **GO, 0 triggers**, 243 labels
all resolved. Unlike the by-construction v19-self check (ac-03a, identical
predictions), this is an *independent* profile run — its predictions carry real
pipeline nondeterminism (top residual movers: `utc` 169 obs at ratio 1.057, `increment`
188 obs at ratio 1.001) and the gate still clears it cleanly. First evidence the gate
does not false-alarm on pipeline noise — a down-payment on the GO-precision question
ac-03 left open (the full answer still needs a genuinely *new* good model).

## Reproduce

```
source eval/gittables/.venv/bin/activate
/usr/bin/time -p python3 scripts/gittables_corpus_pass.py \
  --corpus-index output/corpus-honest-gate/stratified_sample.files.txt \
  --execute --jobs 8 --out-dir output/corpus-honest-gate/sample_pass
python3 scripts/corpus_honest_gate.py \
  --candidate output/corpus-honest-gate/sample_pass/corpus_pass/columns.parquet \
  --label v19fresh
```

Large outputs (`sample_pass/corpus_pass/*.parquet`, ~25 MB) are local-only, not
tracked.
