# Memo: tracked logs + uncommitted artefact bloat

**Date:** 2026-04-27
**Author:** Nightingale (with Hugh)
**Status:** Observation — proposing a cleanup
**Tags:** repo-hygiene, gitignore, ci

## Inventory

```
| Location              | Size   | Status                                     |
|-----------------------|--------|--------------------------------------------|
| target/               | 20 GB  | gitignored ✓                                |
| output/               | 2.9 GB | partially tracked (loaders + sources)     |
| results/              | 1.8 GB | gitignored ✓                                |
| research/             | 414 MB | not gitignored                             |
| models/ (untracked)   | 360 MB | model files gitignored ✓                  |
| diagnostics/          | 1.8 MB | partially tracked (rhh_* TSVs)            |
| train.log             | 104 KB | TRACKED in git (last touched 2026-03-05) |
| train_v12b.log        | 256 KB | TRACKED in git (last touched 2026-03-05) |
| training.ndjson       | 56 KB  | gitignored ✓                                |
```

Two real problems:

1. **Two committed log files** that have no business being in git
2. **Two large directories** (`output/`, `research/`) not gitignored,
   so any new artefact that lands there is on the cusp of getting
   accidentally committed

## The committed logs

```
train.log         — 104 KB, last touched 2026-03-05 (v12 era — six models ago)
train_v12b.log    — 256 KB, last touched 2026-03-05 (v12 era)
```

These are training-loop stdout captures from a sweep that has long
since been superseded. They are not reproducible from source — they
record specific runs at a specific commit on specific data. They have
no diagnostic value today because v12 has been retired.

## The diagnostics folder

`diagnostics/` contains 9 tracked TSVs from the Remove-Header-Hints
work (`rhh_*` prefix, spec
`2026-04-24-remove-header-hints/`). Spec is shipped (PR #48). The
diagnostics were the working substrate, not the deliverable — they
land in git now because the spec uses `.tsv` evidence files for its
ACs.

This is a soft problem: the files are small (1.8 MB) and serve as
audit evidence for an accepted decision. But the pattern — diagnostic
artefacts get committed by default — will not scale. If every
diagnostic spike adds ~10 TSVs, the repo grows unboundedly.

## The not-gitignored output / research dirs

`output/` (2.9 GB locally) holds distillation outputs, multibranch
training intermediates, spike results. Most aren't tracked, but
`output/distillation-v4/SOURCES.md` and a few loader scripts are. So
git sees these directories and watches for changes.

`research/` (414 MB) holds an external project — probably a
crates-vendor or local fork (`prepare.py`, `program.md`, `train.py`).
Not gitignored. Hasn't been committed but `git status` would surface
it on any addition.

## Proposed gitignore additions

```diff
+ # Training logs (per-run, not reproducible from source)
+ /train.log
+ /train_v12b.log
+ /train*.log
+
+ # Working directories — diagnostics, distillation, research artefacts
+ /output/
+ /research/
+ /diagnostics/
+
+ # Snapshot directories (e.g., char-cnn-v12.snapshot.20260305T045033Z)
+ /models/*.snapshot.*
```

Then a one-time `git rm` for the existing tracked entries:

```bash
git rm --cached train.log train_v12b.log
git rm -r --cached output/distillation-v4/ output/llm_label.log output/llm_labels.csv
git rm -r --cached diagnostics/  # if we want a clean break
```

## What to do with diagnostic TSVs going forward

The diagnostics-as-evidence pattern is real — accepted MADRs sometimes
need data files behind them. Three viable patterns:

```
| Pattern                                | Pros                                | Cons                              |
|----------------------------------------|-------------------------------------|-----------------------------------|
| Track in spec dir                      | co-located with spec                | repo grows; binary/TSV is weight   |
| Track summary, drop raw                | smaller; spec is self-contained     | raw not preserved long-term        |
| External archive (HF dataset / S3)     | unbounded scale                     | needs publish step + URL ref       |
```

Recommend: **track summary, drop raw, link to commit SHA for
retrieval if needed.** Spec text references the commit where the raw
artefacts were generated; raw doesn't ship in main. If someone needs
it, `git checkout <sha> && rerun`. Same as how we handle training
data (it's not in git).

## Why this is low priority

No user pain. Repo is large but `target/` dwarfs everything else and
that's already handled. CI doesn't choke. Clones take longer than
they should but only by a few seconds.

But: it's also low effort. ~30 minutes of work to ship the
gitignore + rm-cached pass. Pays itself back the first time someone
new clones the repo.

## When to ship

Bundle with the next housekeeping PR or the v0.7.0 polish PR. Don't
make it a standalone PR — it's not interesting enough to bisect.

## Not action yet

Observation memo. Trivial to action when bundled with another change.
