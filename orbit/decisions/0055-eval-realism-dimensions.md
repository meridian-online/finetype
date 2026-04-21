---
status: accepted
date-created: 2026-04-21
date-modified: 2026-04-21
---
# 0055. Eval Realism Dimensions, Floors, and the Restricted-Registry Carve-Out

## Context and Problem Statement

The v17 retrain surfaced two structural problems with the eval corpus:
(1) 5 of 7 relabel targets (swift_bic, cpt, loinc, excel_format, user_agent)
had zero eval coverage, making measurability impossible by construction;
(2) Hugh's long-running concern that the existing 242-column eval leans
too heavily on synthetic-adjacent data — realism is questionable for a
non-trivial share of columns. Before expanding coverage we need a pinned,
multi-dimensional definition of "realistic eval column" that a programmatic
pre-screen can grade against. Without it, the expansion programme would
retain the same realism ambiguity at larger scale.

## Considered Options

- **A. Provenance-only realism bar** — judge columns by where the data
  came from (real public dataset vs synthetic generator) and nothing else.
  Simple to implement, but lets pathological distributions through as long
  as the source is "real".
- **B. Three-dimensional realism (provenance + messiness + distributional
  fidelity)** — require all three to be gradable on every column, with
  programmatic floors for messiness and distributional fidelity, and a
  provenance_status field recording origin. Header authenticity is
  explicitly excluded (separate future concern — see Open Questions).
- **C. Four-dimensional (add header authenticity)** — also grade the
  header string itself (is "PassengerId" a plausible real-world header?).
  Richer, but currently un-measurable deterministically and touches
  sibling-context attention and header-branch training in ways we aren't
  ready to revisit this sprint.

## Decision Outcome

Chosen option: **B — three-dimensional realism**. The three dimensions are:

1. **Provenance** — `provenance_status` ∈ {`real`, `hand-curated`,
   `synthetic`, `distilled`, `synthetic-necessary`}. Recorded per manifest
   row; source_url + licence + fetched_date make it machine-auditable.
2. **Messiness** — programmatic metrics: `null_rate`, `unique_ratio`,
   `whitespace_ratio`, `format_variance`. Real-world columns carry noise;
   a column with zero nulls and uniform format is suspect for most types.
3. **Distributional fidelity** — programmatic metrics: `shannon_entropy`,
   `top_k_skew`. A column whose top-1 value is 80% of rows is either a
   categorical or a synthetic leak; the floor per type family decides
   which is acceptable.

Header authenticity is deliberately out of scope — flagged as a follow-up
card once eval expansion ships. The three dimensions above are sufficient
to close the v17 measurability gap and raise realism meaningfully without
blocking on changes to model architecture.

### Triage action schema

Every existing eval column carries exactly one triage action after the
pre-screen + human review step (ac-03):

- `keep` — passes provenance + floors; no change.
- `augment` — passes provenance but distribution thin (few rows, low
  entropy); acceptable for sprint close, worklist may remain open.
- `replace` — fails provenance OR fails floors; must be replaced before
  sprint close. If the replacement changes the source column header,
  the triage row carries `gt_label_change=true` so the implicit
  ground-truth change is visible rather than silent.

### Pinned pre-screen floors (referenced by ac-01 and ac-04)

The floors below are the defaults committed to `eval/pre-screen_floors.yaml`.
They are **floors for the pass_floors boolean**, not quality scores.
Family-specific overrides are permitted — the YAML file is the
authoritative source; the table here is a snapshot for the register.

| Metric | Default floor | Family overrides |
|---|---|---|
| `null_rate` | ≤ 0.20 | `identity.person.ssn`, `identity.government.*` → ≤ 0.50 (real-world reg data is sparse) |
| `unique_ratio` — id-like families | ≥ 0.95 | `identity.person.id`, `identity.technology.uuid`, `finance.account.*` |
| `unique_ratio` — categorical families | ≤ 0.50 | `representation.enumeration.*`, `geography.country.*` |
| `unique_ratio` — freeform | none | `representation.freetext.*` (no constraint) |
| `shannon_entropy` | ≥ 1.5 bits | `representation.enumeration.*` may floor at 0.5 bits |
| `top_1_skew` | ≤ 0.50 for non-categorical | categorical families unconstrained |
| `whitespace_ratio` | ≤ 0.10 | higher permitted for `representation.freetext.*` |
| `format_variance` | family-specific | datetime families require ≥ 1 format variant; id-like may be rigid |

The pre-screen script (ac-01) emits both the raw metrics and the
pass_floors boolean per column; ac-04's `replace`-pass verification
calls the same script rather than re-deriving floors.

### Restricted-registry carve-out

Some types have no authoritative source that we can redistribute under a
research-ethical bar. For these, `provenance_status=synthetic-necessary`
is sanctioned with an explicit rationale. This is the only route around
the real/hand-curated floor in ac-04.

| Type | Reason carve-out applies | Rationale |
|---|---|---|
| `identity.medical.cpt` | AMA licence | CPT codes are copyrighted by the AMA; non-licensee redistribution is not permitted. Synthetic CPT-shaped codes graded on format plausibility only. |
| `identity.medical.loinc` | LOINC is freely licensed; carve-out NOT expected — real expected | Placeholder: if real sourcing fails the licence bar we revisit, but LOINC is intended to source real. |
| `identity.person.ssn` | PII | Real US SSNs cannot be redistributed; synthetic SSNs following AANN-GG-SSSS format with area/group number validity are the only ethical source. |
| `identity.government.nino` | PII | UK National Insurance Number; same PII rationale as SSN. |
| `finance.account.credit_card_number` | PII / fraud risk | PAN with valid Luhn, but not tied to any real account. |
| `finance.institution.swift_bic` | Licensing contested; SWIFT publishes BIC directory behind registration | If research-ethical sourcing fails the licence bar, revert to synthetic-necessary with the standard BIC format (AAAABBCC[DDD]). |

Each row in the carve-out must also satisfy the messiness and
distributional floors — `synthetic-necessary` is a provenance
escape hatch, not a floors escape hatch. A synthetic-necessary column
with zero nulls and zero format variance still fails `pass_floors`.

### Consequences

- Good, because realism becomes a 3-dim measurable property rather than a
  vibes call; every column's triage action is defensible from metrics +
  manifest fields.
- Good, because the carve-out is enumerated and auditable — no silent
  drift where new synthetic columns sneak in without a documented reason.
- Good, because the floors are pinned in YAML, not scattered across
  scripts — ac-04's verification and ac-01's pass_floors call the same
  source of truth.
- Bad, because messiness floors can be gamed — a lazy generator can inject
  artificial nulls to pass `null_rate` without actually being messy. This
  is a known limitation; mitigated partially by `format_variance` and by
  the human review step that constraint #1 requires.
- Bad, because header authenticity is left unaddressed. The model relies
  heavily on headers via the multi-branch header branch and sibling-context
  attention; a future card will need to tackle this.
- Neutral, because the carve-out table is expected to grow as new types
  are added to the taxonomy. Each addition requires updating this MADR.

## References

- Spec: `orbit/specs/2026-04-21-eval-expansion/spec.yaml` (v1.1)
- Interview: `orbit/specs/2026-04-21-eval-expansion/interview.md` (Q3, Q5, Q7)
- Related decisions:
  - `orbit/decisions/0049-preserve-synthetic-for-bad-distilled-types.md`
  - `orbit/decisions/0050-per-type-sourcing-policy.md`
  - `orbit/decisions/0052-scope-aware-eval-gate.md`
  - `orbit/decisions/0054-hold-v17-no-promotion.md`
- CLAUDE.md Engineering Principle 3: "LLMs for parsing, programmatic
  checks for validation"
- Floors file: `eval/pre-screen_floors.yaml` (added under ac-01)

## Open Questions

- **Header authenticity** — deferred to a follow-up card; the model depends
  on headers heavily enough that this is load-bearing long-term.
- **Distributional reference distributions** — for some types (medical
  registries especially) the "right" distribution requires domain-expert
  input. Floors here are conservative defaults; per-family tightening is
  expected as expertise arrives.
- **Carve-out growth governance** — when a new type is added to the
  taxonomy, who decides whether it qualifies for carve-out? Proposed:
  default is `real` required; carve-out requires an MADR amendment with
  explicit rationale (this file, same section).

## Status movement

Status is `proposed` until ac-04 verifies the floors work against
sourced replacement columns. On ac-04 pass, status moves to `accepted`
with a dated note under date-modified.
