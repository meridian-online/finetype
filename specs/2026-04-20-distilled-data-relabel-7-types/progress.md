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

- [ ] **ac-01** — Per-type loaders under `output/distillation-v4/loaders/` for public-dataset types in sourcing_table (currently user_agent, LOINC). Fallback-to-generator if candidates fail review.
- [ ] **ac-02** — Generator improvements for SWIFT BIC, CPT, Excel format, SSN; each meets target + `improvements_required` list.
- [ ] **ac-03** — `output/distillation-v4/SOURCES.md` consistent with top-level `sourcing_table:` field.
- [ ] **ac-04** — `scripts/prepare_multibranch_data.py` consumes v4 CSVs; `_DROP_DISTILLED_TYPES` updated (user_agent + LOINC removed; SWIFT/CPT/Excel/SSN/http_method remain).
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
