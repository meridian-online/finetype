# Implementation Progress

Spec path: orbit/specs/2026-04-27-sharpen-rule-audit/spec.yaml
Spec hash: sha256:4e54768c7bfb92837e58458224554118f3b7fba5b6ff169bc4ed84111b8629ea
Started: 2026-04-27
Current AC: complete

## Hard Constraints
- [x] Full 448-row manifest for all measurements — no coverage_closure exclusion
- [x] Rules + model ship together in one PR
- [x] No Sharpen rule additions — only removals and narrowing
- [x] All three Sharpen subsystems in scope: feature_sharpen (F1-F6), value_sharpen (R rules + disambiguate_* functions), header hints (apply_header_sharpen)
- [x] v19-relu-s42 is the candidate model (best ReLU seed from overnight sweep)
- [x] Existing tests for removed rules are deleted — not left as dead code
- [x] Sharpen demotion guard (decision 0059) is audited but expected to be net-positive
  - Demotion guard removed along with disambiguate_categorical — no demotion rules remain to guard

## Detours

## Acceptance Criteria
- [x] ac-01: Ablation script measuring each Sharpen rule's individual net impact
  - Script: `scripts/sharpen_ablation.sh`
  - Output: `diagnostics/sharpen_ablation.tsv` (72 rules), `diagnostics/sharpen_per_column.tsv` (448 rows)
  - Key finding: raw model 317/448, sharpened 369/448, Sharpen net +52
- [x] ac-02 (gate): Ablation results reviewed, each rule categorised KEEP/REMOVE/NARROW
  - REMOVE (3): disambiguate_small_integer_ordinal (net -2), categorical_low_cardinality (net -1), categorical_single_char (net 0)
  - NARROW: none (vacuous)
  - KEEP: all 54 net-positive rules
  - 7 header hints net -1: deferred to model improvement
- [x] ac-03: All REMOVE-verdict rules deleted from column.rs
  - Removed disambiguate_small_integer_ordinal function + 2 call sites
  - Removed disambiguate_categorical function (both branches + demotion guard) + 2 call sites
  - 9 test functions + 3 fixture functions deleted. 406/406 tests pass (was 413)
- [x] ac-04: All NARROW-verdict rules edited (vacuous — no NARROW verdicts)
- [x] ac-05: MADR 0069 gate amendment recorded
  - `orbit/decisions/0069-gate-amendment-rule-count-decrease.md`
  - Amends MADR 0066 §3: accept candidate-parity when rule count decreases
- [x] ac-06 (gate): v19-relu-s42 + cleaned pipeline passes amended gate
  - Score: 369/448 (≥ 365 pre-cleanup threshold) ✓
  - Per-domain max regression: -2 (ceiling: 3) ✓
  - Per-column diff: 14 fixes / 16 regressions / 355 stable_hit / 43 persistent_same / 20 persistent_churn
  - Score file: `diagnostics/v19_cleaned_profile_score.txt`
- [x] ac-07: v19-relu-s42 promoted as models/default
  - `models/default → sherlock-v19-relu-s42`
  - `FINETYPE_CI_MODEL: sherlock-v19-relu-s42` in ci.yml + release.yml
- [x] ac-08: CLAUDE.md updated
  - Default model: sherlock-v19-relu-s42
  - Rule counts: 20 value-based rules (was 23, removed 3)
  - Test count: 406 (was 413)
  - Profile eval: 369/448
  - Decisions: 48 (was 46, added 0068 + 0069)
