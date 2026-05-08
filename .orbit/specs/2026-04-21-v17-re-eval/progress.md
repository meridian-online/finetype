# v17 re-eval on expanded corpus — progress

Spec path: (no spec.yaml — this is a measurement card, not an implementation spec)
Started: 2026-04-21
Status: **COMPLETE — findings shipped, PR pending**

## What this card was

Decision 0054 held v17 from promotion pending expanded-eval measurement.
PR #42 shipped the expanded eval (2026-04-21-eval-expansion). This card
re-scored v17-seed-44 against the expanded 448-row corpus to answer: did
the relabel work (PR #40) move the needle once we could measure it?

## What we learned

- **v17 does not outperform v16** on the expanded eval: −2 label at the
  headline (295/352 vs 297/352), +3 domain (326/352 vs 323/352), −6 on
  the full 448-row combined count.
- **On the 10 rows that specifically measure relabel targets**, v17 and
  v16 are **identical**: 6 stable-hit / 4 stable-miss / 0 fix / 0 regression.
  The relabel work is invisible in eval output.
- **Signal-to-noise is the likely explanation** for the relabel being
  invisible: PR #40 fixed ~45 distilled rows across 7 types; each type
  trains on ~1000+ rows; 45/7000+ ≈ 0.6% — below noise floor.
- **A surfaced pipeline gap** reframes 2 of the 4 stable-misses
  (`http_method`, `excel_format`) as post-processing gaps, not training
  gaps: the pipeline returns generic `representation.discrete.categorical`
  when it should promote to the named type after a validator/enum match.
  This is a concrete, spec-able fix that needs no retrain.

## Recommendations

1. **Do not promote v17.** Ratify decision 0054 as MADR 0058 with
   expanded-eval evidence.
2. **Close the "fix more distilled data" lever.** It's documented-exhausted
   at current relabel scale — fixing 45 more rows across 7 types won't
   move eval. This should be removed from CLAUDE.md "What's next".
3. **Open a validator-promotion spec** as the next implementation card
   — wire a Sharpen rule so `categorical` predictions are promoted to
   the specific named type when every sampled value passes a named-type
   validator. Likely fixes http_method, excel_format, and any other
   schema-authoritative types that currently stable-miss.
4. **Park feature-discrimination discovery** (`user_agent`, look-alike
   types with no validator) until after validator-promotion ships — the
   validator fix may narrow the discovery scope.

## Artefacts on disk

- `findings.md` — the full writeup, including numbers, the
  relabel head-to-head, and the surfaced pipeline gap.
- `v16-baseline-*` and `v17-seed-44-*` profile / delta snapshots for
  reproducibility.

## Reproduction

```
git checkout v17-re-eval-on-expanded-corpus
FINETYPE_MODEL=models/sherlock-v17-seed-44 ./eval/profile_eval.sh eval/datasets/manifest.csv
uv run --with pyyaml python3 scripts/eval_delta_by_coverage.py
```

v17-seed-{42,43} also on disk for additional-seed confirmation if
seed-44 alone is doubted. Seed-44 was PR #40's declared winner.

## Follow-ups opened by this card

- **MADR 0058** (proposed in this PR): `do-not-promote-v17-relabel-scale-too-small`
- **Next implementation spec** (not yet written):
  `.orbit/specs/YYYY-MM-DD-validator-authoritative-promotion/` — wire the
  named-type promotion from validator matches. Entry point: `/orb:spec`
  (scoped, concrete).
- **Deferred discovery card** (to be opened after ^):
  feature-discrimination for validator-less look-alikes like user_agent.
  Entry point: `/orb:discovery`.

## One thing to preserve across compaction

The core reframe: **"training-data quality" as a lever is exhausted at
current intervention scale.** The path to eval score improvement is
(a) wire validator-authoritative promotion, then (b) discover feature
gaps for non-validator types. This is a pipeline/inference question,
not a training-data question. The next retrain (v18+) should only
begin after the pipeline ceiling is established.
