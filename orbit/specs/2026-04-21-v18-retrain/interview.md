# Design: v18 retrain

**Date:** 2026-04-21
**Interviewer:** Nightingale
**Card:** orbit/cards/0002-semantic-type-detection.yaml
**Mode:** design (guided, iteration 1)

---

## Context

v18 retrain on the expanded 352-column eval corpus. Unblocked by m-19
Phase A+B shipping (PR #42, merged 4631b3c). v16 is current production;
v17 is held on branch (decision 0054, net-zero eval delta).

### Baseline state

- **Default model:** sherlock-v16
- **v16 headline on pre-expansion 242-col eval:** 235/242 (97.1% label, 96.3% domain)
- **v16 diagnostic on expanded 352-col eval:** 297/352 (84.4% label, 91.8% domain)
- **Expanded corpus protections live:**
  - Realism pre-screen (`scripts/prescreen_eval.py`) against MADR 0055 floors
  - Coverage floor 240/240 via `coverage_closure_phase_ab.csv` (110 cols × 6 rows)
  - Two-layer leakage firewall: `eval/datasets/sources.yaml` role manifest + row-hash SHA256 filter at `scripts/prepare_multibranch_data.py` (237,860 distinct rows)
- **Sharpen state:** demotion guard shipped yesterday (PR #44) at `column.rs:3902`. Preserves validator-passing top-1 predictions against `disambiguate_categorical` demotion.
- **Held assets on branch `distilled-data-relabel-7-types-v17`:** v4 distilled loaders (17,812 UAs ex ua-parser/uap-core, 2,109 LOINCs ex NIH NLM Clinical Tables), generator improvements (SWIFT BIC / CPT / SSN / Excel format), widened patterns in `labels/definitions_identity.yaml` + `definitions_representation.yaml`, http_method ENUM-only strategy (decision 0051).

### Known inefficiency to fix

`scripts/sweep_v17.sh` line 170 passes `--seed $SEED` to the prep
script. `scripts/prepare_multibranch_data.py` threads the seed through
its RNGs (lines 2592, 1468, 2810, 2824, 2837), producing different
training data per training seed. Cost: ~30-60 min prep × 3 seeds = ~60
min redundant wall-clock, ~1.9 GB redundant disk. Also conflates
data-variance and training-variance in the 3-seed signal. v18 fixes
this by running prep once with a fixed `DATA_SEED=42` and reusing the
single `.ftmb` across all training seeds.

## Q&A

### Q1: Sweep data-prep discipline
**Q:** Fix data seed (vary training only) or per-seed prep like v17?
**A:** **Fix data seed, vary training only.** One prep run with DATA_SEED=42; 3 training seeds reuse the same .ftmb. Clean 3-seed signal measuring training-stability variance only.

### Q2: Training-corpus inheritance
**Q:** Which base does v18 build on — v17 corpus (v4), v16 corpus (v3), or v17 + targeted additions?
**A:** **Delegated to implementer per orbit principle (author goal: maximum accuracy).** Decision recorded as **TBD pending triage**. The failure triage in Q3 is the evidence that determines corpus base — if LOINC / user_agent / SWIFT / CPT failures persist on expanded eval, v4 corpus is the right base; if not, v3 is lower overhead.

### Q3: Triage timing
**Q:** Triage v16 failures on the expanded 352-col eval before the sweep, or sweep blind?
**A:** **Triage first — discovery pass.** Enumerate ~55 v16 misclassifications, categorise by type/error-pattern, decide per-type lever (data / Sharpen rule / architecture). Output gates corpus base (Q2) and scope (Q5).

### Q4: Promotion gate
**Q:** What's v18's promotion floor on the expanded 352-col eval?
**A:** **`v18_score ≥ 297/352` AND per-domain floor (no regression > 3 cols per domain).** Prevents a repeat of v17's 3-fixes / 3-regressions net-zero trap.

### Q5: Scope boundary
**Q:** What's v18 allowed to touch beyond training data + model weights?
**A:** **Delegated to implementer per orbit principle.** Recorded as:
- **Retrain + Sharpen adjustments permitted** under decision 0048 (value-based rules only).
- **Taxonomy edits only if triage surfaces a coverage gap** — i.e., an expanded-eval type missing from the 240-type taxonomy. Otherwise hold.
- **No CLI / Sharpen architecture changes.**

### Q6: Release packaging
**Q:** Ship as v0.6.18 public release, internal promotion only, or decide post-gate?
**A:** **Decide after promotion gate is cleared.** Train → evaluate → decide release scope with real numbers in hand.

---

## Summary

### Goal

Train and promote a sherlock-v18 model that improves v16's accuracy on
the expanded 352-column eval corpus (current baseline 297/352 =
84.4%), without regressing v16's domain accuracy (91.8%) or triggering
concentrated per-domain regressions.

### Constraints (load-bearing)

1. **Fixed data seed** — `DATA_SEED=42` pinned in prep. Training seeds (3) vary only in `train-multi-branch`. One `.ftmb` reused across all training seeds.
2. **Triage before sweep** — discovery pass enumerating v16 failures on expanded eval is a gate ACs before any training run. Output at `orbit/specs/2026-04-21-v18-retrain/triage.md` (schema: `dataset, column, gt_label, v16_label, error_category, proposed_lever`).
3. **Promotion gate** — `v18_score ≥ 297/352` on expanded eval AND no per-domain regression > 3 columns. Pinned v16 baseline via git SHA of eval inputs (same pattern as v17's `v16-baseline.md`).
4. **Leakage firewall must be active** — row-hash filter in prep script on by default (verified via log line), `sources.yaml` role=eval for every manifest source, realism pre-screen passing.
5. **Scope: retrain + Sharpen adjustments (value-based rules only per decision 0048).** Taxonomy edits permitted only if triage surfaces a coverage gap. No CLI changes.
6. **3-seed training-only sweep** — seeds {42, 43, 44}, 100 epochs, patience 15, one shared `.ftmb`. Training gate from decision 0053 (floor 88%, auto-accept 91.2%).
7. **Sibling-context attention preserved** — `classify_columns_with_context` remains the default profile path. v18 training data should include sibling-context synthetic columns (the v13 config already has the sibling-context branch; confirm it's exercised in prep).
8. **No release promotion before gate cleared** — `models/default` symlink unchanged until gate is cleared and release scope is decided.

### Success criteria

- [triage] `triage.md` enumerates every v16 failure on the expanded
  352-col eval (~55 cols), categorised by error type and lever.
- [sweep] 3 training seeds run against a single `.ftmb`; per-seed
  `val_acc` + eval recorded in `results/sweep-v18-summary.csv`.
- [gate] Winner's expanded-eval score ≥ 297/352 AND per-domain
  regression floor holds.
- [evidence] Per-column diff v16→v18 emits fixes, regressions,
  persistent failures (same format as v17 close-out table).
- [decision] MADR recording v18's outcome, corpus base choice (Q2),
  and scope decision (Q5). If held: status `accepted` documenting
  hold rationale. If promoted: status `accepted` with release
  follow-up.

### Decisions surfaced

- **Sweep data-seed discipline (Q1)** — new decision: prep runs with
  fixed DATA_SEED, training seeds vary only. Retires per-seed prep
  pattern from v16/v17 sweeps. → will be recorded as a new MADR
  during spec stage.
- **v18 corpus base (Q2)** — TBD, triage-informed. Recorded as a
  design-deferred decision in the spec; the trigger is the first ac
  of the implementation (triage).
- **v18 scope (Q5)** — retrain + Sharpen (value-based) + conditional
  taxonomy. Recorded as a constraint in the spec.

### Open questions (carried into spec)

- What's the exact v16 per-domain breakdown on the expanded eval?
  (Needs to be computed in triage — informs the per-domain floor in
  the promotion gate.)
- Is the sibling-context-collapse behaviour observed for
  `excel_format` / `http_method` in the sharpen-demotion-guard eval
  symptomatic of a broader v16 weakness that v18 training should
  target? (Triage will surface the scale.)
- Does the v4 distilled corpus on branch contain any rows that
  now hash-collide with the expanded eval corpus? (Must verify
  before adopting; the row-hash firewall will block them but we
  need to know the overlap size.)
