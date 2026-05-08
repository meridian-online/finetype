# Remove header-hint regex rules — measurement + roadmap (MADR 0042)

Executes MADR 0042's deferred "remove regex header hints" direction.
This PR is **measurement-only** — no source-code removal. Deliverables
are the `diagnostics/rhh_*.tsv` roadmap files + methodology doc that
per-domain follow-up specs will consume. All 11 ACs of
`.orbit/specs/2026-04-24-remove-header-hints/spec.yaml` shipped.

## (a) Domain Rollup — `diagnostics/rhh_domain_rollup.tsv`

```
| domain         | families | covered | gap | no_hit | readiness            |
|----------------|----------|---------|-----|--------|----------------------|
| container      | 0        | 0       | 0   | 0      | NO_EVIDENCE          |
| datetime       | 1        | 1       | 0   | 0      | READY                |
| finance        | 1        | 0       | 1   | 0      | NEEDS_FORTIFICATION  |
| geography      | 6        | 0       | 1   | 5      | NEEDS_FORTIFICATION  |
| identity       | 2        | 0       | 1   | 1      | NEEDS_FORTIFICATION  |
| representation | 3        | 0       | 1   | 2      | NEEDS_FORTIFICATION  |
| technology     | 1        | 0       | 1   | 0      | NEEDS_FORTIFICATION  |
| cross-domain   | 8        | 1       | 5   | 2      | NEEDS_FORTIFICATION  |
| **total**      | **22**   | **2**   |**10**| **10**|                      |
```

Only READY domain: **datetime** (1 family, all model-covered). Container
has no families in the inventory (expected — the taxonomy domain exists
but no header-hint rules target it). The remaining 5 taxonomy domains
plus cross-domain all require training-data fortification before any
rule removal.

## (b) Proposed Per-Domain Stage Sequence

Drawn from the `proposed_stage` column of
`diagnostics/rhh_roadmap.tsv`. Ordering within the sequence is loosely
by readiness then by number of families; per-domain specs will refine
the sequence with their own priorities:

1. **datetime-stage** (1 family, all model-covered — lowest-risk
   first-removal target).
2. **technology-stage** (1 family, 1 model-gap —
   `substring_matcher_technology` targeting ip_v6).
3. **identity-stage** (2 families, 1 gap on
   `substring_matcher_identity`).
4. **representation-stage** (3 families, 1 gap on
   `substring_matcher_representation`).
5. **geography-stage** (6 families — 1 gap on
   `substring_matcher_geography`, 5 no-hit; large no-hit cluster makes
   this the lowest-confidence batch).
6. **cross-domain-stage** (8 families, 5 model-gap including both
   `keep_required` threshold-gates; 2 of the 3 keep-required-adjacent
   families stay in code regardless of removal).
7. **finance-stage** — **gated on v19 retrain** per MADR 0066.
   `substring_matcher_finance` awaits training-data fortification; the
   amount-variant super-gap is already covered by MADR 0065 (shipped in
   v0.6.17).

## (c) Model-Gap Families + `blocked_on`

Every model-gap family by spec constraint #5 (<80% no-hint label
accuracy on counterfactual hits):

```
| family_id                         | domain         | blocked_on                                                       |
|-----------------------------------|----------------|------------------------------------------------------------------|
| header_hint_table                 | cross-domain   | v19-retrain (MADR 0065 amount-variant targets)                   |
| substring_matcher_identity        | identity       | training-data-fortification-identity.person.email                |
| substring_matcher_technology      | technology     | training-data-fortification-technology.internet.ip_v6            |
| substring_matcher_geography       | geography      | training-data-fortification-geography.address.postal_code        |
| substring_matcher_finance         | finance        | training-data-fortification-finance.currency.amount              |
| substring_matcher_representation  | representation | training-data-fortification-representation.numeric.integer_number|
| header_hint_hardcoded             | cross-domain   | training-data-fortification-cross-domain (match_table router)    |
| header_hint_generic               | cross-domain   | training-data-fortification-cross-domain (generic substring)     |
| header_hint_cross_domain          | cross-domain   | training-data-fortification-cross-domain (keep_required)         |
| header_hint_same_category         | cross-domain   | training-data-fortification-cross-domain (keep_required)         |
```

Two model-gap families are flagged `keep_required` (spec constraint #2:
Model2Vec threshold gates stay). Their `blocked_on` tickets document
the training-data gap that would let the threshold-gate be loosened;
the rule itself remains.

## (d) Methodology

`diagnostics/rhh_methodology.md` documents:

- **Threshold Rationale** — why 80% (policy choice aligned with model-
  at-large accuracy; explicit in the spec; not a statistical test).
- **Measurement Approach** — counterfactual via ac-02 instrumentation;
  uniform `label_changed == 1` hit definition across direct and internal
  families; schema-mapping-backed label equivalence mirroring
  `eval_profile.sql`.
- **Instrumentation Design** — single `#[cfg]`-gated hook per function
  entry point; 17 disable flags across `apply_header_sharpen` (10) and
  `header_hint` (7); zero-cost default build (const fn resolves to
  `false` and callers compile out).
- **Limitations** — sibling-context interaction not permuted; per-arm
  granularity deferred; 80% is policy not statistics; corpus fixed at
  448 rows; model pinned by sha256.
- **Reproducibility Steps** — 6-script invocation order + test commands
  + `scripts/rhh/regenerate_all.sh` one-shot pointer.

## Artefacts

All staged under `diagnostics/`:

- `rhh_family_inventory.tsv` (22 families × 9 columns)
- `rhh_hit_counts.tsv` (22 × 6)
- `rhh_counterfactual.tsv` (9856 rows, model pinned by sha256)
- `rhh_counterfactual_summary.tsv` (22 × 4)
- `rhh_classification.tsv` (22 × 8)
- `rhh_domain_rollup.tsv` (8 × 6)
- `rhh_roadmap.tsv` (22 × 10)
- `rhh_methodology.md` (1184 words, 5 required sections)
- `rhh_fingerprints.sha256` (ac-09 gate manifest)

Scripts under `scripts/rhh/`:

- `ac01_inventory.py`, `ac03_hit_counts.py`, `ac04_counterfactual.py`,
  `ac05_classify.py`, `ac06_domain_rollup.py`, `ac07_roadmap.py`,
  `regenerate_all.sh` (ac-09), `test_rhh.py` (6 tests passing:
  `rhh_ac05_boundary` ×3, `rhh_ac05_arithmetic`, `rhh_ac09_*` ×2).

Instrumentation under `crates/finetype-model/src/`:

- `rhh.rs` (new module — `is_disabled(family_id)` hook, feature-gated
  const fn off-path, 5 default-build invariant tests + 1 consolidated
  on-feature scenario test).
- `column.rs` — 17 disable flags inserted at family entry points. Off-
  feature build compiles to zero overhead.
- `Cargo.toml` — `rhh-instrumentation` feature added.

## Spec Reference

`.orbit/specs/2026-04-24-remove-header-hints/spec.yaml` (v1.2,
sha256:06708cd…). All 11 ACs marked `[x]` in `progress.md`.

## Follow-ups

Per-domain stage specs will consume this roadmap:
datetime → technology → identity → representation → geography →
cross-domain → finance (gated on v19 retrain). Each will re-measure
under sibling-context attention (spec open question 4) before
proposing actual removals.
