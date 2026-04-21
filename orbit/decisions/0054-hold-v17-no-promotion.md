---
status: accepted
date-created: 2026-04-21
date-modified: 2026-04-21
---
# 0054. Hold v17 — relabel validated but not shipped

## Context and Problem Statement

v17 (sherlock-v17 retrain with relabelled distilled data for 7 target
types: swift_bic, http_method, cpt, loinc, excel_format, ssn,
user_agent) completed its 3-seed sweep (see `scripts/sweep_v17.sh`).
Winner: seed 44 (val_acc 0.9143, eval 235/242 = 97.1%), AUTO_ACCEPT under
decision 0053. Promotion gate from decision 0052 passes at the floor:
`235 ≥ max(235, v16_baseline=235)`.

But the per-column diff against v16 (also 235/242) tells a different
story from the headline number:

```
Fixes (v16 → correct in v17):
  - people_directory / phone  — SSN false positive cleared (target-type win)
  - datetime_coverage / fiscal_year — incidental, non-target
  - multilingual / locale — incidental, non-target

Regressions (correct in v16 → v17 wrong):
  - earthquakes_2024 / gap        → amount_accounting (non-target)
  - tech_systems / server_hostname → plain_text (non-target)
  - new_geography / hs_code        → decimal_number (non-target)

Persistent target-type failures:
  - tech_systems / user_agent      — still wrong, same label (jwt)
  - network_logs / user_agent      — still wrong, shifted label
```

Net: 3 fixes − 3 regressions = 0. Eval score identical (235/242),
domain accuracy slightly worse (230/242 vs 233/242).

The two remaining user_agent errors persist **despite** adding 17,812
real UAs from ua-parser/uap-core. More data did not fix them — they are
edge-case columns (UAs that lexically resemble JWTs or whitespace-
separated tokens). 5 of the 7 target types (swift_bic, http_method,
cpt, loinc, excel_format) had zero eval coverage, so relabel success
for those types is structurally unmeasurable under the current eval set.

## Considered Options

- **Promote v17-seed-44.** Passes the promotion gate, ships the SSN fix
  and the validated v4 data pipeline. Release notes would have to
  acknowledge three unrelated regressions.
- **Hold v17; relabel validated but not promoted.** Keep v16 as shipped
  model; preserve v4 artefacts and pipeline changes on branch; open a
  separate card for eval expansion and another for the user_agent /
  regression follow-up.
- **Promote v17 with a same-PR follow-up card.** Ship the SSN fix,
  simultaneously log the regressions as known-issues.

## Decision Outcome

Chosen option: **"Hold v17; relabel validated but not promoted"**,
because a promotion with an identical eval score and three non-target
regressions adds user-visible risk with no user-visible benefit, and
because the methodology gap (5 of 7 target types have zero eval
coverage) means we cannot actually prove the relabel worked from the
eval data alone.

### Consequences

- Good, because v16 remains the shipped model with its known error
  profile; users see no regressions.
- Good, because the v4 data pipeline, generator improvements, SOURCES.md,
  and the four accepted decisions (0050–0053) stay on branch and inform
  future relabel work.
- Good, because the SSN false-positive fix is real and will ride with the
  next promotion that clears a scope-aware gate.
- Good, because this surfaces a methodology bug — eval coverage must
  precede relabel sprints, not trail them.
- Bad, because the 9-hour sweep and ~2 days of relabel work do not ship
  a user-visible improvement in this cycle.
- Bad, because shelving a trained model creates latent inventory; if
  left unresolved, `sherlock-v17-seed-*/` directories will be orphans in
  3 months.
- Neutral, because ac-11, ac-12, ac-13 in the spec are marked HOLD rather
  than FAIL — the work was completed to gate but the gate itself is
  insufficient to justify promotion here.

## Follow-ups

1. Open a discovery session on **eval-set expansion** — cover the 5
   target types with zero coverage; tighten coverage on user_agent
   edge cases.
2. Open a follow-up card: investigate the 2 persistent user_agent
   failures and the 3 v17 regressions (all non-target). Decide whether
   these are attention/feature problems, narrow distributional gaps, or
   acceptable under an expanded eval.
3. When a future retrain ships, document here whether the v17 artefacts
   were superseded wholesale or if specific pieces (v4 loaders,
   generator changes) propagated forward.

## References

- `orbit/specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml` (v1.3)
- `orbit/specs/2026-04-20-distilled-data-relabel-7-types/v16-baseline.md` (ac-10 pin)
- `results/sweep-v17-summary.csv` (per-seed results)
- Decision 0049 (split _DROP_ALL_TYPES — preserved synthetic for 7 types)
- Decision 0050 (per-type sourcing policy)
- Decision 0051 (http_method ENUM-only)
- Decision 0052 (scope-aware eval gate)
- Decision 0053 (training gate 88% floor + manual-review band)
