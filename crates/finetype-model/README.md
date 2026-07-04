# finetype-model

The inference engine of
[FineType](https://github.com/meridian-online/finetype) — a type inference
engine that detects and classifies semantic types in tabular data.

This crate implements the two-stage classification pipeline:

- **Sense** — a [Candle](https://github.com/huggingface/candle)-based
  multi-branch neural model (character, value-embedding, statistics, header,
  and validation branches) that produces a broad semantic classification per
  column, with optional sibling-context attention for cross-column header
  enrichment.
- **Sharpen** — a deterministic layer of value-based rules on top: validation
  vetoes, schema-contradiction demotions, check-digit and closed-set
  membership guards, datetime format refinement, and recovery rules for types
  the model cannot predict directly.

The split is deliberate: the model proposes, the deterministic layer holds it
to the [taxonomy](https://crates.io/crates/finetype-core)'s validation
standards — a column of stock tickers may *look* like ICAO airport codes to a
neural model, but it does not survive membership against the published
airport list.

## Usage

Inference needs trained model weights (a multi-branch model directory with
`model.safetensors`, `config.json`, and `label_map.json`). The shipped
default is published at
[meridian-online/finetype-model](https://huggingface.co/meridian-online/finetype-model)
on Hugging Face; the `FINETYPE_MODEL` environment variable points the loader
at a model directory.

Most users want the `finetype` CLI rather than this library — it bundles the
model, the taxonomy, and the DuckDB-based CSV/Parquet ingestion:
[installation instructions](https://github.com/meridian-online/finetype#installation).
Library consumers should start from the `column` module
([docs.rs](https://docs.rs/finetype-model)) — `ColumnClassifier` is the
composed pipeline entry point.

## Feature flags

- `cpu` (default path) — inference only.
- `cuda` / `metal` — GPU acceleration for training workflows.

License: MIT
