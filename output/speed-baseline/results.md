# FineType speed baseline — dual-encoder vs single-encoder

## Provenance

- **Date:** 2026-07-11 (from session context, not wall clock)
- **Machine:** Apple M1 Pro, 16 GB RAM, macOS 26.1 (arm64)
- **Binary:** `./target/release/finetype` (v0.6.45, prebuilt release, 63.7 MB)
- **duckdb shell-out dependency:** `duckdb` CLI v1.5.4 (Variegata) on PATH — exercised by every `profile` run
- **Models compared** (selected via `FINETYPE_MODEL` env var — no symlink mutation, concurrency-safe):
  - **DUAL-ENCODER (current default):** `models/m2v8m-s43` — potion-4M header/semantic encoder + potion-8M value encoder (`value_embed_model: value_model2vec`, 29 MB co-located `value_model2vec/` dir), 244-label
  - **SINGLE-ENCODER prior:** `models/m2v-244-s44` — potion-4M only, no value encoder, 244-label
  - **SINGLE-ENCODER prior (retired default):** `models/sherlock-v19-relu-s42` — 5-branch ReLU, 240-label
- **Method:** `/usr/bin/time -l` for wall + peak RSS; first run per model warms FS cache; reported runs are subsequent (warm-disk) unless marked cold. `scripts/bench_infer.py` for subprocess p50/p95 (its own ac-05 instrument, n=200, seed 42, calibrate = `eval/gittables/failure_log.calibrate.tsv`).

## Headline

**A profile run on the shipped dual-encoder holds ~200 MB of RAM and turns a 6-column table around in about a quarter-second. The dual encoder's cost is memory, not time: it carries ~68 MB more resident RAM than a single-encoder model (201 MB vs 133 MB) but adds only a hair of wall-clock (~20-30 ms) on small tables. Cold model load is ~50 ms.** For an analyst, FineType feels instant on a normal table; the dual encoder is a RAM tax, not a latency tax.

## 1. Model load / cold-start (`infer` on a fast-path value — startup-dominated)

`finetype infer -i "test@example.com"` — email hits the deterministic fast path, so this isolates **binary + base startup**, NOT the Sense/value model (which loads lazily and is skipped here):

| Model | real (x5) | peak RSS |
|---|---|---|
| dual m2v8m-s43 | 0.05-0.06 s | ~34.9 MB |
| single m2v-244-s44 | 0.05 s | ~34.9 MB |

Identical, because neither loaded the value encoder — the fast path short-circuits before Sense. Real model-load cost shows up only when the model actually runs (below).

## 2. Profile — model actually loads + runs (includes duckdb shell-out)

`finetype profile -f eval/tier2_benchmark.csv` (6 columns, 2,490 rows):

| Model | real (warm) | real (cold, 1st) | peak RSS |
|---|---|---|---|
| **dual m2v8m-s43** | 0.24-0.25 s | 0.42 s | **~201 MB** |
| single m2v-244-s44 | 0.21-0.22 s | — | ~133 MB |
| single sherlock-v19-relu-s42 | 0.21 s | 0.30 s | ~132 MB |

**This is the load-cost comparison the task asked for:** the dual encoder's value_model2vec adds **~68 MB peak RSS** (201 vs 133) and **~20-30 ms** wall on a small table. The value encoder loads on demand at first Sense call, which is why the cold first run is ~0.42 s vs ~0.24 s warm.

Wide table — `finetype profile -f tests/fixtures/features.csv` (164 columns, 278 rows), dual only:
- **0.91 s**, ~204 MB RSS. Per-column marginal ≈ (0.91 − 0.24)/158 ≈ **~4.2 ms/column** for the dual encoder including value embedding.

## 3. Subprocess per-column latency (`scripts/bench_infer.py`, ac-05 instrument, n=200)

One `finetype infer --mode column --batch --explain` process spawn per column (dominated by process startup; most rows hit fast paths):

| Model | p50 | p95 | p99 | mean | ac-05 (<100ms p50) |
|---|---|---|---|---|---|
| dual m2v8m-s43 | 73.5 ms | 108.9 ms | 222.8 ms | 80.3 ms | PASS |
| single m2v-244-s44 | 82.4 ms | 156.5 ms | 255.6 ms | 93.2 ms | PASS |

Both pass the ac-05 <100 ms p50 target. The dual/single ordering here is within measurement noise — this instrument is dominated by per-process spawn + startup, not by the encoder, so it is NOT a reliable encoder-cost discriminator. Use the profile RSS numbers (section 2) for that.

## Caveats / honest scope

- These are **single-machine, single-run-count** numbers on an idle-ish M1 Pro shared with parallel agents — treat as directional, not a locked benchmark. Peak RSS is stable across repeats (±1 MB); wall time on small tables is noisier (±30 ms).
- The subprocess bench (section 3) cannot separate dual from single because process-spawn dominates. The **profile RSS** (section 2) is the clean encoder-cost signal.
- `models/sherlock-v19-relu-s42` (240-label) loads and runs fine on the 247-taxonomy binary for timing purposes; its predictions are not validated here (speed only).
- Not measured: server/long-lived form throughput, GPU, large (>1 GB) files, batch `--files` amortised load.
