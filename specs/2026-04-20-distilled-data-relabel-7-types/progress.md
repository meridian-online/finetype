# Implementation Progress

**Spec:** `specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml` (v1.3)
**Started:** 2026-04-20
**Branch:** `distilled-data-relabel-7-types-v17`
**Lead:** @nightingale

## Hard Constraints

- [ ] Sourcing policy: public datasets (Kaggle / GitHub) OR synthetic generators only. No restricted registry scraping.
- [ ] http_method handled entirely via YAML schema — no new distilled rows, no generator changes.
- [ ] HTTP-method enum strategy: enumerate all 27 case variants in both `enum` AND `pattern`. Do NOT rely on `(?i)` alone — `CompiledValidator` applies pattern AND enum conjunctively.
- [ ] SSN remains synthetic-only. Do not scrape / download / store real SSNs.
- [ ] Generator improvement bar: ≥1000 unique values per generator (SWIFT BIC, CPT, SSN) and ≥500 for Excel format.
- [ ] Per-type source loaders under `output/distillation-v4/loaders/`. Generator improvements in existing generator modules. No patching of `output/distillation-v3/` in place.
- [ ] Fallback-to-generator path-swap fast path: SOURCES.md + progress.md note, no spec bump. Full spec bump only if target/gate/scope changes.
- [ ] Retrain methodology: 3-seed sweep (seeds 42, 43, 44), 100 epochs each, fresh. Output dirs `models/sherlock-v17-seed-{42,43,44}/`.
- [ ] Training gate: reject `val_acc < 88%`. `88–91.2%` requires Hugh sign-off, not automatic rejection.
- [ ] Eval promotion gate: `v17_winner ≥ max(235/242, v16-baseline-at-corpus-freeze)`. v16 baseline pinned via git SHA of eval inputs.
- [ ] Decision 0049 amended in place, NOT superseded. Status remains `accepted`.
- [ ] N=1 email regression OUT OF SCOPE. Tracked separately in `specs/2026-04-20-v16-n1-email-regression/`.
- [ ] Label remap validator script required (ac-05) — not manual grep.
- [ ] Release ordering strict: HF upload → FINETYPE_CI_MODEL bump + models/default flip in same PR.

## Acceptance Criteria

- [x] **ac-01** — `output/distillation-v4/loaders/user_agent.py` (17,812 unique UAs via ua-parser/uap-core Apache-2.0 fixtures) + `output/distillation-v4/loaders/loinc.py` (2,109 unique codes via NIH NLM Clinical Tables API). Both CSVs emitted with `value,label` schema. Cached under `loaders/_cache/` for offline re-runs. **LOINC attribution string captured for propagation to ac-12 HF model card.**
- [x] **ac-02** — Generator improvements in `crates/finetype-core/src/generator.rs`: SWIFT BIC (~175 ISO country codes, 8/11-char mix, XXX branch suffix), CPT (Cat I 00100-99999 / Cat II NNNNF / Cat III NNNNT), Excel format (12 weighted branches incl. locale prefix, threshold conditionals, multi-section, colour codes), SSN (full SSA range + dashed/undashed mix). `labels/definitions_identity.yaml` + `definitions_representation.yaml` patterns widened for new SSN/Excel output. `cargo run -- check` passes 240/240. `cargo test -p finetype-core` passes 161/161.
- [x] **ac-03** — `output/distillation-v4/SOURCES.md` written. Covers all 7 types with per-type path, source, license, v16 failure motivation, and (for http_method) the 3-surface cascade. LOINC attribution obligation flagged for ac-12.
- [x] **ac-04** — `scripts/prepare_multibranch_data.py` updated: (a) `_DROP_DISTILLED_TYPES` narrowed to 5 types (SSN, SWIFT BIC, CPT, Excel format, http_method); (b) new `_V4_OVERRIDE_TYPES = {user_agent, loinc}`; (c) `_DROP_ALL_TYPES` preserved as union to keep v3 contamination filter behaviour unchanged; (d) new `load_v4_distilled_columns()` loads CSVs from `output/distillation-v4/`, deterministic shuffle + chunk into synthetic columns of `--v4-column-size` (default 100) values; (e) merged into pipeline after v3 filter step with v3-overlap warning. Smoke test: 2 files, 19,921 rows, 22 LOINC cols + 179 UA cols.
- [x] **ac-05** — `scripts/validate_label_remap.py` exists; chain traversal; exits non-zero on broken chain. Smoke-tested clean (240 canonical keys, 37 remap entries, exit 0) and fixture-tested broken (exit 1).
- [x] **ac-06** — `labels/definitions_technology.yaml` L283-298 enum + pattern both enumerate 27 HTTP-method case variants. `cargo run -- check` passes 240/240. SOURCES.md entry deferred to ac-03.
- [x] **ac-07** — Unit test `ac07_http_method_case_variants` in `crates/finetype-core/src/validator.rs`: 27 positives + negatives (GOAT/SAN JOAQUIN/PATROL; gET/POSt; " GET"/"GET \n"; "GET /"/"POST /users"). All 46 validator tests pass.
- [x] **ac-08** — Decision 0049 amended in place: HTML comment under frontmatter + "Update 2026-04-20 — v17 layers distilled data on top" section. Status stays `accepted`.
- [ ] **ac-09** — 3-seed sweep completes via `scripts/sweep_v17.sh`. All-seeds-fail-floor halt path exercised if needed. results.json + epochs.jsonl per seed.
- [ ] **ac-10** — v16 baseline captured at corpus-freeze → `v16-baseline.md` with score/git-SHA/timestamp/CLI-version.
- [ ] **ac-11** — Profile eval per seed; winner = highest profile > highest val_acc > lowest seed. Hugh sign-off for manual-review band. No-promotion halt if winner < max(235, v16_baseline).
- [ ] **ac-12** — HF upload first (curl -fsI captured in progress.md with timestamp); same-PR workflow bump + symlink flip; drift-check silent; rollback if ac-13 fails.
- [ ] **ac-13** — v0.6.18 released: Cargo.toml bump, tag push, 5-platform binaries, Homebrew bump, report.md refresh.
- [x] **ac-14** — 4 MADR files drafted: 0050 (sourcing policy), 0051 (http_method ENUM-only + 3-surface cascade), 0052 (scope-aware eval gate), 0053 (training gate 88% floor + manual-review band). Each references the v17 spec.

## Sequencing

```
Day 1 (today, 2026-04-20): schema / validator / decision warm-up
  ac-06, ac-07, ac-05, ac-08, ac-14 (draft MADRs)

Day 2 (2026-04-21): data sourcing + pipeline
  ac-01 (loaders), ac-02 (generators), ac-03 (SOURCES.md), ac-04 (prep script)

Day 2 evening: corpus freeze
  ac-10 (v16 baseline capture), kick off ac-09 (sweep) overnight

Day 3 (2026-04-22): eval + ship
  ac-11 (winner selection + Hugh sign-off loop)
  ac-12 (promotion PR)
  ac-13 (v0.6.18 release)
  ac-14 (finalise MADRs if any pending)
```

## Day 1 log

**2026-04-20** — Day 1 COMPLETE. 5 of 14 ACs done:

- **ac-05** — `scripts/validate_label_remap.py` written (argparse, regex canonical-key scan, chain traversal with cycle/depth guards, exit 0/1). Clean smoke test: 240 canonical keys, 37 remap entries, all chains resolve. Fixture test (broken chain injected) → exit 1 with explicit error per chain.
- **ac-06** — `labels/definitions_technology.yaml` http_method block: pattern + enum both enumerate 27 case variants (9 methods × {UPPER, lower, Title}). Samples expanded (`GET`, `post`, `Delete`). `cargo run -- check` passes 240/240.
- **ac-07** — `ac07_http_method_case_variants` test added after `test_compiled_validator_enum` in `validator.rs`. 27 positive cases + negatives: GOAT/SAN JOAQUIN/PATROL/OPERATING/IN PROGRESS/ENROUTE (v16 bad-distilled tokens), gET/POSt/geT/DeLeTe/pATCH (mixed-case), whitespace variants, adjacent tokens ("GET /", "POST /users"). All 46 validator tests pass.
- **ac-08** — `decisions/0049-preserve-synthetic-for-bad-distilled-types.md` amended in place. Frontmatter HTML comment + "Update 2026-04-20 — v17 layers distilled data on top" section. Per-type treatments documented. Status `accepted`.
- **ac-14** — 4 MADRs drafted:
  - `0050-per-type-sourcing-policy.md` — public datasets OR generators only; sourcing_table as single source of truth.
  - `0051-http-method-enum-only.md` — YAML-schema ENUM-only for http_method; 3-surface cascade (YAML → validator → training prep).
  - `0052-scope-aware-eval-gate.md` — `max(235/242, v16_baseline_at_corpus_freeze)`; corpus freeze precedes baseline.
  - `0053-training-gate-88-floor.md` — 88% catastrophic floor + 88–91.2% manual-review band with Hugh sign-off.

Next: commit Day 1 work, then move to Day 2 data sourcing (ac-01, ac-02, ac-03, ac-04).

## Day 2 log

**2026-04-20** — Day 2 COMPLETE (same calendar day; early finish). 4 of 14 ACs done this push (9 of 14 total). Split across two parallel subagents + lead coordination:

**Agent A — ac-01 (public-dataset loaders, Python):**
- `output/distillation-v4/loaders/user_agent.py` — pulls ua-parser/uap-core test fixtures (Apache-2.0) from 3 raw GitHub URLs. 17,812 unique UAs.
- `output/distillation-v4/loaders/loinc.py` — NIH NLM Clinical Tables API (no auth, no click-through). 2,109 unique codes via single-letter term sweep a–d.
- Both loaders cache responses under `_cache/` for offline re-runs.
- LOINC attribution obligation captured in the loader docstring; must propagate to HF model card at ac-12.
- Deviation from spec: MIMIC-IV primary candidate swapped for NLM Clinical Tables (stricter authority, no credentials). Same-path refinement, not a fallback-to-generator path-swap.
- Pyright import-style fix: `from urllib.error import HTTPError` (submodule import style had stale diagnostic issue).

**Agent B — ac-02 (Rust generators):**
- `crates/finetype-core/src/generator.rs`: 4 generator blocks rewritten.
  - SWIFT BIC (~line 3393): expanded to ~175 ISO 3166-1 codes, 40% major-bank bias, 45/55 11-char vs 8-char, 15% XXX branch marker.
  - CPT (~line 2014): Category I 00100–99999 + Category II NNNNF + Category III NNNNT, weighted 85/8/6 with 1% rare PLA U.
  - Excel format (~line 2743): 12 weighted branches (locale-prefixed currency, threshold conditionals, 20-unit literal-suffix, multi-section with 6 colour codes, text placeholder, edge cases).
  - SSN (~line 2063): full SSA-valid ranges (area 001..899 excl 666; group 01..99; serial 0001..9999); 80% dashed / 20% undashed.
- `labels/definitions_identity.yaml` + `definitions_representation.yaml`: SSN pattern widened for dashed+undashed; Excel char-class extended with `@ * ! =` and `minLength: 2→1`.
- New unit tests `ac02_*_unique_and_structured` pin uniqueness bars.
- `cargo run -- check` → 240/240 passing, 12000/12000 samples (100%).
- `cargo test -p finetype-core` → 161 passed, 0 failed.
- Dead code noted (unreachable `("payment", "swift_bic")` at ~line 1812) left in place — out of v17 scope.

**Lead — ac-03 (SOURCES.md) + ac-04 (prep script):**
- `output/distillation-v4/SOURCES.md` — per-type entries for all 7 types + overview table + LOINC attribution block + fallback-to-generator section (empty for v17).
- `scripts/prepare_multibranch_data.py`:
  - New `_V4_OVERRIDE_TYPES = {user_agent, loinc}` introduced alongside narrowed `_DROP_DISTILLED_TYPES` (now 5 types).
  - `_DROP_ALL_TYPES = _V4_OVERRIDE_TYPES | _DROP_DISTILLED_TYPES` preserves the v3 contamination-filter invariant.
  - New `load_v4_distilled_columns(v4_dir, min_values, column_size, rng)` function: reads per-type CSVs, deterministic shuffle, chunk into ~100-value synthetic columns.
  - Wired into `main()` after `filter_distilled_columns()` step; v3-overlap warning + replace (not mix) semantics if any v3 rows leak through.
  - New CLI flags: `--distilled-v4-dir` (default `output/distillation-v4`), `--v4-column-size` (default 100).
  - Smoke test: 2 files loaded, 19,921 total values, 22 LOINC columns + 179 UA columns. All ac-04 grep assertions pass.

Next: corpus freeze + ac-10 (v16 baseline capture) + kick off ac-09 (sweep_v17.sh overnight).
