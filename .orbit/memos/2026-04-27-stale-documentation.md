# Memo: documentation has drifted from reality

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a gate
**Tags:** docs, ci, freshness

## What's wrong

Public-facing documentation contradicts the actual state of the system
on multiple fronts.

### Type count drift

```
| Source                                          | Claim         | Actual |
|-------------------------------------------------|---------------|--------|
| README.md:7,22,141                              | 250 types     | 240    |
| labels/*.yaml + finetype-core taxonomy load     | (ground truth)| 240    |
| CLAUDE.md (project)                             | 240 types     | 240 ✓  |
```

README says 250 in three places. The taxonomy ships 240. The number
hasn't been 250 since some point during the v15/v16 work — labels were
consolidated and de-duplicated as part of the data-quality audit
(decision 0049, m-18).

### Eval metric drift

```
| Source             | Claim                                | Actual current                       |
|--------------------|--------------------------------------|--------------------------------------|
| README.md:163      | "81.6% label (155/190)"              | 369/448 (82.4% label) on 448-row corpus |
| README.md:164      | lists Sense→Sharpen as alternative   | retired by decision 0041             |
| CLAUDE.md          | 369/448 (82.4%)                      | (matches reality) ✓                  |
```

The README's eval table is from a pre-eval-expansion state (190-row
corpus, m-17 era). Phase A+B of the eval expansion (m-19, MADR
0055/0056/0057) shipped a 448-row corpus.

### Architectural staleness

```
| Source                              | Claim                              | Actual                                  |
|-------------------------------------|------------------------------------|------------------------------------------|
| docs/SENSE_AND_SHARPEN_PIPELINE.md  | documents Sense→Sharpen as default | retired by decision 0041 (multi-branch)  |
| docs/ARCHITECTURE.md:82             | --sharp-only is "available"        | no-op on shipped pipeline (today's memo) |
| docs/ARCHITECTURE.md:17 occurrences | references Sense classifier        | replaced by multi-branch                 |
```

The Sense→Sharpen pipeline document still presents itself as
authoritative. A new user reading these docs would end up with a
mental model two architectures behind reality.

### Skill staleness

```
| Source                                  | Claim                  | Actual |
|-----------------------------------------|------------------------|--------|
| .claude/skills/finetype-cli/SKILL.md:13 | "FineType v0.6.12"     | 0.6.18 |
```

Six releases stale. The skill is what agents read when working in
this repo — they're being told to expect a CLI six versions back.
The skill also lists `--sharp-only`, `--model-type`, `--model` as
documented flags for users — three flags slated for hide/remove per
today's CLI memos.

## Why this happens

No one source of truth for these numbers. Each ships in a different
artefact:

```
| Datum            | Authoritative source                          | Where it leaks      |
|------------------|-----------------------------------------------|---------------------|
| Type count       | labels/*.yaml (count entries)                 | README, CLAUDE, docs |
| Eval metric      | eval/eval_output/report.md (latest run)       | README, CLAUDE, MADRs |
| Default model    | models/default symlink target                 | README, CLAUDE, CI env |
| CLI flags / cmds | crates/finetype-cli/src/main.rs Commands enum | README, docs, SKILLs |
| Pipeline arch    | crates/finetype-model/src/column.rs           | docs, MADRs, README  |
```

Each is hand-typed in three or four places. Drift is inevitable.

## Three options

**A. Drift-check in CI.** Generate the canonical numbers at build time
(taxonomy YAML count → embed; latest eval report → parse) and fail CI
when README/docs disagree. Same idea as the existing
`check-ci-model-drift.sh` (PR #39 / specs/2026-04-20-ci-decouple-default-symlink).
Concrete:

- `scripts/check-doc-drift.sh`: parse README for "N semantic types" /
  "X/Y label", compare to authoritative sources, exit 1 on mismatch.
- Wire into `.github/workflows/ci.yml` as a non-blocking warn (like
  the existing model-drift check) initially, promote to gate after
  a release of stable green.

**B. Generate the README sections that have numbers.** Make the type
count and eval metric come from a template + embedded values, not
hand-typed. `tools/render-readme.py` runs in CI; PR diff includes the
re-rendered file. Heavier infra but eliminates drift entirely.

**C. Single one-line bump checklist per release.** The `release` skill
(`.claude/skills/release/SKILL.md`) gets a step: "Run
`scripts/refresh-docs.sh`, eyeball the diff, commit." Lowest infra,
relies on discipline.

Recommendation: **A first, B if drift continues.** A is the same
pattern as model-drift, the team already has muscle memory for it,
and it scales without further investment. B is the principled answer
but adds a render step nobody is going to maintain unless drift
recurs.

## Concrete cleanup work that should ship alongside

Independent of the gate, the current drift needs a one-time pass:

1. **README:** type count 250 → 240; eval metric to current 369/448;
   remove the Sense→Sharpen alternative row from the comparison table
   (or mark "deprecated").
2. **docs/SENSE_AND_SHARPEN_PIPELINE.md:** prepend a "this describes
   the legacy pipeline" banner OR rewrite as
   `docs/MULTI_BRANCH_PIPELINE.md` with current architecture.
3. **docs/ARCHITECTURE.md:** rewrite the "tiered architecture
   available via --sharp-only" paragraph (line 82) — the flag is a
   no-op on the shipped pipeline.
4. **.claude/skills/finetype-cli/SKILL.md:** version string + flag
   tables refresh. Best timed with the v0.7.0 CLI polish so we don't
   refresh twice.
5. **CHANGELOG.md:** confirm post-v0.6.18 entry is accurate.

## Composition with v0.7.0 polish

Five of today's CLI memos directly affect docs (`--model`,
`--model-type`, `--sharp-only`, `check`, `generate` — all changing
visibility) and three change pipeline shape (`schema`/`profile`,
`validate`/`load` fold). If we don't gate doc drift before v0.7.0
ships, the gap widens further.

Sequencing:

1. Land doc drift-check (option A) as a CI warning.
2. Ship v0.7.0 CLI polish + the docs refresh in the same PR.
3. Promote the drift-check from warning to gate after one stable
   release.

## Not action yet

Observation memo. The drift-check is a small concrete deliverable;
the docs refresh is a per-file pass that wants to ride v0.7.0. Both
ship together as part of the v0.7.0 polish PR or its immediate
follow-up.
