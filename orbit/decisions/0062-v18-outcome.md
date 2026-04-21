---
status: accepted
date-created: 2026-04-22
date-modified: 2026-04-22
---
# 0062. v18 Outcome — Held, Not Promoted

## Context and Problem Statement

v18 retrained the sherlock multi-branch model on the v3 corpus (decision 0060) with fixed data-seed discipline (decision 0061). Three training seeds {42,43,44} × 100 epochs × patience 15 completed cleanly in 440 minutes wall-clock. All three seeds cleared the training gate's auto-accept threshold (val_acc ≥ 0.912, decision 0053).

Winner selection per spec ac-07 (`highest profile-eval score > highest val_acc > lowest seed number`): **seed 42** at 297/352 label correct, val_acc 0.9134.

The promotion gate from spec v1.3 requires BOTH:

- Score ≥ 297/352 (v16 pinned baseline)
- Per-domain regression ≤ 3 columns in any single domain

Both conditions are mechanically met. The outcome decision (ac-09) is whether v18 ships as the new production model, or whether v16 remains shipped and v18 is held as a branch artefact.

## Considered Options

- Option A — Promote v18 seed 42 as `models/default`. Tag new release, publish to HuggingFace.
- Option B — Hold v18. Keep v16 as `models/default`. Preserve v18 seed 42 artefacts on branch `distilled-data-relabel-7-types-v17` (the active branch for this spec) for future reuse.
- Option C — Halt v18. Delete all three seed directories and the FTMB, record as a failed retrain.

## Decision Outcome

Chosen option: **Option B (hold)**, because v18's label-accuracy delta is exactly zero and the per-column churn matches v17's held-outcome pattern (decision 0054) — mechanical gate pass without a substantive improvement signal.

### Evidence

Full per-column diff in `orbit/specs/2026-04-21-v18-retrain/v16-v18-diff.md`:

- **8 fixes** (v16 wrong → v18 correct): concentrated on newly-added `coverage_closure_phase_ab` synthetic rows — `dna_sequence`, `dot_dmy_24h`, `iso_8601_compact`, `iso_8601_milliseconds`, `sedol`, `si_number`, `word`; plus `datetime_coverage::fiscal_year`.
- **8 regressions** (v16 correct → v18 wrong): `codes_and_ids::sha256` (hash → tsid), `coverage_closure_phase_ab::mdy_short_slash` / `token_urlsafe` / `weekday_full_month`, `new_representation::inchi` / `smiles`, `new_technology::git_sha` (hash → tsid), `tech_systems::server_hostname` (hostname → email).
- **47 persistent misses** (both wrong): 44 share the exact same prediction in both models (including all 11 `amount_*` variants collapsing to `amount`, all container types collapsing to `categorical`, all datetime subtypes collapsing to their nearest specific timestamp). 3 churn rows differ between v16 and v18 but remain wrong in both.
- **Net label delta: +0 columns.** v16 297/352 = v18 297/352.
- Per-domain deltas on a shared-methodology recount: container +0, datetime +3 (worst), finance −1, geography +0, identity +0, representation −3, technology +1. Max regression 3 = at the promotion-gate limit (not over).
- Domain accuracy: v16 323/352 (91.8%) → v18 325/352 (92.3%), +2 columns at the domain-prefix level.

### Why hold, not promote

1. **Zero net label improvement.** A promotion to tie the production baseline would replace a stable artefact with a different-but-equivalent artefact, burning HF bandwidth and release tooling cycles for no user-visible gain.
2. **Datetime regression at the gate limit.** +3 on a single domain (datetime) is the maximum permitted; promoting at the limit forfeits headroom for future sweeps. Better to leave headroom for v19.
3. **v17 precedent (decision 0054).** v17's held verdict was net-zero on the 242-col eval (3 fixes / 3 regressions). v18's is net-zero on the 352-col eval (8/8). The failure mode is the same shape, on a larger eval surface — churn without signal.
4. **Persistent-miss concentration tells the next card, not a promotion.** 44 of 47 persistent misses share the exact same wrong prediction in both models; retraining alone is not the lever. Amount variants (11), container types (7), and datetime subtypes (6) are generator-coverage problems, not architecture problems.

### Why not halt

v18's per-seed artefacts and training logs are valuable:

- 8 fresh fixes on newly-added coverage rows prove the retrain pipeline works end-to-end on expanded eval inputs.
- The training-stability variance (0.0004 across 3 seeds) confirms decision 0061's data-seed discipline — the 3-seed sweep is now a clean training-only signal.
- The v16 failure fingerprint on expanded eval is now cached for v19 / v20 baselines without needing to re-run v16 eval.

### Consequences

**Good, because**:

- v16 remains the production baseline — zero disruption to users, no HF bandwidth spent.
- v18 seed 42 artefacts preserved on branch `distilled-data-relabel-7-types-v17` (the branch this spec lives on). Re-usable if a future sweep picks up from here.
- The 44-row "same-prediction persistent miss" set is now a concrete backlog for generator-coverage cards: amount-variant generators, container-type generators, datetime-subtype generators (referenced in decision 0060's follow-up section).
- Training-seed-discipline decision (0061) validated on its first adopter — one prep artefact, three training runs, clean variance signal.

**Bad, because**:

- v18 seed 42's 8 real fixes remain unshipped. The `coverage_closure` synthetic rows these fixes address are eval-only — no user-visible impact from the fixes either, but a future sweep starting from v18 would regain them for free only if we promote.
- Forfeits headroom on the datetime-domain regression limit. A v19 sweep starting from v18 would need to beat v16 on all domains AND beat v18 on datetime to promote — a tighter bar than retraining from v16.
- Implicit signal to contributors that "shipped the gate" is not the same as "shipped the release". Handover.md makes this explicit.

### Release-scope decision (ac-10)

Handed off to `orbit/specs/2026-04-21-v18-retrain/handover.md`:

- `models/default` symlink: **UNCHANGED** (stays on v16).
- `FINETYPE_CI_MODEL` in workflows: **UNCHANGED** (stays on v16).
- HuggingFace `meridian-online/finetype-model`: **NO UPLOAD**.
- v18 seed artefacts: preserved in-tree under `models/sherlock-v18-seed-{42,43,44}/` on this branch.
- Shared FTMB: deleted post-sweep (preserved in commit history if ever needed).

### Follow-up cards (informed by v18 diff)

The persistent-miss concentration gives three concrete capability cards:

- **Amount-variant generator card** — per-subtype generators producing distinct value-shape signatures for the 11 `amount_*` collapse cases. Referenced in decision 0060.
- **Container-type generator card** — resist `xml` / `csv` / `json_array` / `html` / `yaml` / `query_string` / `semicolon_separated` / `whitespace_separated` collapse to `categorical`. Referenced in decision 0060.
- **Datetime-subtype generator card** — resist `iso_microseconds` / `jp_era_short` / `julian` / `ordinal` / `pg_short_offset` collapse to their nearest-but-wrong specific timestamp.

These are eval-expansion-surfaced gaps, not regressions from v16. Addressable in v19 once the lever choice is made.
