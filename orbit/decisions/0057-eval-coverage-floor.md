---
status: accepted
date-created: 2026-04-21
date-modified: 2026-04-21
---
# 0057. Eval Coverage Floor — Phase A+B vs Phase C

## Context and Problem Statement

The eval-expansion programme is multi-phase: Phase A (audit) and Phase B
(zero-coverage closure) are scoped into the current sprint; Phase C
(edge-case second column per type, pushing toward ~400 total columns) is
deferred. Without an explicit coverage floor decision, the programme
risks both over-delivering (sliding into Phase C early and missing the
measurability goal) and under-delivering (stopping the audit before
zero-coverage is actually closed). The floor needs to be explicit so
that ac-05's gate and the sprint-close handover agree on what "done"
means.

The same decision must settle two adjacent asymmetries:
(1) replace vs augment — why replace is sprint-blocking and augment is
not; (2) the restricted-registry carve-out's interaction with the
coverage gate — whether a synthetic-necessary column counts toward
coverage, and under what constraints.

## Considered Options

- **A. Strict ≥1 realistic column per type, no carve-out** — every type
  in the taxonomy must have at least one column with
  `provenance_status ∈ {real, hand-curated}`. Restricted-registry types
  (CPT, SSN, SWIFT BIC under some readings) would have zero coverage and
  the gate would fail.
- **B. ≥1 column per type, carve-out permitted** — every type has ≥1
  column; the restricted-registry carve-out (MADR 0055) permits
  `synthetic-necessary` for the enumerated types. Phase A+B's gate is
  satisfied when set(taxonomy) ⊆ set(covered), regardless of provenance.
- **C. Per-type realism budget (≥1 realistic + ≥1 edge-case)** — the full
  Phase C target, attempted in one sprint. Not achievable in 1-2 weeks
  given sourcing effort.

## Decision Outcome

Chosen option: **B — ≥1 column per type with carve-out permitted for
Phase A+B; Phase C deferred**.

### Phase A+B floor (current sprint)

- **Per-type minimum:** ≥ 1 eval column per taxonomy type, verifiable
  via ac-05's coverage script.
- **Realism default:** `provenance_status ∈ {real, hand-curated}` is
  required unless the type is in the MADR 0055 carve-out table.
- **Carve-out:** `synthetic-necessary` satisfies the coverage gate for
  carve-out types. The type must still pass `pass_floors` (messiness +
  distributional) — carve-out is a provenance escape hatch, not a
  floors escape hatch.
- **Zero-coverage targets explicitly called out:** `swift_bic`, `cpt`,
  `loinc`, `excel_format`, `user_agent` plus any additional gaps
  surfaced during the audit (ac-03).

### Phase C target (future programme — out of this sprint)

- **Long-term:** ≥ 1 realistic column + ≥ 1 edge-case column per type.
- **Scale estimate:** ~400 columns (240 types × 2 − 160 already covered,
  modulo per-type depth). Multi-sprint programme.
- **Trigger:** separate card + spec; not scheduled here.

### Replace vs augment asymmetry

Both actions come out of ac-03's triage worklist. The sprint-close gate
treats them differently:

- **`replace` must clear.** Every triage row with action=`replace` must
  have a replacement column committed before sprint close. An open
  `replace` row means the audit identified a column as failing realism
  AND nothing has been done about it — structurally worse than having
  done the audit in the first place.
- **`augment` may remain open.** `augment` rows identify columns that
  pass realism but could be stronger. Leaving them open is the honest
  state between sprints — the worklist survives to Phase C.

### Carve-out × coverage gate interaction

The ac-05 gate accepts a carve-out column as closing zero-coverage only
if both conditions hold:

1. The type has an entry in MADR 0055's carve-out table.
2. The column has `provenance_status=synthetic-necessary`.

A synthetic-necessary column for a type *not* in the carve-out table
fails the gate. A `real` column for a carve-out type passes normally
(the carve-out permits synthetic-necessary; it does not require it).

### Consequences

- Good, because "done" is unambiguous: set(taxonomy) ⊆ set(covered),
  every column has a file that exists with ≥5 non-null values, and the
  ac-05 script returns exit code 0.
- Good, because the asymmetry between replace and augment matches the
  cost of remediation — replace is sprint-blocking (a known-bad column
  in the eval set is worse than no column at all for the measurement
  goal); augment is not.
- Good, because carve-out acceptance is bounded (the MADR table) and
  visible (every carve-out row's provenance_status declares itself),
  so "every type has a column" doesn't silently collapse into "every
  type has a synthetic generator".
- Bad, because Phase C deferral means edge-case coverage remains weak
  for another sprint. Some types are harder to measure with a single
  realistic column than with a realistic + edge-case pair.
- Neutral, because `augment` may grow unboundedly if the audit is
  thorough. Its size at sprint close is a quality signal, not a failure
  signal — a large augment list means the audit found real work for
  Phase C, which is useful.

## References

- Spec: `orbit/specs/2026-04-21-eval-expansion/spec.yaml` (v1.1) — ac-03, ac-05, ac-11
- Related decisions:
  - `orbit/decisions/0052-scope-aware-eval-gate.md` — prior scope-aware gate
  - `orbit/decisions/0055-eval-realism-dimensions.md` — realism + carve-out
- Coverage script: linked from ac-05 implementation (path TBD)
- Triage worklist: `orbit/specs/2026-04-21-eval-expansion/triage.md` (ac-03)

## Status movement

Status is `proposed` until ac-05 passes on the expanded corpus with exit
code 0 and the carve-out types resolve through the gate as specified.
On that pass, status moves to `accepted`.
