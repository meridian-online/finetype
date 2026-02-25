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
VENV_PYTHON    ?= $(HOME)/.venvs/finetype-eval/bin/python3

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

# ─── Build ────────────────────────────────────
.PHONY: build build-release check test generate

build:
	cargo build

build-release:
	cargo build --release
	cargo build -p finetype_duckdb --release
	@# Append DuckDB extension metadata to the .so
	python3 $(HOME)/github/noon-org/duckdb-finetype/extension-ci-tools/scripts/append_extension_metadata.py \
		-l target/release/libfinetype_duckdb.so \
		-n finetype_duckdb \
		-o target/release/finetype_duckdb.duckdb_extension \
		-p $$(echo "SELECT platform FROM pragma_platform();" | duckdb -noheader -csv 2>/dev/null || echo "linux_amd64") \
		-dv v1.2.0 \
		-ev $$(cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print([p['version'] for p in d['packages'] if p['name']=='finetype_duckdb'][0])" 2>/dev/null || echo "0.1.5") \
		--abi-type C_STRUCT

check:
	cargo run -- check

test:
	cargo test

generate:
	cargo run -- generate --localized -s 100 -o training.ndjson

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
		$(VENV_PYTHON) $(EVAL_DIR)/extract_metadata_1m.py

eval-values:
	@echo "═══ Extracting column values from sampled tables ═══"
	GITTABLES_DIR="$(GITTABLES_DIR)" EVAL_OUTPUT="$(EVAL_OUTPUT)" \
		$(VENV_PYTHON) $(EVAL_DIR)/prepare_1m_values.py

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
		$(VENV_PYTHON) $(EVAL_DIR)/eval_cli.py
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
		$(VENV_PYTHON) $(SOTAB_EVAL_DIR)/prepare_values.py --split $(SOTAB_SPLIT)

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
		$(VENV_PYTHON) $(SOTAB_EVAL_DIR)/eval_cli.py --split $(SOTAB_SPLIT)
	export SOTAB_DIR="$(SOTAB_DATA)" SOTAB_SPLIT="$(SOTAB_SPLIT)" && \
		envsubst $(ENVSUBST_VARS) < $(SOTAB_EVAL_DIR)/eval_cli.sql | duckdb

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
	python3 -c "import yaml,csv; d=yaml.safe_load(open('eval/schema_mapping.yaml')); w=csv.writer(open('eval/schema_mapping.csv','w')); w.writerow(['gt_label','source','finetype_label','finetype_domain','match_quality','expand']); [w.writerow([m['gt_label'],m['source'],m.get('finetype_label') or '',m.get('finetype_domain',''),m['match_quality'],'true' if m.get('expand') else 'false']) for m in d['mappings']]"
	@echo "✓ eval/schema_mapping.csv generated"

eval-profile: eval-mapping
	@echo "═══ Running profile evaluation ═══"
	./eval/profile_eval.sh $(MANIFEST)

# ─── Taxonomy stats ───────────────────────────
.PHONY: stats taxonomy

stats:
	@cargo run -- check 2>&1 | tail -20

taxonomy:
	@cargo run -- taxonomy 2>&1 | head -10
