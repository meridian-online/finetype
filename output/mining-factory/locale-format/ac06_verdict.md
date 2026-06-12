# ac-06 — honest evaluation: NO-GO. Manufacturing not validated as a drop-in Sense replacement.

Spec `2026-06-07-reference-data-mining-factory`, ac-06. Best of 3 ReLU seeds
(`sherlock-mfg-localefmt-relu-s42`, val_acc 91.38%) scored through all three honest
instruments. The candidate does NOT ship; no `models/default` swap.

## Verdict across the three instruments

| Instrument | Result | Reads |
|---|---|---|
| **Gold anchor** (efficacy, 931 cols) | **0.700** (CI 0.670–0.729) | +4.5pp over v19 baseline (0.655); **−1.9pp under the current shipped default (0.719)** |
| **Sense post-train** (mandatory) | **NO-GO** | `user_agent` 0.33%→2.78% (8.27×) |
| **Corpus-honest gate** (BLOCKING, H05) | **NO-GO** | 7 triggers — see below |

A blocking NO-GO is final (H05). ac-06's pre-set bar — gold confusion families
improve-or-hold AND corpus-honest GO AND no per-type ≥5% regression — fails on the
corpus-honest NO-GO alone.

## What the gold anchor shows: the core target landed, the new content backfired

- **Lat/lon starvation DISSOLVED.** The `C_lat_lon_temperature` confusion family —
  the entire reason this campaign exists — is now **perfect (1.000)**: latitude
  P=0.975/R=1.000, longitude P=1.000/R=0.978. The manufactured 28k-distinct
  coordinate corpus did exactly what it was built to do.
- **Currency-variant manufacturing BACKFIRED.** `finance.currency.amount` precision
  **0.100** (family accuracy 0.111). The model now predicts the manufactured
  *subtypes* (`amount_comma`, `amount_lakh`…) where gold uses the generic `amount`
  label — a granularity mismatch the locale-format split CREATED. The headline new
  content is a net negative on gold.

## What the corpus-honest gate caught that gold could not (7 triggers)

The 931-col gold anchor is blind to these — they are corpus-scale relocations on
labels gold barely covers:

| label | band | v19 → cand marginal | oracle-confirmed | read |
|---|---|---|---|---|
| `technology.internet.user_agent` | over_emit | 21k → **150k (7.1×)** | 0 → 0 | massive spurious explosion, oracle confirms none |
| `identity.commerce.isbn` | over_emit + oracle_fp | 8k → **50k (6.2×)** | 1.9k → 3.2k | over-emits onto oracle-refuted columns |
| `representation.identifier.numeric_code` | collapse | 59k → **4k (0.07×)** | 6,862 → 222 | identifier boundary gutted |
| `identity.medical.npi` | collapse | 43k → **7.6k (0.18×)** | 3,157 → 412 | confirmed npi lost |
| `datetime.date.compact_ymd` | collapse | 2k → 1k (0.50×) | 1,397 → 645 | |
| `representation.numeric.si_number` | oracle_fp | 9.5k → 10.7k | net_contra_in 2,844 | created FPs |
| `representation.file.file_size` | oracle_fp | 1.3k → 3.3k | net_contra_in 1,254 | created FPs |

## The finding: a new failure mode, not the old one

The locale-format blend **broke the numeric-collapse curse** that closed ac-05's
first attempt (decimal held 31.50%, integer 13.85%) — but it introduced a NEW
collateral: it collapses the **identifier boundary** (`numeric_code` to 7%, `npi`
to 18%) and explodes `user_agent`/`isbn`. This is the same additive-blend
untargeted-neighbour collateral as v22/v23/v24, relocated from numerics to
identifiers. Additive multi-branch blend retrains remain unable to absorb
manufactured reference data without exploding an untargeted boundary.

## What is and isn't validated

- **Manufacturing dissolves starvation: VALIDATED** at the efficacy level (lat/lon
  family perfect). The census proof + the gold confusion families confirm the
  manufactured data is clean and learnable.
- **Manufacturing as a drop-in Sense replacement: NOT validated.** Corpus-scale
  collateral on identifier types is disqualifying.
- **Gate GO-precision: STILL OPEN.** This candidate is a genuine NO-GO, so it does
  not test whether the gate cleanly passes a truly good candidate without false
  alarm. The gate's NO-GO detection is validated again (it caught a 59k→4k collapse
  and a 150k over-emit invisible to gold); GO-precision awaits a candidate that
  actually clears it.

## Next moves (for the author)

1. The currency-subtype-vs-generic-`amount` granularity clash is independently
   fixable and worth isolating — it is hurting gold without helping the corpus.
2. The identifier-boundary collapse (`numeric_code`/`npi`) is the new blocker for any
   additive-blend route; it is the v22/v23/v24 pattern on different labels. The
   architecture conclusion (additive blend cannot absorb manufactured data without
   untargeted collateral) now has a fourth independent data point — on a corpus that
   fixed the numeric half.
3. The lat/lon efficacy win is real and worth banking via a route that does not
   relocate identifiers — e.g. a targeted coordinate-only fix rather than the full
   34-type blend, or the value-level / late-fusion architecture the spec was the
   prerequisite for.

Substrate: `output/mining-factory/locale-format/` — gold report
`report_mfg-localefmt-s42_2026-06-13.md`, gate `corpus_honest_gate.txt` +
`output/corpus-honest-gate/gate_mfg-localefmt-s42.json`, Sense drift
`drift_report_full.txt`, per-column proxy diagnosis `drift_diagnosis.md`.
