# Next-step diagnosis — the lead target for a new default model

Oracle: the gold-anchor metrics on the SHIPPED default (v19 / models/default),
`output/gold-eval-anchor/metrics_v19-models-default_2026-06-05.tsv`. The gated
cell-2 lens is retired as a promotion signal — it over-credited v22 for fixing
non-problems. The gold anchor is the trusted oracle now.

## The headline

v19 is never *wrong* on the hard families — it's *silent*. Every recall gap
below is precision 1.000: when v19 names the label it is right, it just declines
to name it often enough.

| family | label | recall | FN | precision | read |
|--------|-------|-------:|---:|----------:|------|
| C_lat_lon_temperature | decimal_number | **0.600** | 12 | 1.000 | 12 plain decimals silenced |
| C_lat_lon_temperature | latitude | 1.000 | 0 | **0.714** | grabs 12 non-lat floats as latitude |
| A_tight_code | alphanumeric_id | **0.167** | 25 | 1.000 | tight codes silenced |
| D_year_vs_integer | integer_number | **0.333** | 20 | 1.000 | integers silenced |
| D_year_vs_integer | year | 1.000 | 0 | 0.938 | 2 integers over-grabbed as year |

The single most leveraged cell is **C**: the 12 decimal false-negatives ARE the
12 latitude false-positives — the same temperature/feature-float columns. One
fix moves two numbers: decimal recall up AND latitude precision up. And it is the
*exact opposite* direction of how v24 died (v24 pushed plain numerics INTO
latitude and exploded it). So the lead target is both high-value and naturally
de-risked against the failure we just instrumented against.

## Corpus coverage for the C fix — abundant

Columns where the diagnostic's sense=latitude, ydf=decimal_number: **3,974**.
The headers split cleanly into two populations:

- TRUE latitudes v22 got right (~1,280): `Lat` 540, `lat` 366, `latitude` 188,
  `Latitude` 111, `LATITUDE` 75. ydf merely undersells these as "decimal".
- The FALSE positives — generic feature floats grabbed as latitude:
  `RealTime(ms)` 285, `cumulative_gpa` 181, `beta.exposure` 115,
  `prev_term_gpa` 107, `mag` 67, `vz/vx/vy` (velocities), `LnPrior` 40,
  `HitRate_0.5*` (betting hit-rates), `info/learner/.../min_q` (ML metrics),
  `z.outcome`, `BSP`, `PPWAP`, `mid_offspring`.

This is a rich, clean hard-negative pool: thousands of columns whose CORRECT
label is decimal_number and which the model currently mislabels latitude. That
is exactly the additive-hard-negative shape — and the proxy pre-check now gates
whether adding them destabilises a third boundary before we spend the overnight.

safety_score for the latitude→decimal cluster (from corroborated_gaps,
sample_evidence unnested): **0.589 — MODERATE**. Per the advisory bands that
mandates the pre/post check, which is precisely what the new proxy gate provides.

## Value voting (the user's idea) — tested, honest scope

The idea: vote each cell value against the predicted type's validator and require
a majority. For BOUNDED labels this is a free precision guard. We tested it on
the lead target and it is **weak there, strong elsewhere**.

On the C family, a hard latitude-range vote (`|v| ≤ 90`) catches only **4 of
3,974 columns (0.1%)**. The impostors sit inside the band:
`cumulative_gpa` 2.7/3.4, `HitRate` 0–1, `mag` ~15.0, `vz` −47…62, `BSP` 1.6…18,
`RealTime(ms)` 1.4…6.3. A per-value range check cannot separate a GPA from a
latitude when both read 3.4. So value voting does NOT recover the C win — that
needs the hard-negative retrain (a distributional/contextual signal: these
columns cluster in [0,4] or [0,1], real latitudes spread across a geographic
range and pair with a longitude sibling).

Where value voting DOES pay off — bounded / set-membership labels:
- `year` ∈ [~1000, 2100] → disciplines the 2 year false-positives in family D.
- `longitude` |v| ≤ 180, `country_code` / `iata` / `currency` set membership —
  reject-on-minority is pure precision, can only remove a wrong claim, never add.

Recommendation on value voting: adopt it as a cheap Sharpen-stage precision
guard for bounded/enumerated labels (decision 0048 "value-based rules only" and
the Precision Principle both endorse it), but do NOT bank on it for the C family
— the temperature-float confusion lives inside latitude's legal range, so only
the retrain moves it.

## Recommended sequence

1. **Lead spec — C family hard-negative retrain.** Add the non-lat feature
   floats (gpa/mag/hitrate/velocity/RealTime/ML-metric columns) as
   decimal_number hard negatives. Gate with the proxy pre-check BEFORE the
   overnight run. Target: decimal recall 0.60→↑ and latitude precision 0.714→↑
   with NO new untargeted boundary tripped (the proxy is the judge).
2. **Parallel, cheap — value-voting Sharpen guard** for bounded labels
   (latitude ±90 as a backstop, longitude ±180, year range). Free precision,
   independent of the retrain, ships without an overnight run.
3. **Hold** — alphanumeric_id recall (25 FN) and integer recall (20 FN): real
   but recall-side silences, a separate investigation (why does the model
   decline? competing-label margin, not a value rule).

## What we don't know yet

- Whether v19 reproduces v22's exact FP headers (the diagnostic ran on v22
  sense). The gold anchor confirms v19 has the SAME failure *shape* (latitude P
  0.714, decimal R 0.600) but not the same column identities — confirm on a v19
  snapshot before fixing the blend.
- The proxy's verdict on THIS blend. It is 2-for-2 on the paid-for failures; it
  has never been run on a blend we then promoted. The C retrain is its first
  forward use, not a retro-calibration.
