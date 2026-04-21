# Sherlock Distillation Results

**Date:** 2026-04-04
**Dataset:** [meridian-online/sherlock-annotated](https://huggingface.co/datasets/meridian-online/sherlock-annotated)

## Summary

The Sherlock distillation is complete. 102,461 unique column-type annotations
(74.6% of the 137,353-column Sherlock test set) have been published to
HuggingFace as the source of truth.

## Pipeline

Each Sherlock column was processed through blind-first adjudication:

1. **Blind classification** — LLM classifies the column from sample values alone,
   without seeing FineType's prediction
2. **FineType inference** — CharCNN engine produces its own prediction
3. **Adjudication** — when labels disagree, a reasoned judgement picks the final
   label (blind-first: the blind label is preferred unless FineType is clearly
   more accurate)

This produces training signal that isn't anchored to FineType's existing biases.

## Execution

- **Batches:** 1,374 batches of 100 columns each
- **Runner:** `scripts/distill_batches.sh` — bash loop launching Claude Code
  sessions via `claude -p`, each processing 3-5 waves of batch agents
- **Duration:** ~2 weeks of overnight runs (March–April 2026)
- **Model:** Claude Haiku (orchestrator) + Claude Sonnet (batch agents)

## Output Schema

| Column | Type | Description |
|---|---|---|
| `sherlock_index` | int64 | Column index in the Sherlock corpus |
| `split` | string | Dataset split (`test`) |
| `sample_values` | string | JSON array of sample values |
| `blind_label` | string | LLM type label (blind) |
| `blind_confidence` | string | `high`, `medium`, `low` |
| `finetype_label` | string | FineType engine prediction |
| `finetype_confidence` | float64 | FineType confidence (0-1) |
| `agreement` | string | `yes`/`no` |
| `final_label` | string | Adjudicated type label |
| `reasoning` | string | Adjudication reasoning |
| `ground_truth_label` | string | Original Sherlock label |

## Coverage Analysis

```
Sherlock test set:     137,353 columns
Annotated (unique):    102,461 (74.6%)
Not annotated:          34,892 (25.4%)
```

The 25.4% gap has two causes:

1. **Duplicate batch processing (~30k)** — the overnight runner's `next` command
   had a race window where parallel sessions could claim the same batch. The
   concat script's dedup was disabled for Sherlock (empty `source_file` and
   `column_name` fields), so duplicates consumed batch slots that should have
   covered new columns.

2. **Unrecoverable annotations (~5k)** — batch agents sometimes sampled different
   values than the input JSONL, or produced malformed CSV output (truncated JSON,
   pipe-delimited instead of JSON arrays). These couldn't be joined back to the
   Sherlock corpus for the `sherlock_index` provenance field.

## Lessons Learned

1. **Verify completion, not just markers.** The `.done` marker tracked "agent
   finished" but not "agent produced valid output." Status reported 100% while
   actual unique coverage was 74.6%.

2. **Dedup must always be active.** The concat script disabled dedup for Sherlock
   because the join key (`source_file`, `column_name`) was empty. This let
   duplicates inflate the output from ~103k unique to ~120k rows.

3. **Standardise agent output format.** Agents produced sample_values in
   multiple formats (JSON arrays, pipe-delimited, truncated). A schema
   validation step on each batch CSV would have caught this early.

## Cleanup

With HuggingFace as the source of truth, the following were removed from the repo:

- 1,374 `.done` marker files
- 522 batch CSV files
- `sherlock_distilled.csv.gz` (stale concatenated output)
- `distill_concat.log`

Retained for future distillation (gittables, eval):

- `sherlock_test.jsonl` — input data
- `gittables_sample.jsonl` — pending source
- `eval_columns.jsonl` — pending source
- All `scripts/distill_*.py` pipeline scripts
