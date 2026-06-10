# Inference-speed baseline (fossil-cleanup ac-05)

**Date:** 2026-06-10 · **Binary:** `target/release/finetype` 0.6.26 (built 2026-06-10) ·
**Machine:** this M-series Mac (darwin 25.1.0) · **Model:** `models/default` → sherlock-v19-relu-s42
**Command:** `finetype profile -f <csv> -o json-schema`, one invocation per file, wall-clock via Python `perf_counter`.

## Headline

A typical CSV profiles in **~155 ms end-to-end**, and almost all of it is fixed
per-invocation startup (model load), not data work. Marginal cost per column/row is
tiny: a 4-column file with 33,237 rows (173 ms) costs barely more than a 4-column file
with 50 rows, and a 110-column file clears in 197 ms.

## Numbers (32-file curated corpus, 417 columns, 59,080 rows)

| metric | value |
|---|---|
| total wall-clock | 5.04 s |
| per-file median / mean / p95 / max | 155 / 157 / 197 / 249 ms |
| throughput (incl. model load each time) | 82.8 cols/s · ~11,700 rows/s |
| first invocation (cold) | 246 ms |
| slowest file | earthquakes_2024.csv — 249 ms (22 cols, 14,132 rows) |

## Reading

- **Interactive use is healthy.** Sub-quarter-second per file is comfortably inside
  "feels instant" for a CLI; no evidence of a speed problem at analyst scale.
- **The cost is the fixed startup, not inference.** Estimated fixed overhead ~140–150 ms
  per invocation; data-dependent cost is single-digit ms for typical files. Long-lived
  hosts (DuckDB extension, MCP server) that load the model once amortise this away.
- **Where it compounds: corpus scale.** 33k-file eval passes take ~41 min at `--jobs 10`
  (recorded in memory `corpus-pass-and-duckdb-scripting-friction`) — consistent with
  per-invocation overhead × file count. If corpus passes need to get faster, the lever
  is batch/daemon mode (one model load, many files), not faster per-column inference.

No optimisation performed — this is the baseline that makes the "very fast inference"
success criterion testable. Re-run after any model swap or pipeline change that touches
startup.
