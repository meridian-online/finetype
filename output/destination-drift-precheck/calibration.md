# Destination-drift pre-check — band calibration (ac-04)

The drift gate must trip on the two explosions we already paid for, and stay
quiet on model-seed noise. Calibrated band, now the `drift_report.py` defaults:

| knob          | value      | what it suppresses |
|---------------|-----------:|--------------------|
| `--abs-floor` | 0.0040 (0.40pp) | small-base labels whose raw counts wobble |
| `--rel-mult`  | 3.0×       | large stable labels that drift a few columns |
| `--direction` | up         | shrinking labels (a cluster shedding FPs / benign reshuffle) |

All evidence on ONE fixed 1000-file / 13,533-column corpus sample (identical
file list across every snapshot — DuckDB reservoir sampling is not
seed-reproducible, so a shared list is mandatory or the diff conflates corpus
churn with the model change).

## Verdicts

| pair | meaning | verdict | binding label |
|------|---------|---------|---------------|
| v22 → v23 | paid-for categorical explosion | **NO-GO** | `representation.discrete.categorical` +6.13pp, 4.10× |
| v19 → v24 | paid-for untargeted-boundary explosion | **NO-GO** | `geography.coordinate.latitude` +0.64pp, 5.65× |
| v19 s42 → s43 | pure model-seed noise (same files) | **GO** | — none trips — |

The v24 row is the one the hand-picked `watch` block missed: latitude was never
pre-registered, yet the full-label-space report flags it. That is the whole
reason this instrument supersedes the watch block.

## Margins — why these thresholds, not tighter

- **Relative axis.** Worst noise mover = `technology.internet.user_agent` at
  2.23×; smallest true signal = v23 categorical at 4.10×. 3.0× sits
  near-centrally in that (2.23×, 4.10×) gap: **0.77× headroom below, 1.10×
  above.** Tighter (2.5×) leaves only 0.27× over the worst noise mover.
- **Absolute axis.** The floor is load-bearing for small-base wobblers that
  clear the rel line: `identity.commerce.isbn` moves +0.32pp at 4.19× and is
  excluded ONLY by the 0.40pp floor. The smallest true signal it must pass is
  v24 latitude at +0.64pp → **0.24pp headroom**, against 0.08pp to isbn below.
- **Direction.** The control's `container.array.comma_separated` collapses
  −2.93pp; v23/v24 shrink many labels. All excluded structurally by
  `--direction up`, not by a threshold — destination drift over-emits INTO a
  concentrated destination, benign noise disperses across many shrinking labels.

## Reproduce

```
cd output/destination-drift-precheck
scripts/drift_report.py sense_dist_v19fx_s42.json sense_dist_v19fx_s43.json  # GO  (exit 0)
scripts/drift_report.py sense_dist_v22fx.json     sense_dist_v23fx.json      # NO-GO (exit 1)
scripts/drift_report.py sense_dist_v19fx_s42.json sense_dist_v24fx.json      # NO-GO (exit 1)
```

Structured reports: `drift_{control,v23,v24}.json`. Exit code is the gate (0 GO,
1 NO-GO), so a launcher gates on it directly.
