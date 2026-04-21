# Handover — v17 relabel session (2026-04-21)

**For:** next Nightingale session.
**From:** session that closed out the v17 relabel sprint.

## TL;DR

v17 trained, evaluated, **held from promotion**. v16 remains the shipped
model (`models/default` untouched). PR #40 open on branch
`distilled-data-relabel-7-types-v17`, ready for merge review. Next work
is a discovery session on eval-set expansion.

## State of the repo

```
| Item                        | State                                             |
|-----------------------------|---------------------------------------------------|
| Current branch              | distilled-data-relabel-7-types-v17 (pushed)       |
| PR                          | #40 open — https://github.com/meridian-online/finetype/pull/40 |
| main                        | unchanged — does not yet have v17 artefacts       |
| models/default              | symlink → sherlock-v16 (unchanged)                |
| Shipped release             | v0.6.17 (sherlock-v16) — no v0.6.18 triggered     |
| Working tree                | clean (modulo untracked model dirs + v3 data)     |
```

## What got done this sprint

Full trail in `progress.md`; decision in `orbit/decisions/0054-hold-v17-no-promotion.md`.

Commits on branch (oldest → newest):
1. `1fd34aa` — Day 1: schema / validator / decisions warm-up (ac-05/06/07/08/14)
2. `0c22a8e` — Day 2: v4 loaders + generator rewrites (ac-01/02/03/04)
3. `bfd851b` — corpus freeze + sweep script + v16 baseline (ac-10)
4. `3225336` — close-out: progress.md + decision 0054

## v17 sweep result

Sweep wall clock: 9h 07m (Mon 20 Apr 16:05 → Tue 21 Apr 01:13 AEST).
All 3 seeds AUTO_ACCEPT under decision 0053 gate.

```
| Seed | val_acc | eval    | gate         | early-stop epoch |
|------|---------|---------|--------------|------------------|
| 42   | 0.9136  | 235/242 | AUTO_ACCEPT  | 53               |
| 43   | 0.9143  | 232/242 | AUTO_ACCEPT  | 47               |
| 44   | 0.9143  | 235/242 | AUTO_ACCEPT  | 56  ← WINNER     |
```

Promotion gate (`235 ≥ max(235, v16=235)`) **passes at the floor**, but
per-column diff vs v16 showed 3 fixes + 3 non-target regressions = net
zero. That's the reason for the hold.

Artefacts on disk (gitignored, stay local):
- `models/sherlock-v17-seed-{42,43,44}/` — keep for possible re-eval under expanded set
- `results/sweep-v17-summary.csv`, `results/sweep-v17.log` (~52 MB)

## Why we held

Decision 0054 is the canonical record. One-paragraph summary:

> Eval score is identical (235/242). SSN false-positive was fixed (a
> real target-type win). Two user_agent columns still fail despite
> 17,812 real UAs added — they're edge cases (UAs that lexically
> resemble JWTs / whitespace-separated tokens); more data won't fix
> them. Three non-target regressions appeared (gap, server_hostname,
> hs_code). Plus — 5 of 7 relabel target types (swift_bic, http_method,
> cpt, loinc, excel_format) have **zero eval coverage**, so relabel
> success on them is structurally unmeasurable. Shipping a v0.6.18 with
> identical user-facing eval + 3 new regressions adds risk without
> benefit.

## Next work — DO THIS FIRST

**Run `/orb:discovery eval-expansion`** in the next session.

The sprint surfaced a methodology bug: we spent a full sprint improving
types we can't measure. Eval expansion must precede the next retrain.

Questions worth working through in discovery (from the prior session):
1. What does "coverage" mean? One column per type, or variety (happy path + edge)?
2. Sourcing strategy — HF datasets, public APIs, hand-curated, synthetic? Licence/provenance per source.
3. Ground-truth labelling — who labels, review process, drift prevention.
4. Scope — just the 7 target types, or a broader audit of all 240 types?
5. Eval/training leakage — if v4 distilled data enters eval, we've contaminated. How do we enforce separation?
6. Target size — currently 242 columns; aim for 300? 500? Trade-off with eval cycle time.

## Follow-ups parked (do NOT start yet)

1. **User_agent edge cases + v17 non-target regressions.** Blocked on
   eval expansion — the expanded eval may reframe these. Decision 0054
   §Follow-ups tracks it.
2. **v4 artefact propagation.** When the next retrain ships, document
   in 0054 whether v17 artefacts propagated wholesale or piecemeal.

## Important context not obvious from the files

- The v17 branch contains **4 new decisions (0050–0053)** that are keepers independent of the hold — sourcing policy, http_method ENUM-only, scope-aware eval gate, training gate 88% floor. Merging PR #40 puts them in main.
- `scripts/sweep_v17.sh` is a working, reusable template for future 3-seed sweeps. It enforces the decision-0053 training gate and has no auto-promotion.
- `scripts/prepare_multibranch_data.py` now has `_V4_OVERRIDE_TYPES` + `load_v4_distilled_columns()` — the next relabel sprint can drop more types into `output/distillation-v4/` and reuse the pipeline as-is.
- Pyright on `scripts/prepare_multibranch_data.py` has pre-existing warnings (`min_match_rate`, `_padding`, `n_records`, `_reserved`, `contaminated_count`) that were already there before this sprint. Not ours to fix.
- BSD `ln -sf` gotcha from the v16 promotion is documented in CLAUDE.md's promotion flow — future promotions must use `ln -sfn`.

## Key files to skim on session start

```
Essential (read in order):
  orbit/decisions/0054-hold-v17-no-promotion.md
  orbit/specs/2026-04-20-distilled-data-relabel-7-types/progress.md (§Day 3 log)
  orbit/specs/2026-04-20-distilled-data-relabel-7-types/spec.yaml (context only)

Supporting (skim if touched):
  orbit/decisions/0050-per-type-sourcing-policy.md
  orbit/decisions/0051-http-method-enum-only.md
  orbit/decisions/0052-scope-aware-eval-gate.md
  orbit/decisions/0053-training-gate-88-floor.md
  output/distillation-v4/SOURCES.md
  results/sweep-v17-summary.csv

For eval-expansion discovery:
  eval/datasets/manifest.csv (current 35 datasets)
  eval/schema_mapping.yaml (GT label → canonical type map)
  models/sherlock-v16/eval/report.md (v16 misclassifications, 7 rows)
```

## Open questions for Hugh (next session)

1. Push ahead with PR #40 merge to main, or hold the merge until eval-expansion discovery lands first? (Weak preference: merge — the decisions are independently useful.)
2. Eval expansion — internal (solo) or should we bring in domain help for the medical (LOINC, CPT), finance (SWIFT BIC), and systems (http_method, user_agent, excel_format) types?
3. Budget — how much time does eval expansion get before it starts blocking other sprints?

## One thing the outgoing session would do differently

Capture v16's eval misclassifications before starting the sweep. We
had them from the CLAUDE.md "What's next" section but didn't formally
diff until after the sweep completed. Doing that diff at corpus-freeze
would have made the "relabel doesn't have eval coverage" finding
visible earlier — possibly before the 9-hour sweep.
