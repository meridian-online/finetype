# Spec Review

**Date:** 2026-04-18
**Reviewer:** Context-separated agent (fresh session)
**Spec:** .orbit/specs/2026-04-18-v16-data-audit-retrain/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] Coverage gap count is wrong — 64 uncovered types should be ~100

**Category:** assumption
**Description:** The spec states "64 types have zero training examples" (interview Q6) and sizes the synthetic data effort at "64 x 600 = ~38k rows" (ac-04). The actual number is substantially larger. Cross-referencing the v14 label map (240 types) against the distilled training data (sherlock_distilled.csv.gz, 176 raw labels) with the label remap applied (data/label_remap.json, 35 mappings), only 140 taxonomy types have any distilled training data. The remaining 100 types are uncovered by distilled data.

The v14 training manifest (v14-blend-70-30.manifest.json) confirms this: `blended_types: 239` with 121 having distilled data and 118 being synthetic-only. (One type — `container.object.json_array` — appears to be excluded entirely.)

The "64" figure may come from a calculation that double-counted some remapped labels, or from an older taxonomy version. Regardless, the synthetic data generation effort is nearly double what the spec estimates: ~100 x 600 = ~60k rows, not ~38k. This affects the training data balance (70/30 distilled/synthetic ratio shifts toward synthetic), training time estimates, and the risk of synthetic data regression.

**Evidence:** `python3` analysis of `models/sherlock-v14/label_map.json` (240 types), `output/distillation-v3/sherlock_distilled.csv.gz` (176 raw labels, 140 after remap), and `output/multibranch-training/v14-blend-70-30.manifest.json` (118 synthetic-only types).
**Recommendation:** Correct ac-04 to state the actual uncovered count. Re-derive the number by running the label remap against the current taxonomy. Update the row estimate. Consider whether the 600/type cap is still appropriate given that synthetic data will now constitute a larger share of training data than stated.

---

### [MAJOR] ac-02/ac-03 operate on the raw distilled CSV but training uses remapped labels — which labels does the audit target?

**Category:** missing-requirement
**Description:** The model-as-critic audit (ac-02) will run v14 inference on "all 102k training rows." But the distilled CSV contains 37 labels not in the current taxonomy (e.g., `geography.admin.county`, `representation.text.sentence`, `finance.instrument.ticker_symbol`). The training pipeline remaps these via `data/label_remap.json` before training, meaning the model has never seen the raw labels — it was trained on the remapped versions.

When the v14 model predicts `geography.location.region` for a row labeled `geography.admin.county` in the raw CSV, that is not a "disagreement" — it is expected behavior because `geography.admin.county` was remapped to `geography.location.region` during training. The spec does not say whether the audit should:
(a) Compare model predictions against raw CSV labels (will flag ~243 false disagreements from label remap),
(b) Compare against remapped labels (correct, but requires applying remap first), or
(c) Compare against something else entirely.

Similarly, ac-03 says to clean the distilled CSV directly (`sherlock_distilled.csv.gz`), but the training pipeline reconstructs FTMB files from scratch using `prepare_multibranch_data.py` with its own label remap, filtering, decontamination, and augmentation. Fixing the raw CSV may not propagate to training unless the data prep script is also updated.

**Evidence:** `data/label_remap.json` contains 35 non-trivial label mappings. `scripts/overnight_v14_retraining.sh` passes `--label-remap data/label_remap.json` to `prepare_multibranch_data.py`.
**Recommendation:** Specify that the model-as-critic audit must apply the label remap before comparing predictions to training labels. Add a constraint or AC requiring that data corrections flow through the data prep pipeline (not just the raw CSV) to reach the FTMB training file.

---

### [MAJOR] No rollback plan if v16 regresses

**Category:** missing-requirement
**Description:** ac-06 requires "no regressions on currently correct columns" but does not specify what happens if the retrained model improves on some columns but regresses on others (net improvement but not zero-regression). The v12 data quality audit review (same repo) documented exactly this scenario: v12 gained 13 columns but lost 15, for a net regression. The exit condition "Profile eval >= 221/227 with zero regressions" may be unachievable — every previous retrain has had some churn.

The "zero regressions" constraint combined with "221+ correct" creates a very narrow success window. If v16 scores 222/227 but regresses on 1 column that was correct in v14, the spec says it fails. There is no provision for: (a) accepting a regression if it fixes a GT error, (b) iterating with a second training run, (c) falling back to v14, or (d) adjusting rules to compensate.

**Evidence:** v13 retrain: +11 columns but had churn. v14 retrain: net +3 but had churn. v15 hint narrowing: +3 net. Every model version has had at least some regressions alongside improvements. The zero-regression exit condition has never been achievable in practice.
**Recommendation:** Soften the regression constraint to "no net regressions on domains that were 100% correct in v14" or "any regression must be explained and documented in the eval report." Add a rollback plan: if v16 does not meet the bar after one training run, what is the next step? Second run with different hyperparameters? Rule additions? Accept v14 as final?

---

### [MAJOR] ac-01 "manually verify all 338 labels" lacks a methodology

**Category:** test-gap
**Description:** ac-01 requires all 338 eval labels to be "manually verified against actual column values." The verification is: "Audit report exists listing every correction made." This means if the auditor reviews all 338, makes zero corrections, and writes "all correct," the AC passes. There is no evidence standard for what constitutes adequate manual verification.

For 338 columns across 35 datasets, manual verification means looking at actual column values and confirming the label matches. But many types are ambiguous without domain context — is a column of 3-letter codes `iata`, `iso_alpha3`, or `currency_code`? The auditor needs access to column headers, sample values, and ideally the dataset context. The spec does not describe the verification procedure, what tools to use, or how to document the review for each of the 338 labels.

The v12 audit (prior art) was scoped to 23 misclassified items and produced a detailed per-item audit table with evidence. Scaling that to 338 items requires either a much lighter touch or significant effort. The spec should set expectations.

**Evidence:** Spec ac-01 verification: "Audit report exists at specs/.../eval-audit.md listing every correction made. Corrected manifest.csv committed." The report could list zero corrections and still pass.
**Recommendation:** Require the audit report to document the review methodology (e.g., "for each label, inspected 5 sample values and confirmed type match"). For labels where the type is unambiguous (e.g., `id`, `boolean`, `number`), a bulk confirmation is fine. For ambiguous labels (e.g., `code`, `category`, `name`), require explicit evidence. This focuses manual effort where it matters.

---

### [MODERATE] ac-02 "model-as-critic" may produce thousands of disagreements with no triage priority

**Category:** failure-mode
**Description:** The interview explicitly flags this as an open question: "How many training data disagreements will the model-as-critic find?" With 102k rows and a model at 91.2% val_acc, a rough estimate is ~9k disagreements (8.8% error rate on the validation set, likely higher on the full training set since the model has memorized some training data and will disagree less on training examples, but the distilled data includes test-split rows too).

The heuristic adjudication (validation regex auto-adjudicate) will handle some, but the spec does not define what "manual review a sample of the remainder" means. What sample size? How is the sample selected? What if the sample reveals systematic issues in a label that require reviewing all examples of that label?

**Evidence:** Interview Q4 answer: "Heuristic + sample — use validation patterns to auto-adjudicate where possible." Spec ac-02 verification: "Audit report exists with disagreement count, breakdown by label, and resolution summary."
**Recommendation:** Add a concrete sample methodology: e.g., "Review at least 10 disagreements per label for labels with >50 disagreements, or all disagreements for labels with <=50." Define what "resolution" means for the summary: were disagreeing rows corrected, removed, or kept?

---

### [MODERATE] Training time estimate may be significantly off

**Category:** constraint-conflict
**Description:** The spec estimates "75-100 epochs, ~3-4 hours on Metal." v14 trained 50 epochs in 127 min (~2.5 min/epoch). Scaling linearly, 75 epochs = ~188 min (~3.1 hours), 100 epochs = ~250 min (~4.2 hours). But the spec also adds ~60k synthetic rows to the training data (100 types x 600, not 64 x 600), increasing the dataset from ~131k to potentially ~160k+ rows. More data means longer epochs.

A rough estimate: 160k/131k = 1.22x data increase, so ~3.1 min/epoch, meaning 75 epochs = ~232 min (3.9 hours), 100 epochs = ~310 min (5.2 hours). This is not a blocker but the time estimate in the spec should be updated.

**Evidence:** v14: 50 epochs, 127 min, 131k records. v16: 75-100 epochs, ~160k records (corrected).
**Recommendation:** Update the training time estimate in the constraints to account for the larger dataset. Consider whether 100 epochs is the right upper bound — v14 achieved best val_acc at epoch ~35-40. More epochs with more data may plateau earlier.

---

### [MODERATE] Spec does not address the 5 unresolved "extra" training labels

**Category:** missing-requirement
**Description:** After applying label_remap.json, 5 training labels still do not map to any taxonomy type: `finance.currency.amount_minor_int`, `representation.scientific.metric_prefix`, `representation.text.paragraph`, `representation.text.sentence`, and `yes`. These represent training examples that will either be dropped by the data prep pipeline or cause label collisions. The spec's training data audit should account for these, but neither ac-02 nor ac-03 mentions resolving unmapped labels.

**Evidence:** Cross-reference of `data/label_remap.json` (35 mappings) against `sherlock_distilled.csv.gz` (37 non-canonical labels) shows 5 labels with no remap entry. The `yes` label is clearly an error/artifact.
**Recommendation:** Add to ac-03 (training data cleanup): "Resolve all training labels that do not map to current taxonomy types. Either add remap entries to data/label_remap.json or remove the rows."

---

### [MINOR] ac-04 verification says "All 240 taxonomy types have >= 1 training example" but 239 types were blended in v14

**Category:** assumption
**Description:** The v14 manifest shows `blended_types: 239`, not 240. One type (`container.object.json_array` or similar) appears to be excluded. ac-04 requires all 240 types to be represented. If the generator for one type fails, the AC fails. The spec should acknowledge this possibility and define the fallback.

**Evidence:** `v14-blend-70-30.manifest.json`: `"blended_types": 239`, `"taxonomy_types": 240`.
**Recommendation:** Verify which type is missing from v14 and ensure its generator works. If a generator cannot produce valid data for a type, document the exception rather than blocking on it.

---

### [MINOR] ac-07 golden test count may be stale

**Category:** assumption
**Description:** ac-07 says "passes all 13 tests" but the test count may have changed since the spec was written. The CLAUDE.md says "413 model tests" but the golden test count is separate. If tests were added or removed, the "13 tests" assertion in the AC becomes a latent failure.

**Evidence:** Spec ac-07: "all 13 tests (may need assertion updates for improved predictions)." The parenthetical acknowledges that assertions may change, but the count should not be hardcoded.
**Recommendation:** Change to "all golden integration tests pass" without specifying a count.

---

### [MINOR] No version bump plan

**Category:** missing-requirement
**Description:** The spec produces a v16 model and updates the default symlink, but does not mention bumping the crate version (currently 0.6.16). Previous model promotions have been accompanied by version bumps and releases. If v16 changes predictions for existing types, downstream consumers (DuckDB extension, MCP server, Homebrew) need a new release.

**Evidence:** CLAUDE.md: "Version: 0.6.16." Distribution includes crates.io, Homebrew, GitHub releases, DuckDB extension.
**Recommendation:** Add a note that version bump and release are out of scope (separate spec/card) or include them as an AC.

---

## Assumption Audit

```
| # | Assumption                                                    | Validated by AC? | Risk if wrong                                                |
|---|---------------------------------------------------------------|------------------|--------------------------------------------------------------|
| 1 | 64 types have zero training examples                          | ac-04            | CRITICAL: Actually ~100 types. Synthetic effort is ~60% larger|
| 2 | Training labels match taxonomy (after remap)                  | No               | MAJOR: 5 unmapped labels cause silent data loss or errors    |
| 3 | Model-as-critic disagreements are manageable                  | ac-02            | MODERATE: Could be thousands, no triage methodology          |
| 4 | Zero regressions is achievable                                | ac-06            | MAJOR: Never achieved in prior retrains                      |
| 5 | 75-100 epochs / 3-4 hours on Metal                            | ac-05            | MODERATE: Larger dataset extends training time               |
| 6 | 600/type synthetic cap is appropriate                          | No               | LOW: With more synthetic types, synthetic share grows        |
| 7 | Cleaning distilled CSV propagates to training                 | ac-03            | MAJOR: Data prep pipeline may override or ignore changes     |
| 8 | 338 eval labels can be meaningfully manually verified          | ac-01            | MODERATE: Ambiguous types need methodology, not just review  |
| 9 | Synthetic data for new types will not cause regression         | ac-06            | MODERATE: Interview flags this as open question              |
| 10| v14 model is reliable enough as critic for its own training data | ac-02          | LOW: 91.2% val_acc means ~9% error rate as critic            |
```

---

## Honest Assessment

The spec is well-structured and tackles the right problem — data quality is clearly the primary lever for accuracy improvements at this stage, and the three-phase approach (audit eval GT, audit training data, retrain) is sound. The interview demonstrates good judgment on scoping (comprehensive over targeted, heuristic adjudication). However, the spec has one factual error that materially changes the work estimate (64 uncovered types is actually ~100), and two structural gaps that could cause implementation friction: the lack of clarity on how the model-as-critic audit handles label remapping, and the zero-regression exit condition that has never been achievable in practice. The biggest risk is that the implementer follows the spec literally, generates synthetic data for only 64 types (missing ~36), and then the model still has coverage gaps that prevent hitting the 221/227 target. With the corrections below, this is a solid plan.

**Required changes before implementation:**
1. Correct the uncovered type count (64 -> actual number, verify with label remap)
2. Specify that model-as-critic audit must apply label remap before comparison
3. Soften zero-regression constraint or add a decision checkpoint for acceptable churn
4. Add resolution plan for 5 unmapped training labels
