# v24 numeric-precision — overnight handoff (2026-06-03)

Ran autonomously while you slept. Did all the **reversible data prep**; stopped
short of training a model or repointing any symlink. One decision blocks the
training step — flagged below.

## Done (committed + pushed)

| AC | What | Evidence |
|----|------|----------|
| ac-00 | Default resolved (v19 ships; CLAUDE.md fixed) + four clusters re-baselined on v19 | `rebaseline_v19.md` — all four KEEP: utc 0.837, bool 0.993, url 1.000, int→dec 1.000. None drop. |
| ac-01 | Numeric-target hard negatives extracted, safety≥0.80, MADR 0056 dedup, **zero categorical** | `hard_negatives.parquet` (78,612), `hard_negatives_summary.md` |
| ac-02 (part) | Pre-train Sense-distribution baseline (v19) + blend audit gate | `sense_dist_v19.json`, `v24_blend_manifest.json` — AUDIT PASSED, 251,270 rows |

**Headline:** the four false positives are real on what ships. v19 (not just the
v22 the diagnostic recorded) mistypes these columns 84–100% of the time, so the
retrain target is genuine. The hard-negative pile is numeric-only by
construction — the categorical explosion that killed v23 cannot originate here.

**v19 pre-train guard rails** (from `sense_dist_v19.json`, 800 files / 11,555 cols):
- categorical = **0.99%** — post-train v24 must stay within a small band of this.
- geography: city 63, region 25, country 16 — must not regress.
- (Snapshot is a sample; widen with `--files` if you want tighter bands.)

## The one decision blocking ac-03

**Distilled cap per type.** The spec is internally inconsistent:
- ac-02 text says `distilled cap <= 600/type`
- ac-03 text says `~1200 cols/type`
- v23 actually shipped `--distilled-cap 1800` (`overnight_v23_precision.sh:223`)

Pick one before the FTMB build. I did **not** guess — it materially shapes the
training distribution and a wrong pick wastes the 3-seed run. My read: match v23
(1800) so v24 is a clean data-composition delta over v23's recipe, not a
confounded cap change. But that's your call.

## Remaining steps (after the cap decision)

The blend is already built: `output/distillation-v24/sherlock_distilled_v24.csv.gz`
(regenerate with `eval/gittables/.venv/bin/python scripts/build_v24_distilled.py`).
Use `scripts/overnight_v23_precision.sh` as the template — the v24 deltas are:
inputs → `output/v24-numeric-precision/hard_negatives.parquet` and
`output/distillation-v24/...`; outputs → `models/sherlock-v24-numeric-relu-s{42,43,44}`.

1. **FTMB prepare** (`prepare_multibranch_data.py`, `--distilled <v24 blend>
   --distilled-cap <CAP>`) — this is where the ac-02 FTMB audit (VALID_DIM=240,
   per-type volume) actually runs. Not yet done.
2. **Train** 3 seeds (50 epochs, Metal). `discrete.categorical` is not a target.
3. **ac-04 eval** — corpus pass under v24, then:
   - per-cluster FP drop (adapt `compute_v23_per_cluster_fp_rate.py`);
   - post-train snapshot: `FINETYPE_MODEL=models/sherlock-v24-numeric-relu-s44
     scripts/snapshot_sense_distribution.py --label v24 --files 800 --seed 42`
     then diff `sense_dist_v24.json` vs `sense_dist_v19.json` — categorical must
     not explode, geography must not regress.
4. **ac-05** — `scripts/roundtrip_metrics.sh earthquakes_2024.csv` and
   `roundtrip_ab.py` under v24: non_trivial_pct must stay ≥ 0.80.

## Not touched (deliberately)
- No model trained. No `models/default` repoint. No promotion.
- The earthquake blog round-trip stays decoupled (its failures are LOW-safety,
  out of v24 scope — spec `2026-06-03-earthquake-roundtrip-precision`).
- The CharCNN-vs-multi-branch scaling experiment is held (topology memory
  `charcnn-vs-multibranch-data-scaling`); it's bigger than v24 and v24 is safe
  under either architecture.
