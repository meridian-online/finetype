# ac-07 — head-to-head collateral footprint (150 files, v19 reference)

- columns profiled: **1871**

| cell | utc collateral | latitude collateral | utc footprint | lat footprint |
|---|---|---|---|---|
| clean_v15 | 1 | 0 | 1 | 0 |
| dirty_v15 | 1 | 0 | 1 | 0 |
| v19 (ref) | — | — | 10 | 1 |

## Net-regression check (win condition part b)

Per-column agreement with v19 across the same 150 files — the runnable proxy for 'net cell-2 does not regress vs v19'. The v15 CharCNNs are value-level models, not column-level Sense models in the gated cell-2 pipeline, so full-corpus gated cell-2 accuracy is out of scope; this agreement rate is the faithful approximation. `disagree (non-disease)` isolates broad regression from the intended utc/latitude suppression.

| cell | cols compared | agreement with v19 | disagree (non-disease) |
|---|---|---|---|
| clean_v15 | 1870 | 37.9% (709) | 1160 |
| dirty_v15 | 1870 | 49.3% (922) | 947 |

Reference: v24 multi-branch on this lens was 26 utc + 3 latitude = 29 collateral.
v14 synthetic-data CharCNN drove latitude collateral to 0 — this table tests whether that survives once trained on cleaned REAL data (clean_v15) and whether it beats the un-cleaned control (dirty_v15).
