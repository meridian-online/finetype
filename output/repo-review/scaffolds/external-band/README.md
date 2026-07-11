# Rotating external-data advisory band — v0 scaffold

**Status:** SCAFFOLD (runnable stub). Not wired into the promotion order yet —
needs the author sign-off below on external-row label provenance/tier before it
prints a headline anyone acts on.

## The gap this closes

We have three accuracy instruments and none of them profiles a whole real
external table:

| instrument | what it sees | what it misses |
|---|---|---|
| gold corpus (blocking headline) | curated-**hard** columns | realistic full-table behaviour |
| representative band (advisory) | one random **GitTables** column | non-GitTables / external distributions |
| corpus-honest gate (blocking) | rare-label **relocation** vs the YDF oracle | correctness on named external sources |

The 2026-07 company-reference audit made the gap concrete: GLEIF, SEC EDGAR,
NYC DOB and Chicago-crimes tables exposed real ticker / NAICS / org-name / WKT
failures that none of the three standing instruments had flagged. This band is
the standing home for that class of miss: **profile a real external table with
the shipped binary, score the columns we have adjudicated labels for, triage the
rest.**

## Design (minimal, v0, no network)

- **Data — already on disk.** The 10 external tables the audit itself used, under
  `eval/datasets/gold_external/` (openflights, ourairports, usgs_earthquakes,
  seattle_checkouts, nyc_dob_permits, nyc_payroll, chicago_crimes, sf_businesses,
  uk_price_paid, majestic_million). No fetch needed for v0.
- **Held labels — already signed.** `held_labels_v0.tsv` (56 columns / 10 tables)
  is mechanically extracted from the `external:*` rows of the canonical gold
  corpus (`eval/gold/gold_corpus.tsv`). Those labels are already author/panel
  adjudicated — v0 scores against ground truth that needs no fresh sign-off *to
  reuse*. The `provenance` column records the originating gold family.
- **Harness — mirrors the representative band.** `run_external_band.py` profiles
  each full table (`finetype profile -f <table>.csv -o json-schema`, sibling
  context live), reads `x-finetype-label` per column, joins to the held labels,
  and emits: an advisory headline (correct/total), a per-type recall table, a
  miss list, and an **unlabelled-emission triage block** (profiled columns with
  no held label — the candidate-expansion queue where new over-emissions surface).
- **Read the delta, not the absolute.** Because the held labels overlap gold, the
  absolute headline is common-mode across candidates. The trustworthy signal is
  the candidate-vs-baseline delta — identical to the representative band's rule.

## How to run

First run (all 10 tables, from repo root or this dir):

```bash
bash output/repo-review/scaffolds/external-band/run_external_band.sh
```

Concrete equivalent (explicit paths, a rotated 4-table round):

```bash
python3 output/repo-review/scaffolds/external-band/run_external_band.py \
  --binary target/release/finetype \
  --datasets-dir eval/datasets/gold_external \
  --labels output/repo-review/scaffolds/external-band/held_labels_v0.tsv \
  --rotate 4 --seed 20260711 \
  --out output/repo-review/scaffolds/external-band/report.md
```

Prereq: a release binary (`cargo build --release -p finetype-cli`) and
`models/default` set (it is: `m2v8m-s43`). No network, no DuckDB corpus pass.

## Where it plugs into the promotion order

Advisory, alongside the representative band — **never blocking**:

```
gold-anchor (efficacy)
  -> drift proxy (pre-train)
  -> gold corpus accuracy + rare-type scoreboard   [blocking headline]
  -> representative accuracy                        [ADVISORY]
  -> external-data band  <-- NEW                    [ADVISORY]
  -> corpus-honest gate                             [BLOCKING, H05]
  -> swap
```

The advisory flag fires when a candidate's external headline drops more than the
round's noise below the baseline candidate's — same convention as the
representative band. No external headline overrides a blocking corpus-honest
NO-GO.

## Rotation policy

- The 10-table pool is the sampling frame. Each promotion round, draw a subset
  with `--rotate K --seed <round-date>` and record the seed in the report so the
  round is reproducible.
- Re-draw every round so the band is not memorised into the model over successive
  retrains. v0 pool is fixed (on-disk tables); v1 grows the pool by fetching a
  fresh external table per quarter (see sign-off item 3).
- For a full-signal round, `--rotate 0` scores all 10.

## What needs author sign-off

1. **Reusing gold's external labels as this band's ground truth.** v0 scores only
   the 56 already-adjudicated `external:*` gold columns — they carry existing
   provenance/tier. Confirm that reusing them *outside* the gold headline (as a
   separate advisory band) is acceptable, and how to attribute the overlap so the
   two instruments are not double-counted when both are cited in a promotion memo.

2. **Tier for any NEW adjudications from the unlabelled-emission queue.** The
   triage block will surface over-emissions on columns with no held label (this is
   the failure-hunting value). Those need a truth tier before they count toward a
   headline — panel tier (like the representative band) or author tier? Until
   assigned, they stay triage-only and never move the headline.

3. **Rotation source for v1.** Whether re-draw stays within the fixed 10-table
   on-disk pool, or the band fetches a fresh external table each quarter
   (network + PII screen + snapshot-register, per the retired
   `build_gold_corpus_external.py` / `gold_corpus_external_plan.md` pipeline).
   v0 is deliberately network-free; growing the pool is the v1 crank.

## Files

- `run_external_band.py` — the runner (profile -> join -> advisory report).
- `run_external_band.sh` — wrapper with a concrete first-run command.
- `held_labels_v0.tsv` — 56 adjudicated external labels (table, column, label,
  provenance), extracted from `eval/gold/gold_corpus.tsv` `external:*` rows.
