# FineType — build, test, and evaluation targets
# ═══════════════════════════════════════════════
SHELL := /bin/bash

TAXONOMY_DIR   := labels
EXTENSION      := target/release/finetype_duckdb.duckdb_extension
EVAL_DIR       := eval/gittables
GT_DIR         := $(HOME)/git-tables
EVAL_OUTPUT    := $(GT_DIR)/eval_output

# ─── Build ────────────────────────────────────
.PHONY: build build-release check test generate

build:
	cargo build

build-release:
	cargo build --release

check:
	cargo run -- check

test:
	cargo test

generate:
	cargo run -- generate --localized -s 100 -o training.ndjson

# ─── Evaluation ───────────────────────────────
# Prerequisites:
#   1. GitTables 1M corpus at ~/git-tables/topics/
#   2. DuckDB extension built: make build-release
#   3. Pre-extracted metadata: make eval-extract
#
# Full pipeline: make eval-extract eval-values eval-1m

.PHONY: eval-extract eval-values eval-1m eval-benchmark

eval-extract:
	@echo "═══ Extracting metadata from GitTables 1M corpus ═══"
	python3 $(EVAL_DIR)/extract_metadata_1m.py

eval-values:
	@echo "═══ Extracting column values from sampled tables ═══"
	python3 $(EVAL_DIR)/prepare_1m_values.py

eval-1m: $(EXTENSION)
	@echo "═══ Running GitTables 1M evaluation ═══"
	@echo "Extension: $(EXTENSION)"
	@echo "Eval output: $(EVAL_OUTPUT)"
	duckdb -unsigned < $(EVAL_DIR)/eval_1m.sql

eval-benchmark: $(EXTENSION)
	@echo "═══ Running GitTables benchmark (1,101 tables) ═══"
	duckdb -unsigned < $(EVAL_DIR)/eval.sql

eval-all: eval-extract eval-values eval-1m
	@echo "═══ Full evaluation pipeline complete ═══"

# ─── Taxonomy stats ───────────────────────────
.PHONY: stats taxonomy

stats:
	@cargo run -- check 2>&1 | tail -20

taxonomy:
	@cargo run -- taxonomy 2>&1 | head -10
