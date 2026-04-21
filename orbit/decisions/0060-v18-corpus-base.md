---
status: accepted
date-created: 2026-04-21
date-modified: 2026-04-21
---
# 0060. v18 Corpus Base — Stay on v3

## Context and Problem Statement

v18 retrains the sherlock multi-branch model targeting ≥ 297/352 on the expanded 352-col eval (v16 baseline) without per-domain regression > 3 cols. The first design decision is which corpus the retrain builds on:

- **v3** — current production corpus (used by v16). Known-good baseline.
- **v4** — v17's corpus on branch `distilled-data-relabel-7-types-v17`: v4 distilled loaders (17,812 UAs ex ua-parser/uap-core, 2,109 LOINCs ex NIH NLM Clinical Tables) + generator improvements for swift_bic / cpt / ssn / excel_format + widened `labels/definitions_identity.yaml` / `definitions_representation.yaml` patterns + http_method ENUM-only strategy (decision 0051).
- **v4+additions** — v4 base with targeted generator additions to cover new v18 failure clusters surfaced in triage.

The triage pass (ac-01, `orbit/specs/2026-04-21-v18-retrain/triage.md`) enumerated all 55 v16 failures on the expanded eval and categorised them per-row. The distribution determines which corpus base has the best accuracy ceiling for v18.

## Considered Options

- Option A — v3 base (current production corpus)
- Option B — v4 base (v17 branch)
- Option C — v4+additions (v4 + new generator work for triage-surfaced clusters)

## Decision Outcome

Chosen option: **Option A (v3)**, because the triage evidence shows that 47 of 55 v16 failures are orthogonal to what v4 targets, and v4 carries concrete regression risk from v17's held outcome (decision 0054: 3 fixes / 3 regressions / net-zero on 242-col eval).

### Evidence citation (from triage.md)

Triage row evidence **in favour of v3** (dominant failure mass not addressed by v4):

- `coverage_closure_phase_ab::amount_nodecimal` → `finance.currency.amount` (label-confusion, add-data). Also `amount_comma`, `amount_lakh`, `amount_crypto`, `amount_accounting`, `amount_apostrophe`, `amount_comma_suffix`, `amount_code_prefix`, `amount_neg_trailing`, `amount_space`, `amount_multisym` — **12 rows** of finance amount-variant collapse. v4 does not target amount subtypes.
- `coverage_closure_phase_ab::json_array`, `csv`, `xml`, `html`, `yaml`, `query_string`, `whitespace_separated`, `semicolon_separated` → `representation.discrete.categorical` (label-confusion, add-data) — **8 rows** of container-type collapse. v4 does not target container types.
- `coverage_closure_phase_ab::julian`, `iso_8601_compact`, `iso_8601_milliseconds`, `jp_era_short`, `pg_short_offset`, `ordinal` — **6 rows** of datetime-specific-to-representation collapse. v4 does not target these datetime subtypes.

Triage row evidence **in favour of v4** (v4-targeted types still failing at v16):

- `tech_systems::user_agent` → `technology.cryptographic.jwt` (label-confusion, add-data)
- `network_logs::user_agent` → `technology.development.docker_ref` (label-confusion, add-data)
- `coverage_closure_phase_ab::excel_format` → `representation.text.word` (label-confusion, add-data)

That is **2 real-data UA rows + 1 synthetic excel_format row = 3 of 55** directly targeted by v4.

### Negative-evidence argument

Triage did NOT surface:

- LOINC failures (v4's 2,109-row NLM Clinical Tables loader would be the primary lever). No `loinc` entries appear in the 55-failure set — v4 LOINC corpus addition is NOT load-bearing for v18's expanded-eval gate.
- SSN failures (v4 ssn generator improvements). No ssn-as-gt-label row in the miss set (the one `ssn` appearance is v16 mispredicting phone as ssn — not an ssn training gap).
- CPT failures. No `cpt` entries. v4 cpt generator improvements are NOT load-bearing.
- swift_bic failures. No `swift_bic` entries. v4 swift_bic generator improvements are NOT load-bearing.

Of the four "v4 specifically targeted this" type families (UA, LOINC, SSN, CPT/swift_bic), only UA shows failure mass in the v18 eval set. The v4 corpus's other gains are not exercised by the v18 gate.

### Risk analysis

- **v4 regression risk is measured, not hypothetical** (decision 0054): v17's measured outcome on the corrected 242-col eval was 3 fixes + 3 regressions, net-zero. Adopting v4 for v18 without addressing the regression sources repeats v17's trap.
- **v3 has no known open regressions** and was the production baseline for v16's 297/352 expanded-eval score.
- **The dominant failure mass (47 of 55) is amount-variant / container-type / datetime-specific collapse on coverage_closure synthetic rows.** Neither v3 nor v4 was trained against these rows — they are m-19 Phase A+B additions that post-date v17's corpus freeze (commit `bfd851b`). Corpus choice is secondary to prep-distribution quality on the new synthetic types; both v3 and v4 need the same training-data investment to close the synthetic-row gap.

### Consequences

**Good, because**:

- No inherited v17 regressions (decision 0054 trap avoided).
- Known-good baseline; v18's signal is cleanly attributable to training-seed variance + any generator additions explicitly scoped for v18.
- Matches ac-03's evidence requirement (≥3 triage rows OR negative-evidence argument) via both channels (3 in-favour rows + explicit negative-evidence argument for LOINC/SSN/CPT/swift_bic).
- Unblocks v18 prep without rebasing `distilled-data-relabel-7-types-v17` onto main (constraint-relaxed).

**Bad, because**:

- Forfeits the 2 real-data UA gains v4's distilled UA loader would address. Both columns remain on the v18 failure list as "persistent v16 failures", awaiting a follow-up v4-UA-adoption card.
- Forfeits v4's excel_format generator improvement for the synthetic excel_format row (1 of 55).
- Leaves the v4 branch in its held state (decision 0054); artefact disposition is deferred to the v18 outcome MADR (0062).

### Rebase status

Not applicable — v18 does not pull from `distilled-data-relabel-7-types-v17` under this decision. No rebase or merge-conflict work required.

### Follow-up cards

- **v4-UA-adoption card** — adopt the v4 distilled UA loader (17,812 UAs ex ua-parser/uap-core) in isolation, gated on v18's completion. Expected impact: +2 real-data UA columns, measured v16/v18 → next-version UA delta. Out of scope for v18 per this decision.
- **Amount-variant generator card** — add per-subtype amount generators producing distinct value-shape signatures (`amount_lakh: "1,23,456.78"`, `amount_apostrophe: "1'234.56"`, `amount_accounting: "(1,234.56)"`, etc.) for v18 or follow-up. Explicit scope decision in ac-02/ac-05 once sweep results are in hand.
- **Container-type generator card** — improve `json_array`, `xml`, `csv`, `query_string`, etc. training exemplars to resist collapse to `categorical`.
