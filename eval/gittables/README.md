# GitTables Evaluation

Evaluation pipeline for FineType against the [GitTables](https://gittables.github.io/) corpus.

## Benchmark Types

| Benchmark | Tables | Coverage | Primary? |
|-----------|--------|----------|----------|
| **1M Stratified** | 4,380 (50/topic) | Full 1M corpus | **Yes** |
| Benchmark Subset | 1,101 | Curated subset | No (legacy) |

The **1M stratified sample** is the primary benchmark. It samples 50 tables per topic from the full ~1M table corpus, providing better statistical coverage than the curated subset.

## Baseline Metrics (v0.1.0)

| Metric | Value |
|--------|-------|
| Overall domain accuracy | **55.3%** |
| Identity | 71.3% |
| Technology | 64.8% |
| Datetime | 53.9% |
| Geography | 45.7% |
| Representation | 38.7% |

## Prerequisites

1. **GitTables 1M corpus** downloaded and extracted:
   ```
   ~/git-tables/topics/{topic}/{parquet files}
   ```

2. **DuckDB extension** built:
   ```bash
   cargo build --release
   # Extension at target/release/finetype_duckdb.duckdb_extension
   ```

3. **Python dependencies** (for metadata extraction):
   ```bash
   pip install pyarrow pandas
   ```

## Running the Evaluation

### Full Pipeline (from scratch)

```bash
# From repo root:
make eval-all
```

Or step by step:

```bash
# Step 1: Extract metadata from parquet files (generates catalog.csv, metadata.csv)
python3 eval/gittables/extract_metadata_1m.py

# Step 2: Extract column values from sampled tables (generates column_values.parquet)
python3 eval/gittables/prepare_1m_values.py

# Step 3: Run evaluation (classifies with FineType, compares to ground truth)
duckdb -unsigned < eval/gittables/eval_1m.sql
```

### Re-run After Model Changes

If the model or taxonomy changed but not the corpus:

```bash
make eval-1m
```

This re-runs classification against the pre-extracted column values.

## Output Files

Generated in `~/git-tables/eval_output/`:

| File | Size | Description |
|------|------|-------------|
| `catalog.csv` | 4 KB | Table counts per topic |
| `metadata.csv` | 1.6 MB | Per-table metadata with annotation JSON |
| `column_values.parquet` | 12 MB | Sampled column values (up to 20 per column) |
| `sampled_files.txt` | 292 KB | List of sampled parquet file paths |

## Files in This Directory

| File | Description |
|------|-------------|
| `eval_1m.sql` | **Primary evaluation** — 1M stratified sample |
| `eval.sql` | Legacy benchmark — 1,101 table subset |
| `extract_metadata_1m.py` | Metadata extraction from parquet KV metadata |
| `prepare_1m_values.py` | Column value sampling and unpivoting |
| `investigate_tech.sql` | Technology domain regression analysis |
| `REPORT.md` | Full evaluation report with analysis |
