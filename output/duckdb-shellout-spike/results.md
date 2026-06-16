# Shell-out ingestion spike — ac-01 GREEN (choice 0100)

## Feasibility — YES
`SUMMARIZE SELECT * FROM read_csv(f)` returns per-column count / approx_unique /
min / max / null% in one parallel query (the free-stats). `USING SAMPLE 100 ROWS`
gives the value sample for the model. No compile/link — invokes the duckdb binary.

## Performance (read + per-column distinct/min/max)
| input | duckdb SUMMARIZE | csv-crate (Rust) read+stats |
|---|---|---|
| large 1M rows / 143MB | 0.71s (+0.31s sample) | ~1.25s |
| small 100 rows | 0.04s (spawn floor) | ~0.000s |
duckdb ~1.8x faster on the large file (parallel C++); csv-crate wins tiny files
by the ~40ms process-spawn cost. Crossover is small-file territory.

## Cross-platform — YES (no compile risk, unlike choice 0099's link path)
spike-duckdb-shellout-xplat #27611258343: windows-latest GREEN + ubuntu-latest
GREEN (download official duckdb CLI, run SUMMARIZE + read_csv). macOS confirmed
locally. The decisive contrast with 0099: shell-out invokes a binary, so the
Windows/MSVC amalgamation compile failure cannot occur.

## Verdict
ac-01 GATE PASSES. Shell-out ingestion is viable, faster on large files, and
cross-platform. Adoption (hard duckdb dep + consolidation) is the author's call.
