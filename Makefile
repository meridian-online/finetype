# FineType — build, test, and evaluation targets
# ═══════════════════════════════════════════════
SHELL := /bin/bash

TAXONOMY_DIR   := labels
EXTENSION      := target/release/finetype.duckdb_extension

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
	@if command -v rustup >/dev/null 2>&1; then \
		rustup update stable; \
		rustup component add rustfmt clippy; \
	elif command -v brew >/dev/null 2>&1; then \
		brew upgrade rust 2>/dev/null || true; \
	fi
	git config core.hooksPath .githooks
	@echo "✓ Rust updated, git hooks installed (pre-commit: fmt; pre-push: fmt+clippy)"

# ─── CI (run locally before pushing) ─────────
.PHONY: ci lint fmt clippy workspace-test types hygiene check-gate-routing

ci: fmt clippy test check types hygiene check-gate-routing
	@echo "═══ All CI checks passed ═══"

lint: fmt clippy

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy -- -D warnings

# The bare clippy/test above build `default-members` — three of the eight
# workspace crates. This is the one that compiles the other five, including the
# DuckDB extension this repo ships. Its own CI job, `workspace-test`, is separate
# for the same reason it is separate here: it drags in candle and a bundled
# duckdb and is much slower.
workspace-test:
	cargo test --workspace

# Python type check over the scripts that resolve a bar, a score or a gate
# verdict. The file set is pyrightconfig.json's `include`, so this and CI's
# `python-types` job see exactly the same files. Version pinned to match the
# workflow — an unpinned pyright adopts new checks between minor releases.
types:
	npx --yes pyright@1.1.411

# Paths into the private planning repo, and absolute home paths, in tracked
# files. Seconds, no toolchain. The gate's own regression test runs first: a
# hygiene gate fails silently in BOTH directions, so "it still works" has to be
# established before "the tree is clean" means anything.
hygiene:
	./scripts/check-public-hygiene-selftest.sh
	./scripts/check-public-hygiene.sh

# Which gate self-test a diff re-runs, and whether the wiring that decides still
# holds. The audit is what CI runs unconditionally; the self-test is the router's
# own proof and is itself routed. Stdlib python, no build.
check-gate-routing:
	.github/scripts/gate-self-tests.py audit
	.github/scripts/gate-self-tests.py --self-test

# ─── CLI Tests ─────────────────────────────────
.PHONY: test-smoke test-docs test-golden test-cli check-doc-counts
.PHONY: check-duckdb-catalog check-sql-examples check-docs

test-smoke:
	./tests/smoke.sh --skip-build

# INFORMATIONAL ONLY — doc_tests.sh always exits 0, so a successful
# `make test-docs` says nothing about whether the documentation is correct.
# The documentation checks that DO fail are `make check-docs`.
test-docs:
	./tests/doc_tests.sh --skip-build

test-golden:
	./tests/doc_tests.sh --skip-build --golden-only

# Fails if a taxonomy count in the docs disagrees with labels/definitions_*.yaml.
# Same two commands CI runs on the `evidence` job. Stdlib python, no build.
check-doc-counts:
	./scripts/check_doc_taxonomy_counts.py
	./scripts/check_doc_taxonomy_counts.py --self-test

# Fails if the documented DuckDB surface disagrees with duckdb_functions() of a
# loaded LOCAL build — names, kinds, return types. Needs `make build-extension`
# and the duckdb CLI. No model: LOAD populates the catalog on its own.
check-duckdb-catalog: build-extension
	./scripts/check_duckdb_catalog.py
	./scripts/check_duckdb_catalog.py --self-test

# Fails if a documented ```sql example does not run against the local build, or
# does not have the shape its own comment claims. Needs the extension AND a
# model (FINETYPE_MODEL_DIR, or models/default).
check-sql-examples: build-extension
	./scripts/check_sql_examples.py
	./scripts/check_sql_examples.py --self-test

# Every documentation gate. The three that fail; test-docs is not one of them.
check-docs: check-doc-counts check-duckdb-catalog check-sql-examples

test-cli: test-smoke test-docs

# ─── Build ────────────────────────────────────
.PHONY: build build-release build-extension check test generate

build:
	cargo build

build-release: build-extension
	cargo build --release

# Just the DuckDB extension: the cdylib plus the metadata stamp. Split out of
# build-release so a documentation gate can ask for the artifact it loads
# without also building the CLI, the eval binaries and the trainer.
build-extension:
	cargo build -p finetype_duckdb --release
	cargo build -p finetype-build-tools --release
	@# Append DuckDB extension metadata to the cdylib (pure Rust, no Python).
	@# The cdylib lib name is `finetype` (see crates/finetype-duckdb/Cargo.toml),
	@# so the artifact is lib finetype.{dylib,so} per platform. Stamp the STABLE
	@# C API (C_STRUCT) at the v1.2.0 floor so one artifact loads on DuckDB 1.2+
	@# (choice 0063). For the community-extensions build contract use `make
	@# configure release` instead, which drives extension-ci-tools.
	@libext=$$([ "$$(uname -s)" = "Darwin" ] && echo dylib || echo so); \
	libpath=target/release/libfinetype.$$libext; \
	if [ ! -f $$libpath ]; then \
		echo "✗ $$libpath not found after build"; exit 1; \
	fi; \
	if [ -f target/release/append-duckdb-metadata ]; then \
		target/release/append-duckdb-metadata \
			-l $$libpath \
			-n finetype \
			-o $(EXTENSION) \
			-p $$(echo "SELECT platform FROM pragma_platform();" | duckdb -noheader -csv 2>/dev/null || echo "$$([ "$$(uname -s)" = "Darwin" ] && echo "$$([ "$$(uname -m)" = "arm64" ] && echo osx_arm64 || echo osx_amd64)" || echo linux_amd64)") \
			--duckdb-version v1.2.0 \
			--extension-version $$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1) \
			--abi-type C_STRUCT; \
	else \
		echo "⚠ append-duckdb-metadata not found — copying lib without metadata"; \
		cp $$libpath $(EXTENSION); \
	fi

check:
	cargo run -- check

test:
	cargo test

# Train<->gold leakage firewall (spec 2026-06-10-human-verified-gold-corpus
# ac-05): guard unit tests + the standing identity audit over every gold
# column (anchor + corpus candidates). Non-zero exit on any overlap.
# Run before any training-data build; train_ydf.py imports the same guard.
leakage-guard:
	python3 scripts/test_gold_anchor_guard.py
	python3 scripts/audit_gold_anchor_leakage.py
	python3 scripts/eval_leakage/test_validate_corpus_firewall.py

# Taxonomy examples round-trip: a column of each type's own `examples` array must
# profile back to that type. Regression-gated against
# output/taxonomy-examples/baseline.json (exit 1 on a regression or a new
# unacknowledged fail). Needs the model artifact (models/default), so it is a
# standalone target, not wired into `ci`. Refresh the baseline after an
# intentional taxonomy/example change: append `-- --update-baseline`.
test-examples: build-release
	FINETYPE_BIN=target/release/finetype python3 scripts/test_taxonomy_examples.py $(ARGS)

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

# ─── GitTables CLI Evaluation ──────
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

# ─── SOTAB CLI Evaluation ─────────
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

# ─── Actionability Evaluation ────
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

# ─── Eval Report ─────────────────
# Generates a unified markdown dashboard from all eval outputs.
# Prerequisites: make eval-profile eval-actionability
#
# Usage: make eval-report

.PHONY: eval-report

eval-report: eval-profile eval-actionability validate-corpus
	@echo "═══ Generating evaluation report ═══"
	$(EVAL_RUN) eval-report -- \
		--profile-results eval/eval_output/profile_results.csv \
		--actionability-results eval/eval_output/actionability_results.csv \
		--labels-dir labels \
		--output eval/eval_output/report.md
	@echo "✓ Report written to eval/eval_output/report.md"
	@# ── Surface validate-corpus headline alongside profile-eval ──
	@# `validate-corpus` (above) writes its own report; we append the
	@# headline here so a single `make eval-report` run shows all three
	@# numbers (profile-eval, actionability-eval, validate-corpus) in
	@# eval/eval_output/report.md. The grep is robust to the report
	@# layout — it pulls only the bolded headline line.
	@if [ -f eval/eval_output/validate_corpus.md ]; then \
		echo "" >> eval/eval_output/report.md; \
		echo "## Validate-Corpus Headline" >> eval/eval_output/report.md; \
		grep -E '^\*\*[0-9]+ of [0-9]+ datasets pass' eval/eval_output/validate_corpus.md \
			>> eval/eval_output/report.md || true; \
	fi

# ─── Validate-Corpus Harness (spec 2026-04-28-validate-precision-corpus) ──
# Profile→validate round-trip over real-world CSVs in
# eval/datasets/validate_corpus/. Reports `N of M datasets pass at P=99%`
# and attributes each failing column to a deterministic mechanism.
#
# Output: eval/eval_output/validate_corpus.md (delta vs the committed
# eval/eval_output/validate_corpus.baseline.md if present).
#
# The target builds release binaries first so a fresh checkout's
# `make validate-corpus` succeeds without a prior `cargo build`.

.PHONY: validate-corpus

validate-corpus:
	@echo "═══ Building validate-corpus prerequisites (finetype + validate-corpus) ═══"
	cargo build --release --bin finetype
	cargo build --release -p finetype-eval --bin validate-corpus
	@echo "═══ Running validate-corpus harness ═══"
	./target/release/validate-corpus \
		--manifest eval/datasets/validate_manifest.csv \
		--output eval/eval_output/validate_corpus.md \
		--baseline eval/eval_output/validate_corpus.baseline.md \
		--finetype-bin ./target/release/finetype
	@echo "✓ Report written to eval/eval_output/validate_corpus.md"

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
	$(EVAL_RUN) eval-mapping -- --validate
	@echo "✓ eval/schema_mapping.csv generated and validated"

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

# ─── Community-extensions build contract ──────────────────────────────────────
# Drives the DuckDB extension-ci-tools makefiles so a tagged ref of this repo is
# buildable by duckdb/community-extensions CI without manual steps. The community
# harness runs `make configure_ci` then `make release` / `make test_release` at
# the repo root; the targets below satisfy that contract while building the
# in-tree workspace member `finetype_duckdb` (not the whole workspace).
#
# STABLE C API: USE_UNSTABLE_C_API is deliberately left unset → append metadata
# stamps the stable C_STRUCT ABI at the v1.2.0 floor, so one artifact loads on
# DuckDB 1.2 through 1.5+ (choice 0063 pin-strategy addendum). This is the fix
# for the community-channel 404 — the standalone repo used the *unstable* C API,
# which version-locked each artifact to one exact DuckDB release.
#
# Local quick build (no Python): use `make build-release` above instead.
EXTENSION_NAME        := finetype
TARGET_DUCKDB_VERSION := v1.2.0

include extension-ci-tools/makefiles/c_api_extensions/base.Makefile

# Cross-compile target selection (the community matrix builds osx_arm64 + osx_amd64
# from a single macOS runner). Mirrors extension-ci-tools/rust.Makefile, but our
# build targets a single workspace member rather than the whole workspace.
DUCKDB_CARGO_TARGET :=
DUCKDB_CARGO_OUT    := target
ifeq ($(DUCKDB_PLATFORM),osx_amd64)
	DUCKDB_CARGO_TARGET := --target x86_64-apple-darwin
	DUCKDB_CARGO_OUT    := target/x86_64-apple-darwin
else ifeq ($(DUCKDB_PLATFORM),osx_arm64)
	DUCKDB_CARGO_TARGET := --target aarch64-apple-darwin
	DUCKDB_CARGO_OUT    := target/aarch64-apple-darwin
endif

.PHONY: configure all release debug test_release test_debug clean_ext clean_ext_all

configure: venv platform extension_version
all: release

# The version file is REWRITTEN on every configure step, and the phony
# prerequisite below is the whole mechanism.
#
# base.Makefile declares configure/extension_version.txt as a file target with
# no prerequisites, so make treats an existing one as up to date and does not run
# the autodetection again. Two consequences, and the first one shipped: the file
# used to be tracked, so a checkout arrived with it, the autodetection did not
# run in CI, and the artefacts published between the commit that added the file
# and the commit that removed it carry the committed literal rather than their
# tag. Untracking it fixes CI, where each checkout is fresh, and does nothing
# for a developer's tree, where the previous build's file persists and a second
# tag built in it keeps the FIRST tag's version.

#
# A phony prerequisite is never up to date, so the target never is. This adds a
# prerequisite to the included rule rather than redefining its recipe: a second
# recipe would print "overriding recipe for target" and pin this repository to a
# copy of upstream's implementation.
#
# `scripts/check_extension_stamp.py --regenerates-version-file` asks make itself
# whether this still holds, and --untracked-version-file refuses the file's
# return to git.
.PHONY: extension_version_is_regenerated
extension_version_is_regenerated:

configure/extension_version.txt: extension_version_is_regenerated

build_extension_library_release: check_configure
	DUCKDB_EXTENSION_NAME=$(EXTENSION_NAME) DUCKDB_EXTENSION_MIN_DUCKDB_VERSION=$(TARGET_DUCKDB_VERSION) \
		cargo build -p finetype_duckdb --release $(DUCKDB_CARGO_TARGET)
	mkdir -p $(EXTENSION_BUILD_PATH)/release/extension/$(EXTENSION_NAME)
	cp $(DUCKDB_CARGO_OUT)/release/$(EXTENSION_LIB_FILENAME) $(EXTENSION_BUILD_PATH)/release/$(EXTENSION_LIB_FILENAME)

build_extension_library_debug: check_configure
	DUCKDB_EXTENSION_NAME=$(EXTENSION_NAME) DUCKDB_EXTENSION_MIN_DUCKDB_VERSION=$(TARGET_DUCKDB_VERSION) \
		cargo build -p finetype_duckdb $(DUCKDB_CARGO_TARGET)
	mkdir -p $(EXTENSION_BUILD_PATH)/debug/extension/$(EXTENSION_NAME)
	cp $(DUCKDB_CARGO_OUT)/debug/$(EXTENSION_LIB_FILENAME) $(EXTENSION_BUILD_PATH)/debug/$(EXTENSION_LIB_FILENAME)

release: build_extension_library_release build_extension_with_metadata_release
debug:   build_extension_library_debug   build_extension_with_metadata_debug

test_release: test_extension_release
test_debug:   test_extension_debug

clean_ext:     clean_build
clean_ext_all: clean_configure clean_build
