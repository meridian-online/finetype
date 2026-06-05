# v24-numeric-precision — re-litigation memo (eval Failed, do not ship)

**Verdict: NO-GO. Default stays `sherlock-v19-relu-s42`. v24 is not promoted.**

Per spec `2026-06-03-v24-numeric-precision` ac-04: *"If Failed, open a
re-litigation memo and do not ship."* This is that memo.

Measured 2026-06-05 on the freshly-built binary (`./target/release/finetype`
0.6.23, with Rule 32 `schema_fail_demotion` compiled in — **not** the stale
`~/.cargo/bin/finetype` 0.6.20; see memory `roundtrip-harness-uses-path-binary`).
Candidate evaluated: seed s42 (the promotion-convention seed, matching the v19
default). The failure is structural — data composition, not seed variance — so
s43/s44 were not separately evaluated.

## Headline

v24 trained cleanly (val_acc 0.944 across all three seeds, above v22's 0.9305)
and it *did* avoid v23's exact failure mode — `representation.discrete.categorical`
did not explode (corpus 1.01% → 0.86%). But it opened a new one in its place: a
**`geography.coordinate.latitude` false-positive explosion**. v24 relabels plain
numeric columns as coordinates. The negative transfer didn't disappear; it moved.

The reachability `safety_score` rated all four target clusters HIGH/MODERATE-safe
(0.84–0.95). That score gates *categorical*-direction risk — whether a cluster is
distinct from its correct-label neighbours — and it was right that categorical
didn't blow up. It does **not** measure whether adding those examples
destabilises *other* boundaries. Here, additive numeric hard-negatives shifted
the numeric↔coordinate decision: small-range decimals (magnitudes, RMS, errors)
now read as latitude. **A HIGH safety_score is necessary, not sufficient. The
mandatory Sense-distribution pre/post check is what caught this** — exactly why
CLAUDE.md keeps it mandatory even for HIGH-safety bets.

## Evidence

### ac-04(a) — FP drop on the four target clusters: FAILED in aggregate

Per-cluster, on the known-bad members (n≈300/cluster, v22 fired FP label & YDF
said the numeric correct label), v24-s42 vs the v19 re-baseline:

| cluster   | v19 FP rate | v24 FP rate | read |
|-----------|------------:|------------:|------|
| bool→int  | 0.993 | 0.436 | win (−56%) |
| int→dec   | 1.000 | 0.000 | win (−100%, but reassigns mostly to `unknown`, not decimal) |
| utc→int   | 0.837 | 0.800 | marginal (−4%) |
| url→int   | 1.000 | 1.000 | untouched (likely header-driven, not reachable by value hard-negatives) |

Two clean wins, one marginal, one total miss. But the corpus-wide label counts
(1000-file snapshot, seed 42) tell the real story — utc predictions *expanded*:

| target label (as Sense output) | v19 | v24 | factor |
|--------------------------------|----:|----:|-------:|
| datetime.offset.utc            |  50 | 256 | **5.1×** |
| technology.internet.url        | 389 | 368 | 0.95× |
| representation.boolean.binary  | 109 |  84 | 0.77× |
| representation.numeric.integer | 1912| 1658| 0.87× |

So v24 still emits utc on 80% of the old bad columns **and** fires it on ~5× as
many columns overall. That is a precision regression, not a fix.

### ac-04(b) — no collateral: FAILED

Matched 1000-file snapshots, same seed, fresh binary (total_cols 12771 v19 /
12206 v24; rates used where the count gap matters):

| family | v19 | v24 | read |
|--------|----:|----:|------|
| representation.discrete.categorical (pct) | 1.01% | 0.86% | **OK — v23 mode avoided** |
| geography.coordinate.latitude  | 15 | 65 | **4.3× (0.12%→0.53%)** |
| geography.coordinate.longitude | 16 | 44 | 2.75× |
| geography.location.region      | 35 | 61 | ~1.7× |
| geography.location.city        | 93 |123 | +32% |
| geography.location.country     | 26 | 19 | −27% |

The coordinate/geography inflation is the disqualifier. ac-04(b) requires "NO
geography regression"; v24 mistypes numeric columns as latitude/longitude
corpus-wide.

### ac-05 — recall non-regression: passes literally, but precision regresses

Earthquake round-trip (`scripts/roundtrip_metrics.sh`, fresh binary):

| model | non_trivial_pct | reject_rate | grade |
|-------|----------------:|------------:|-------|
| v19   | 1.00 | 0.0130 | C |
| v24   | 1.00 | 0.0583 | **F** |

Recall holds (ac-05's literal bar), but v24 drops the flagship demo from C to F.
Per-column cause: six plain decimals (`mag`, `dmin`, `rms`, `horizontalError`,
`depthError`, `magError`) → `geography.coordinate.latitude` (pass validation
because lat range is lenient, so they don't reject but are semantically wrong),
and `type` → `datetime.component.periodicity` (pass 0.0 → all 14,132 cells
reject → the C→F jump).

## What this means for the next bet

1. **The reachability metric needs a second axis.** safety_score v3 scores
   distinctness-from-correct-label (categorical-direction). It is blind to
   *destination drift* — whether training a cluster pulls a neighbouring,
   untargeted boundary (numeric→coordinate here). Any v25 design should add a
   pre/post boundary-stability check across the whole label space, not just the
   watched categorical+city/region/country set. The snapshot's `watch` block
   should add the coordinate family.
2. **Additive hard-negative retrain is now 0-for-2** (v23 categorical, v24
   coordinate) at producing a clean precision win without collateral. The
   "strength through simplification" prior (decision 0038 — prefer retraining
   over rules) is under real strain for *these* clusters. The earthquake families
   are already routed to Sharpen/taxonomy (correctly); the numeric clusters may
   need the same humility.
3. **url and utc were never reachable this way.** url is untouched (header-driven
   FP); utc got worse. Only bool→int and int→dec responded — and int→dec mostly
   to `unknown`. A future bet should narrow to the two clusters that actually
   moved, or change lever entirely.

## Disposition

- Default unchanged: `models/default → sherlock-v19-relu-s42`.
- v24 model dirs (`sherlock-v24-numeric-relu-s{42,43,44}`) kept on disk as the
  failed-bet artefact; not published to HuggingFace, not symlinked.
- Substrate: this memo; snapshots `sense_dist_v{19,24}_clean.json`; per-cluster
  `fp_v24.md`; memory `v24-numeric-retrain-failed-coordinate-explosion`.
