# v24 ac-00 — re-baseline the four numeric clusters on the shipped default (v19)

**Result: all four clusters stay in scope. None drop.**

The multi-lens diagnostic recorded `sense_prediction` on **v22** (the campaign
head). What ships is **v19** (`models/default → sherlock-v19-relu-s42`; promotion
of v22 was deferred — see CLAUDE.md / spec `2026-05-26-v22-gated-direction-review`).
So before treating the diagnostic's gaps as ground truth we re-measured each
cluster's false-positive rate on v19 itself.

Method: `scripts/v24_rebaseline_fp.py` samples N=300 member columns per cluster
(members = v22 fired the FP label AND YDF said the numeric correct label), pulls
each real column from its source parquet, profiles the batch once under v19, and
records how often v19's shipped pipeline label is still the FP label.

| cluster | sense FP label | ydf correct | safety | v19 FP rate | verdict |
|---|---|---|---:|---:|---|
| utc→int | `datetime.offset.utc` | `…numeric.integer_number` | 0.95 | **0.837** | KEEP |
| bool→int | `representation.boolean.binary` | `…numeric.integer_number` | 0.94 | **0.993** | KEEP |
| url→int | `technology.internet.url` | `…numeric.integer_number` | 0.91 | **1.000** | KEEP |
| int→dec | `…numeric.integer_number` | `…numeric.decimal_number` | 0.84 | **1.000** | KEEP |

(n=300/cluster, rows=1000, seed=42, model=`models/sherlock-v19-relu-s42`.)

## Reading

The false positives are **not a v22-only artefact** — v19, the shipped default,
reproduces every one of them at 84–100%. The retrain target is real on what
analysts actually run today.

`utc→int` is the only cluster v19 partially resists: it correctly returns
`integer_number` on 48/300 (16%). For the other three v19 is wrong on
essentially every sampled column. So no cluster meets ac-00's "drop if ~0 on the
true default" bar; the v24 hard-negative scope is unchanged at all four.

Full per-cluster JSON (including the histogram of what v19 assigns instead) is in
`rebaseline_v19.txt`.
