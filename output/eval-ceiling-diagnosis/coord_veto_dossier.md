# Coordinate header-veto — promotion dossier (overnight, 2026-06-08)

**Status: validated GO candidate, NOT promoted. Awaiting your ship decision + 0094 acceptance.**

A feature-flagged Sharpen rule (choice 0094) that fixes the latitude/longitude
false-positive problem unwinnable across v22/v23/v24/latdec. Built and validated
overnight; `models/default` and default behaviour are unchanged (flag off).

## What it does

`apply_header_sharpen` (column.rs): a latitude/longitude prediction whose header
carries **no coordinate token**, on a generic numeric column, is demoted to
`decimal_number`. Demotion-only — the header can veto a false coordinate, never
promote one. Gated by `FINETYPE_COORD_HEADER_VETO` (default **OFF**).

## Validation — clears both gates

| evidence | result |
|---|---|
| **Unit tests** (corroboration + value gate) | 4/4 pass; `latency`/`translate`/`mag` not corroborated, `lat_dd`/`y_lat` are |
| **Rare-type scoreboard dry-run** (`validate_coord_veto.py`) | latitude FP-rate v19 0.0013→0, v22 0.0070→0, **v23 0.0163→0**; recall 0.995→0.995 (no loss) |
| **Corpus-honest gate (BLOCKING)** on 33k stratified sample | **verdict GO, triggers: []**. latitude 7974→2969, longitude 10200→5746; **`correct_ratio 1.0` both** — every oracle-confirmed coordinate preserved, only confident-wrong ones demoted |
| **Binary integration** (real GeoY gittables file) | flag OFF → GeoY=latitude; flag ON → GeoY=`decimal_number`, tag `coord_header_veto:geoy`. Default unchanged. |
| **fmt + clippy -D warnings + 642 lib tests** | green |

The gate is the load-bearing result: the rule demotes 11,267 of v19's 18,174
coordinate predictions corpus-wide, and the blocking gate confirms **none of the
demoted columns were oracle-confirmed coordinates** — they were exactly the
confident-wrong over-emissions. This is a clean precision win with no cross-type
collateral.

## The one judgment call for you

The rule demotes **projected/axis headers** — `geox`/`geoy` (~280 cols), `x`,
`dec` (declination). These are *arguably* coordinates (British-Grid eastings/
northings, celestial declination), though calling them WGS84 `latitude` is itself
dubious, and the gate confirms none were oracle-backed. **If you want to preserve
them as coordinates, it's a one-line change** — add `geox`/`geoy`/`x`/`y` to the
corroboration token set in `header_corroborates_coordinate`. Otherwise they ship
as `decimal_number`. This is exactly what the human gold-set review
(`rare_type_gold_review.py`) should settle.

## Ship path (your call in the morning)

1. **Read + accept choice 0094** (it amends settled decision 0048 — your call).
2. Decide the geo-axis question above (preserve or demote).
3. **Flip the flag default-on:** change `coord_header_veto_enabled()` to return
   `true` (one line), or set the env in the release. I left it off so nothing
   ships without you.
4. *(Official record, optional — ~1.5h)* run the real binary through
   `gittables_corpus_pass.py` on the 33k sample + re-run the gate. The SQL
   candidate I gated is a faithful equivalent (rule is deterministic; the GeoY
   binary spot-check confirms parity), so this is confirmation, not discovery.
5. **Release** as a Sharpen patch on v19 — no model swap, same change-class as
   0.6.24.

## What I did NOT do (left for you)

- Did not promote / swap `models/default`, did not cut a binary release (H10).
- Did not enable the flag by default.
- Did not mark 0094 `accepted` (it amends a settled decision — your read).

## Reproduce

```bash
# scoreboard dry-run (before/after FP + recall)
python3 scripts/validate_coord_veto.py
# regenerate candidate + run the blocking gate
duckdb -init /dev/null -c "<candidate SQL in this dir's git log>"   # or rerun the pass
python3 scripts/corpus_honest_gate.py \
  --baseline output/ydf-validation-gate/v19_gated.parquet \
  --candidate output/eval-ceiling-diagnosis/coord_veto_candidate.parquet \
  --sample output/corpus-honest-gate/stratified_sample.files.txt --label coord-veto \
  --out-dir output/eval-ceiling-diagnosis
```

Gate report: `output/eval-ceiling-diagnosis/gate_coord-veto.json` (verdict GO).
Candidate parquet is gitignored (279 MB, real values); regenerable.
