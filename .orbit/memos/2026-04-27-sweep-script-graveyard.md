# Memo: sweep script graveyard in `scripts/`

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing an archive pass
**Tags:** repo-hygiene, scripts, training

## Inventory

`scripts/` carries nineteen training-run scripts. Seventeen of them
describe sweeps for models that are promoted-and-superseded, held, or
retired:

```
| Script                            | Era      | Status today                        |
|-----------------------------------|----------|-------------------------------------|
| overnight_sherlock.sh             | pre-v5   | retired                             |
| overnight_v5.sh                   | v5       | superseded                          |
| overnight_v6.sh                   | v6       | superseded                          |
| overnight_v7.sh                   | v7       | superseded                          |
| overnight_v8_gelu.sh              | v8       | superseded                          |
| overnight_v9_conservative.sh      | v9       | superseded                          |
| overnight_v10_gelu_headers.sh     | v10      | superseded                          |
| overnight_v11_retraining.sh       | v11      | superseded                          |
| overnight_v12_retraining.sh       | v12      | retired (logs already in repo)      |
| overnight_v13_retraining.sh       | v13      | superseded                          |
| overnight_v14_retraining.sh       | v14      | superseded                          |
| overnight_v16_retraining.sh       | v16      | promoted, then superseded           |
| overnight_v19_paired.sh           | v19      | LIVE — produced sherlock-v19-relu-s42|
| sweep_v16.sh                      | v16      | superseded                          |
| sweep_v17.sh                      | v17      | held (decision 0054), not promoted  |
| sweep_v18.sh                      | v18      | held (decision 0062), not promoted  |
```

Two are live or recent enough to keep in the working tree:
`overnight_v19_paired.sh` (produced the current default model) and
arguably `sweep_v18.sh` (held but recent — see decision 0062).

The rest are fossilised configuration. They cannot be re-run without
the corresponding training data state, and that state is gone:
distilled corpora have been regenerated, label remaps have changed,
and the multi-branch architecture has shifted under their feet.

## Why this matters (a little)

Three small problems:

1. **Discovery noise.** `ls scripts/` is the first thing anyone does
   when looking for "how do I train a model" or "how was v16
   produced." Sixteen near-identical filenames push the live one off
   the screen.
2. **Copy-paste hazard.** The pattern when starting a new sweep is
   "copy the last one." If the last one in alphabetical order is
   v14, you end up porting v14's data path to v20 instead of starting
   from v19's known-good shape.
3. **Implied currency.** Files in `scripts/` look maintained.
   `overnight_v5.sh` is not maintained. It has been broken for months;
   nobody would notice until they tried to run it.

## What's salvageable

Each old script is a primary-source record of the sweep config that
produced its model. That has historical value when reading old
decisions — "what did the v11 sweep actually do?" — but the answer
lives in git history. `git show <commit>:scripts/overnight_v11_retraining.sh`
recovers it any time.

## Proposal

Create `scripts/archive/<model-version>/` and move retired sweeps
there. Two-stage:

```
scripts/archive/sherlock/
  overnight_sherlock.sh
scripts/archive/v5/  through  v18/
  overnight_<vN>_*.sh
  sweep_v<N>.sh   (where applicable)
```

Working tree keeps:

```
scripts/
  overnight_v19_paired.sh   # current
  sweep_v18.sh              # last-held, may resurface
  train.sh / eval.sh / package.sh / prepare_*.py / ...
```

Alternative — **delete entirely.** Git history preserves them. Their
only consumer is human curiosity, which `git log -- scripts/` serves
just as well. This is what the Meridian pillar "design for the future"
actually says: don't carry the past on your back when version control
already does.

Recommendation: **delete the v5–v14 era** (eleven scripts, all
multiple architecture-shifts behind reality), **archive v16/v17**
(promoted/held but recent enough to reference without git), **keep
v18/v19** (held + live).

## Composition with repo-cleanliness memo

Bundle with the gitignore + tracked-log cleanup. Same theme: the repo
is full of mid-flight artefacts that should have been cleaned up at
each sweep's end. One housekeeping PR can absorb both.

## What to do next time

When a sweep ships (or definitively doesn't), the closing PR should
either delete the script or move it to `scripts/archive/`. Add a one-
line item to the v0.7.0 release skill: "If a model promotion lands,
delete or archive the sweep script in the same PR." Discipline at the
moment of completion — not a backlog of cleanup later.

## Not action yet

Observation memo. Trivial to action when bundled with another
housekeeping change. ~10 minutes of `git mv` and `rm`.
