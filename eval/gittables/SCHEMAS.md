# GitTables autonomy-contract artefact schemas

This document is the source of truth for the column layout of the
append-only TSV files the cron-firing agent writes per cycle. It exists
because every consumer (cycle preamble, log-integrity check, harvest
processor, retrain trigger) parses these files positionally — adding a
column without consumer-graph review would silently corrupt downstream
state, exactly the m-19 manifest schema migration failure pattern
referenced by the autonomy contract's load-bearing-paths registry.

Editing this file requires updating every consumer in the same commit
(see `orbit/contracts/load-bearing-paths.yaml` once these paths are
registered).

## File layout

| File | Purpose | Bead | Permissions |
|---|---|---|---|
| `failure_log.tsv` | Cycle records of B01/B04 misclassification or trivial-fallback events | finetype-87j | `chflags uchg` post-cycle (macOS uschg) |
| `working_slice_coverage.tsv` | Cycle records of every file the working slice visited | finetype-87j | `chflags uchg` post-cycle |
| `harvest_pool.tsv` | Training-data candidate pool — column samples earmarked for generator widening | finetype-s16 | append-only (uschg) |
| `holdout_paths.txt` | Frozen 2000-file gate-metric surface | finetype-e6d / finetype-s16 | read-only at sprint start |
| `cycle_log.jsonl` | Per-cycle summary (cycle_id, start, end, scores, line counts of the above) | finetype-nms / finetype-87j | append-only |

## `failure_log.tsv`

Append-only TSV. One row per (cycle, file, column) triple where a
misclassification or trivial-fallback was detected. Columns:

| # | Column | Type | Notes |
|---|---|---|---|
| 1 | `cycle_id` | UUID | from `/tmp/finetype-cron.lock` payload |
| 2 | `timestamp` | ISO 8601 UTC | `date -u +%Y-%m-%dT%H:%M:%SZ` |
| 3 | `file_path` | string | absolute path to source parquet |
| 4 | `file_content_sha256` | hex64 | per `eval_leakage.content_hash` |
| 5 | `column_name` | string | offending column |
| 6 | `predicted_type` | string | `x-finetype-label` from profile |
| 7 | `observed_values_sample` | string | up to 8 values, `│`-separated, max 200 chars total |
| 8 | `inferred_correct_type` | string | scribe's read of the right type, or `unknown` |
| 9 | `mechanism` | enum | `header-signal` \| `value-shape` \| `prefix-shape` |

Header line:

```
cycle_id\ttimestamp\tfile_path\tfile_content_sha256\tcolumn_name\tpredicted_type\tobserved_values_sample\tinferred_correct_type\tmechanism
```

## `working_slice_coverage.tsv`

Append-only TSV. One row per (cycle, file) pair the working slice
visited. Columns:

| # | Column | Type | Notes |
|---|---|---|---|
| 1 | `cycle_id` | UUID | matches failure_log entries from same cycle |
| 2 | `timestamp` | ISO 8601 UTC | |
| 3 | `file_path` | string | absolute path |
| 4 | `file_content_sha256` | hex64 | round-robin invariant key (s16 ac-05) |
| 5 | `outcome` | enum | `clean_pass` \| `classifier_quality_issue` \| `corpus_quality_issue` |
| 6 | `predicted_type_distribution_json` | JSON | `{type: count}` over columns |

Header line:

```
cycle_id\ttimestamp\tfile_path\tfile_content_sha256\toutcome\tpredicted_type_distribution_json
```

## `harvest_pool.tsv`

Append-only TSV. One row per (file, column) pair earmarked for
generator widening. The file is permanently excluded from holdout AND
working slice once present here. Columns:

| # | Column | Type | Notes |
|---|---|---|---|
| 1 | `cycle_id` | UUID | when the harvest decision was made |
| 2 | `timestamp` | ISO 8601 UTC | |
| 3 | `file_path` | string | source parquet path |
| 4 | `file_content_sha256` | hex64 | exclusion key (s16 ac-04) |
| 5 | `column_name` | string | source column |
| 6 | `target_generator` | string | which taxonomy generator should ingest these |
| 7 | `samples_json` | JSON | up to 64 values, JSON array of strings |

Header line:

```
cycle_id\ttimestamp\tfile_path\tfile_content_sha256\tcolumn_name\ttarget_generator\tsamples_json
```

## `cycle_log.jsonl`

Append-only NDJSON. One line per cycle. Required keys:

```json
{
  "cycle_id": "<uuid>",
  "cycle_start": "<iso8601>",
  "cycle_end": "<iso8601>",
  "contract_path": "orbit/contracts/2026-05-03-gittables-90-percent-roundtrip.yaml",
  "contract_sha256": "<hex64>",
  "model_tag": "<symlink-target>",
  "model_sha256": "<hex64>",
  "harness_sha": "<git-rev>",
  "gate_score": <float>,
  "files_passed": <int>,
  "files_total": <int>,
  "working_slice_visited": <int>,
  "failure_log_lines_before": <int>,
  "failure_log_lines_after": <int>,
  "coverage_log_lines_before": <int>,
  "coverage_log_lines_after": <int>,
  "branches_taken": ["B01", "B04", ...],
  "halts_fired": ["H06", ...],
  "escalations_raised": ["E01", ...],
  "free_disk_gb_start": <int>,
  "free_disk_gb_end": <int>
}
```

The `*_lines_before`/`*_lines_after` integers materialise the
append-only invariant for H08/H09: any subsequent cycle reads the
previous cycle's `lines_after` and asserts the current `lines_before`
equals it. A drop indicates corruption — halt all cycles, surface to
human.

## Corruption recovery flow (87j ac-05, H08/H09)

When `scripts/log_integrity_check.sh` exits 1, ALL cycles are halted
until a human inspects. Recovery is intentionally manual — the cron
agent cannot decide whether the deletion was malicious, accidental, or
intentional. Steps:

1. **Stop launchd**: `scripts/launchd/install.sh uninstall` so further
   cycles do not fire while you investigate.
2. **Inspect** `cycle_log.jsonl` (read-only) for the last `*_lines_after`
   value vs. current `wc -l` of the affected log. Diff against any
   backup you have.
3. **Establish ground truth**: which version is correct? If the loss is
   accidental (errant `truncate`, a stray test write), restore from
   backup or git history. If intentional (operator chose to compress
   stale entries elsewhere), update `cycle_log.jsonl`'s last entry's
   `*_lines_after` to match the new floor — making the new state the
   recognised baseline.
4. **Re-apply uschg flag** if it had been dropped: `chflags uchg
   eval/gittables/failure_log.tsv eval/gittables/working_slice_coverage.tsv`.
5. **Re-enable cron**: `scripts/launchd/install.sh install`. The next
   cycle's preamble will record the new line counts and resume.
6. **Tabletop**: file a memo at `orbit/cards/memos/` describing the
   incident — the next pass-2 tabletop will decide whether to amend
   H08/H09 thresholds.
