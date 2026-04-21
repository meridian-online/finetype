---
status: accepted
date-created: 2026-04-20
date-modified: 2026-04-20
---
# 0050. Per-Type Sourcing Policy for Distilled Training Data

## Context and Problem Statement

Decision 0049 preserved synthetic generators for the 7 types with bad
distilled data but deferred the question of how to re-source real
distilled rows. Candidates span a wide quality spectrum: public datasets
(Kaggle, GitHub, CLDR), restricted registries (AMA CPT, ICAO, IANA with
click-through terms), scraped web pages, and synthetic generators alone.
Each path has different licensing, reproducibility, and maintenance
implications. Without a policy, sourcing decisions would be made ad hoc
per type, and the project would drift toward whatever happened to be
easiest on a given day.

## Considered Options

- **A. Public datasets or generators only** — accept only data from
  permissively-licensed public sources (Kaggle/GitHub CC-BY/MIT/public
  domain) OR synthetic generators with improvement bars. Restricted
  registries are out of scope.
- **B. Mixed sourcing with license audit** — allow restricted-registry
  scraping with per-row license tags and takedown workflow.
- **C. Generators-only for all 7 types** — don't try to source any
  distilled data; rely entirely on synthetic generators (the v16 approach
  from decision 0049, but with improved generators).

## Decision Outcome

Chosen option: **A — public datasets or generators only**. Restricted
registries introduce compliance overhead that's disproportionate to the
accuracy gain, and generator-only (Option C) leaves user_agent and LOINC
under-served (both have high-quality public sources).

The v17 spec's top-level `sourcing_table:` is the single source of truth
for which path each of the 7 types takes. Per-type loaders live under
`output/distillation-v4/loaders/` for the public-dataset path, and
improvements to existing generator modules (`finetype-core`) cover the
generator path. http_method takes a third, orthogonal path —
YAML-schema-only (see decision 0051).

### Consequences

- Good, because every row in the training corpus has a clear provenance
  and permissive license, eliminating legal risk.
- Good, because public datasets are reproducible by any contributor
  without credentials or click-through agreements.
- Good, because the policy is auditable: if a row's source isn't in
  `sourcing_table` or `output/distillation-v4/SOURCES.md`, it shouldn't
  be in the corpus.
- Bad, because some high-quality registries (AMA CPT, IANA) are off the
  table even where fair use might apply — we accept narrower coverage in
  exchange for simpler governance.
- Neutral, because fallback-to-generator is permitted if a public dataset
  fails review at load time — that's a path-swap within the policy, not a
  violation of it.

## References

- Spec: `orbit/specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml` (v1.3)
- Prior decision: `orbit/decisions/0049-preserve-synthetic-for-bad-distilled-types.md`
- Sources manifest: `output/distillation-v4/SOURCES.md`
- Loader directory: `output/distillation-v4/loaders/`
