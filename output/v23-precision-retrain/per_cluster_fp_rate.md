# v23 vs v22 — per-cluster false-positive rate

Per spec `2026-05-27-v23-precision-retrain` ac-04.

Each row counts columns in the gittables corpus pass where Sense fires the v22 FP label AND YDF disagrees with the cluster's correct label — the same `(sense_prediction, ydf_prediction)` pair the ac-01 extraction filtered on. The v22 numbers are recomputed here for ground truth; the `expected (report)` column comes from `eval/gittables/corpus_pass/report.md` and is kept for sanity-checking only.

## Per-cluster trajectory

| cluster_id | FP label (v22) | correct label (YDF) | expected (report) | v22 (recomputed) | v23 | Δ | Δ% |
|---|---|---|---:|---:|---:|---:|---:|
| `721b890ea74d…` | `identity.person.gender_code` | `rep.discrete.categorical` | 22,952 | 12,221 | **527** | −11,694 | **−95.7%** |
| `1b858e0d073b…` | `datetime.offset.utc` | `rep.numeric.integer_number` | 21,956 | 30,861 | **2,415** | −28,446 | **−92.2%** |
| `20803deffbad…` | `technology.internet.url` | `rep.numeric.integer_number` | 9,779 | 3,686 | 3,687 | +1 | +0.0% |
| `81b63a52e3ef…` | `rep.boolean.binary` | `rep.numeric.integer_number` | 8,649 | 21,227 | 21,049 | −178 | −0.8% |
| `cdde5d05b73a…` | `datetime.component.periodicity` | `rep.discrete.categorical` | 5,764 | 1,104 | 2,640 | +1,536 | +139.1% |
| `3f2aa8465552…` | `rep.identifier.alphanumeric_id` | `rep.discrete.categorical` | 4,835 | 40,419 | **1,682** | −38,737 | **−95.8%** |

## Totals

- Total v22 FP columns across the six clusters: **109,518**
- Total v23 FP columns across the six clusters: **32,000**
- Drop: **−77,518 (−70.8%)**

## ac-04 band — FP-rate component

Pre-committed thresholds: Met ≥ 50%; Partial ≥ 20%; Failed < 20%.

**Verdict: Met** (−70.8% drop on the top-6 cluster columns).

The final ac-04 band combines this FP-rate verdict with the gated cell-2 verdict (see `cell_deltas_v23_vs_v22.md`):
  - **Met** (ship as default) — FP drop ≥ 50% AND gated cell-2 vs v19 non-worse than v22's −10.4%.
  - **Partial** (review-and-decide) — FP drop 20–50% AND cell-2 within ±2pp of v22.
  - **Failed** — FP drop below partial threshold OR cell-2 regresses ≥ 2pp vs v22.
