# FineType — build, test, and evaluation targets
# ═══════════════════════════════════════════════
SHELL := /bin/bash

TAXONOMY_DIR   := labels
EXTENSION      := target/release/finetype_duckdb.duckdb_extension

# ─── Dataset paths (override via env vars or eval/config.env) ────
# These defaults match eval/config.env. Export env vars to override.
GITTABLES_DIR  ?= $(HOME)/datasets/gittables
EVAL_OUTPUT    ?= $(GITTABLES_DIR)/eval_output
SOTAB_DATA     ?= $(HOME)/datasets/sotab/cta
SOTAB_SPLIT    ?= validation
EVAL_DIR       := eval/gittables
SOTAB_EVAL_DIR := eval/sotab
# Rust eval binaries (finetype-eval crate)
EVAL_RUN       := cargo run -p finetype-eval --bin

# Absolute extension path for DuckDB LOAD
EXTENSION_PATH ?= $(CURDIR)/$(EXTENSION)

# Variables to substitute in SQL templates
ENVSUBST_VARS  := '$$EXTENSION_PATH $$EVAL_OUTPUT $$SOTAB_DIR $$SOTAB_SPLIT'

# ─── Setup ───────────────────────────────────
.PHONY: setup

setup:
	git config core.hooksPath .githooks
	@echo "✓ Git hooks installed (.githooks/pre-commit)"

# ─── CI (run locally before pushing) ─────────
.PHONY: ci lint fmt clippy

ci: fmt clippy test check
	@echo "═══ All CI checks passed ═══"

lint: fmt clippy

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy -- -D warnings

# ─── CLI Tests ─────────────────────────────────
.PHONY: test-smoke test-docs test-golden test-cli

test-smoke:
	./tests/smoke.sh --skip-build

test-docs:
	./tests/doc_tests.sh --skip-build

test-golden:
	./tests/doc_tests.sh --skip-build --golden-only

test-cli: test-smoke test-docs

# ─── Build ────────────────────────────────────
.PHONY: build build-release check test generate

build:
	cargo build

build-release:
	cargo build --release
	cargo build -p finetype_duckdb --release
	cargo build -p finetype-build-tools --release
	@# Append DuckDB extension metadata to the .so (pure Rust, no Python)
	@if [ -f target/release/append-duckdb-metadata ]; then \
		target/release/append-duckdb-metadata \
			-l target/release/libfinetype_duckdb.so \
			-n finetype_duckdb \
			-o target/release/finetype_duckdb.duckdb_extension \
			-p $$(echo "SELECT platform FROM pragma_platform();" | duckdb -noheader -csv 2>/dev/null || echo "linux_amd64") \
			--duckdb-version v1.2.0 \
			--extension-version $$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/') \
			--abi-type C_STRUCT; \
	else \
		echo "⚠ append-duckdb-metadata not found — copying .so without metadata"; \
		cp target/release/libfinetype_duckdb.so target/release/finetype_duckdb.duckdb_extension; \
	fi

check:
	cargo run -- check

test:
	cargo test

generate:
	cargo run -- generate --localized -s 1000 -o training.ndjson

# ─── GitTables Evaluation ────────────────────
# Prerequisites:
#   1. GitTables 1M corpus at $(GITTABLES_DIR)/topics/
#   2. DuckDB extension built: make build-release
#   3. Pre-extracted metadata: make eval-extract
#
# Full pipeline: make eval-extract eval-values eval-1m
# Override paths: GITTABLES_DIR=~/my-data/gittables make eval-1m

.PHONY: eval-extract eval-values eval-1m eval-benchmark eval-all

eval-extract:
	@echo "═══ Extracting metadata from GitTables 1M corpus ═══"
	GITTABLES_DIR="$(GITTABLES_DIR)" EVAL_OUTPUT="$(EVAL_OUTPUT)" \
		$(EVAL_RUN) eval-extract --

eval-values:
	@echo "═══ Extracting column values from sampled tables ═══"
	GITTABLES_DIR="$(GITTABLES_DIR)" EVAL_OUTPUT="$(EVAL_OUTPUT)" \
		$(EVAL_RUN) eval-prepare-values --

eval-1m: $(EXTENSION)
	@echo "═══ Running GitTables 1M evaluation ═══"
	@echo "Extension: $(EXTENSION_PATH)"
	@echo "Eval output: $(EVAL_OUTPUT)"
	export EXTENSION_PATH="$(EXTENSION_PATH)" EVAL_OUTPUT="$(EVAL_OUTPUT)" && \
		envsubst $(ENVSUBST_VARS) < $(EVAL_DIR)/eval_1m.sql | duckdb -unsigned

eval-benchmark: $(EXTENSION)
	@echo "═══ Running GitTables benchmark (1,101 tables) ═══"
	export EXTENSION_PATH="$(EXTENSION_PATH)" && \
		envsubst $(ENVSUBST_VARS) < $(EVAL_DIR)/eval.sql | duckdb -unsigned

eval-all: eval-extract eval-values eval-1m
	@echo "═══ Full evaluation pipeline complete ═══"

# ─── GitTables CLI Evaluation (NNFT-130) ──────
# Uses CLI batch mode (tiered + Model2Vec + disambiguation) instead of DuckDB extension.
# Prerequisites: make eval-extract eval-values eval-mapping
# Full pipeline: make eval-extract eval-values eval-1m-cli

.PHONY: eval-1m-cli

eval-1m-cli: eval-mapping
	@echo "═══ Running GitTables 1M CLI evaluation ═══"
	@echo "Eval output: $(EVAL_OUTPUT)"
	GITTABLES_DIR="$(GITTABLES_DIR)" EVAL_OUTPUT="$(EVAL_OUTPUT)" \
		FINETYPE_BIN="cargo run --release --" \
		$(EVAL_RUN) eval-gittables-cli --
	export EVAL_OUTPUT="$(EVAL_OUTPUT)" && \
		envsubst $(ENVSUBST_VARS) < $(EVAL_DIR)/eval_cli.sql | duckdb

# ─── SOTAB Evaluation ─────────────────────────
# Prerequisites:
#   1. SOTAB CTA data at $(SOTAB_DATA)/{validation,test}/
#   2. DuckDB extension built: make build-release
#   3. Pre-extracted values: make eval-sotab-values
#
# Full pipeline: make eval-sotab-values eval-sotab
# Override paths: SOTAB_DATA=~/my-data/sotab/cta make eval-sotab
# Switch split:  SOTAB_SPLIT=test make eval-sotab

.PHONY: eval-sotab-values eval-sotab eval-sotab-all

eval-sotab-values:
	@echo "═══ Extracting SOTAB $(SOTAB_SPLIT) column values ═══"
	SOTAB_DIR="$(SOTAB_DATA)" \
		$(EVAL_RUN) eval-sotab-prepare -- --split $(SOTAB_SPLIT)

eval-sotab: $(EXTENSION)
	@echo "═══ Running SOTAB CTA evaluation ($(SOTAB_SPLIT)) ═══"
	export EXTENSION_PATH="$(EXTENSION_PATH)" SOTAB_DIR="$(SOTAB_DATA)" SOTAB_SPLIT="$(SOTAB_SPLIT)" && \
		envsubst $(ENVSUBST_VARS) < $(SOTAB_EVAL_DIR)/eval_sotab.sql | duckdb -unsigned

eval-sotab-all: eval-sotab-values eval-sotab
	@echo "═══ SOTAB evaluation pipeline complete ═══"

# ─── SOTAB CLI Evaluation (NNFT-130) ─────────
# Uses CLI batch mode (tiered + disambiguation) instead of DuckDB extension.
# No header hints — SOTAB uses integer column indices.
# Prerequisites: make eval-sotab-values
# Full pipeline: make eval-sotab-values eval-sotab-cli

.PHONY: eval-sotab-cli

eval-sotab-cli:
	@echo "═══ Running SOTAB CTA CLI evaluation ($(SOTAB_SPLIT)) ═══"
	SOTAB_DIR="$(SOTAB_DATA)" \
		FINETYPE_BIN="cargo run --release --" \
		$(EVAL_RUN) eval-sotab-cli -- --split $(SOTAB_SPLIT)
	export SOTAB_DIR="$(SOTAB_DATA)" SOTAB_SPLIT="$(SOTAB_SPLIT)" && \
		envsubst $(ENVSUBST_VARS) < $(SOTAB_EVAL_DIR)/eval_cli.sql | duckdb

# ─── Actionability Evaluation (NNFT-147) ────
# Tests whether FineType's format_string predictions work on real data.
# Runs TRY_STRPTIME on profile eval datasets to measure parse success rates.
# Prerequisites: make eval-profile (generates profile_results.csv)
#
# Usage: make eval-actionability

.PHONY: eval-actionability

eval-actionability:
	@echo "═══ Running actionability evaluation ═══"
	$(EVAL_RUN) eval-actionability -- \
		--manifest eval/datasets/manifest.csv \
		--predictions eval/eval_output/profile_results.csv \
		--labels-dir labels \
		--output eval/eval_output/actionability_results.csv

# ─── Eval Report (NNFT-147) ─────────────────
# Generates a unified markdown dashboard from all eval outputs.
# Prerequisites: make eval-profile eval-actionability
#
# Usage: make eval-report

.PHONY: eval-report

eval-report: eval-profile eval-actionability
	@echo "═══ Generating evaluation report ═══"
	$(EVAL_RUN) eval-report -- \
		--profile-results eval/eval_output/profile_results.csv \
		--actionability-results eval/eval_output/actionability_results.csv \
		--labels-dir labels \
		--output eval/eval_output/report.md
	@echo "✓ Report written to eval/eval_output/report.md"

# ─── Profile Evaluation ─────────────────────
# Evaluate finetype profile against annotated CSVs.
# Uses schema mapping (eval/schema_mapping.csv) for scoring.
#
# Usage:
#   make eval-profile                               # default manifest
#   make eval-profile MANIFEST=path/to/manifest.csv # custom manifest

MANIFEST ?= eval/datasets/manifest.csv

.PHONY: eval-profile eval-mapping

eval-mapping:
	@echo "═══ Generating schema_mapping.csv from YAML ═══"
	$(EVAL_RUN) eval-mapping --
	@echo "✓ eval/schema_mapping.csv generated"

eval-profile: eval-mapping
	@echo "═══ Running profile evaluation ═══"
	./eval/profile_eval.sh $(MANIFEST)

# ─── Training (Pure Rust / Candle) ────────────
# All training uses the finetype-train crate (no Python required).
# Prerequisites: SOTAB data at $(SOTAB_DATA), Model2Vec at models/model2vec/
#
# Full pipeline: make train-prepare-sense train-sense train-entity
# Eval after training: make eval-report

TRAIN_RUN      := cargo run --release -p finetype-train --bin
SENSE_DATA_DIR ?= data/sense_prod
SENSE_MODEL_DIR ?= models/sense_prod/arch_a
ENTITY_MODEL_DIR ?= models/entity-classifier

.PHONY: train-prepare-sense train-prepare-model2vec train-sense train-entity train-all

train-prepare-sense:
	@echo "═══ Preparing Sense training data ═══"
	$(TRAIN_RUN) prepare-sense-data -- \
		--sotab-dir $(SOTAB_DATA) \
		--output $(SENSE_DATA_DIR) \
		--include-profile \
		--synthetic-headers \
		--model2vec-dir models/model2vec
	@echo "✓ Training data written to $(SENSE_DATA_DIR)"

train-prepare-model2vec:
	@echo "═══ Generating Model2Vec type embeddings ═══"
	$(TRAIN_RUN) prepare-model2vec -- \
		--labels-dir labels \
		--model2vec-dir models/model2vec \
		--output models/model2vec
	@echo "✓ Type embeddings written to models/model2vec/"

train-sense:
	@echo "═══ Training Sense model ═══"
	$(TRAIN_RUN) train-sense-model -- \
		--data $(SENSE_DATA_DIR) \
		--output $(SENSE_MODEL_DIR) \
		--epochs 50 \
		--batch-size 64 \
		--lr 5e-4 \
		--patience 10
	@echo "✓ Sense model saved to $(SENSE_MODEL_DIR)"

train-entity:
	@echo "═══ Training Entity classifier ═══"
	$(TRAIN_RUN) train-entity-classifier -- \
		--sotab-dir $(SOTAB_DATA) \
		--model2vec-dir models/model2vec \
		--output $(ENTITY_MODEL_DIR)
	@echo "✓ Entity model saved to $(ENTITY_MODEL_DIR)"

train-all: train-prepare-sense train-prepare-model2vec train-sense train-entity
	@echo "═══ All training complete ═══"

# ─── Taxonomy stats ───────────────────────────
.PHONY: stats taxonomy

stats:
	@cargo run -- check 2>&1 | tail -20

taxonomy:
	@cargo run -- taxonomy 2>&1 | head -10
