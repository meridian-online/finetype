# Handover — eval-expansion Phase A+B (m-19, 2026-04-21)

**For:** next Nightingale session.
**From:** session that landed the eval-corpus realism / coverage / leakage
overhaul.

## TL;DR

All 15 acceptance criteria on `.orbit/specs/2026-04-21-eval-expansion/spec.yaml`
are complete. Three MADRs (0055 realism, 0056 leakage, 0057 coverage)
are in `accepted` status. Manifest migrated 4→7 columns; coverage gate
passes at 240/240; row-hash firewall is live and filter is
active-by-default in `scripts/prepare_multibranch_data.py`. v16
diagnostic re-score on the expanded eval: **297/352 (84.4% label,
91.8% domain)** — the drop from 235/242 is the signal, not noise.
`models/default` untouched — still sherlock-v16 at v0.6.17. No new
release, no v18 sweep yet (blocked by constraint #4 until Phase A+B
ships as a PR).

## State of the repo

```
| Item                        | State                                             |
|-----------------------------|---------------------------------------------------|
| Current branch              | distilled-data-relabel-7-types-v17 (unchanged)    |
| Eval-expansion branch       | NOT YET CUT — work is in working tree             |
| PR                          | not yet opened — next step                        |
| main                        | unchanged — does not yet have eval-expansion      |
| models/default              | symlink → sherlock-v16 (unchanged)                |
| Shipped release             | v0.6.17 (sherlock-v16)                            |
| Working tree                | heavy — see §Working tree below                   |
```

## Working tree

Modified (9):

- `CLAUDE.md` — Sprint Goal rewritten for m-19, Evaluation infra section + Key File Reference extended
- `eval/datasets/manifest.csv` — 4→7 cols, 242→448 rows (110 coverage closure added)
- `eval/eval_output/report.md` — expanded-eval diagnostic re-score
- `eval/profile_eval.sh` — `read -r` extended to 7 fields (lines 78, 148)
- `eval/schema_mapping.csv` + `eval/schema_mapping.yaml` — 110 new identity mappings
- `.orbit/cards/0002-semantic-type-detection.yaml` — spec appended to `specs[]`
- `scripts/prepare_multibranch_data.py` — row-hash filter active-by-default, `--no-dedup` escape hatch
- `scripts/sweep_v17.sh` — v18 retrain-block comment

Untracked (new artefacts, 11):

- `eval/coverage_report.json` — coverage-gate JSON output
- `eval/datasets/csv/coverage_closure_phase_ab.csv` — 110 cols × 6 rows
- `eval/datasets/sources.yaml` — role manifest (35 sources, role=eval)
- `eval/licence_allowlist.txt` — internal | public-domain | permissive | synthetic-necessary | restricted-registry
- `eval/pre-screen_floors.yaml` — family-override floors from MADR 0055
- `eval/prescreen_results.tsv` — 338-row pre-screen output
- `eval/row_hashes.tsv` — 237,860 rows for leakage firewall
- `.orbit/choices/0055-eval-realism-dimensions.md` (accepted)
- `.orbit/choices/0056-train-eval-leakage-prevention.md` (accepted)
- `.orbit/choices/0057-eval-coverage-floor.md` (accepted)
- `.orbit/specs/2026-04-21-eval-expansion/` — spec, interview, review-spec, progress, triage, handover
- `scripts/compute_row_hashes.py`
- `scripts/eval_coverage_check.py`
- `scripts/eval_leakage/` — shared normaliser + `__init__.py`
- `scripts/generate_coverage_closure.py`
- `scripts/generate_triage.py`
- `scripts/prescreen_eval.py`

## What got done this sprint

Full trail in `progress.md`. 15 ACs closed; 3 MADRs moved proposed → accepted.

```
| AC     | Area                                           | Artefact                                |
|--------|------------------------------------------------|------------------------------------------|
| ac-01  | Pre-screen script + pinned floors              | scripts/prescreen_eval.py, pre-screen_floors.yaml |
| ac-02  | Manifest 4→7 cols + licence allowlist          | manifest.csv (448 rows), licence_allowlist.txt |
| ac-03  | Triage worklist (338 keep / 0 aug / 0 replace) | .orbit/specs/.../triage.md (DRAFT)        |
| ac-04  | Replace-worklist cleared (no-op)               | triage.md §no-replacements               |
| ac-05 G| Coverage gate ≥1 col per type                  | eval_coverage_check.py (240/240 exit 0)  |
| ac-06  | Row-hash firewall + shared normaliser          | scripts/eval_leakage/, row_hashes.tsv    |
| ac-07  | Training-pipeline dedup filter                 | prepare_multibranch_data.py (active)     |
| ac-08  | sources.yaml role manifest                     | eval/datasets/sources.yaml (35 sources)  |
| ac-09  | MADR 0055 realism dimensions                   | accepted                                 |
| ac-10  | MADR 0056 leakage prevention                   | accepted                                 |
| ac-11  | MADR 0057 coverage floor                       | accepted                                 |
| ac-12 G| v16 diagnostic re-score on expanded eval       | 297/352 (84.4%)                          |
| ac-13  | Sweep block + CLAUDE.md                        | sweep_v17.sh header, Sprint Goal §m-19   |
| ac-14  | Handover + daily progress                      | this file                                |
| ac-15  | Consumer inventory + profile_eval.sh patch     | progress.md §Consumer Inventory          |
```

## v16 diagnostic — 297/352 (84.4% label)

```
Profile: 297/352 (84.4% label, 91.8% domain) on 448-row expanded manifest
Actionability: 579440/579554 (100%)
```

Drop from 235/242 (97.1%) baseline is expected and is the whole point
of ac-12: newly-covered types were never in v16's training corpus, so
v16 defaults to nearby domain neighbours (e.g. `amount_nodecimal →
amount`, `yield → percentage`). This is NOT a regression — it's the
measurable weak-spot map that any v18 retrain must close. Actionability
held at 100%, so the profile's transform layer is not the bottleneck.

Per-type previously_covered vs newly_covered tagging is readable
directly from the report: columns present in the pre-closure 242-column
set are previously_covered; all 110 `coverage_closure_phase_ab.csv`
columns are newly_covered.

## Why this programme mattered

Decision 0054 held v17 because 5 of 7 relabel target types had zero
eval coverage — relabel success was structurally unmeasurable. This
sprint closes that gap three times over:

1. **Realism floor** pins what "good enough" data means — and publishes
   a carve-out for the 6 types (cpt, loinc, ssn, ein, swift_bic,
   credit_card_number) whose only authoritative source is a restricted
   registry. `synthetic-necessary` is no longer an ad-hoc workaround.

2. **Coverage floor** guarantees every one of the 240 types has ≥1 eval
   column with ≥5 non-null values. The 110 hand-curated rows in
   `coverage_closure_phase_ab.csv` are author-attested representative
   formats — small, direct, and easy to replace later with real-world
   samples as they become available.

3. **Leakage firewall** prevents the silent contamination class where a
   training distilled row becomes indistinguishable from its eval
   twin. Two layers: source-level role manifest AND row-hash SHA256
   filter over normalised (header, value). Filter is active-by-default
   in the training pipeline with a `--no-dedup` escape hatch for
   diagnostic runs.

## Next work — DO THIS NEXT

**Open PR against `main` for this sprint.** Branch cut + PR is the
natural next step. Constraint #4 ("no v18 sweep until Phase A+B
ships") reads "ships as a merged PR" — not just "lands in working
tree". Suggested PR title: `eval-expansion Phase A+B — realism +
coverage + leakage (spec 2026-04-21-eval-expansion)`.

Before opening the PR:

1. **Human review of `triage.md`** — constraint #1 is load-bearing (no
   LLM-as-judge). Hugh needs to confirm the 338 keep / 0 augment / 0
   replace mechanical result or flag any rows his eye doesn't trust.
2. **`/orb:review-pr`** via the drive pipeline — this is the final gate
   before merge. The review will cross-reference `progress.md` AC
   coverage against the diff.
3. **Confirm pyright is clean** on `scripts/prepare_multibranch_data.py`
   — pre-existing warnings remain (not ours to fix) but the ac-07 edits
   added no new ones.

After merge, constraint #4 is discharged and the v18 retrain can be
designed.

## Follow-ups parked (do NOT start yet)

1. **Phase C — edge-case second-column coverage.** Explicitly out of
   scope per constraint #6. Decision 0057 records the Phase C deferred
   target and the rationale.
2. **7 manifest rows with missing files during hash regeneration.** The
   row-hash script logged 7 manifest rows whose `file_path` did not
   resolve. Minor cleanup — either the files moved or the paths drifted.
   Not a blocker for merge; fix when convenient.
3. **v16 errors on the newly-covered 110 types.** The 297/352 re-score
   enumerates them. Some will melt as soon as a v18 is trained against
   the closed-coverage corpus; others will need distilled-data follow-up.
   Defer until post-merge v18 design.
4. **Real-world replacement of coverage_closure_phase_ab.csv rows.**
   Every row is `hand-curated` provenance — valid per MADR 0055, but a
   long-term goal is to swap hand-curated for real samples as HuggingFace
   or public-domain sources become available per type.
5. **Augment worklist.** Constraint #7 permits the augment worklist to
   remain open at sprint end; only replace must clear. Currently zero
   augment flags — but pre-screen output (`eval/prescreen_results.tsv`)
   is the seed for future augment work.

## Important context not obvious from the files

- **The manifest is the single source of truth** (constraint #9). Do
  not add parallel metadata files. `sources.yaml` is NOT a second copy
  — it's the role-manifest layer with the resolution rule documented in
  its header. The rule text is canonical in MADR 0056.
- **Header authenticity is out of scope** (constraint #5). The
  interview flagged it as a future concern — don't let it scope-creep
  into this programme. We handle value realism, not header realism.
- **MADR ordering constraint #11 was honoured** — 0055/0056/0057 were
  drafted in `proposed` BEFORE ac-01 and ac-07 implementation began.
  They moved to `accepted` only after their verifying ACs shipped.
  Preserves the register's role as a decision forum rather than a
  post-hoc rubber-stamp surface.
- **Restricted-registry carve-out** covers exactly 6 types. Do NOT
  extend this list without an explicit amendment to MADR 0055 §carve-out
  table — it is the only sanctioned route around the real/hand-curated
  floor per constraint #10.
- **`scripts/sweep_v17.sh`** now carries the v18 retrain-block header
  comment (ac-13). Do not remove when cutting `sweep_v18.sh`; copy the
  block and flip the gating reference.
- **v16 N=1 email regression** (from `.orbit/specs/2026-04-20-v16-n1-email-regression/`)
  is NOT addressed by this sprint. Independent.

## Key files to skim on session start

```
Essential (read in order):
  .orbit/specs/2026-04-21-eval-expansion/spec.yaml
  .orbit/specs/2026-04-21-eval-expansion/progress.md
  .orbit/specs/2026-04-21-eval-expansion/triage.md (awaits human review)
  .orbit/choices/0055-eval-realism-dimensions.md
  .orbit/choices/0056-train-eval-leakage-prevention.md
  .orbit/choices/0057-eval-coverage-floor.md

Supporting (skim if touched):
  eval/datasets/manifest.csv (7-col, 448 rows)
  eval/datasets/sources.yaml
  eval/row_hashes.tsv (237,860 rows)
  eval/pre-screen_floors.yaml
  eval/eval_output/report.md (diagnostic re-score)
  scripts/prescreen_eval.py, scripts/eval_coverage_check.py
  scripts/eval_leakage/__init__.py (shared normaliser)
  CLAUDE.md §Sprint Goal, §Evaluation infrastructure
```

## Open questions for Hugh (next session)

1. **Triage review** — can you confirm the mechanical 338/0/0 on
   `triage.md`, or is there a row your eye flags as augment / replace?
2. **PR scope** — merge as one PR (clean but large), or split into
   three logical commits (realism / coverage / leakage) within one PR?
3. **v18 retrain timing** — do we open the v18 design interview
   immediately after merge, or park it behind a separate card for the
   7 distilled-data types first?
4. **Augment worklist policy** — do we clear the augment backlog during
   v18 design, or run it as a standing quality-improvement stream?

## One thing the outgoing session would do differently

Cut the working branch at the START of the sprint rather than at the
end. Working on `distilled-data-relabel-7-types-v17` (the v17 branch)
was fine for staging but leaves the eval-expansion diff entangled with
the prior sprint's untracked model dirs. Future sprints: `git checkout
-b eval-expansion` on day 0, even before the spec lands. It costs
nothing and keeps the diff auditable throughout.
