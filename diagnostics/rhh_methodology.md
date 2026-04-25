# RHH Methodology — Rule-Based Header-Hint Removal Roadmap

Companion document to spec `orbit/specs/2026-04-24-remove-header-hints/`
(MADR 0042 execution). This doc explains how the diagnostics under
`diagnostics/rhh_*.tsv` were produced — the threshold, the measurement
approach, the instrumentation design, the known limitations, and the
reproducibility steps.

## Threshold Rationale

Every rule-based header-hint family is classified at the **80%** mark:
a family is `model-covered` (safe to remove) when the multi-branch
model's raw top-1 prediction matches ground truth on ≥80% of columns
where that family currently fires, and `model-gap` (removal blocked,
training-data fortification needed) below that.

80% is a **policy choice**, not a statistical test. It is explicitly
named in the spec (constraint #5) so the bar cannot drift during
review. The reasoning: the current multi-branch baseline sits at
84.4% label accuracy on the 352-scored expanded eval (CLAUDE.md m-19
re-score) — asking a family to clear the same bar as the model-at-
large is a consistent standard. A higher bar (90%+) would keep
families alive whose absence costs one or two percentage points of
recall; a lower bar (70%) would greenlight removal on the back of
shaky model coverage and bounce problems downstream. 80% is the
point at which "the model does this now" is a defensible claim.

A family with zero hits on the 352-column eval is classified
`no-hit` — safe to remove with **zero measured risk** but also
unverifiable. These are flagged for the next eval-corpus expansion
rather than kept indefinitely; evidence-of-absence is not the same
as evidence.

## Measurement Approach

The core measurement is a **counterfactual**: for every family with
≥1 hit on the eval, run the same 448-row manifest twice — once with
the family active (baseline) and once with the family disabled via
the ac-02 instrumentation hook — and diff the predictions.

Hit definition (uniform across direct and internal measurement modes):
a column counts as a hit iff disabling the family **changed** the
baseline label (`label_changed == 1` in `rhh_counterfactual.tsv`).
This is the honest, measurable "family was effective" criterion. It
applies symmetrically to direct-mode families (which emit a rule
prefix into `disambiguation_rule`) and to **internal** families
(`header_hint_table` plus the six `substring_matcher_*` groupings),
which fire inside `header_hint()` without a dedicated rule prefix
and surface only through their label effect on callers.

Scoring against ground truth respects the eval-wide label
equivalence classes from `eval/eval_profile.sql`: timestamp
interchangeability within format-compatible families (ISO, SQL,
RFC 2822, MDY, DMY, YMD), boolean-subtype interchangeability,
geography hierarchy collapse, coordinate-subtype collapse, and
full_name ↔ entity_name. `ac04_counterfactual.py` implements the
same classes the profile eval uses. Without this, per-family
accuracy would be distorted by scoring artefacts that have nothing
to do with the family under test.

## Instrumentation Design

Per-family disablement is implemented via a **single cfg-gated
hook per function entry point**, not per-arm. The hook reads env
var `RHH_DISABLE_HINTS` (comma-separated `family_id` list) and
early-returns when the current family is in the disabled set.

Feature flag: `rhh-instrumentation`. When the feature is off, the
hook resolves to a `const fn` returning `false` and the call site
compiles out entirely — zero runtime cost for release builds.
Instrumentation points: `apply_header_sharpen` (10 disable flags),
`header_hint` (7 disable flags) in `crates/finetype-model/src/column.rs`,
covering all 22 families from the ac-01 inventory.

Why one hook per entry point and not per-arm? Two reasons. First,
runtime `disambiguation_rule` tags are already emitted at the family
level, not per-arm — matching that granularity avoids sprawl and
keeps the inventory auditable against live profiling output. Second,
per-arm instrumentation would require ~185 separate flags on the
`header_hint_table` match alone; the measurement needed for the
roadmap is family-level, and per-arm decomposition is deferred to
a future spec if the roadmap surfaces evidence that warrants it.

Default builds keep five invariant tests guarding the
zero-cost-when-off property; on-feature builds run one consolidated
scenario test with five sub-scenarios (baseline, direct-family
disable, internal-family disable, substring-family disable, and
disable-all). Parallelism-racey single-family tests were folded
into the sequential scenario to eliminate env-var contention.

## Limitations

1. **Sibling-context attention interaction not measured.** The
   spec's open question 4: when sibling-context attention is active,
   it enriches headers with cross-column context before families
   are consulted. This measurement disables each family in isolation
   but does not also permute the sibling-context layer. A family
   that looks removable here may still be load-bearing in tandem
   with sibling context; per-domain follow-up specs will re-measure
   under the relevant conditions.

2. **Per-arm granularity deferred.** Families like
   `header_hint_table` with ~185 match arms are treated as atomic
   units. A family may classify as model-gap because two arms fail
   while 183 succeed. Per-arm enumeration is a follow-up if the
   roadmap surfaces a family whose aggregate classification masks
   high-value sub-behaviours.

3. **80% is a policy choice, not a statistical test.** There is no
   confidence interval attached. The threshold was chosen to line
   up with the model-at-large accuracy; re-running with n_bootstrap
   resamples would give a CI on the per-family accuracy but does
   not change the policy bar.

4. **Eval corpus is fixed.** Measurement is against the 448-row
   expanded manifest (post-m-19). Families classified `no-hit`
   cannot be distinguished from families that the eval simply
   does not exercise. The corpus-expansion roadmap addresses this
   in successive sprints, but each spec ships against the corpus
   pinned at that spec's date.

5. **Model pinning is sha256 of `model.safetensors` at measurement
   time.** The counterfactual TSV records the resolved model path
   and weights hash (`58dcba8ea723…`). A future `models/default` promotion
   invalidates the per-family accuracy numbers; downstream consumers
   must re-run the counterfactual step before relying on these
   classifications.

## Reproducibility Steps

Every diagnostic under `diagnostics/rhh_*.tsv` is produced by a
script under `scripts/rhh/`. Regeneration is sequential — each
step depends on its predecessor — and each script writes to a
fixed output path documented in its docstring.

Prerequisites:

- `models/default` resolves to a multi-branch model directory
  (sherlock-v16 at time of writing) with `model.safetensors`
  present.
- `eval/datasets/manifest.csv` is populated at 448 rows with the
  post-m-19 7-column schema.
- For ac-04 only: binary built with
  `cargo build --release -p finetype-cli --features finetype-model/rhh-instrumentation`.

Invocation order:

1. `python3 scripts/rhh/ac01_inventory.py`
   — grep-walks `column.rs` for every rule-based header-hint
   family. Produces `diagnostics/rhh_family_inventory.tsv` (22 rows).

2. `python3 scripts/rhh/ac03_hit_counts.py`
   — runs `finetype profile --verbose` on every unique manifest
   file, parses `disambiguation_rule` prefixes, tallies per-family
   hits. Produces `diagnostics/rhh_hit_counts.tsv`.

3. `python3 scripts/rhh/ac04_counterfactual.py`
   — baseline profile + one disablement profile per family. Diffs
   predictions against ground truth using schema-mapping-backed
   label equivalence. Produces `diagnostics/rhh_counterfactual.tsv`
   (9856 rows) and `diagnostics/rhh_counterfactual_summary.tsv`.
   Requires the `rhh-instrumentation`-featured binary on PATH
   (the script auto-detects `target/release/finetype`).

4. `python3 scripts/rhh/ac05_classify.py`
   — applies the 80% threshold to produce
   `diagnostics/rhh_classification.tsv` (one row per family_id).

5. `python3 scripts/rhh/ac06_domain_rollup.py`
   — aggregates ac-05 by family domain. Produces
   `diagnostics/rhh_domain_rollup.tsv` (7 taxonomy domains +
   cross-domain row when applicable).

6. `python3 scripts/rhh/ac07_roadmap.py`
   — joins ac-01 inventory + ac-05 classification into the final
   artefact `diagnostics/rhh_roadmap.tsv`. Applies the v19-retrain
   gate for families targeting `finance.currency.amount_*` variants
   and emits `training-data-fortification-<primary-target>` tickets
   for all other model-gap families.

Tests:

- `python3 -m pytest scripts/rhh/test_rhh.py -q` — runs four
  `rhh_ac05_*` tests covering boundary classification and TSV
  arithmetic.
- `cargo test -p finetype-model` — default-build invariant suite.
- `cargo test -p finetype-model --features rhh-instrumentation`
  — on-feature scenario suite.

Single-shot regeneration: `scripts/rhh/regenerate_all.sh`
(ac-09) runs steps 1–6 in order and diffs outputs against the
committed baseline; see that script's header for the sha256
fingerprint policy and the FP-drift carve-out for the
counterfactual + classification TSVs.
