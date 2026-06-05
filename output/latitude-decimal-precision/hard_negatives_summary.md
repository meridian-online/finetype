# latitude->decimal — hard-negative extraction

Per spec `2026-06-06-latitude-decimal-hard-negative-retrain` ac-02. The inverse of v24's failure: feature floats v19 wrongly grabs as latitude, pushed back to decimal_number.

## Coverage

| cluster_id | sense FP label | correct label | advisory safety | candidates | true-coord excluded | leakage dropped | kept |
|---|---|---|---:|---:|---:|---:|---:|
| `lat_to_dec` | `geography.coordinate.latitude` | `rep.numeric.decimal_number` | 0.589 | 3,974 | 1,434 | 0 | **2,540** |

## Gates

- Safety floor: NONE. safety_score is advisory only and structurally blind to destination drift; the proxy pre-check (ac-03) is the gate.
- Coordinate-header guard: 1,434 columns excluded as real latitude/longitude/projected coordinates (protects coordinate recall — ac-02's invariant).
- Leakage dedup (MADR 0056): 0 columns dropped on eval row-hash collision.
- correct_label set = `['representation.numeric.decimal_number']` — decimal only: **PASS** (asserted).

## Total kept: 2,540
