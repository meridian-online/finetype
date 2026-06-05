# Proxy pre-check — retro-calibration (ac-02 / ac-03)

The question: can a cheap single-seed / few-epoch proxy, run BEFORE the
overnight retrain, tell us the candidate blend will explode an untargeted
boundary? Tested against the two blends we already paid the overnight cost on
and know the answer for.

## Protocol (ac-02)

`scripts/proxy_pretrain.sh` trains ONE seed for 10 epochs on the candidate's
already-built FTMB (same `train-multi-branch` invocation the overnight run uses,
not forked — just 3 seeds → 1, 50 epochs → 10), snapshots the proxy model's
Sense distribution on the fixed 13,533-column file list, and runs the calibrated
`drift_report.py` gate vs the pre-train baseline. GO/NO-GO is the exit code.

Wall-clock — proxy vs the full overnight run it gates:

| | proxy (1 seed × 10 ep) | full overnight | proxy fraction |
|--|--:|--:|--:|
| v24 | 52 min | 828 min (blend + 3 seeds × ~245 min) | **6.3%** |
| v23 | 60 min | ~comparable (~800 min) | **~7.5%** |

Both well under the ≤ ~20% target. The proxy also SKIPS the blend / FTMB build
(hours) by reusing the existing FTMB, so the real-world saving is larger.
10 epochs surfaced both explosions far above the band (4.69× and 6.19× vs a 3.0×
gate), so the epoch floor likely has headroom below 10 — but 10 is the validated
default; lowering it is unmeasured.

## Retro-calibration (ac-03) — both are true positives

| arm | known full-run signal | proxy signal | rank | verdict |
|-----|----------------------|--------------|------|---------|
| **v23** (v22 → blend) | categorical +6.13pp / 4.10× | categorical **+7.30pp / 4.69×** | #1 mover, both | NO-GO = NO-GO ✓ |
| **v24** (v19 → blend) | coordinate.latitude +0.64pp / 5.65× | coordinate.latitude **+0.71pp / 6.19×** | top flag, both | NO-GO = NO-GO ✓ |

Three things the proxy gets right:

1. **Ranking preserved.** The label that explodes in the full run is the same
   label the proxy flags first — categorical for v23, coordinate.latitude for
   v24. The pre-filter points at the right boundary, not just "something moved".
2. **Direction preserved, magnitude slightly OVER-stated.** Both proxy signals
   are a touch larger than the full run (7.30 vs 6.13pp; 0.71 vs 0.64pp). For a
   pre-filter this is the safe error: it never under-states a real explosion
   into a false GO.
3. **The v23 geography regression shows too.** The proxy's v23 report has
   city −0.48pp and region −0.32pp — the "geography eaten by categorical"
   signature that closed v23 Failed — even though `--direction up` doesn't flag
   them. The collateral is visible in the ranked context, not hidden.

## Honest scope

The proxy is noisier than a converged model and can add an EXTRA flag from
under-convergence: the v24 proxy also tripped `datetime.date.compact_ymd` (23×),
which the full run did not. This does not change the v24 decision (already NO-GO
on latitude), and it errs conservative (extra NO-GO, never a false GO). So:
a proxy **GO** is a strong green light; a proxy **NO-GO** should be read for
WHICH label tripped — a targeted-neighbour boundary (real drift, abort) vs a
single small-base label the full run wouldn't converge to (under-convergence
noise). The verdict held on both paid-for cases; the named-label discipline is
how you use it on a fresh blend.

## Reproduce

```
scripts/proxy_pretrain.sh --name v24proxy \
  --ftmb output/multibranch-training/v24-numeric-blend.ftmb \
  --baseline output/destination-drift-precheck/sense_dist_v19fx_s42.json
scripts/proxy_pretrain.sh --name v23proxy \
  --ftmb output/multibranch-training/v23-precision-blend.ftmb \
  --baseline output/destination-drift-precheck/sense_dist_v22fx.json
```

Evidence: `proxy_{v23,v24}.log`, `sense_dist_v{23,24}proxy.json`,
`proxy_drift_v{23,24}proxy.json`.
