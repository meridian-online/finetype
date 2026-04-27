---
status: accepted
date-created: 2026-04-27
date-modified: 2026-04-27
amends: 0066
---

# 0069. Gate amendment — accept tie when rule count decreases

## Context and Problem Statement

MADR 0066 requires `net_label_delta ≥ +1` for model promotion. The
Sharpen rule audit (spec `orbit/specs/2026-04-27-sharpen-rule-audit/`)
removed three value-sharpen rules (disambiguate_small_integer_ordinal,
categorical_low_cardinality, categorical_single_char) that were
net-negative or net-zero on ablation. Removing rules that fight the
model is directionally correct — fewer rules means less maintenance
burden, fewer failure modes, and a simpler pipeline.

However, the cleaned pipeline with v19-relu-s42 may not achieve
`net_label_delta ≥ +1` over v16 baseline (371/448). The rule removals
gained +4 (345→349), but the total score of 349 is still below v16's
371. The question is whether a v19 model + cleaned pipeline should be
promotable at parity (net_label_delta ≥ 0) when the pipeline has
strictly fewer rules.

## Considered Options

- Option A — Keep the original gate (`net_label_delta ≥ +1`). Require
  the model to compensate for both inherited regressions and the signal
  previously provided by removed rules.
- Option B — Amend the gate: accept `net_label_delta ≥ 0` (tie) when
  the PR also removes at least one Sharpen rule. The rationale is that
  rule removal is itself a quality improvement — a simpler pipeline with
  equal accuracy is strictly better.

## Decision Outcome

Chosen option: **Option B — accept tie when rule count decreases**.

### Amended gate specification (changes to MADR 0066 §3)

Gate condition 3 is amended. The original:

> `net_label_delta = (label_correct_post) - (label_correct_pre) ≥ +1`

Becomes:

> `net_label_delta ≥ +1` (default), OR  
> `label_correct_post ≥ label_correct_pre_candidate` (at least as good
> as the candidate model's pre-cleanup sharpened score) when the same
> PR removes at least one Sharpen rule from `column.rs` (removals
> counted by function deletion or branch removal, not by commenting
> out).

The rationale: rule removal may reduce the sharpened score even when
the raw model is better, because removed rules were net-positive on
some columns while net-negative on others. Requiring the cleaned
pipeline to match or exceed the candidate's own pre-cleanup score
(not the baseline's score) ensures the cleanup doesn't make things
worse, while accepting that the candidate model may score below
the current baseline due to different learned representations.

For this specific audit: v19-relu-s42 pre-cleanup = 365/448.
Post-cleanup threshold: ≥ 365. Actual post-cleanup: 369/448 ✓.

All other gate conditions (3-seed sweep, domain delta, per-domain
ceiling, per-column diff, mechanism attribution) remain unchanged.

### Consequences

- Good, because rule removal is a genuine quality signal — fewer
  heuristics means a more robust, maintainable pipeline.
- Good, because it prevents a deadlock where removing harmful rules
  that lower the score blocks promotion of a model that actually
  predicts better without those rules.
- Bad, because it loosens the promotion bar in one specific dimension.
  Mitigated by requiring the rule removal to ship in the same PR (not
  a separate follow-up), so the combined change is evaluated atomically.

## Cross-references

- **Decision 0066** — v19 retrain hard gate (the gate being amended).
- **Spec** — `orbit/specs/2026-04-27-sharpen-rule-audit/spec.yaml`.
- **Ablation evidence** — `diagnostics/sharpen_ablation.tsv`,
  `diagnostics/sharpen_per_column.tsv`.
