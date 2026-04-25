# Spec Review

**Date:** 2026-04-24
**Reviewer:** Context-separated agent (fresh session)
**Spec:** orbit/specs/2026-04-24-remove-header-hints/spec.yaml (v1.1)
**Verdict:** APPROVE

---

## Review Depth

```
| Pass                       | Triggered by                                                    | Findings |
|----------------------------|------------------------------------------------------------------|----------|
| 1 — Structural scan        | always                                                           | 0        |
| 2 — Assumption & failure   | content signals (eval datasets, model instrumentation, FP det.)  | 4        |
| 3 — Adversarial            | no structural cascades — Pass 2 findings all MEDIUM/LOW          | —        |
```

v1.0 review raised 7 findings (2H / 3M / 2L). v1.1 claims "addresses all." I verified each against the current spec body and the repo ground truth (grep counts, file line counts, eval manifest size). All seven are resolved:

```
| v1.0 finding                                      | v1.1 resolution                                                                                                                             | Status     |
|---------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------|------------|
| HIGH granularity (hint_id vs rule_family)         | Constraint 3 picks Option B: rule-family, not per-arm. family_id replaces hint_id end-to-end. ac-01 floor dropped from ≥60 to ≥15.          | RESOLVED   |
| HIGH "12 tags" undercount                         | Constraint 3 drops the "12", documents grep evidence of 36 distinct suffixes, names ac-01 as the canonical source of truth.                 | RESOLVED   |
| MEDIUM ac-04 spot-check                           | ac-04 verification now requires full-row diff against profile_results.csv with "0 rows may differ" tolerance on same machine.               | RESOLVED   |
| MEDIUM ac-02 wall-time flake                      | Wall-time check dropped. Binary size recorded-not-gated. Verification uses public-API diff + clean builds in two feature configurations.    | RESOLVED   |
| MEDIUM ac-06 self-referential READY floor         | ac-06 verification now says "No evidence-prescribing verification — the rollup reports whatever the data says."                             | RESOLVED   |
| LOW instrumentation design                        | Constraint 6 mandates single cfg-gated check at each function entry, feature flag `rhh-instrumentation`, "zero per-arm diff sprawl".        | RESOLVED   |
| LOW ac-03 tautological invariant                  | Redundant clause removed; replaced with independent-total cross-check between per-family hits and raw baseline prefix counts.               | RESOLVED   |
```

Independent repo verification (Pass 1):

- `crates/finetype-model/src/column.rs` = 10,853 lines (matches the v1.0 review's claim).
- `grep -oE "header_hint_[a-z_]+" column.rs | sort -u` = **36 distinct suffixes**. Consistent with constraint 3's grep evidence.
- `grep -cE "h\.contains\(" column.rs` = **69 substring-matcher sites**. Consistent with the interview's "~70 sites" figure.
- `sense_geo_` branches found at lines 1558, 1573 (2 branches). Consistent.
- `fn header_hint(header: &str)` at column.rs:3999. Consistent with constraint 3 and ac-01's `header_hint_table` identifier.
- `eval/datasets/manifest.csv` = 449 lines (header + 448 rows). Consistent with spec's "448-row eval manifest".
- `eval/eval_output/profile_results.csv` exists (ac-04 baseline reference file is real).

All Pass 1 gate-AC checks pass (ac-09 and ac-10 verifications are non-empty, non-placeholder, ≥20 chars). No structural conflicts between ACs. Scope matches goal (measurement-only roadmap). Every AC has a testable verification method.

Pass 2 triggered by content signals (eval datasets, model instrumentation via env var, FP determinism concerns) — four MEDIUM/LOW findings that do not block APPROVE.

---

## Findings

### [MEDIUM] ac-07 blocked_on rule for non-amount model-gap families is under-specified
**Category:** test-gap
**Pass:** 2
**Description:** ac-07 description specifies the rule for amount-variant model-gap families (`blocked_on == "v19-retrain"`) and verifies it explicitly. It also says "Other model-gap families get blocked_on == 'training-data-fortification-<primary-target>'" — but `<primary-target>` is undefined when a family has multiple type_targets (type_targets is a semicolon-separated list per ac-01). The verification block only gates the amount-variant rule; the generic rule has no verification assertion.
**Evidence:** spec.yaml ac-07 description (lines 210-222) and verification (lines 223-229). Verification says: "Every model-gap family has non-empty blocked_on" — correct — and "Every family with domain=='finance' and classification=='model-gap' AND type_targets containing 'amount_' has blocked_on=='v19-retrain'" — correct for the finance-amount case. But nothing asserts the `<primary-target>` substitution is consistent (e.g., first type_target alphabetically? highest-hit target? domain-prefixed?).
**Recommendation:** Add a one-line rule to ac-07 description — e.g. "`<primary-target>` is the first element of type_targets (semicolon-separated, in the order emitted by ac-01's walk)" — and a matching verification clause: "For every model-gap family with classification != 'v19-retrain', blocked_on == 'training-data-fortification-' + first_type_target." This is a 2-line diff, not a structural issue. APPROVE notwithstanding; worth capturing before ac-07 runs so the roadmap TSV is deterministically regenerable (ac-09 depends on this).

### [MEDIUM] ac-09 byte-identical regeneration assumes FP-deterministic model inference
**Category:** assumption
**Pass:** 2
**Description:** ac-09 requires byte-identical regeneration of rhh_family_inventory.tsv, rhh_hit_counts.tsv, rhh_classification.tsv, rhh_domain_rollup.tsv, and rhh_roadmap.tsv. The first is pure source-walk output — safe. The rest are derived from `finetype profile --verbose` output, which involves Candle tensor ops on Metal. Metal FP ops are not bit-exact reproducible across runs in general; for the multi-branch model specifically, softmax boundary cases can flip top-1 between runs when the model is near-tied. The spec carves out rhh_counterfactual.tsv ("≤0 rows of drift on the same machine") but does not extend that carve-out to rhh_hit_counts.tsv — which derives columns_correct/columns_wrong from the same baseline predictions that can flip.
**Evidence:** spec.yaml ac-09 (lines 247-261) — enumerates byte-identical files explicitly, permits drift only on rhh_counterfactual.tsv. The same baseline predictions that feed rhh_counterfactual.tsv also feed rhh_hit_counts.tsv (ac-03) and rhh_classification.tsv (ac-05), since classification is a join of ac-03 and ac-04 outputs.
**Recommendation:** Either (a) extend the same-machine drift tolerance to all profile-derived TSVs ("≤0 rows on same machine; documented ≤1% cross-machine"), or (b) pin the baseline profile output as a committed fixture (capture once, check in to `eval/eval_output/profile_results.csv`, regenerate only when the model changes) and have downstream TSVs derive from the fixture rather than live inference. Option (b) makes ac-09 truly byte-identical and has the side-benefit of pinning ac-04's baseline file regardless of `models/default` symlink state (see next finding).

### [MEDIUM] ac-04 baseline is `models/default` — symlink not pinned
**Category:** failure-mode
**Pass:** 2
**Description:** ac-04 verification requires baseline_prediction to match `eval/eval_output/profile_results.csv` for every row. That CSV was generated by whichever checkpoint `models/default` pointed at when `make eval-report` last ran. `models/default` is a symlink that changes during model promotion (current: sherlock-v16). If the symlink flips between `profile_results.csv` capture and this spec's counterfactual regeneration, ac-04 will silently diff against a stale baseline, and either flake ("every row" will violate) or — worse — pass against the wrong reference while measuring counterfactuals under the new model. No AC records the sha256 of the model checkpoint used.
**Evidence:** CLAUDE.md release section names `models/default` as a symlink; PR #39 introduced `FINETYPE_CI_MODEL` specifically because the symlink-promotion dance is error-prone. spec.yaml constraint 4 (lines 33-36) pins "the sherlock-v16 model (current models/default)" but does not pin the checkpoint hash or the profile_results.csv capture date.
**Recommendation:** Add a constraint: "ac-03 and ac-04 runs pin `FINETYPE_MODEL=models/sherlock-v16` explicitly (not via the symlink). The baseline profile output is regenerated from that pinned checkpoint at the top of `scripts/rhh/regenerate_all.sh` and its sha256 is recorded in diagnostics/rhh_baseline_hash.txt." This interlocks cleanly with the MEDIUM #2 fix above.

### [LOW] ac-02 coverage weaker for families that do not emit runtime tags
**Category:** test-gap
**Pass:** 2
**Description:** ac-01 includes grouped family_ids that are not currently emitted at runtime — `header_hint_table` (the top-level match at column.rs:3999) and `substring_matcher_*` (per constraint 3, "a grouped tag for families without a runtime emitter"). These families don't produce a `disambiguation_rule` with a matching prefix. The ac-02 test `rhh_ac02_disable_all` asserts "no disambiguation_rule emitted by the profile pipeline starts with any of the disabled family prefixes" — which is trivially satisfied for non-emitting families. So the instrumentation hook's correctness for `header_hint_table` and `substring_matcher_finance` (etc.) is asserted less strongly than for `header_hint_hardcoded` or `header_hint_measurement`. A hook that fails to early-return inside those non-emitting branches would still pass `rhh_ac02_disable_all`.
**Evidence:** spec.yaml constraint 3 lines 24-31 ("substring-matcher category plus the top-level header_hint() match table"); ac-02 verification lines 108-116. Non-emitting families are real — header_hint() returns an `Option<&'static str>` and its callers emit the family prefix, not the match-arm detail.
**Recommendation:** Add a complementary coverage test for non-emitting families: "For each family_id in ac-01 with rule_family_class ∈ {match_table, substring}, assert that ac-04's no_hint_prediction differs from baseline_prediction on at least one column (proves the hook actually short-circuits the family, not just that the prefix isn't emitted)." Or explicitly accept this as a known limitation in rhh_methodology.md §Limitations. Low severity — the counterfactual measurement (ac-04) will catch a broken hook indirectly, and the prior v1.0 review's instrumentation-design LOW is already resolved.

---

## Honest Assessment

The spec is in shipping shape. v1.1 materially closes every v1.0 finding, and fresh-eyes scrutiny turns up only four Pass-2 concerns — three MEDIUM (all fixable with 2-5 lines of spec prose; none invalidate the roadmap) and one LOW (accepted limitation or minor test addition). The load-bearing axis (rule-family granularity, source-of-truth inventory, 80% threshold, measurement-only scope) is internally consistent and grounded in repo reality — I reproduced the grep counts independently and they match constraint 3's prose.

The two MEDIUM findings about FP determinism (ac-09) and symlink pinning (ac-04) travel together — fixing them by committing a pinned baseline fixture and recording the checkpoint hash in `regenerate_all.sh` resolves both, and incidentally hardens the reproducibility gate that the spec already claims. The ac-07 blocked_on rule is a genuinely-undefined substitution that deserves one line of spec text before the roadmap generator runs; it's not a blocker because the verification catches the common case (amount-variant) and the uncommon case produces a diagnostic artefact, not a behaviour change. The LOW on non-emitting family coverage is the kind of observation that the methodology doc (ac-08 §Limitations) is designed to absorb.

Biggest remaining risk: the 7 downstream per-domain specs listed in `metadata.follow_up_specs` all consume `rhh_roadmap.tsv` as their entry point. If the primary-target substitution drifts between runs (MEDIUM #1) or the baseline symlink flips mid-measurement (MEDIUM #3), the roadmap could ship internally consistent but non-reproducible — and downstream specs would inherit that drift. Both are cheap to fix; neither justifies blocking. APPROVE with a strong recommendation that the three MEDIUM spec-prose tweaks go in before `scripts/rhh/regenerate_all.sh` is first executed. The v1.0 → v1.1 iteration was substantive and correct; the skeleton is sound.
