# Composition-aware over_emit (author-accepted 2026-06-12)

Sibling of the 0.6.24 oracle-aware band refinement
(`oracle_aware_bands.md`), closing the second false-alarm blind spot: the
`over_emit` band read RAW marginal growth, so two stacked honest fixes in
the same direction (0.6.28 veto-fallback 2.61× + R32 word-vocab override
+0.54×) summed past the 3.0× line and produced a NO-GO on a candidate whose
oracle-refuted move count (1,154) was IDENTICAL to the previously-passed
baseline — i.e. zero new detectable errors. Stacking would have tripped
every future categorical-direction fix forever, since the band measures
cumulative drift against the fixed v19 sense baseline.

## The amendment

`over_emit` now nets oracle-CONFIRMED correct growth out of the ratio:

    adj_ratio = (est_cand_marginal − max(0, cand_correct − base_correct)) / v19_marginal

Growth the oracle confirms as correct cannot trip the band; relocation
(oracle-refuted or oracle-blind growth) still counts in full. `oracle_fp`
and `collapse` are unchanged. The raw `ratio` stays in the report alongside
`adj_ratio` for legibility.

## Verdict preservation (re-run 2026-06-12, amended band)

| candidate | expected | amended gate | mechanism |
|---|---|---|---|
| v19 vs itself | GO | **GO** | zero moves |
| latdec | NO-GO | **NO-GO** | latitude `oracle_fp` — untouched by amendment |
| veto-fallback (0.6.28, shipped) | GO | **GO** | confirmed/silent growth |
| R32 broad (round 1) | NO-GO | **NO-GO** | 47,785 oracle-refuted moves — `oracle_fp`; netting CONFIRMED growth does not bless refuted growth |
| R32 word-only (round 2) | NO-GO under raw band | **GO** | +60.6k confirmed, +0 net refuted; raw 3.152× → adj 2.26× |

v22 and v23 (parquets cleaned in fossil-cleanup) are preserved analytically
from the ac-03 record: v22 fires `oracle_fp` on region/country (untouched)
and its city 3.06× growth was a correctness collapse (no confirmed growth
to net out); v23's categorical 8.68× / latitude 8.75× growth was
oracle-refuted absorption (the 48k city columns the oracle confirms as
city), so `adj_ratio ≈ ratio` and `over_emit` still fires.

The round-1 broad-R32 row is the load-bearing negative control: the
amendment passes honest stacked growth while the SAME RULE's
over-broad variant — actual relocation at scale — stays caught.

## Standing caveat

Stacking pressure on ratio bands is a symptom of the fixed sense baseline
ageing under successive shipped fixes. A baseline refresh (full corpus pass
on the current default) resets all ratios to 1.0 and remains worth doing
when the cumulative drift becomes hard to read; the composition-aware band
makes it less urgent, not unnecessary.
