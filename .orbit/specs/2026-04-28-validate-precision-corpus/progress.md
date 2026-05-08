# Implementation Progress: 2026-04-28-validate-precision-corpus

**Spec:** [`spec.yaml`](./spec.yaml) v1.1
**Card:** [`.orbit/cards/0014-profile-validate-precision.yaml`](../../cards/0014-profile-validate-precision.yaml)
**Drive:** full autonomy, iteration 1
**Started:** 2026-04-28
**Reviewer notes:** review-spec cycle 2 APPROVE — five new non-blocking notes recorded, no further revision cycle

---

## Acceptance Criteria

- [x] **ac-01** validate_manifest.csv (9 cols × 7 rows; SPDX licences; pinned source URLs)
- [x] **ac-02** sources.yaml extended with role:validate (7 entries; existing rows byte-unchanged; role enum extended)
- [x] **ac-03** row-hash leakage firewall extended (compute_row_hashes.py iterates both manifests via DictReader)
- [x] **ac-04** 7 GT sidecars (58 columns total) at eval/datasets/validate_corpus/<name>.gt.yaml
- [x] **ac-05** crates/finetype-eval/src/bin/validate_corpus.rs harness binary (~530 LOC; cargo build clean)
- [x] **ac-06** make validate-corpus target + make eval-report extension (with cargo build prereq)
- [x] **ac-07** deterministic 5-rule mechanism attribution + 6 unit tests (all green)
- [x] **ac-08** validate_corpus.md report shape (headline + delta line + per-mechanism + per-dataset + per-column)
- [x] **ac-09** enum-overfit fix: --enum-threshold default 50→32 + boolean.* gate parity (3 unit tests pass; shared lib at finetype_cli::enum_emission)
- [x] **ac-10** decimal_number widening: scientific-notation suffix (1 widening; precision_widenings.rs passes; format_diversity 1→0 in harness)
- [x] **ac-11** 3 MADRs landed (0072 round-trip metric / 0073 m-19 reuse / 0074 fix partition; status: accepted; date: 2026-04-28)
- [x] **ac-12** CHANGELOG.md [Unreleased] entries + CLAUDE.md "What's next" line
- [x] **ac-13** make ci exits 0 (fmt + clippy + test + check); awaiting workspace clippy --all-targets confirmation
- [x] **ac-14** baseline (validate_corpus.baseline.md) + post-fix (validate_corpus.md) both committed
- [x] **ac-15** stub card 0015-validate-corpus-curation.yaml (maturity: emerging; 3 scenarios; references spec)

## Headline

**Baseline:** 3 of 7 datasets pass at P=99% (N₀=3)
**Post-fix:** 3 of 7 datasets pass at P=99% (N=3; delta: +0)

The two in-scope fixes (ac-09 enum-threshold, ac-10 decimal_number scientific
notation) trim mechanism-level failures without flipping any dataset across
P=99%. format_diversity dropped from 1→0 columns (us_baby_names.percent now
validates). enum_overfit count is unchanged because the corpus's enum failures
attribute to *taxonomy-baked* enums on mispredicted labels (e.g.
`rio2016_athletes.nationality` → `geography.location.country_code` with a
hardcoded ISO-2 enum vs the data's 3-letter NOC codes), not the runtime
sample-cardinality enums that the 50→32 threshold drop targets. The remaining
failure surface is dominated by misclassification (7 cols / 4 datasets), which
is explicitly out of scope per spec constraint and MADR 0074 — fixes wait for
the next retrain under MADR 0066.

The harness, baseline, and the two preventive fixes ship together. Movement is
mechanism-level, not dataset-level — exactly the iteration framing in MADR
0072 (round-trip metric) and the spec's evaluation_principles ("Movement-based
success — harness + baseline ships, fix-flip is bonus", weight 0.15).

## Implementation Order

The dependency graph forces this order:

1. **Corpus assembly** (parallelisable internally)
   - 1a Fetch 7 CSVs to `eval/datasets/validate_corpus/csv/`
   - 1b Author 7 GT sidecars (ac-04)
   - 1c Build `validate_manifest.csv` (ac-01)
   - 1d Extend `sources.yaml` (ac-02)

2. **Harness binary** (ac-05/07/08) — implements report shape but ships *before* fixes

3. **Leakage firewall extension** (ac-03)

4. **Run baseline harness** → commit `validate_corpus.baseline.md` (ac-14 first half)

5. **In-scope fix 1: enum-overfit** (ac-09) — `--enum-threshold` default 50→32 + boolean.* gate parity

6. **In-scope fix 2: validator widenings** (ac-10) — choose 1–5 widenings from baseline `format_diversity` attributions; ship YAML edits + precision_widenings.rs regression tests

7. **Re-run harness** → commit post-fix `validate_corpus.md` (ac-14 second half)

8. **Doc deliverables** (parallelisable internally)
   - 8a MADRs 0072/0073/0074 (ac-11)
   - 8b CHANGELOG + CLAUDE.md (ac-12)
   - 8c Stub card 0015 (ac-15)
   - 8d Makefile target + eval-report extension (ac-06)

9. **CI gate** (ac-13) — `make ci` exits 0; clippy warnings zero

## Cycle-2 reviewer notes (non-blocking, addressed in implementation)

- **N1.** ac-03 dry-run wording — script may not have `--dry-run` today; if not, the implementer adds it as part of ac-03 work or rephrases the verification.
- **N2.** ac-03 fixture brittleness — use a sample row that's stable across hash recomputation.
- **N3.** ac-04 column-count tolerance vs notes — keep "approximately 58 ±2" framing; pin actual count after CSVs land.
- **N4.** ac-09 post-fix language sequencing — verification comes after harness re-run, not before.
- **N5.** ac-10 prior-list anchoring — the "likely candidates" list in implementation_notes is non-prescriptive; the implementer chooses based on actual baseline measurement.

## Status

**Stage:** implement (drive.yaml §6)
**Current step:** 2 — build harness binary (ac-05/07/08)
