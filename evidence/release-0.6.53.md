> Generated from `evidence/fixtures.json` by `scripts/evidence.py render-release --binary 0.6.53`. Do not hand-edit — `scripts/evidence.py verify` re-renders this file and fails if it has drifted from the manifest.

# finetype 0.6.53 — what it measured at

**Every score below names the gold fixture version it was measured on.** A score without a fixture version is not evidence: ground truth moves under a bar, and a float remembered without its fixture becomes a false-rejection generator. Two scores measured on different fixture versions are not comparable, and this report refuses to subtract them rather than presenting the difference as a result.

## Headline

| Fixture version | Taxonomy | Model | Pipeline | Correct / scored | Accuracy | Measured |
|---|---|---|---|---:|---:|---|
| `gold-2026-06-28` | `tax-a8494466d9c1` | `m2v8m-s43` | predict_multibranch -> reshape -> compose_predictions (Sharpen) -> score_gold_anchor --reframe | 805 / 931 | **0.865** | 2026-07-25 |
| `gold-2026-07-14` | `tax-e0baf2e4b3bd` | `m2v8m-s43` | predict_multibranch -> reshape -> compose_predictions (Sharpen) -> score_gold_anchor --reframe | 819 / 931 | **0.880** | 2026-07-25 |
| `gold-2026-07-14` | `tax-e0baf2e4b3bd` | `m2v8m-s43` | predict_multibranch -> reshape -> score_gold_anchor --reframe (no Sharpen) | 492 / 931 | **0.528** | 2026-07-25 |

- `gold-2026-06-28` · `m2v8m-s43/composed-reframe/0.6.53` — Same fixture as the pre-0.6.41 bar, current binary: the +11 columns are the 0.6.41/0.6.50/0.6.53 Sharpen typing guards.
- `gold-2026-07-14` · `m2v8m-s43/composed-reframe/0.6.53` — 931 of the fixture's 1037 rows are scored: the 2026-06-22 corpus-pass FTMB does not cover the 106 columns added since.
- `gold-2026-07-14` · `m2v8m-s43/sense-reframe/0.6.53` — Standalone Sense, before composition.

## Same-fixture comparison

A delta is only meaningful when the fixture version and the pipeline are held fixed and the binary is the only thing that moved. These are the pairs in the manifest that satisfy that.

| Fixture version | Against | Then | Now | Δ columns | Δ accuracy |
|---|---|---:|---:|---:|---:|
| `gold-2026-06-28` | `pre-0.6.41` | 794/931 = 0.853 | 805/931 = 0.865 | +11 | +0.012 |

### Refused comparisons

Same pipeline, different ground truth. The difference between these numbers is not a measurement of anything, so it is not stated as one.

- `gold-2026-06-28` 805/931 = 0.865 vs `gold-2026-07-14` 819/931 = 0.880 (`m2v8m-s43/composed-reframe/0.6.53`) — different fixture versions.
- `gold-2026-07-14` 819/931 = 0.880 vs `gold-2026-06-28` 794/931 = 0.853 (`m2v8m-s43/composed-reframe/pre-0.6.41`) — different fixture versions.

## Fixture versions cited

| Version | Rows | Taxonomy | Types | Content hash (sha256) | Path |
|---|---:|---|---:|---|---|
| `gold-2026-06-28` | 931 | `tax-a8494466d9c1` | 245 | `9dbbcd3abeed9f4477166673772c36b7ffacfc8e5619921eae00737194215edb` | `eval/gold/gold_corpus.tsv` |
| `gold-2026-07-14` | 1037 | `tax-e0baf2e4b3bd` | 251 | `760ee4ace67064edd465d245103677e30171a9ce4bb07decc44bd69f914586a7` | `eval/gold/gold_corpus.tsv` |

- `gold-2026-06-28` — Historical: the gold corpus as it stood when the clean-label go/no-go bar was written (git blob at f903a2c). Retained so the bar it carried stays attributable to the ground truth that produced it.
- `gold-2026-07-14` — Gold corpus as of the technology.filesystem.filename adjudication; 106 columns added and 37 labels re-adjudicated since gold-2026-06-28.

## Reproducing this

Every score above cites its `source` in the manifest:

- `gold-2026-06-28` · `m2v8m-s43/composed-reframe/0.6.53` — docs/branch-ablation-m2v8m-s43.md, attribution table
- `gold-2026-07-14` · `m2v8m-s43/composed-reframe/0.6.53` — docs/branch-ablation-m2v8m-s43.md, un-ablated control
- `gold-2026-07-14` · `m2v8m-s43/sense-reframe/0.6.53` — docs/branch-ablation-m2v8m-s43.md, un-ablated control

The fixture is content-addressed, so a checkout can confirm it is holding the same ground truth before it measures anything:

```sh
scripts/evidence.py resolve-fixture --path eval/gold/gold_corpus.tsv
scripts/evidence.py taxonomy-version
scripts/evidence.py list
```

The run artefacts behind these numbers — FTMBs, prediction TSVs, per-column report tables — are regenerable and live in the ignored `output/`. What is tracked here is the part that cannot be regenerated: which ground truth this was, and what it came out as.

## What this does not record

The taxonomy column above is the vocabulary **the fixture was adjudicated under**, read from the commit in each fixture's `taxonomy.commit`. It is not the vocabulary the measuring binary was built with. The taxonomy is compiled into the binary by `include_str!`, so those are separable and a run should stamp both — but these scores predate the manifest, and the taxonomy of the binary that produced them was not captured at the time. It is not reconstructed here, because a stamp inferred after the fact is a guess wearing a hash. Measurements recorded from now on carry the taxonomy version they were measured under.
