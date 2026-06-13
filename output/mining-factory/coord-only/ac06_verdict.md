# coord-only ac-06 — NO-GO, but the most informative one yet

Spec `2026-06-07-reference-data-mining-factory`, the minimal-blend (lat+lon only,
100 manufactured columns) candidate `sherlock-mfg-coords-relu-s43` (best of 3,
val_acc 91.53%), scored through all three honest instruments.

## Verdict

| Instrument | Result |
|---|---|
| **Gold anchor** (931 cols) | **0.718** (CI 0.688–0.745) — +6.3pp over v19 baseline (0.655), **matches the current shipped default (0.719)**. lat/lon family PERFECT (latitude P0.975/R1.0, longitude P1.0/R0.978). |
| **Sense post-train** (1,000-file full-label) | **GO** — no label drifted. The proxy's `wkt` trip did NOT survive convergence, as predicted. |
| **Corpus-honest gate** (33k, BLOCKING, H05) | **NO-GO** — 7 numeric-family triggers |

The candidate is **gold-clean and Sense-clean** but a corpus-honest NO-GO. It does
not ship.

## What the gate caught that the other two could not

| label | band | v19→cand | oracle-confirmed | read |
|---|---|---|---|---|
| `representation.identifier.numeric_code` | collapse | 59k→10.6k (0.18×) | 6,875→956 | numeric_code gutted |
| `identity.commerce.upc` | collapse | 12.9k→860 (0.07×) | 1,524→832 | UPC gutted |
| `datetime.date.compact_ymd` | collapse | 2k→481 (0.24×) | 1,397→412 | |
| `finance.currency.amount` | oracle_fp | 59k→63.5k | net_contra_in **16,821** | huge created-FP |
| `representation.numeric.si_number` | oracle_fp | 9.5k→20.8k (2.2×) | net_contra_in 9,042 | over-emit |
| `datetime.date.iso_week` | oracle_fp | 166→1,825 (11×) | net_contra_in 1,635 | |
| `geography.coordinate.dms` | oracle_fp | 3.7k→2.1k | net_contra_in 1,252 | coordinate-adjacent (expected) |

Gold has only 3 `numeric_code` columns and scores them 3/3 — it is structurally blind
to the 59k→10.6k corpus collapse. The gate is the only instrument that sees it.

## The decisive finding

**Even the cleanest possible blend — pure lat+lon decimals, 100 columns, balanced —
causes corpus-scale collateral on the NUMERIC family.** The collateral relocated from
identifiers (locale-format: numeric_code/npi/user_agent/isbn) to numerics
(coord-only: numeric_code/upc/amount/si_number/iso_week), but it did not vanish. This
is the 5th consecutive additive-blend NO-GO (v22, v23, v24, locale-format, coord-only).

The mechanism is now unambiguous and it is REPRESENTATIONAL, not frequency-based:
the manufactured coordinate VALUES are decimal numbers, and teaching the flat softmax
"these decimals are coordinates" expands the coordinate decision boundary into the
territory of numeric_code / si_number / amount / compact-date — all decimal/digit
shaped. No amount of dose-balancing removes it because the interference is between the
coordinate value-shape and every other numeric value-shape that shares the region.

## Consequence for the roadmap (updates choice 0097 / 0096)

1. **Logit-adjusted loss (choice 0097 lever 2, now built) is NOT the fix for THIS
   collateral.** Logit adjustment corrects class-FREQUENCY imbalance; this is
   representational boundary interference (coordinate decimals vs numeric decimals),
   not a rare-class problem. It remains the right lever for a genuinely
   frequency-starved class — coordinates are not that.
2. **Coordinates should ship via a Sharpen VALUE-RULE, not Sense retraining** — the
   choice 0096 "rule-shaped by measurement" pattern. Coordinates have a tight,
   checkable signature (latitude ∈ [−90,90], longitude ∈ [−180,180], decimal with
   sub-degree precision) that a value-based rule can confirm WITHOUT touching the
   Sense model's numeric boundary. This is the v24-latitude lesson, now confirmed a
   third way: the Sense model structurally cannot hold the coordinate-vs-numeric
   boundary, so deliver it deterministically.
3. **Manufacturing is validated as an EFFICACY tool** (lat/lon perfect on gold,
   gold-neutral headline) but is confirmed NOT viable as an additive-blend delivery
   mechanism for value-shape-overlapping types.

## Recommended next move

Do NOT spend another overnight retrain on coordinates. Build the coordinate Sharpen
value-rule (range + precision veto/assert) and gold+gate it — it is the delivery
mechanism the evidence points to, zero model risk, and it banks the lat/lon efficacy
win the manufacturing proved is reachable. Reserve the logit-adjusted-loss lever for
the next genuinely frequency-starved (not shape-overlapping) class.

Substrate: gold `report_mfg-coords-s43_2026-06-13.md`, gate
`output/corpus-honest-gate/gate_mfg-coords-s43.json`, Sense `drift_report_full.txt`.
Links [[mfg-localefmt-identifier-collapse]], [[catastrophic-forgetting-cure-is-train-time]].
