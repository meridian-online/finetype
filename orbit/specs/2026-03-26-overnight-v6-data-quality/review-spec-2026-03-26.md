# Spec Review

**Date:** 2026-03-26
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-03-26-overnight-v6-data-quality/spec.yaml
**Verdict:** REQUEST_CHANGES

---

## Findings

### [CRITICAL] AC-1: Test assertions also contain stale labels
**Category:** assumption
**Description:** AC-1 lists stale label references in Sharpen rules but the same stale labels appear in test assertions within `column.rs` (lines 5767, 5779-5780, 5950-5978, 8960/8977/8982/9221). The verification says "zero stale references remain" and "cargo test passes" but doesn't require new tests asserting the *correct* labels post-fix. Tests could be deleted rather than updated, passing verification without proving the fix works.
**Evidence:** Grep output shows 20 occurrences of stale labels across `column.rs`, several in `#[test]` functions.
**Recommendation:** Add to AC-1 verification: after fixing each reference, add a test asserting the correct canonical label fires. The zero-stale-references grep and cargo-test-passing are necessary but not sufficient.

### [CRITICAL] AC-5: Augmentation root cause misdiagnosed
**Category:** assumption
**Description:** The spec says "many columns are skipped" but the actual bug is a double-probability gate. `augment_column()` has 13/20 internal slots as `no_aug` (~65% no-op rate). At `augmentation_rate=0.35`, effective coverage is `0.35 × 0.35 = 12.25%` — exactly matching v5's observed 12.2%. A developer following the spec would look for a column-selection problem that doesn't exist.
**Evidence:** `augment_column()` at lines 567-575: 13/20 slots are `no_aug`. `augment_columns()` at line 648 gates on `rng.random() < augmentation_rate`. Combined: `0.35 × (7/20) = 12.25%`.
**Recommendation:** Name the root cause explicitly: the double-probability gate. Specify the fix: either remove `no_aug` slots or set `augmentation_rate` to ~1.0 to compensate.

### [CRITICAL] AC-6: Oversampling mechanism misidentified
**Category:** assumption
**Description:** The spec says "multiplier should apply to total allocation, not just distilled portion." But the code already applies it to total. The real cause: the synthetic pool is capped at `samples_per_type=3000` flat across all types. For oversampled types wanting 9000, you need 9000 synthetic available — but `generate_synthetic_columns()` only generates 3000 per type regardless of multiplier.
**Evidence:** `blend_columns()` at lines 950-959 caps `actual_s` at `len(s_cols)`. Synthetic generation at line 1797 uses flat `synthetic_columns_per_type`. For 3x oversample: need 9000 synthetic, only 3000 available.
**Recommendation:** AC-6 should specify: generate `samples_per_type × oversample_multiplier` synthetic columns for oversampled types.

### [WARN] AC-5: format_mixing is a new feature, not a bug fix
**Category:** missing-requirement
**Description:** The spec requires "all 5 augmentation types including format_mixing" but `format_mixing` was never implemented — it doesn't exist in `prepare_multibranch_data.py`. The spec treats it as a verification target without a design spec for what it does.
**Evidence:** `augment_column()` at lines 567-626 has no `format_mixing` branch. v5 findings: "no format mixing was implemented."
**Recommendation:** Either remove `format_mixing` from AC-5 (separate feature AC with design spec) or expand AC-5 to specify what format_mixing means concretely.

### [WARN] AC-2: Collision verification only covers hs_code
**Category:** test-gap
**Description:** AC-2 requires auditing "every generator pair that shares a broad_type" but verification only checks hs_code cross-contamination. If the audit finds 10 collisions, only 1 has a defined check.
**Recommendation:** Generalize: "For every approved collision fix, generate 100 samples from each colliding type and run finetype infer. Cross-contamination <5% per pair."

### [WARN] AC-4: Baseline type count discrepancy (131 vs 129)
**Category:** assumption
**Description:** Interview says 131 types covered, but manifest analysis shows 129 unique `gt_label` values. Unclear counting method means the ≥150 target could be measured differently than the baseline.
**Recommendation:** Pin the baseline using a specific counting method before implementation. Use the same method for the target.

### [WARN] AC-7: Sharpen trace infrastructure doesn't exist
**Category:** missing-requirement
**Description:** AC-7 requires a "Sharpen trace report" logging which rules fired per column, but `ColumnResult` only captures one `disambiguation_rule`. No intermediate state logging exists. Building trace instrumentation is non-trivial hidden prerequisite work.
**Evidence:** `ColumnResult` struct records only one `disambiguation_rule`. `feature_sharpen()` and `value_sharpen()` return immediately on first match — no intermediate logging.
**Recommendation:** Add explicit sub-task: "instrument Sharpen to log trace per column (label before/after each rule, which rules evaluated)."

### [WARN] AC-8: "Same parameters" is misleading after AC-5/AC-6 fixes
**Category:** constraint-conflict
**Description:** The overnight script will use the same CLI arguments but the behavior changes after augmentation and oversampling fixes. "Reuse v5 infrastructure" and "fix data quality" are in tension — the script must be modified.
**Recommendation:** Document what changed from v5 in the overnight v6 script. The "same parameters" assumption is intentionally broken by AC-5/AC-6.

### [WARN] Exit condition: "stale label fix recovers ≥2 columns" — which model?
**Category:** missing-requirement
**Description:** The exit condition tests AC-1 against the "v5-current model" but the spec doesn't pin which model binary. Multiple model directories exist.
**Recommendation:** Pin to `models/sherlock-v5-current/`. Confirm the path exists before running.

### [INFO] AC-3: Training corpus may contain stale labels
**Category:** test-gap
**Description:** AC-3 checks source code for `identity.financial` references but not the distilled training corpus. If distilled CSVs contain stale labels, they'd be noise in training.
**Recommendation:** Scan distilled training corpus for stale labels before training.

### [INFO] AC-9: No minimum improvement delta on expanded eval
**Category:** test-gap
**Description:** AC-9 requires "improvement over v5-current" without a minimum delta. A 1-column improvement technically passes despite 10 ACs of work. CLAUDE.md mentions a 170/190 target that doesn't appear in any AC.
**Recommendation:** Set a minimum delta (e.g., ≥5 net improvements on expanded eval).

---

## Honest Assessment

The spec correctly identifies v5's root causes and the stale label discovery is valuable forensic work. However, three CRITICALs need addressing: the augmentation bug is misdiagnosed (double-probability gate, not column skipping), the oversampling bug is misidentified (synthetic pool cap, not multiplier application), and AC-1's test updates need explicit verification. The format_mixing requirement is a new feature smuggled in as a bug fix. The Sharpen trace infrastructure is a hidden prerequisite that needs scoping.

ACs 1, 2, 3, and 4 are ready to execute (with minor fixes). ACs 5, 6, and 7 need root-cause corrections before anyone writes code against them.
